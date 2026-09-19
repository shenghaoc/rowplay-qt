// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Live` QML singleton: logbook page-1 polling on a worker thread.
//!
//! Mirrors [`crate::backend::sync`]: `LiveSession` decides cadence on the Qt
//! thread; `poll_recent` (or a demo no-op) runs off-thread; results travel
//! over `mpsc` and a `QmlMethodInvoker` poke drains them via `pumpEvents`. A
//! QML `Timer` (~50 ms while polling, ~1 s for `tick`) is the safety net.
//!
//! Live mode is logbook polling, not PM5 / Bluetooth / FTMS. "New" means a
//! result **id** absent from the known set — never a count or position.

use std::collections::BTreeSet;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::{QObjectHolder, QmlMethodInvoker, QmlRegister};
use rowplay_platform::concept2::http::Concept2HttpClient;
use rowplay_platform::live::{poll_recent, LivePollError, LivePollResult};
use rowplay_platform::token_store::SecretToken;
use rowplay_platform::workout_cache::WorkoutCache;
use rowplay_viewmodel::dates::fmt_time_from_epoch_millis;
use rowplay_viewmodel::live::{
    parse_interval, InstantSource, LiveAction, LiveFailureKind, LiveSession, SystemClock,
    DEFAULT_LIVE_INTERVAL_SEC,
};

use crate::backend::AppState;

/// What the worker thread hands back to the Qt thread.
enum LiveEvent {
    Finished {
        outcome: Result<LivePollResult, LivePollError>,
        /// `false` after a manual refresh so `next_poll_at` stays put.
        reschedule: bool,
    },
}

/// Backend for the settings-screen live-mode panel.
pub struct LiveBackend {
    enabled: bool,
    polling: bool,
    /// Locale message id of the status line ("" = clear / idle).
    status_id: String,
    /// Wall-clock of the last completed poll, formatted in Rust.
    last_poll_text: String,
    has_warning: bool,
    /// Consecutive failures (for `liveMode.warning` `{count}`).
    failure_count: i64,
    interval_sec: i32,
    can_enable: bool,

    session: LiveSession,
    events: Option<Receiver<LiveEvent>>,
}

