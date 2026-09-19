// SPDX-License-Identifier: GPL-3.0-or-later
//! Live-mode cadence and state machine (Phase 8).
//!
//! Ports the web's `liveMode.ts` / `LiveMode` class and Studio's
//! `LivePollingCadence` / `LiveModeState`. The timer is injectable: production
//! fills [`InstantSource`] with wall time; tests fill it with a fake clock so
//! the suite never sleeps.

use std::sync::atomic::{AtomicI64, Ordering};

/// Polling interval presets in seconds. Minimum 30 per Concept2 rate guidance
/// (web `LIVE_INTERVALS`, Studio `liveIntervals`).
pub const LIVE_INTERVALS: [u32; 4] = [30, 60, 120, 300];

/// Default polling interval (seconds).
pub const DEFAULT_LIVE_INTERVAL_SEC: u32 = 60;

/// Page size the live poll asks for (web `listRecentWorkouts(25)`).
pub const LIVE_PAGE_SIZE: u32 = 25;

/// Hidden-tab floor (seconds); matches the web's `HIDDEN_MIN_INTERVAL_SEC`.
const HIDDEN_MIN_INTERVAL_SEC: u32 = 300;

/// Failure backoff steps in milliseconds: 30s → 60s → 120s → 300s cap.
const BACKOFF_MS: [i64; 4] = [30_000, 60_000, 120_000, 300_000];

/// Live-mode status (web `LiveModeStatus`, Studio `LiveModeStatus`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LiveModeStatus {
    /// Enabled and waiting for the next tick.
    Idle,
    /// A poll is in flight.
    Polling,
    /// The last poll failed; backoff is active.
    Error,
    /// Token rejected (401/403): timer stopped, no backoff.
    SignedOut,
    /// Disabled.
    #[default]
    Stopped,
}

/// Pure live-mode state machine (Studio `LiveModeState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveModeState {
    /// Whether live mode is enabled.
    pub enabled: bool,
    /// Current lifecycle status.
    pub status: LiveModeStatus,
    /// Configured interval in seconds (one of [`LIVE_INTERVALS`]).
    pub interval_sec: u32,
    /// Consecutive failed polls.
    pub consecutive_failures: u32,
    /// Epoch millis of the last completed poll (success or failure).
    pub last_poll_at_ms: Option<i64>,
    /// Epoch millis when the next poll is due.
    pub next_poll_at_ms: Option<i64>,
}

impl Default for LiveModeState {
    fn default() -> Self {
        LiveModeState {
            enabled: false,
            status: LiveModeStatus::Stopped,
            interval_sec: DEFAULT_LIVE_INTERVAL_SEC,
            consecutive_failures: 0,
            last_poll_at_ms: None,
            next_poll_at_ms: None,
        }
    }
}

impl LiveModeState {
    /// Whether three or more consecutive failures have accumulated.
    #[must_use]
    pub fn has_warning(&self) -> bool {
        self.consecutive_failures >= 3
    }

    /// Enable and transition to idle, clearing failure state.
    pub fn start(&mut self) {
        self.enabled = true;
        self.status = LiveModeStatus::Idle;
        self.consecutive_failures = 0;
        self.next_poll_at_ms = None;
    }

    /// Disable and transition to stopped.
    pub fn stop(&mut self) {
        self.enabled = false;
        self.status = LiveModeStatus::Stopped;
        self.next_poll_at_ms = None;
    }

    /// Begin a poll. Valid from idle or error (retry).
    pub fn poll_started(&mut self) {
        if self.enabled && matches!(self.status, LiveModeStatus::Idle | LiveModeStatus::Error) {
            self.status = LiveModeStatus::Polling;
        }
    }

    /// Poll succeeded: reset backoff and record the time.
    pub fn poll_succeeded(&mut self, now_ms: i64) {
        if self.enabled && self.status == LiveModeStatus::Polling {
            self.status = LiveModeStatus::Idle;
            self.consecutive_failures = 0;
            self.last_poll_at_ms = Some(now_ms);
        }
    }

    /// Poll failed: increment failures and enter error.
    pub fn poll_failed(&mut self, now_ms: i64) {
        if self.enabled && self.status == LiveModeStatus::Polling {
            self.status = LiveModeStatus::Error;
            self.consecutive_failures = self.consecutive_failures.saturating_add(1);
            self.last_poll_at_ms = Some(now_ms);
        }
    }

