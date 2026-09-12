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

use std::collections::BTreeMap;
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
    /// Details whose cached stamp already matched the summary (incremental
    /// syncs only; always 0 for a full sync).
    pub skipped_count: usize,
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
    /// Details processed so far (saved, skipped or failed).
    pub completed: usize,
    /// Details the summary pass found.
    pub total: usize,
    /// Details written to the cache so far.
    pub saved: usize,
    /// Details skipped because the cache already held them unchanged.
    pub skipped: usize,
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
    ///
    /// Incremental by default: unchanged details are skipped and the summary
    /// walk stops at the first page the cache already holds in full.
    pub fn sync_all(&self) -> Result<WorkoutSyncResult, WorkoutSyncError> {
        self.sync_all_mode(false)
    }

    /// Sync everything, choosing the mode explicitly.
    pub fn sync_all_mode(&self, full: bool) -> Result<WorkoutSyncResult, WorkoutSyncError> {
        let cancel = AtomicBool::new(false);
        self.sync_with(&cancel, full, &mut |_| {})
            .map(|outcome| outcome.result)
    }

    /// Sync, stopping when `cancel` is set.
    ///
    /// `full` selects the mode, matching the web's two buttons
    /// (`settings.syncIncremental` / `settings.syncFull`):
    ///
    /// - **incremental** (`false`): a detail is skipped when the cache already
    ///   holds that id with the same identity stamp
    ///   ([`SummaryStamp`](crate::workout_cache::SummaryStamp)), and the
    ///   summary walk may stop early — but only when the previous run is
    ///   known to have completed cleanly, so an interrupted first sync still
    ///   pages all the way to the end and picks up the older workouts;
    /// - **full** (`true`): every page is walked and every detail re-fetched.
    ///
    /// A clean run (every page walked, zero failures, no cancellation) is
    /// recorded through [`WorkoutCache::set_fully_synced`]; anything else
    /// clears the flag.
    ///
    /// `progress` is called once per processed detail.
    pub fn sync_with(
        &self,
        cancel: &AtomicBool,
        full: bool,
        progress: &mut dyn FnMut(SyncProgress),
    ) -> Result<SyncOutcome, WorkoutSyncError> {
        let started_at = Utc::now();

        // 0. The schema must exist before anything is written.
        self.cache.migrate().map_err(|error| {
            let message = redact(&error.to_string());
            self.logger.warn("cache migration failed", &[&message]);
            WorkoutSyncError::CacheFailed(message)
        })?;

        // 0b. The change-detection baseline (incremental only) and the
        // checkpoint that makes the early page stop sound.
        let (stamps, may_stop_early) = if full {
            (BTreeMap::new(), false)
        } else {
            let stamps = self.cache.summary_stamps().map_err(|error| {
                let message = redact(&error.to_string());
                self.logger.warn("could not read cache stamps", &[&message]);
                WorkoutSyncError::CacheFailed(message)
            })?;
            let fully_synced = self.cache.is_fully_synced().map_err(|error| {
                let message = redact(&error.to_string());
                self.logger
                    .warn("could not read sync checkpoint", &[&message]);
                WorkoutSyncError::CacheFailed(message)
            })?;
            (stamps, fully_synced)
        };

        // 1. Summaries, page by page (early stop when caught up *and* the
        // previous run is known to have finished the job). A paging failure
        // interrupts the walk, so the cache is no longer known to be complete.
        let summaries = match self.fetch_summaries(cancel, full, may_stop_early, &stamps) {
            Ok(summaries) => summaries,
            Err(error) => {
                self.record_checkpoint(false);
                return Err(error);
            }
        };
        let total = summaries.len();
        if cancel.load(Ordering::Relaxed) {
            self.record_checkpoint(false);
            return Ok(outcome(started_at, total, 0, 0, 0, true));
        }

        // 2. Details, one request at a time.
        let mut saved_count = 0;
        let mut skipped_count = 0;
        let mut failed_count = 0;
        for (completed, summary) in summaries.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                self.record_checkpoint(false);
                return Ok(outcome(
                    started_at,
                    total,
                    saved_count,
                    skipped_count,
                    failed_count,
                    true,
                ));
            }

            // Incremental: the cached row already carries this stamp.
            if stamps
                .get(&summary.id)
                .is_some_and(|stamp| stamp.matches(summary))
            {
                skipped_count += 1;
                progress(SyncProgress {
                    completed: completed + 1,
                    total,
                    saved: saved_count,
                    skipped: skipped_count,
                });
                continue;
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
                        self.record_checkpoint(false);
                        progress(SyncProgress {
                            completed: completed + 1,
                            total,
                            saved: saved_count,
                            skipped: skipped_count,
                        });
                        return Err(WorkoutSyncError::ClientFailed(message));
                    }
                }
            }

            progress(SyncProgress {
                completed: completed + 1,
                total,
                saved: saved_count,
                skipped: skipped_count,
            });
        }

        // Clean means zero failures and no cancellation. Reaching here also
        // means the walk either paged to the reported end or stopped early,
        // and an early stop is only permitted when the cache was already
        // known to be complete — so both are consistent with a complete cache.
        self.record_checkpoint(failed_count == 0);
        Ok(outcome(
            started_at,
            total,
            saved_count,
            skipped_count,
            failed_count,
            false,
        ))
    }

    /// Page through the summaries, following the page count the API reports.
    ///
    /// The page count is monotone (the web app takes the running maximum), so a
    /// server that reports fewer pages on a later response cannot cut the walk
    /// short.
    ///
    /// `may_stop_early` allows the walk to end at the first page whose every
    /// summary is already cached with a matching stamp: the results are
    /// newest-first, so once a whole page is unchanged *and the previous run
    /// fetched every page*, everything older is present too.
    ///
    /// Without that guarantee — a first sync that was cancelled or hit
    /// failures — the walk pages all the way to the reported end while still
    /// skipping unchanged details, which costs only the summary fetches.
    fn fetch_summaries(
        &self,
        cancel: &AtomicBool,
        full: bool,
        may_stop_early: bool,
        stamps: &BTreeMap<i64, crate::workout_cache::SummaryStamp>,
    ) -> Result<Vec<Workout>, WorkoutSyncError> {
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
            let page_fully_cached = !result.workouts.is_empty()
                && result.workouts.iter().all(|summary| {
                    stamps
                        .get(&summary.id)
                        .is_some_and(|stamp| stamp.matches(summary))
                });
            summaries.extend(result.workouts);
            total_pages = total_pages.max(result.total_pages.max(1));
            page += 1;
            if page > total_pages {
                break;
            }
            if !full && may_stop_early && page_fully_cached {
                break;
            }
        }
        Ok(summaries)
    }
}