impl Default for LiveBackend {
    fn default() -> Self {
        let state = AppState::get();
        let prefs = state.prefs();
        let interval = parse_interval(prefs.live_interval_sec);
        let mut session = LiveSession {
            demo: prefs.demo_mode_enabled,
            ..LiveSession::default()
        };
        session.state.interval_sec = interval;
        if let Ok(workouts) = state.cache.list_workouts() {
            session.known_ids = workouts.into_iter().map(|w| w.id).collect();
        }

        let can_enable = state.has_token() || prefs.demo_mode_enabled;
        let mut backend = LiveBackend {
            enabled: false,
            polling: false,
            status_id: String::new(),
            last_poll_text: String::new(),
            has_warning: false,
            failure_count: 0,
            interval_sec: i32::try_from(interval).unwrap_or(DEFAULT_LIVE_INTERVAL_SEC as i32),
            can_enable,
            session,
            events: None,
        };

        // Restore the persisted enable flag; the first `tick` starts the poll
        // once the QObject (and invoker) is fully wired.
        if prefs.live_mode_enabled && can_enable {
            let now = SystemClock.now_ms();
            backend.session.enable(now);
            backend.enabled = true;
        }
        backend.sync_from_session();
        backend
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
impl LiveBackend {
    qproperty!("enabled", Member = enabled, Notify = live_changed);
    qproperty!("polling", Member = polling, Notify = live_changed);
    qproperty!("statusId", Member = status_id, Notify = live_changed);
    qproperty!(
        "lastPollText",
        Member = last_poll_text,
        Notify = live_changed
    );
    qproperty!("hasWarning", Member = has_warning, Notify = live_changed);
    qproperty!(
        "failureCount",
        Member = failure_count,
        Notify = live_changed
    );
    qproperty!("intervalSec", Member = interval_sec, Notify = live_changed);
    // True with a stored token or demo mode. Syncing does not block enable
    // (live polls page 1 only and never touches `fully_synced`).
    qproperty!("canEnable", Member = can_enable, Notify = live_changed);

    #[qsignal]
    fn live_changed(&mut self);

    /// Emitted when a poll saved new workouts: the shell re-reads the library.
    #[qsignal(qml_name = "libraryRefreshRequested")]
    fn library_refresh_requested(&mut self);

    /// Enable or disable live mode and persist the preference.
    #[qslot]
    fn set_enabled(&mut self, on: bool) {
        self.refresh_can_enable();
        if on {
            if !self.can_enable {
                return;
            }
            let now = SystemClock.now_ms();
            self.session.demo = AppState::get().prefs().demo_mode_enabled;
            self.seed_known_ids();
            self.session.enable(now);
            self.enabled = true;
            self.status_id.clear();
        } else {
            self.session.disable();
            self.enabled = false;
            self.polling = false;
            self.status_id.clear();
        }
        let _ = AppState::get().update_prefs(|prefs| {
            prefs.live_mode_enabled = on && self.can_enable;
        });
        self.sync_from_session();
        self.live_changed();
    }

    /// Change the poll interval (one of 30 / 60 / 120 / 300) and persist.
    #[qslot]
    fn set_interval(&mut self, sec: i32) {
        let sec = parse_interval(u32::try_from(sec).unwrap_or(DEFAULT_LIVE_INTERVAL_SEC));
        self.session.state.interval_changed(sec);
        self.interval_sec = i32::try_from(sec).unwrap_or(DEFAULT_LIVE_INTERVAL_SEC as i32);
        let _ = AppState::get().update_prefs(|prefs| {
            prefs.live_interval_sec = sec;
        });
        // Match the web: an interval change while enabled schedules soon.
        if self.session.state.enabled && !self.polling {
            let now = SystemClock.now_ms();
            self.session.state.tick_scheduled(now);
        }
        self.sync_from_session();
        self.live_changed();
    }

    /// Manual refresh: poll now without moving `next_poll_at`.
    #[qslot]
    fn refresh(&mut self) {
        match self.session.manual_refresh() {
            LiveAction::StartPoll => self.start_poll_worker(false),
            LiveAction::None | LiveAction::SignedOut => {}
        }
        self.sync_from_session();
        self.live_changed();
    }

    /// Called by a QML `Timer` (~1 s): start a poll when due.
    #[qslot]
    fn tick(&mut self) {
        self.refresh_can_enable();
        self.session.demo = AppState::get().prefs().demo_mode_enabled;
        let now = SystemClock.now_ms();
        match self.session.on_tick(now) {
            LiveAction::StartPoll => self.start_poll_worker(true),
            LiveAction::SignedOut => {
                "liveMode.reauth".clone_into(&mut self.status_id);
                let _ = AppState::get().update_prefs(|prefs| {
                    prefs.live_mode_enabled = false;
                });
            }
            LiveAction::None => {}
        }
        self.sync_from_session();
        self.live_changed();
    }

    /// Drains the worker's event channel. Queued from the invoker and, as a
    /// safety net, by a QML timer while `polling`.
    #[qslot]
    fn pump_events(&mut self) {
        let Some(receiver) = self.events.take() else {
            return;
        };
        let mut drained = Vec::new();
        let mut disconnected = false;
        loop {
            match receiver.try_recv() {
                Ok(event) => drained.push(event),
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    disconnected = true;
                    break;
                }
            }
        }
        if !disconnected {
            self.events = Some(receiver);
        }

        let now = SystemClock.now_ms();
        for event in drained {
            match event {
                LiveEvent::Finished {
                    outcome,
                    reschedule,
                } => {
                    self.polling = false;
                    self.events = None;
                    match outcome {
                        Ok(result) => {
                            self.session.on_poll_ok(now, &result.new_ids, reschedule);
                            self.status_id.clear();
                            self.update_last_poll_text(now);
                            if !result.new_ids.is_empty() {
                                self.library_refresh_requested();
                            }
                        }
                        Err(LivePollError::Unauthorized) => {
                            let _ = self.session.on_poll_err(now, LiveFailureKind::Unauthorized);
                            "liveMode.reauth".clone_into(&mut self.status_id);
                            self.update_last_poll_text(now);
                            let _ = AppState::get().update_prefs(|prefs| {
                                prefs.live_mode_enabled = false;
                            });
                        }
                        Err(LivePollError::RateLimited { retry_after_secs }) => {
                            let _ = self.session.on_poll_err(
                                now,
                                LiveFailureKind::RateLimited { retry_after_secs },
                            );
                            "liveMode.rateLimit".clone_into(&mut self.status_id);
                            self.update_last_poll_text(now);
                        }
                        Err(_) => {
                            let _ = self.session.on_poll_err(now, LiveFailureKind::Transient);
                            // Prefer the retry string: every transient failure
                            // reschedules. `hasWarning` surfaces the ≥3 case.
                            if self.session.state.has_warning() {
                                "liveMode.error".clone_into(&mut self.status_id);
                            } else {
                                "liveMode.errorRetry".clone_into(&mut self.status_id);
                            }
                            self.update_last_poll_text(now);
                        }
                    }
                    self.sync_from_session();
                    self.live_changed();
                }
            }
        }

        if disconnected && self.polling {
            self.polling = false;
            self.events = None;
            let _ = self
                .session
                .on_poll_err(SystemClock.now_ms(), LiveFailureKind::Transient);
            "liveMode.error".clone_into(&mut self.status_id);
            self.sync_from_session();
            self.live_changed();
        }
    }
}

impl LiveBackend {
    fn refresh_can_enable(&mut self) {
        let state = AppState::get();
        let prefs = state.prefs();
        self.can_enable = state.has_token() || prefs.demo_mode_enabled;
    }