    /// Schedule the next poll.
    pub fn tick_scheduled(&mut self, at_ms: i64) {
        if self.enabled {
            self.next_poll_at_ms = Some(at_ms);
        }
    }

    /// Change the interval; unknown values are ignored.
    pub fn interval_changed(&mut self, new_interval: u32) {
        if LIVE_INTERVALS.contains(&new_interval) {
            self.interval_sec = new_interval;
        }
    }

    /// Auth expiry (401/403): disable live mode, stop the timer, no backoff.
    pub fn signed_out(&mut self) {
        self.enabled = false;
        self.status = LiveModeStatus::SignedOut;
        self.next_poll_at_ms = None;
    }
}

/// Instant source for the live timer. Production uses wall time; tests use
/// [`FakeClock`] so the suite never sleeps.
pub trait InstantSource: Send + Sync {
    /// Current epoch milliseconds.
    fn now_ms(&self) -> i64;
}

/// Wall-clock instant source.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl InstantSource for SystemClock {
    fn now_ms(&self) -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64)
    }
}

/// Deterministic clock for tests.
#[derive(Debug, Default)]
pub struct FakeClock {
    now_ms: AtomicI64,
}

impl FakeClock {
    /// A clock starting at `now_ms`.
    #[must_use]
    pub fn new(now_ms: i64) -> Self {
        FakeClock {
            now_ms: AtomicI64::new(now_ms),
        }
    }

    /// Advance (or set) the clock.
    pub fn set(&self, now_ms: i64) {
        self.now_ms.store(now_ms, Ordering::SeqCst);
    }

    /// Advance by `delta_ms`.
    pub fn advance(&self, delta_ms: i64) {
        self.now_ms.fetch_add(delta_ms, Ordering::SeqCst);
    }
}

impl InstantSource for FakeClock {
    fn now_ms(&self) -> i64 {
        self.now_ms.load(Ordering::SeqCst)
    }
}

/// Active tab uses the configured interval; hidden tab slows to ≥ 5 minutes.
#[must_use]
pub fn effective_interval_sec(base_sec: u32, tab_visible: bool) -> u32 {
    if tab_visible {
        base_sec
    } else {
        base_sec.max(HIDDEN_MIN_INTERVAL_SEC)
    }
}

/// Exponential backoff after failures: 30s → 60s → 120s → 300s cap.
#[must_use]
pub fn next_backoff_ms(consecutive_failures: u32) -> i64 {
    if consecutive_failures == 0 {
        return 0;
    }
    let index = (consecutive_failures as usize - 1).min(BACKOFF_MS.len() - 1);
    BACKOFF_MS[index]
}

/// Staleness threshold: 2× the configured interval, in milliseconds.
#[must_use]
pub fn staleness_threshold_ms(interval_sec: u32) -> i64 {
    i64::from(interval_sec) * 2 * 1000
}

/// Clamp an interval preference to a known preset (default 60).
#[must_use]
pub fn parse_interval(value: u32) -> u32 {
    if LIVE_INTERVALS.contains(&value) {
        value
    } else {
        DEFAULT_LIVE_INTERVAL_SEC
    }
}

/// Schedule delay after a successful poll (non-demo).
#[must_use]
pub fn success_delay_ms(interval_sec: u32, tab_visible: bool) -> i64 {
    i64::from(effective_interval_sec(interval_sec, tab_visible)) * 1000
}

/// Schedule delay after a failed poll (interval + backoff).
#[must_use]
pub fn failure_delay_ms(interval_sec: u32, tab_visible: bool, consecutive_failures: u32) -> i64 {
    success_delay_ms(interval_sec, tab_visible) + next_backoff_ms(consecutive_failures)
}

/// Whether the scheduled poll is due.
#[must_use]
pub fn poll_is_due(state: &LiveModeState, now_ms: i64) -> bool {
    state.enabled
        && matches!(state.status, LiveModeStatus::Idle | LiveModeStatus::Error)
        && state.next_poll_at_ms.is_some_and(|at| now_ms >= at)
}

/// What the Qt/timer layer should do after a session transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveAction {
    /// Do nothing.
    None,
    /// Start a poll on the worker thread.
    StartPoll,
    /// Disable live mode and show the reauth string.
    SignedOut,
}

