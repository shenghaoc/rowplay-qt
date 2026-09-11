// SPDX-License-Identifier: GPL-3.0-or-later
//! Concept2 logbook sync.
//!
//! Port of rowplay-studio's `Sync/WorkoutSyncCoordinator.swift`,
//! `Sync/SyncStateTracker.swift`, `Sync/WorkoutSyncResult.swift` and
//! `Sync/WorkoutSyncError.swift`.
//!
//! The coordinator is synchronous: Phase 4 runs it on a worker thread, so
//! cancellation is an [`AtomicBool`] checked between requests and progress is a
//! callback. It depends only on the [`Concept2Client`] and [`WorkoutCache`]
//! traits — it owns no token, opens no socket and touches no SQL.
//!
//! Failure policy, exactly as Studio implements it:
//!
//! - the cache schema is migrated first; a migration failure is fatal;
//! - paging through the summaries is fatal on any failure;
//! - a per-workout detail fetch or cache save failure is counted and the sync
//!   continues, **except** for 401/403/429, which abort so the client does not
//!   hammer the API with requests that cannot succeed.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use rowplay_core::models::Workout;
use rowplay_core::privacy::{PrivacySafeLogger, redact};

use crate::concept2::Concept2Client;
use crate::logging::logger;
use crate::workout_cache::WorkoutCache;

/// Results requested per API page: the Concept2 maximum, as the web app uses.
pub const DEFAULT_PER_PAGE: u32 = 250;

/// What a completed sync did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkoutSyncResult {
    /// Summaries fetched from the Logbook (across every page).
    pub fetched_count: usize,
    /// Details fetched **and** written to the cache.
    pub saved_count: usize,
    /// Details that could not be fetched or saved.
    pub failed_count: usize,
    /// When the sync started.
    pub started_at: DateTime<Utc>,
    /// When the sync stopped.
    pub finished_at: DateTime<Utc>,
}

/// How far a running sync has got.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SyncProgress {
    /// Details processed so far (saved or failed).
    pub completed: usize,
    /// Details the summary pass found.
    pub total: usize,
}

/// A sync that stopped, either because it finished or because it was cancelled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOutcome {
    /// Counts and timestamps for the work actually done.
    pub result: WorkoutSyncResult,
    /// Whether the caller's cancel flag stopped the sync early.
    pub cancelled: bool,
}

/// Sync failures. Descriptions are privacy-safe: no tokens, headers or payloads.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WorkoutSyncError {
    /// The Concept2 client failed (network, auth, rate limiting, decode).
    #[error("Sync client failed: {0}")]
    ClientFailed(String),
    /// The workout cache failed (open, migration, query).
    #[error("Sync cache failed: {0}")]
    CacheFailed(String),
    /// A raw payload could not be mapped onto the domain model.
    #[error("Sync mapping failed: {0}")]
    MappingFailed(String),
}

/// Fetches every workout summary and detail from the Logbook into the cache.
pub struct WorkoutSyncCoordinator<'a> {
    client: &'a dyn Concept2Client,
    cache: &'a dyn WorkoutCache,
    per_page: u32,
    logger: PrivacySafeLogger<'static>,
}

impl std::fmt::Debug for WorkoutSyncCoordinator<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkoutSyncCoordinator")
            .field("per_page", &self.per_page)
            .finish_non_exhaustive()
    }
}

impl<'a> WorkoutSyncCoordinator<'a> {
    /// A coordinator using [`DEFAULT_PER_PAGE`].
    #[must_use]
    pub fn new(client: &'a dyn Concept2Client, cache: &'a dyn WorkoutCache) -> Self {
        Self::with_per_page(client, cache, DEFAULT_PER_PAGE)
    }

    /// A coordinator asking for `per_page` results per page.
    #[must_use]
    pub fn with_per_page(
        client: &'a dyn Concept2Client,
        cache: &'a dyn WorkoutCache,
        per_page: u32,
    ) -> Self {
        WorkoutSyncCoordinator {
            client,
            cache,
            per_page: per_page.max(1),
            logger: logger("sync-coordinator"),
        }
    }