    fn seed_known_ids(&mut self) {
        if let Ok(workouts) = AppState::get().cache.list_workouts() {
            for workout in workouts {
                self.session.known_ids.insert(workout.id);
            }
        }
    }

    fn sync_from_session(&mut self) {
        self.enabled = self.session.state.enabled;
        self.polling = self.session.state.status
            == rowplay_viewmodel::live::LiveModeStatus::Polling;
        self.has_warning = self.session.state.has_warning();
        self.failure_count = i64::from(self.session.state.consecutive_failures);
        self.interval_sec = i32::try_from(self.session.state.interval_sec)
            .unwrap_or(DEFAULT_LIVE_INTERVAL_SEC as i32);
        if let Some(at) = self.session.state.last_poll_at_ms {
            self.update_last_poll_text(at);
        }
    }

    fn update_last_poll_text(&mut self, epoch_ms: i64) {
        let state = AppState::get();
        let prefs = state.prefs();
        let language = state.language();
        self.last_poll_text = fmt_time_from_epoch_millis(
            epoch_ms as f64,
            language,
            prefs.home_timezone.as_deref(),
        );
    }

    /// Spawn the worker. `reschedule` is what `on_poll_ok` will receive.
    fn start_poll_worker(&mut self, reschedule: bool) {
        if self.events.is_some() {
            // A worker is already draining; ignore a duplicate start.
            return;
        }
        self.polling = true;
        self.sync_from_session();
        self.live_changed();

        let state = AppState::get();
        let demo = self.session.demo || state.prefs().demo_mode_enabled;
        let known_ids = self.session.known_ids.clone();
        let (sender, receiver) = channel::<LiveEvent>();
        self.events = Some(receiver);

        let invoker = self.get_qml_method_invoker();
        let cache = Arc::clone(&state.cache);

        if demo {
            // Demo mode: skip HTTP. The web's `generateMockWorkout` port is a
            // follow-up (needs an injected RNG + random 30 s–3 min delay);
            // for now treat the poll as an empty success so the cadence and
            // UI path stay exercisable without inventing a network client.
            std::thread::Builder::new()
                .name("rowplay-live".to_owned())
                .spawn(move || {
                    let _ = known_ids; // demo does not consult ids yet
                    finish(
                        &sender,
                        &invoker,
                        Ok(LivePollResult::default()),
                        reschedule,
                    );
                })
                .expect("spawn the live worker");
            return;
        }

        let token = match state.token_store.load() {
            Ok(Some(token)) => token,
            Ok(None) => {
                // No token and not demo: surface reauth and stop.
                self.events = None;
                self.polling = false;
                let now = SystemClock.now_ms();
                let _ = self.session.on_poll_err(now, LiveFailureKind::Unauthorized);
                "liveMode.reauth".clone_into(&mut self.status_id);
                let _ = state.update_prefs(|prefs| {
                    prefs.live_mode_enabled = false;
                });
                self.sync_from_session();
                self.live_changed();
                return;
            }
            Err(_) => {
                self.events = None;
                self.polling = false;
                let now = SystemClock.now_ms();
                let _ = self.session.on_poll_err(now, LiveFailureKind::Transient);
                "liveMode.error".clone_into(&mut self.status_id);
                self.sync_from_session();
                self.live_changed();
                return;
            }
        };

        std::thread::Builder::new()
            .name("rowplay-live".to_owned())
            .spawn(move || {
                run_live_poll(token, &cache, &known_ids, reschedule, &sender, &invoker);
            })
            .expect("spawn the live worker");
    }
}

fn finish(
    sender: &Sender<LiveEvent>,
    invoker: &QmlMethodInvoker,
    outcome: Result<LivePollResult, LivePollError>,
    reschedule: bool,
) {
    let _ = sender.send(LiveEvent::Finished {
        outcome,
        reschedule,
    });
    invoker.invoke_method("pumpEvents");
}

fn run_live_poll(
    token: SecretToken,
    cache: &Arc<dyn WorkoutCache>,
    known_ids: &BTreeSet<i64>,
    reschedule: bool,
    sender: &Sender<LiveEvent>,
    invoker: &QmlMethodInvoker,
) {
    let client = match Concept2HttpClient::new(token) {
        Ok(client) => client,
        Err(_) => {
            finish(
                sender,
                invoker,
                Err(LivePollError::Client("client init failed".into())),
                reschedule,
            );
            return;
        }
    };
    let outcome = poll_recent(&client, cache.as_ref(), known_ids);
    finish(sender, invoker, outcome, reschedule);
}

// Manual registration keeps the `RowPlay` URI (qt-bridges-notes #1).
impl QmlRegister for LiveBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Live";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
