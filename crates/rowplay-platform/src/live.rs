// SPDX-License-Identifier: GPL-3.0-or-later
//! Live-mode poll: page-1 only over the existing Concept2 client and cache.
//!
//! A live poll must **never** set or clear [`WorkoutCache::set_fully_synced`]:
//! that flag means "every page back to the beginning has been fetched." Page-1
//! polling is not a full history walk. Getting this wrong in the setting
//! direction permanently loses older history on the next incremental sync;
//! wrong in the clearing direction re-downloads everything on every restart.
//!
//! "New" is identified by result **id**, not by count or position (web
//! `knownIds`).

use std::collections::BTreeSet;

use rowplay_core::privacy::redact;

use crate::concept2::{Concept2Client, Concept2Error};
use crate::logging::logger;
use crate::workout_cache::WorkoutCache;

/// Outcome of one live poll against page 1.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LivePollResult {
    /// Summaries returned by page 1.
    pub fetched_count: usize,
    /// Details newly written to the cache.
    pub saved_count: usize,
    /// Page-1 ids already present in the cache (or already known).
    pub skipped_count: usize,
    /// Ids that were unknown before this poll and were saved.
    pub new_ids: Vec<i64>,
}

/// Typed live-poll failures the UI can branch on (auth, rate limit, other).
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum LivePollError {
    /// Token rejected (401) — live mode must stop and surface signed-out.
    #[error("Concept2 rejected the token")]
    Unauthorized,
    /// Rate limited (429).
    #[error("Concept2 rate limit reached")]
    RateLimited {
        /// Seconds from `Retry-After`, when present.
        retry_after_secs: Option<u64>,
    },
    /// Cache I/O failure.
    #[error("live poll cache failed: {0}")]
    Cache(String),
    /// Any other client / transport failure.
    #[error("live poll failed: {0}")]
    Client(String),
}

/// Page size the live poll asks for (web `listRecentWorkouts(25)`).
pub const LIVE_PER_PAGE: u32 = 25;

/// Fetch the newest results page, import unknown ids by id, leave the
/// full-history checkpoint untouched.
///
/// `known_ids` is the set of result ids the session already treats as seen
/// (typically the library plus any ids discovered earlier in this live
/// session). Ids already in the cache via [`WorkoutCache::list_workouts`]
/// are also treated as known — a corrected older result with the same id is
/// not "new."
pub fn poll_recent(
    client: &dyn Concept2Client,
    cache: &dyn WorkoutCache,
    known_ids: &BTreeSet<i64>,
) -> Result<LivePollResult, LivePollError> {
    let log = logger("live-poll");

    cache.migrate().map_err(|error| {
        let message = redact(&error.to_string());
        log.warn("cache migration failed", &[&message]);
        LivePollError::Cache(message)
    })?;

    // Snapshot the checkpoint *before* any network work so the invariant
    // test can prove we neither set nor clear it.
    let checkpoint_before = cache.is_fully_synced().map_err(|error| {
        let message = redact(&error.to_string());
        LivePollError::Cache(message)
    })?;

    let page = client
        .list_results(1, LIVE_PER_PAGE)
        .map_err(|error| map_client_error(&log, error))?;

    let cached_ids: BTreeSet<i64> = cache
        .list_workouts()
        .map_err(|error| {
            let message = redact(&error.to_string());
            LivePollError::Cache(message)
        })?
        .into_iter()
        .map(|w| w.id)
        .collect();

    let mut result = LivePollResult {
        fetched_count: page.workouts.len(),
        ..LivePollResult::default()
    };

    for summary in &page.workouts {
        let id = summary.id;
        if known_ids.contains(&id) || cached_ids.contains(&id) {
            result.skipped_count += 1;
            continue;
        }
        match client.result_detail(id) {
            Ok(detail) => match cache.save_details(std::slice::from_ref(&detail)) {
                Ok(_) => {
                    result.saved_count += 1;
                    result.new_ids.push(id);
                }
                Err(error) => {
                    let message = redact(&error.to_string());
                    log.warn(
                        "cache save failed",
                        &[&id as &dyn std::fmt::Display, &message],
                    );
                    return Err(LivePollError::Cache(message));
                }
            },
            Err(error) => {
                return Err(map_client_error(&log, error));
            }
        }
    }

    let checkpoint_after = cache.is_fully_synced().map_err(|error| {
        let message = redact(&error.to_string());
        LivePollError::Cache(message)
    })?;
    debug_assert_eq!(
        checkpoint_before, checkpoint_after,
        "live poll must not touch fully_synced"
    );
    // Defence in depth: if somehow the flag moved, restore it. A live poll
    // must never be the writer of this checkpoint.
    if checkpoint_after != checkpoint_before {
        let _ = cache.set_fully_synced(checkpoint_before);
        log.warn(
            "live poll restored a corrupted fully_synced checkpoint",
            &[],
        );
    }

    Ok(result)
}