    /// Sync everything, without cancellation or progress reporting.
    pub fn sync_all(&self) -> Result<WorkoutSyncResult, WorkoutSyncError> {
        let cancel = AtomicBool::new(false);
        self.sync_with(&cancel, &mut |_| {})
            .map(|outcome| outcome.result)
    }

    /// Sync everything, stopping when `cancel` is set.
    ///
    /// `progress` is called once per processed detail.
    pub fn sync_with(
        &self,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(SyncProgress),
    ) -> Result<SyncOutcome, WorkoutSyncError> {
        let started_at = Utc::now();

        // 0. The schema must exist before anything is written.
        self.cache.migrate().map_err(|error| {
            let message = redact(&error.to_string());
            self.logger.warn("cache migration failed", &[&message]);
            WorkoutSyncError::CacheFailed(message)
        })?;

        // 1. Summaries, page by page.
        let summaries = self.fetch_all_summaries(cancel)?;
        let total = summaries.len();
        if cancel.load(Ordering::Relaxed) {
            return Ok(outcome(started_at, total, 0, 0, true));
        }

        // 2. Details, one request at a time.
        let mut saved_count = 0;
        let mut failed_count = 0;
        for (completed, summary) in summaries.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Ok(outcome(started_at, total, saved_count, failed_count, true));
            }

            match self.client.result_detail(summary.id) {
                Ok(detail) => match self.cache.save_details(std::slice::from_ref(&detail)) {
                    Ok(_) => saved_count += 1,
                    Err(error) => {
                        let message = redact(&error.to_string());
                        self.logger.warn(
                            "cache save failed",
                            &[&summary.id as &dyn std::fmt::Display, &message],
                        );
                        failed_count += 1;
                    }
                },
                Err(error) => {
                    let message = redact(&error.to_string());
                    self.logger.warn(
                        "detail fetch failed",
                        &[&summary.id as &dyn std::fmt::Display, &message],
                    );
                    failed_count += 1;
                    if error.should_abort_sync() {
                        progress(SyncProgress {
                            completed: completed + 1,
                            total,
                        });
                        return Err(WorkoutSyncError::ClientFailed(message));
                    }
                }
            }

            progress(SyncProgress {
                completed: completed + 1,
                total,
            });
        }

        Ok(outcome(started_at, total, saved_count, failed_count, false))
    }

    /// Page through every summary, following the page count the API reports.
    ///
    /// The page count is monotone (the web app takes the running maximum), so a
    /// server that reports fewer pages on a later response cannot cut the walk
    /// short.
    fn fetch_all_summaries(&self, cancel: &AtomicBool) -> Result<Vec<Workout>, WorkoutSyncError> {
        let mut summaries = Vec::new();
        let mut page = 1;
        let mut total_pages = 1;
        loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let result = self
                .client
                .list_results(page, self.per_page)
                .map_err(|error| {
                    let message = redact(&error.to_string());
                    self.logger.error("summary fetch failed", &[&message]);
                    WorkoutSyncError::ClientFailed(message)
                })?;
            summaries.extend(result.workouts);
            total_pages = total_pages.max(result.total_pages.max(1));
            page += 1;
            if page > total_pages {
                break;
            }
        }
        Ok(summaries)
    }
}

/// Build an outcome for the work done so far.
fn outcome(
    started_at: DateTime<Utc>,
    fetched_count: usize,
    saved_count: usize,
    failed_count: usize,
    cancelled: bool,
) -> SyncOutcome {
    SyncOutcome {
        result: WorkoutSyncResult {
            fetched_count,
            saved_count,
            failed_count,
            started_at,
            finished_at: Utc::now(),
        },
        cancelled,
    }
}

/// Snapshot of the sync lifecycle (web `SyncState`, Studio `SyncState`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SyncState {
    /// When the last successful sync finished.
    pub last_sync_date: Option<DateTime<Utc>>,
    /// How many workouts the cache holds.
    pub total_workouts: usize,
    /// Whether a sync is running.
    pub in_progress: bool,
    /// Redacted message from the last failed sync.
    pub last_error: Option<String>,
    /// When that failure happened.
    pub last_error_date: Option<DateTime<Utc>>,
}