impl WorkoutSyncCoordinator<'_> {
    /// Persists whether the cache is now known to hold every page.
    ///
    /// A failure here never fails the sync: the worst case is that the next
    /// incremental run walks the summary pages again instead of stopping
    /// early, which is the safe direction.
    fn record_checkpoint(&self, fully_synced: bool) {
        if let Err(error) = self.cache.set_fully_synced(fully_synced) {
            let message = redact(&error.to_string());
            self.logger
                .warn("could not record sync checkpoint", &[&message]);
        }
    }
}

/// Build an outcome for the work done so far.
fn outcome(
    started_at: DateTime<Utc>,
    fetched_count: usize,
    saved_count: usize,
    skipped_count: usize,
    failed_count: usize,
    cancelled: bool,
) -> SyncOutcome {
    SyncOutcome {
        result: WorkoutSyncResult {
            fetched_count,
            saved_count,
            skipped_count,
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
    /// Whether the last completed run walked every page with no failures and
    /// no cancellation — the precondition for the incremental early page stop.
    pub fully_synced: bool,
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

    /// Re-read the cached workout count and the sync checkpoint.
    ///
    /// A cache failure is logged and leaves the previous values in place: they
    /// are display hints, not a reason to break the sync state machine.
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
        match self.cache.is_fully_synced() {
            Ok(fully_synced) => {
                self.state.lock().expect("sync state lock").fully_synced = fully_synced;
            }
            Err(error) => {
                let message = redact(&error.to_string());
                self.logger
                    .warn("could not read the sync checkpoint", &[&message]);
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
    use crate::workout_cache::{CacheError, InMemoryWorkoutCache, SqliteWorkoutCache};
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

        fn is_fully_synced(&self) -> Result<bool, CacheError> {
            self.inner.is_fully_synced()
        }

        fn set_fully_synced(&self, fully_synced: bool) -> Result<(), CacheError> {
            self.inner.set_fully_synced(fully_synced)
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

        fn is_fully_synced(&self) -> Result<bool, CacheError> {
            Ok(false)
        }

        fn set_fully_synced(&self, _fully_synced: bool) -> Result<(), CacheError> {
            Ok(())
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
            .sync_with(&AtomicBool::new(false), false, &mut |progress| {
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
        assert_eq!(seen[0].completed, 1);
        assert_eq!(seen[0].total, 17);
        assert_eq!(seen[0].saved, 1);
        assert_eq!(seen[0].skipped, 0);
        assert_eq!(seen[16].completed, 17);
        assert_eq!(seen[16].total, 17);
        assert_eq!(seen[16].saved, 17);
        assert_eq!(seen[16].skipped, 0);
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
            .sync_with(&AtomicBool::new(false), false, &mut |_| {})
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
            .sync_with(&AtomicBool::new(false), false, &mut |_| {})
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
            let outcome = WorkoutSyncCoordinator::new(&client, &cache).sync_with(
                &AtomicBool::new(false),
                false,
                &mut |_| {},
            );
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
            .sync_with(&cancel, false, &mut |_| {})
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
            .sync_with(&cancel, false, &mut |progress| {
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

    /// The blocker case: a cancelled first sync leaves the newest page cached
    /// and older pages missing, so the next incremental run must *not* stop at
    /// page 1 — it has to page all the way and fetch the missing older
    /// details, while still skipping what is already cached.
    #[test]
    fn an_incremental_sync_after_a_cancelled_run_fetches_the_missing_pages() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();

        // First page of five saved, then the user cancels.
        let cancel = AtomicBool::new(false);
        let first = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5).sync_with(
            &cancel,
            false,
            &mut |progress| {
                if progress.completed == 5 {
                    cancel.store(true, Ordering::Relaxed);
                }
            },
        );
        let outcome = first.unwrap();
        assert!(outcome.cancelled);
        assert_eq!(outcome.result.saved_count, 5);
        assert_eq!(cache.list_workouts().unwrap().len(), 5);
        assert!(
            !cache.is_fully_synced().unwrap(),
            "a cancelled run must not leave the cache marked complete"
        );
        let after_cancel = client.calls().len();

        // Next incremental run: the stamp set says pages 2-4 are missing, so
        // the walk must continue past page 1.
        // A fresh cache handle stands in for an app restart: the checkpoint is
        // persisted, so the guard survives it (the in-memory cache shares its
        // state by design; SQLite has its own coverage).
        let second = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(second.saved_count, 12, "the older details are fetched");
        assert_eq!(second.skipped_count, 5, "page 1 is still skipped");
        assert_eq!(cache.list_workouts().unwrap().len(), 17);
        let calls = &client.calls()[after_cancel..];
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.starts_with("list:"))
                .count(),
            4,
            "an interrupted history must be walked to the end: {calls:?}"
        );
        assert!(
            !calls.contains(&"detail:1001".to_owned()),
            "the cancelled run's details must not be refetched: {calls:?}"
        );
        assert!(cache.is_fully_synced().unwrap(), "and now it is complete");
    }

    /// A failed detail is retried on the next incremental run, because the
    /// failure cleared the checkpoint.
    #[test]
    fn a_failed_detail_is_refetched_on_the_next_incremental_run() {
        let client = demo_client().failing_detail(1001, Concept2Error::NotFound(1001));
        let cache = InMemoryWorkoutCache::default();
        let first = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_all()
            .unwrap();
        assert_eq!(first.failed_count, 1);
        assert_eq!(first.skipped_count, 0);
        assert!(
            !cache.is_fully_synced().unwrap(),
            "a run with failures must not be marked complete"
        );

        // The next run retries that one detail and pages to the end. A fresh
        // client stands in for the Logbook recovering (the previous one still
        // has the failure configured).
        let recovered = demo_client();
        let second = WorkoutSyncCoordinator::new(&recovered, &cache)
            .sync_all()
            .unwrap();
        assert_eq!(
            second.saved_count, 1,
            "only the failed workout is refetched"
        );
        assert_eq!(second.skipped_count, 16);
        assert_eq!(second.failed_count, 0);
        let calls = recovered.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.starts_with("detail:"))
                .count(),
            1,
            "{calls:?}"
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.as_str() == "detail:1001")
                .count(),
            1
        );
        assert!(cache.is_fully_synced().unwrap());
    }

    /// After one clean run the early stop fires again — and keeps firing.
    #[test]
    fn a_clean_run_restores_the_early_stop() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert!(cache.is_fully_synced().unwrap());
        let baseline = client.calls().len();

        let second = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(second.fetched_count, 5, "early stop at page 1");
        assert_eq!(second.skipped_count, 5);
        assert_eq!(&client.calls()[baseline..], &["list:1"]);
        assert!(
            cache.is_fully_synced().unwrap(),
            "a permitted early stop keeps the cache marked complete"
        );

        // And it stays cached: a third run behaves identically.
        let third_baseline = client.calls().len();
        let third = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(third.fetched_count, 5);
        assert_eq!(&client.calls()[third_baseline..], &["list:1"]);
    }

    /// The second sync over an unchanged library issues zero detail requests.
    #[test]
    fn an_incremental_resync_skips_every_unchanged_detail() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();

        // First (incremental) sync: everything is new.
        let first = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(first.fetched_count, 17);
        assert_eq!(first.saved_count, 17);
        assert_eq!(first.skipped_count, 0);
        let first_calls = client.calls();
        assert_eq!(first_calls.len(), 4 + 17, "4 pages + 17 details");

        // Second sync: one page proves the library is caught up, no details.
        let second = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(second.fetched_count, 5, "only the first page is walked");
        assert_eq!(second.saved_count, 0);
        assert_eq!(second.skipped_count, 5);
        assert_eq!(second.failed_count, 0);
        let second_calls = &client.calls()[first_calls.len()..];
        assert_eq!(
            second_calls,
            &["list:1"],
            "an unchanged library must issue zero detail requests"
        );
    }

    /// A changed stamp refetches just that workout.
    #[test]
    fn a_changed_summary_stamp_refetches_only_that_workout() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();

        // The Logbook reports a new stamp for one result.
        let mut edited = client.details[&1001].clone();
        edited.workout.date = "2026-05-27 09:00:00".to_owned();
        let client = TestClient {
            summaries: {
                let mut summaries = client.summaries.clone();
                for summary in &mut summaries {
                    if summary.id == 1001 {
                        summary.date = edited.workout.date.clone();
                    }
                }
                summaries
            },
            details: {
                let mut details = client.details.clone();
                details.insert(1001, edited);
                details
            },
            detail_failures: BTreeMap::new(),
            summary_failure: None,
            calls: Mutex::new(Vec::new()),
        };

        let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        // The changed summary keeps the walk going past its page (the early
        // stop needs a *fully* cached page), then that page is unchanged.
        assert_eq!(outcome.saved_count, 1, "only 1001 is refetched");
        assert_eq!(
            outcome.skipped_count,
            outcome.fetched_count - 1,
            "every other walked summary is skipped"
        );
        let calls = client.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.starts_with("detail:"))
                .count(),
            1,
            "{calls:?}"
        );
        assert!(calls.contains(&"detail:1001".to_owned()), "{calls:?}");
    }

    /// Full mode re-downloads everything and never skips.
    #[test]
    fn a_full_resync_ignores_the_cache_and_walks_every_page() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        let calls_after_incremental = client.calls().len();

        let full = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all_mode(true)
            .unwrap();
        assert_eq!(full.fetched_count, 17);
        assert_eq!(full.saved_count, 17);
        assert_eq!(full.skipped_count, 0);
        let full_calls = &client.calls()[calls_after_incremental..];
        assert_eq!(
            full_calls
                .iter()
                .filter(|call| call.starts_with("list:"))
                .count(),
            4,
            "full mode pages to the reported end"
        );
        assert_eq!(
            full_calls
                .iter()
                .filter(|call| call.starts_with("detail:"))
                .count(),
            17
        );
    }

    /// The early stop must not fire on a page that is only partly cached.
    #[test]
    fn incremental_paging_continues_past_a_partly_changed_page() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        // Seed the cache with only the newest result, then sync: page 1 is
        // not fully cached, so paging continues to the end.
        cache
            .save_details(&[client.details[&1005].clone()])
            .unwrap();
        let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_all()
            .unwrap();
        assert_eq!(outcome.fetched_count, 17);
        assert_eq!(outcome.saved_count, 16);
        assert_eq!(outcome.skipped_count, 1);
    }

    /// Progress carries the running saved/skipped counts.
    #[test]
    fn progress_reports_saved_and_skipped() {
        let client = demo_client();
        let cache = InMemoryWorkoutCache::default();
        cache.save_details(&demo_details()).unwrap();
        // A prior clean run, so the early stop applies and the walk is one page.
        cache.set_fully_synced(true).unwrap();
        let mut seen = Vec::new();
        WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
            .sync_with(&AtomicBool::new(false), false, &mut |progress| {
                seen.push(progress);
            })
            .unwrap();
        assert_eq!(seen.len(), 5, "one entry per summary on the walked page");
        assert_eq!(seen[4].completed, 5);
        assert_eq!(seen[4].skipped, 5);
        assert_eq!(seen[4].saved, 0);
    }

    #[test]
    fn an_empty_logbook_syncs_cleanly() {
        let client = TestClient::new(Vec::new());
        let cache = InMemoryWorkoutCache::default();
        let outcome = WorkoutSyncCoordinator::new(&client, &cache)
            .sync_with(&AtomicBool::new(false), false, &mut |_| {})
            .unwrap();
        assert!(!outcome.cancelled);
        assert_eq!(outcome.result.fetched_count, 0);
        assert_eq!(client.calls(), vec!["list:1"]);
    }

    /// The checkpoint is persisted, so the guard survives an app restart
    /// between an interrupted first sync and the recovery run. Exercised on
    /// the real SQLite store; the in-memory runs above share state by design.
    #[test]
    fn the_early_stop_guard_survives_a_restart_on_sqlite() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workouts.sqlite");
        let client = demo_client();

        // Interrupted first run: cancelled after the first page of five.
        {
            let cache = SqliteWorkoutCache::open(&path).unwrap();
            let cancel = AtomicBool::new(false);
            let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
                .sync_with(&cancel, false, &mut |progress| {
                    if progress.completed == 5 {
                        cancel.store(true, Ordering::Relaxed);
                    }
                })
                .unwrap();
            assert!(outcome.cancelled);
            assert!(!cache.is_fully_synced().unwrap());
        }

        // Restart: reopen the store and sync incrementally.
        {
            let cache = SqliteWorkoutCache::open(&path).unwrap();
            assert!(
                !cache.is_fully_synced().unwrap(),
                "the checkpoint is persisted, not in-memory"
            );
            let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
                .sync_all()
                .unwrap();
            assert_eq!(outcome.saved_count, 12, "older pages are fetched");
            assert_eq!(outcome.skipped_count, 5);
            assert_eq!(cache.list_workouts().unwrap().len(), 17);
            assert!(cache.is_fully_synced().unwrap());
        }

        // A third run, also reopened, early-stops as usual.
        {
            let cache = SqliteWorkoutCache::open(&path).unwrap();
            let outcome = WorkoutSyncCoordinator::with_per_page(&client, &cache, 5)
                .sync_all()
                .unwrap();
            assert_eq!(outcome.fetched_count, 5, "early stop restored");
            assert_eq!(outcome.skipped_count, 5);
        }
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