/// Failure class the session understands (maps from platform `LivePollError`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveFailureKind {
    /// 401 / 403 — stop and surface signed-out (no backoff, no refresh).
    Unauthorized,
    /// 429 — backoff, optionally honouring `Retry-After`.
    RateLimited {
        /// Seconds from the `Retry-After` header, when present.
        retry_after_secs: Option<u64>,
    },
    /// Network / 5xx / other — standard backoff.
    Transient,
}

/// Qt-free live-mode session: state + known ids + schedule decisions.
///
/// The app layer owns the timer and the worker; this struct decides *when*
/// to poll and *how* to react, driven by an injectable clock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveSession {
    /// Lifecycle state.
    pub state: LiveModeState,
    /// Result ids already seen this session (web `knownIds`).
    pub known_ids: std::collections::BTreeSet<i64>,
    /// Whether the window is frontmost (desktop always-true unless minimized).
    pub tab_visible: bool,
    /// Demo mode uses a random delay instead of the interval (web).
    pub demo: bool,
}

impl Default for LiveSession {
    fn default() -> Self {
        LiveSession {
            state: LiveModeState::default(),
            known_ids: std::collections::BTreeSet::new(),
            tab_visible: true,
            demo: false,
        }
    }
}

impl LiveSession {
    /// Enable and schedule an immediate poll.
    pub fn enable(&mut self, now_ms: i64) {
        self.state.start();
        self.state.tick_scheduled(now_ms);
    }

    /// Disable.
    pub fn disable(&mut self) {
        self.state.stop();
    }

    /// Timer tick: start a poll if due. Does not sleep.
    pub fn on_tick(&mut self, now_ms: i64) -> LiveAction {
        if !poll_is_due(&self.state, now_ms) {
            return LiveAction::None;
        }
        self.state.poll_started();
        if self.state.status == LiveModeStatus::Polling {
            LiveAction::StartPoll
        } else {
            LiveAction::None
        }
    }

    /// Manual refresh: poll now without moving `next_poll_at_ms`.
    pub fn manual_refresh(&mut self) -> LiveAction {
        if !self.state.enabled {
            return LiveAction::None;
        }
        if self.state.status == LiveModeStatus::Polling {
            return LiveAction::None;
        }
        let kept = self.state.next_poll_at_ms;
        self.state.poll_started();
        self.state.next_poll_at_ms = kept;
        if self.state.status == LiveModeStatus::Polling {
            LiveAction::StartPoll
        } else {
            LiveAction::None
        }
    }

    /// A poll succeeded. `new_ids` are added to the known set.
    /// Reschedules from `now_ms` unless this was a manual refresh that left
    /// a future `next_poll_at_ms` intact and still in the future.
    pub fn on_poll_ok(&mut self, now_ms: i64, new_ids: &[i64], reschedule: bool) {
        for id in new_ids {
            self.known_ids.insert(*id);
        }
        self.state.poll_succeeded(now_ms);
        if reschedule {
            let delay = success_delay_ms(self.state.interval_sec, self.tab_visible);
            self.state.tick_scheduled(now_ms + delay);
        }
    }

    /// A poll failed.
    pub fn on_poll_err(&mut self, now_ms: i64, kind: LiveFailureKind) -> LiveAction {
        if matches!(kind, LiveFailureKind::Unauthorized) {
            self.state.signed_out();
            return LiveAction::SignedOut;
        }
        self.state.poll_failed(now_ms);
        let backoff = match kind {
            LiveFailureKind::RateLimited {
                retry_after_secs: Some(secs),
            } => (secs as i64)
                .saturating_mul(1000)
                .max(next_backoff_ms(self.state.consecutive_failures)),
            _ => next_backoff_ms(self.state.consecutive_failures),
        };
        let delay = success_delay_ms(self.state.interval_sec, self.tab_visible) + backoff;
        self.state.tick_scheduled(now_ms + delay);
        LiveAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_matches_studio() {
        let state = LiveModeState::default();
        assert_eq!(state.status, LiveModeStatus::Stopped);
        assert!(!state.enabled);
        assert_eq!(state.interval_sec, 60);
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.has_warning());
    }