/// Tracks the sync lifecycle for the UI.
///
/// Transitions: idle → syncing (`sync_started`), syncing → complete
/// (`sync_completed`), syncing → error (`sync_failed`), error → syncing (a
/// retry calls `sync_started` again, which clears the error).
pub struct SyncStateTracker<'a> {
    cache: &'a dyn WorkoutCache,
    state: Mutex<SyncState>,
    logger: PrivacySafeLogger<'static>,
}

impl std::fmt::Debug for SyncStateTracker<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SyncStateTracker")
            .field("state", &self.state())
            .finish_non_exhaustive()
    }
}

impl<'a> SyncStateTracker<'a> {
    /// A tracker reading its workout count from `cache`.
    #[must_use]
    pub fn new(cache: &'a dyn WorkoutCache) -> Self {
        SyncStateTracker {
            cache,
            state: Mutex::new(SyncState::default()),
            logger: logger("sync"),
        }
    }

    /// The current state.
    #[must_use]
    pub fn state(&self) -> SyncState {
        self.state.lock().expect("sync state lock").clone()
    }

    /// Re-read the cached workout count.
    ///
    /// A cache failure is logged and leaves the previous count in place: the
    /// count is a display hint, not a reason to break the sync state machine.
    pub fn refresh_workout_count(&self) {
        match self.cache.list_workouts() {
            Ok(workouts) => {
                self.state.lock().expect("sync state lock").total_workouts = workouts.len();
            }
            Err(error) => {
                let message = redact(&error.to_string());
                self.logger
                    .warn("could not count cached workouts", &[&message]);
            }
        }
    }

    /// Mark a sync as running and clear the previous error.
    pub fn sync_started(&self) {
        let mut state = self.state.lock().expect("sync state lock");
        state.in_progress = true;
        state.last_error = None;
        state.last_error_date = None;
    }

    /// Mark a sync as successfully finished and refresh the count.
    pub fn sync_completed(&self) {
        {
            let mut state = self.state.lock().expect("sync state lock");
            state.in_progress = false;
            state.last_sync_date = Some(Utc::now());
            state.last_error = None;
            state.last_error_date = None;
        }
        self.refresh_workout_count();
    }