fn map_client_error(
    log: &rowplay_core::privacy::PrivacySafeLogger<'static>,
    error: Concept2Error,
) -> LivePollError {
    let message = redact(&error.to_string());
    log.warn("live poll client error", &[&message]);
    match error {
        Concept2Error::Unauthorized => LivePollError::Unauthorized,
        Concept2Error::RateLimited { retry_after_secs } => {
            LivePollError::RateLimited { retry_after_secs }
        }
        other => LivePollError::Client(redact(&other.to_string())),
    }
}

/// Assert the live-poll checkpoint invariant directly (for tests and the
/// bite self-test).
#[must_use]
pub fn checkpoint_untouched(before: bool, after: bool) -> bool {
    before == after
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use rowplay_core::models::{Sport, Workout, WorkoutDetail};

    use crate::concept2::{Concept2Client, Concept2Error, ResultsPage};
    use crate::workout_cache::{InMemoryWorkoutCache, WorkoutCache};

    fn detail(id: i64, date: &str) -> WorkoutDetail {
        WorkoutDetail {
            workout: Workout::new(id, date, Sport::Rower, 2000.0, 400.0, 100.0),
            strokes: Vec::new(),
            splits: Vec::new(),
        }
    }

    /// Scripted client for the live-poll fake-API matrix.
    struct ScriptedClient {
        list: Mutex<Vec<Result<ResultsPage, Concept2Error>>>,
        details: BTreeMap<i64, WorkoutDetail>,
        calls: Mutex<Vec<String>>,
    }

    impl ScriptedClient {
        fn new(list: Vec<Result<ResultsPage, Concept2Error>>, details: Vec<WorkoutDetail>) -> Self {
            ScriptedClient {
                list: Mutex::new(list),
                details: details.into_iter().map(|d| (d.id(), d)).collect(),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    impl Concept2Client for ScriptedClient {
        fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("list:{page}:{per_page}"));
            let mut queue = self.list.lock().expect("list");
            if queue.is_empty() {
                return Ok(ResultsPage::default());
            }
            queue.remove(0)
        }

        fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("detail:{id}"));
            self.details
                .get(&id)
                .cloned()
                .ok_or(Concept2Error::NotFound(id))
        }

        fn strokes(
            &self,
            id: i64,
            _sport: Sport,
        ) -> Result<Vec<rowplay_core::models::Stroke>, Concept2Error> {
            self.calls
                .lock()
                .expect("calls")
                .push(format!("strokes:{id}"));
            Ok(Vec::new())
        }
    }

    fn page(workouts: Vec<Workout>) -> ResultsPage {
        ResultsPage {
            workouts,
            page: 1,
            total_pages: 1,
        }
    }

    #[test]
    fn checkpoint_invariant_holds_when_fully_synced_is_true() {
        let cache = InMemoryWorkoutCache::default();
        cache.set_fully_synced(true).unwrap();
        let existing = detail(1, "2026-01-01 12:00:00");
        cache.save_details(std::slice::from_ref(&existing)).unwrap();

        let client = ScriptedClient::new(
            vec![Ok(page(vec![existing.workout.clone()]))],
            vec![existing],
        );
        let before = cache.is_fully_synced().unwrap();
        let result = poll_recent(&client, &cache, &BTreeSet::new()).unwrap();
        let after = cache.is_fully_synced().unwrap();

        assert!(checkpoint_untouched(before, after));
        assert!(before && after, "true must stay true");
        assert_eq!(result.saved_count, 0);
        assert_eq!(result.skipped_count, 1);
        assert_eq!(client.calls(), vec!["list:1:25"]);
    }

    #[test]
    fn checkpoint_invariant_holds_when_fully_synced_is_false() {
        let cache = InMemoryWorkoutCache::default();
        assert!(!cache.is_fully_synced().unwrap());

        let fresh = detail(42, "2026-09-01 08:00:00");
        let client = ScriptedClient::new(vec![Ok(page(vec![fresh.workout.clone()]))], vec![fresh]);
        let before = cache.is_fully_synced().unwrap();
        let result = poll_recent(&client, &cache, &BTreeSet::new()).unwrap();
        let after = cache.is_fully_synced().unwrap();

        assert!(checkpoint_untouched(before, after));
        assert!(!before && !after, "false must stay false");
        assert_eq!(result.new_ids, vec![42]);
        assert_eq!(result.saved_count, 1);
    }

    #[test]
    fn checkpoint_invariant_assert_bites_on_corruption() {
        let caught = std::panic::catch_unwind(|| {
            assert!(
                checkpoint_untouched(true, false),
                "checkpoint must not flip"
            );
        });
        assert!(caught.is_err(), "a flipped checkpoint must fail the assert");
        assert!(checkpoint_untouched(true, true));
        assert!(checkpoint_untouched(false, false));
    }

    #[test]
    fn unchanged_page_skips_everything() {
        let cache = InMemoryWorkoutCache::default();
        let a = detail(1, "2026-01-02 10:00:00");
        let b = detail(2, "2026-01-01 10:00:00");
        cache.save_details(&[a.clone(), b.clone()]).unwrap();
        cache.set_fully_synced(true).unwrap();

        let client = ScriptedClient::new(
            vec![Ok(page(vec![a.workout.clone(), b.workout.clone()]))],
            vec![a, b],
        );
        let result = poll_recent(&client, &cache, &BTreeSet::new()).unwrap();
        assert_eq!(result.saved_count, 0);
        assert_eq!(result.skipped_count, 2);
        assert!(result.new_ids.is_empty());
        assert!(cache.is_fully_synced().unwrap());
        // No detail fetches when everything is known.
        assert_eq!(client.calls(), vec!["list:1:25"]);
    }

    #[test]
    fn one_new_result_is_imported_by_id() {
        let cache = InMemoryWorkoutCache::default();
        let old = detail(1, "2026-01-01 10:00:00");
        cache.save_details(std::slice::from_ref(&old)).unwrap();

        let fresh = detail(99, "2026-09-18 18:00:00");
        let client = ScriptedClient::new(
            vec![Ok(page(vec![fresh.workout.clone(), old.workout.clone()]))],
            vec![fresh, old],
        );
        let result = poll_recent(&client, &cache, &BTreeSet::new()).unwrap();
        assert_eq!(result.new_ids, vec![99]);
        assert_eq!(result.saved_count, 1);
        assert_eq!(result.skipped_count, 1);
        let ids: BTreeSet<i64> = cache
            .list_workouts()
            .unwrap()
            .into_iter()
            .map(|w| w.id)
            .collect();
        assert!(ids.contains(&99));
        assert!(ids.contains(&1));
    }

    #[test]
    fn corrected_older_result_same_id_is_not_new() {
        // A corrected/deleted logbook row can change the page count without
        // introducing a new id. Count-based checks would mis-fire; id wins.
        let cache = InMemoryWorkoutCache::default();
        let mut corrected = detail(7, "2026-01-01 10:00:00");
        corrected.workout.distance = 2500.0; // "correction"
        cache
            .save_details(&[detail(7, "2026-01-01 10:00:00")])
            .unwrap();

        let client = ScriptedClient::new(
            vec![Ok(page(vec![corrected.workout.clone()]))],
            vec![corrected],
        );
        let known = BTreeSet::from([7]);
        let result = poll_recent(&client, &cache, &known).unwrap();
        assert!(result.new_ids.is_empty());
        assert_eq!(result.skipped_count, 1);
        assert_eq!(client.calls(), vec!["list:1:25"]);
    }

    #[test]
    fn unauthorized_surfaces_without_touching_checkpoint() {
        let cache = InMemoryWorkoutCache::default();
        cache.set_fully_synced(true).unwrap();
        let client = ScriptedClient::new(vec![Err(Concept2Error::Unauthorized)], vec![]);
        let before = cache.is_fully_synced().unwrap();
        let err = poll_recent(&client, &cache, &BTreeSet::new()).unwrap_err();
        assert_eq!(err, LivePollError::Unauthorized);
        assert_eq!(cache.is_fully_synced().unwrap(), before);
    }

    #[test]
    fn rate_limited_with_retry_after_surfaces() {
        let cache = InMemoryWorkoutCache::default();
        let client = ScriptedClient::new(
            vec![Err(Concept2Error::RateLimited {
                retry_after_secs: Some(42),
            })],
            vec![],
        );
        let err = poll_recent(&client, &cache, &BTreeSet::new()).unwrap_err();
        assert_eq!(
            err,
            LivePollError::RateLimited {
                retry_after_secs: Some(42)
            }
        );
        assert!(!cache.is_fully_synced().unwrap());
    }

    #[test]
    fn connection_failure_surfaces_without_touching_checkpoint() {
        let cache = InMemoryWorkoutCache::default();
        cache.set_fully_synced(true).unwrap();
        let client = ScriptedClient::new(
            vec![Err(Concept2Error::Transport("connection reset".into()))],
            vec![],
        );
        let before = cache.is_fully_synced().unwrap();
        let err = poll_recent(&client, &cache, &BTreeSet::new()).unwrap_err();
        assert!(matches!(err, LivePollError::Client(_)));
        assert_eq!(cache.is_fully_synced().unwrap(), before);
    }

    #[test]
    fn live_poll_asks_for_page_one_with_twenty_five() {
        let cache = InMemoryWorkoutCache::default();
        let client = ScriptedClient::new(vec![Ok(page(vec![]))], vec![]);
        poll_recent(&client, &cache, &BTreeSet::new()).unwrap();
        assert_eq!(client.calls(), vec!["list:1:25"]);
    }
}