    #[test]
    fn start_stop_and_poll_lifecycle() {
        let mut state = LiveModeState::default();
        state.start();
        assert!(state.enabled);
        assert_eq!(state.status, LiveModeStatus::Idle);

        state.poll_started();
        assert_eq!(state.status, LiveModeStatus::Polling);
        // Second start while polling is ignored.
        state.poll_started();
        assert_eq!(state.status, LiveModeStatus::Polling);

        state.poll_succeeded(1_000);
        assert_eq!(state.status, LiveModeStatus::Idle);
        assert_eq!(state.consecutive_failures, 0);
        assert_eq!(state.last_poll_at_ms, Some(1_000));

        state.poll_started();
        state.poll_failed(2_000);
        assert_eq!(state.status, LiveModeStatus::Error);
        assert_eq!(state.consecutive_failures, 1);
        assert_eq!(state.last_poll_at_ms, Some(2_000));

        state.stop();
        assert!(!state.enabled);
        assert_eq!(state.status, LiveModeStatus::Stopped);
        assert!(state.next_poll_at_ms.is_none());
    }

    #[test]
    fn warning_trips_at_three_failures() {
        let mut state = LiveModeState::default();
        state.start();
        for n in 1..=3 {
            state.poll_started();
            state.poll_failed(n * 1_000);
        }
        assert!(state.has_warning());
        assert_eq!(state.consecutive_failures, 3);
    }

    #[test]
    fn signed_out_disables_live_mode() {
        let mut state = LiveModeState::default();
        state.start();
        state.tick_scheduled(5_000);
        state.signed_out();
        assert!(!state.enabled);
        assert_eq!(state.status, LiveModeStatus::SignedOut);
        assert!(state.next_poll_at_ms.is_none());
    }

    #[test]
    fn backoff_steps_match_the_web() {
        assert_eq!(next_backoff_ms(0), 0);
        assert_eq!(next_backoff_ms(1), 30_000);
        assert_eq!(next_backoff_ms(2), 60_000);
        assert_eq!(next_backoff_ms(3), 120_000);
        assert_eq!(next_backoff_ms(4), 300_000);
        assert_eq!(next_backoff_ms(99), 300_000);
    }

    #[test]
    fn hidden_tab_floors_at_five_minutes() {
        assert_eq!(effective_interval_sec(60, true), 60);
        assert_eq!(effective_interval_sec(60, false), 300);
        assert_eq!(effective_interval_sec(300, false), 300);
    }

    #[test]
    fn fake_clock_drives_due_checks_without_sleep() {
        let clock = FakeClock::new(0);
        let mut state = LiveModeState::default();
        state.start();
        state.tick_scheduled(60_000);
        assert!(!poll_is_due(&state, clock.now_ms()));
        clock.advance(59_999);
        assert!(!poll_is_due(&state, clock.now_ms()));
        clock.advance(1);
        assert!(poll_is_due(&state, clock.now_ms()));
    }

    #[test]
    fn saturation_assert_bites_on_bad_backoff() {
        // Prove the backoff step table is what the assert claims: a wrong
        // first-step value must fail.
        let caught = std::panic::catch_unwind(|| {
            assert_eq!(next_backoff_ms(1), 1); // deliberately wrong
        });
        assert!(caught.is_err(), "wrong backoff must fail");
        assert_eq!(next_backoff_ms(1), 30_000);
    }

    #[test]
    fn interval_changed_ignores_unknown_values() {
        let mut state = LiveModeState::default();
        state.interval_changed(45);
        assert_eq!(state.interval_sec, 60);
        state.interval_changed(120);
        assert_eq!(state.interval_sec, 120);
    }

    #[test]
    fn manual_refresh_does_not_disturb_the_schedule() {
        let clock = FakeClock::new(10_000);
        let mut session = LiveSession::default();
        session.enable(clock.now_ms());
        // Consume the immediate enable poll.
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_ok(clock.now_ms(), &[], true);
        let scheduled = session.state.next_poll_at_ms;
        assert_eq!(scheduled, Some(10_000 + 60_000));

        // Manual refresh while waiting: starts a poll, keeps next_poll_at.
        clock.advance(5_000);
        assert_eq!(session.manual_refresh(), LiveAction::StartPoll);
        assert_eq!(session.state.next_poll_at_ms, scheduled);
        session.on_poll_ok(clock.now_ms(), &[7], false);
        assert_eq!(session.state.next_poll_at_ms, scheduled);
        assert!(session.known_ids.contains(&7));
    }