    /// Record a failure; a partial sync still refreshes the count.
    pub fn sync_failed(&self, error: &WorkoutSyncError) {
        let message = redact(&error.to_string());
        {
            let mut state = self.state.lock().expect("sync state lock");
            state.in_progress = false;
            state.last_error = Some(message.clone());
            state.last_error_date = Some(Utc::now());
        }
        self.logger.error("sync failed", &[&message]);
        self.refresh_workout_count();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concept2::{Concept2Error, ResultsPage};
    use crate::token_store::SecretToken;
    use crate::workout_cache::{CacheError, InMemoryWorkoutCache};
    use rowplay_core::demo::demo_details;
    use rowplay_core::models::{Sport, WorkoutDetail};
    use std::collections::BTreeMap;

    /// A scripted client: fixed summaries, a detail per id, and per-id or
    /// global failures.
    struct TestClient {
        summaries: Vec<Workout>,
        details: BTreeMap<i64, WorkoutDetail>,
        detail_failures: BTreeMap<i64, Concept2Error>,
        summary_failure: Option<Concept2Error>,
        calls: Mutex<Vec<String>>,
    }

    impl TestClient {
        fn new(details: Vec<WorkoutDetail>) -> Self {
            let mut sorted = details;
            sorted.sort_by(|a, b| {
                b.workout
                    .date
                    .cmp(&a.workout.date)
                    .then_with(|| b.id().cmp(&a.id()))
            });
            TestClient {
                summaries: sorted.iter().map(WorkoutDetail::summary).collect(),
                details: sorted.into_iter().map(|d| (d.id(), d)).collect(),
                detail_failures: BTreeMap::new(),
                summary_failure: None,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn failing_detail(mut self, id: i64, error: Concept2Error) -> Self {
            self.detail_failures.insert(id, error);
            self
        }

        fn failing_summaries(mut self, error: Concept2Error) -> Self {
            self.summary_failure = Some(error);
            self
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls lock").clone()
        }
    }

    impl Concept2Client for TestClient {
        fn list_results(&self, page: u32, per_page: u32) -> Result<ResultsPage, Concept2Error> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(format!("list:{page}"));
            if let Some(error) = &self.summary_failure {
                return Err(error.clone());
            }
            if per_page == 0 {
                return Ok(ResultsPage {
                    workouts: Vec::new(),
                    page,
                    total_pages: page,
                });
            }
            let total_pages = self.summaries.len().div_ceil(per_page as usize).max(1) as u32;
            let start = (page.max(1) as usize - 1) * per_page as usize;
            Ok(ResultsPage {
                workouts: self
                    .summaries
                    .iter()
                    .skip(start)
                    .take(per_page as usize)
                    .cloned()
                    .collect(),
                page: page.max(1),
                total_pages,
            })
        }

        fn result_detail(&self, id: i64) -> Result<WorkoutDetail, Concept2Error> {
            self.calls
                .lock()
                .expect("calls lock")
                .push(format!("detail:{id}"));
            if let Some(error) = self.detail_failures.get(&id) {
                return Err(error.clone());
            }
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
                .expect("calls lock")
                .push(format!("strokes:{id}"));
            self.details
                .get(&id)
                .map(|detail| detail.strokes.clone())
                .ok_or(Concept2Error::NotFound(id))
        }
    }

    /// A cache that migrates and reads fine but refuses every write.
    struct UnwritableCache {
        inner: InMemoryWorkoutCache,
    }

    impl WorkoutCache for UnwritableCache {
        fn migrate(&self) -> Result<(), CacheError> {
            self.inner.migrate()
        }

        fn list_workouts(&self) -> Result<Vec<Workout>, CacheError> {
            self.inner.list_workouts()
        }

        fn details(&self, ids: &[i64]) -> Result<BTreeMap<i64, WorkoutDetail>, CacheError> {
            self.inner.details(ids)
        }

        fn save_details(&self, _details: &[WorkoutDetail]) -> Result<usize, CacheError> {
            Err(CacheError::Query("disk is full".to_owned()))
        }

        fn clear(&self) -> Result<(), CacheError> {
            self.inner.clear()
        }
    }

    /// A cache whose migration always fails.
    struct UnmigratableCache;

    impl WorkoutCache for UnmigratableCache {
        fn migrate(&self) -> Result<(), CacheError> {
            Err(CacheError::Migration("bad schema".to_owned()))
        }

        fn list_workouts(&self) -> Result<Vec<Workout>, CacheError> {
            Ok(Vec::new())
        }

        fn details(&self, _ids: &[i64]) -> Result<BTreeMap<i64, WorkoutDetail>, CacheError> {
            Ok(BTreeMap::new())
        }

        fn save_details(&self, _details: &[WorkoutDetail]) -> Result<usize, CacheError> {
            Ok(0)
        }

        fn clear(&self) -> Result<(), CacheError> {
            Ok(())
        }
    }

    fn demo_client() -> TestClient {
        TestClient::new(demo_details())
    }

    #[test]
    fn syncs_every_page_then_every_detail() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        let coordinator = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5);

        let mut seen = Vec::new();
        let outcome = coordinator
            .sync_with(&AtomicBool::new(false), &mut |progress| {
                seen.push(progress);
            })
            .unwrap();

        assert!(!outcome.cancelled);
        assert_eq!(outcome.result.fetched_count, 17);
        assert_eq!(outcome.result.saved_count, 17);
        assert_eq!(outcome.result.failed_count, 0);
        assert!(outcome.result.started_at <= outcome.result.finished_at);
        assert_eq!(cache.list_workouts().unwrap().len(), 17);
        assert!(cache.is_migrated());

        // Four pages of five, five, five, two, then one detail request each.
        assert_eq!(
            &client.calls()[..4],
            &["list:1", "list:2", "list:3", "list:4"]
        );
        assert_eq!(client.calls().len(), 4 + 17);
        assert!(
            client.calls()[4..]
                .iter()
                .all(|call| call.starts_with("detail:"))
        );

        // Progress runs to completion, one entry per detail.
        assert_eq!(seen.len(), 17);
        assert_eq!(
            seen[0],
            SyncProgress {
                completed: 1,
                total: 17
            }
        );
        assert_eq!(
            seen[16],
            SyncProgress {
                completed: 17,
                total: 17
            }
        );
    }

    #[test]
    fn sync_all_is_the_uncancellable_shortcut() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        let result = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_all()
            .unwrap();
        assert_eq!(result.fetched_count, 17);
        assert_eq!(result.saved_count, 17);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn counts_per_workout_failures_and_continues() {
        let client = demo_client()
            .failing_detail(1001, Concept2Error::NotFound(1001))
            .failing_detail(1004, Concept2Error::Decode("malformed payload".into()));
        let cache = InMemoryWorkoutCache::default();
        let outcome = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_with(&AtomicBool::new(false), &mut |_| {})
            .unwrap();

        assert_eq!(outcome.result.fetched_count, 17);
        assert_eq!(outcome.result.saved_count, 15);
        assert_eq!(outcome.result.failed_count, 2);
        assert!(!outcome.cancelled);
        assert_eq!(cache.list_workouts().unwrap().len(), 15);
        // The sync really did try every summary (one default-sized page).
        assert_eq!(client.calls().len(), 1 + 17);
    }

    #[test]
    fn a_failed_cache_save_counts_and_continues() {
        let client = demo_client();
        let cache = UnwritableCache {
            inner: InMemoryWorkoutCache::default(),
        };
        let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 10)
            .sync_with(&AtomicBool::new(false), &mut |_| {})
            .unwrap();