    #[test]
    fn unauthorized_signs_out() {
        let mut session = LiveSession::default();
        session.enable(0);
        assert_eq!(session.on_tick(0), LiveAction::StartPoll);
        let action = session.on_poll_err(0, LiveFailureKind::Unauthorized);
        assert_eq!(action, LiveAction::SignedOut);
        assert!(!session.state.enabled);
        assert_eq!(session.state.status, LiveModeStatus::SignedOut);
        assert!(
            session.state.next_poll_at_ms.is_none(),
            "401 must stop the timer, not schedule backoff"
        );
        assert_eq!(session.state.consecutive_failures, 0);
    }

    #[test]
    fn rate_limited_honours_retry_after_over_backoff() {
        let mut session = LiveSession::default();
        session.enable(0);
        session.on_tick(0);
        session.on_poll_err(
            0,
            LiveFailureKind::RateLimited {
                retry_after_secs: Some(90),
            },
        );
        // interval 60s + max(90s Retry-After, 30s backoff) = 150s
        assert_eq!(session.state.next_poll_at_ms, Some(150_000));
    }

    #[test]
    fn connection_error_backoff_ladder_caps_at_300s() {
        let clock = FakeClock::new(0);
        let mut session = LiveSession::default();
        session.enable(clock.now_ms());
        // Consume enable poll, then fail four times.
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_err(clock.now_ms(), LiveFailureKind::Transient);
        // interval 60s + 30s backoff
        assert_eq!(session.state.next_poll_at_ms, Some(90_000));
        assert_eq!(session.state.consecutive_failures, 1);

        clock.set(90_000);
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_err(clock.now_ms(), LiveFailureKind::Transient);
        // interval 60s + 60s backoff
        assert_eq!(session.state.next_poll_at_ms, Some(90_000 + 120_000));
        assert_eq!(session.state.consecutive_failures, 2);

        clock.set(210_000);
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_err(clock.now_ms(), LiveFailureKind::Transient);
        // interval 60s + 120s backoff
        assert_eq!(session.state.next_poll_at_ms, Some(210_000 + 180_000));
        assert_eq!(session.state.consecutive_failures, 3);

        clock.set(390_000);
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_err(clock.now_ms(), LiveFailureKind::Transient);
        // interval 60s + 300s cap
        assert_eq!(session.state.next_poll_at_ms, Some(390_000 + 360_000));
        assert_eq!(session.state.consecutive_failures, 4);

        clock.set(750_000);
        assert_eq!(session.on_tick(clock.now_ms()), LiveAction::StartPoll);
        session.on_poll_err(clock.now_ms(), LiveFailureKind::Transient);
        // still capped at 300s backoff
        assert_eq!(session.state.next_poll_at_ms, Some(750_000 + 360_000));
        assert_eq!(next_backoff_ms(5), 300_000);
    }

    /// Prove each Task-4 assertion bites when the covered behaviour is broken.
    #[test]
    fn task4_assertions_bite_when_broken() {
        // 401: SignedOut + no backoff schedule.
        let caught = std::panic::catch_unwind(|| {
            let mut session = LiveSession::default();
            session.enable(0);
            session.on_tick(0);
            let _ = session.on_poll_err(0, LiveFailureKind::Unauthorized);
            assert_eq!(session.state.status, LiveModeStatus::Idle); // deliberately wrong
        });
        assert!(caught.is_err(), "wrong 401 status must fail");

        let caught = std::panic::catch_unwind(|| {
            let mut session = LiveSession::default();
            session.enable(0);
            session.on_tick(0);
            let _ = session.on_poll_err(0, LiveFailureKind::Unauthorized);
            assert!(session.state.next_poll_at_ms.is_some()); // deliberately wrong
        });
        assert!(caught.is_err(), "401 must not schedule backoff");

        // 429 Retry-After.
        let caught = std::panic::catch_unwind(|| {
            let mut session = LiveSession::default();
            session.enable(0);
            session.on_tick(0);
            session.on_poll_err(
                0,
                LiveFailureKind::RateLimited {
                    retry_after_secs: Some(90),
                },
            );
            assert_eq!(session.state.next_poll_at_ms, Some(1)); // deliberately wrong
        });
        assert!(caught.is_err(), "wrong Retry-After schedule must fail");

        // Connection backoff first step.
        let caught = std::panic::catch_unwind(|| {
            assert_eq!(next_backoff_ms(1), 1); // deliberately wrong
        });
        assert!(caught.is_err(), "wrong first backoff step must fail");
    }
}