        assert_eq!(outcome.result.fetched_count, 17);
        assert_eq!(outcome.result.saved_count, 0);
        assert_eq!(outcome.result.failed_count, 17);
        assert!(cache.inner.list_workouts().unwrap().is_empty());
    }

    #[test]
    fn aborting_statuses_stop_the_sync() {
        for error in [
            Concept2Error::Unauthorized,
            Concept2Error::Forbidden,
            Concept2Error::RateLimited {
                retry_after_secs: Some(30),
            },
            Concept2Error::Http { status: 401 },
            Concept2Error::Http { status: 403 },
            Concept2Error::Http { status: 429 },
        ] {
            let client = demo_client().failing_detail(1001, error.clone());
            let cache = InMemoryWorkoutCache::default();
            let outcome = WorkoutSyncCoordinator::new(&client, &cache)
                .sync_with(&AtomicBool::new(false), &mut |_| {});
            assert_eq!(
                outcome,
                Err(WorkoutSyncError::ClientFailed(redact(&error.to_string()))),
                "{error}"
            );
            // The first detail request failed and nothing followed it.
            let calls = client.calls();
            assert_eq!(calls.len(), 1 + 1, "{error}: {calls:?}");
            assert!(cache.list_workouts().unwrap().is_empty());
        }
    }

    #[test]
    fn a_summary_failure_is_fatal() {
        let client = demo_client().failing_summaries(Concept2Error::Unauthorized);
        let cache = InMemoryWorkoutCache::default();
        let error = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_all()
            .unwrap_err();
        assert_eq!(
            error,
            WorkoutSyncError::ClientFailed("Concept2 rejected the token".to_owned())
        );
        assert_eq!(client.calls(), vec!["list:1"]);
    }

    #[test]
    fn a_migration_failure_is_fatal_and_touches_nothing() {
        let client = demo_client();
        let cache = UnmigratableCache;
        let error = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_all()
            .unwrap_err();
        assert_eq!(
            error,
            WorkoutSyncError::CacheFailed("cache migration failed: bad schema".to_owned())
        );
        assert!(
            client.calls().is_empty(),
            "no request may precede migration"
        );
    }

    #[test]
    fn a_pre_set_cancel_flag_stops_before_the_summaries() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        let cancel = AtomicBool::new(true);
        let outcome = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_with(&cancel, &mut |_| {})
            .unwrap();

        assert!(outcome.cancelled);
        assert_eq!(outcome.result.fetched_count, 0);
        assert_eq!(outcome.result.saved_count, 0);
        assert!(client.calls().is_empty());
        // Migration still happened: the cache is usable afterwards.
        assert!(cache.is_migrated());
    }

    #[test]
    fn cancelling_mid_way_keeps_what_was_written() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        let cancel = AtomicBool::new(false);
        let outcome = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_with(&cancel, &mut |progress| {
                if progress.completed == 3 {
                    cancel.store(true, Ordering::Relaxed);
                }
            })
            .unwrap();

        assert!(outcome.cancelled);
        assert_eq!(outcome.result.fetched_count, 17);
        assert_eq!(outcome.result.saved_count, 3);
        assert_eq!(outcome.result.failed_count, 0);
        assert_eq!(cache.list_workouts().unwrap().len(), 3);
        // One summary page plus the three details fetched before the stop.
        assert_eq!(client.calls().len(), 1 + 3);
    }

    #[test]
    fn an_empty_logbook_syncs_cleanly() {
        let client = TestClient::new(Vec::new());
        let cache = InMemoryWorkoutCache::default();
        let outcome = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_with(&AtomicBool::new(false), &mut |_| {})
            .unwrap();
        assert!(!outcome.cancelled);
        assert_eq!(outcome.result.fetched_count, 0);
        assert_eq!(client.calls(), vec!["list:1"]);
    }

    #[test]
    fn the_state_tracker_follows_the_lifecycle() {
        let cache = InMemoryWorkoutCache::default();
        cache.migrate().unwrap();
        cache.save_details(&demo_details()).unwrap();
        let tracker = SyncStateTracker::new(&cache);

        assert_eq!(tracker.state(), SyncState::default());

        tracker.refresh_workout_count();
        assert_eq!(tracker.state().total_workouts, 17);

        tracker.sync_started();
        let started = tracker.state();
        assert!(started.in_progress);
        assert_eq!(started.last_error, None);

        tracker.sync_completed();
        let completed = tracker.state();
        assert!(!completed.in_progress);
        assert!(completed.last_sync_date.is_some());
        assert_eq!(completed.last_error, None);
        assert_eq!(completed.total_workouts, 17);

        tracker.sync_started();
        tracker.sync_failed(&WorkoutSyncError::ClientFailed("rate limited".to_owned()));
        let failed = tracker.state();
        assert!(!failed.in_progress);
        assert_eq!(
            failed.last_error.as_deref(),
            Some("Sync client failed: rate limited")
        );
        assert!(failed.last_error_date.is_some());
        // The partial sync is still reflected in the count.
        assert_eq!(failed.total_workouts, 17);

        // A retry clears the error again.
        tracker.sync_completed();
        let retried = tracker.state();
        assert_eq!(retried.last_error, None);
        assert_eq!(retried.last_error_date, None);
    }

    #[test]
    fn the_tracker_survives_a_cache_that_cannot_be_read() {
        let cache = UnwritableCache {
            inner: InMemoryWorkoutCache::default(),
        };
        let tracker = SyncStateTracker::new(&cache);
        tracker.refresh_workout_count();
        assert_eq!(tracker.state().total_workouts, 0);
    }

    #[test]
    fn sync_errors_are_privacy_safe() {
        let token = SecretToken::new("test-token-abc123").unwrap();
        assert_eq!(token.expose(), "test-token-abc123");
        for error in [
            WorkoutSyncError::ClientFailed("Concept2 rejected the token".to_owned()),
            WorkoutSyncError::CacheFailed("cache query failed".to_owned()),
            WorkoutSyncError::MappingFailed("malformed payload".to_owned()),
        ] {
            let text = format!("{error} {error:?}");
            assert!(!text.contains("test-token-abc123"), "{text}");
        }
    }
}
