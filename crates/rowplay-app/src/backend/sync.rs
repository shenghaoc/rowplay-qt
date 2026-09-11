// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Sync` QML singleton: runs `rowplay_platform::sync` on a worker
//! thread and marshals progress back to the Qt thread.
//!
//! Threading (Phase 4 ground rule 3): the coordinator never runs on the Qt
//! thread. `start()` spawns a `std::thread`; progress and completion events
//! travel over an `mpsc` channel; the worker pokes the Qt event loop through
//! qtbridge's `QmlMethodInvoker` (a queued `QMetaObject::invokeMethod`, see
//! docs/qt-bridges-notes.md), which calls the `pumpEvents` slot here. A QML
//! `Timer` polls `pumpEvents` while a sync runs as a safety net for dropped
//! pokes. Cancel flips the coordinator's `AtomicBool`.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::{QObjectHolder, QmlMethodInvoker, QmlRegister};
use rowplay_core::privacy::redact;
use rowplay_platform::concept2::MockConcept2Client;
use rowplay_platform::concept2::http::Concept2HttpClient;
use rowplay_platform::sync::{SyncProgress, WorkoutSyncCoordinator, WorkoutSyncError};
use rowplay_platform::token_store::SecretToken;
use rowplay_platform::workout_cache::WorkoutCache;
use rowplay_viewmodel::dates::fmt_date;

use crate::backend::AppState;

/// What the worker thread hands back to the Qt thread.
enum SyncEvent {
    Progress {
        completed: usize,
        total: usize,
        /// Pre-rendered "completed/total · remaining" line (formatting stays
        /// in Rust; the worker builds it off the Qt thread).
        text: String,
    },
    Finished {
        outcome: Result<FinishedSync, String>,
    },
}

/// A finished (or cancelled) sync, already reduced to display data.
struct FinishedSync {
    added: i64,
    cancelled: bool,
}

/// Backend for the settings-screen sync section.
pub struct SyncBackend {
    is_running: bool,
    can_sync: bool,
    progress_completed: i64,
    progress_total: i64,
    progress_fraction: f64,
    progress_text: String,
    /// Locale message id of the status line ("" = none yet).
    status_id: String,
    /// `{added}` for `sync.done`.
    status_added: i64,
    /// `{total}` for `sync.done` / `settings.lastSync`.
    status_total: i64,
    /// `{date}` for `settings.lastSync`, formatted by the view-model.
    status_date: String,
    /// `{message}` for `sync.errorHint` — always redacted.
    status_message: String,
    cancel_flag: Option<Arc<AtomicBool>>,
    events: Option<Receiver<SyncEvent>>,
    /// Last completion, for `lastSyncText` after a restart-free reload.
    last_finished: Option<chrono::DateTime<chrono::Utc>>,
    last_counts: Arc<Mutex<(i64, Option<String>)>>,
}

impl Default for SyncBackend {
    fn default() -> Self {
        let state = AppState::get();
        let total = state
            .cache
            .list_workouts()
            .map_or(0, |workouts| workouts.len() as i64);
        SyncBackend {
            is_running: false,
            can_sync: false,
            progress_completed: 0,
            progress_total: 0,
            progress_fraction: 0.0,
            progress_text: String::new(),
            status_id: String::new(),
            status_added: 0,
            status_total: total,
            status_date: String::new(),
            status_message: String::new(),
            cancel_flag: None,
            events: None,
            last_finished: None,
            last_counts: Arc::new(Mutex::new((total, None))),
        }
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
impl SyncBackend {
    qproperty!("isRunning", Member = is_running, Notify = sync_changed);
    // True with a stored token, demo mode off and no sync running.
    qproperty!("canSync", Member = can_sync, Notify = sync_changed);
    qproperty!(
        "progressCompleted",
        Member = progress_completed,
        Notify = sync_changed
    );
    qproperty!(
        "progressTotal",
        Member = progress_total,
        Notify = sync_changed
    );
    // 0.0–1.0, or -1 while the summary pass has not sized the detail pass.
    qproperty!(
        "progressFraction",
        Member = progress_fraction,
        Notify = sync_changed
    );
    qproperty!("statusId", Member = status_id, Notify = sync_changed);
    qproperty!("statusAdded", Member = status_added, Notify = sync_changed);
    qproperty!("statusTotal", Member = status_total, Notify = sync_changed);
    qproperty!("statusDate", Member = status_date, Notify = sync_changed);
    // Redacted error detail for `sync.errorHint`; never token material.
    qproperty!(
        "statusMessage",
        Member = status_message,
        Notify = sync_changed
    );

    #[qsignal]
    fn sync_changed(&mut self);

    /// Starts a full sync on the worker thread. No-op while one is running.
    #[qslot]
    fn start(&mut self) {
        self.refresh_can_sync();
        if self.is_running || !self.can_sync {
            return;
        }
        let state = AppState::get();
        let mock = std::env::var_os("ROWPLAY_SYNC_MOCK").is_some();
        let token = if mock {
            None
        } else {
            match state.token_store.load() {
                Ok(Some(token)) => Some(token),
                Ok(None) => {
                    "token.empty".clone_into(&mut self.status_id);
                    self.sync_changed();
                    return;
                }
                Err(error) => {
                    "sync.failed".clone_into(&mut self.status_id);
                    self.status_message = redact(&error.to_string());
                    self.sync_changed();
                    return;
                }
            }
        };

        let (sender, receiver) = channel::<SyncEvent>();
        let cancel = Arc::new(AtomicBool::new(false));
        self.events = Some(receiver);
        self.cancel_flag = Some(Arc::clone(&cancel));
        self.is_running = true;
        self.progress_completed = 0;
        self.progress_total = 0;
        self.progress_fraction = -1.0;
        "sync.inProgress".clone_into(&mut self.status_id);
        self.status_message.clear();
        self.refresh_can_sync();
        self.sync_changed();

        // The invoker is created on the Qt thread; the queued call lands back
        // here on `pumpEvents`. The Rc keeps nothing alive across threads —
        // only the invoker (Send) and the channel sender move.
        let invoker = self.get_qml_method_invoker();
        let cache = Arc::clone(&state.cache);
        let counts = Arc::clone(&self.last_counts);
        std::thread::Builder::new()
            .name("rowplay-sync".to_owned())
            .spawn(move || {
                run_sync(token, &cache, &cancel, &sender, &invoker, &counts);
            })
            .expect("spawn the sync worker");
    }

    /// Asks the worker to stop at the next detail boundary.
    #[qslot]
    fn cancel(&mut self) {
        if let Some(flag) = &self.cancel_flag {
            flag.store(true, Ordering::Relaxed);
        }
    }

    /// Drains the worker's event channel. Called through the invoker's queued
    /// connection and, as a safety net, by a QML timer while `isRunning`.
    #[qslot]
    fn pump_events(&mut self) {
        // Drain first, then process: the receiver lives in `self`, so it is
        // taken out for the non-borrowing drain and put back afterwards.
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

        for event in drained {
            match event {
                SyncEvent::Progress {
                    completed,
                    total,
                    text,
                } => {
                    self.progress_completed = completed as i64;
                    self.progress_total = total as i64;
                    self.progress_fraction = if total == 0 {
                        -1.0
                    } else {
                        (completed as f64 / total as f64).clamp(0.0, 1.0)
                    };
                    self.progress_text = text;
                    self.sync_changed();
                }
                SyncEvent::Finished { outcome } => {
                    self.is_running = false;
                    self.cancel_flag = None;
                    self.events = None;
                    self.progress_fraction = 0.0;
                    match outcome {
                        Ok(finished) => {
                            let state = AppState::get();
                            let prefs = state.prefs();
                            let language = state.language();
                            self.last_finished = Some(chrono::Utc::now());
                            "sync.done".clone_into(&mut self.status_id);
                            self.status_added = finished.added;
                            self.status_total = self.last_counts.lock().expect("sync counts").0;
                            // Display dates come from the view-model (ground
                            // rule 2): the instant is formatted in Rust.
                            self.status_date = fmt_date(
                                &rowplay_core::datetime::instant_iso_from_epoch_millis(
                                    self.last_finished.map_or(0, |at| at.timestamp_millis()),
                                ),
                                language,
                                prefs.home_timezone.as_deref(),
                            );
                            if finished.cancelled {
                                // A cancelled sync reports its partial counts
                                // like Studio does; the web has no
                                // cancelled-sync string.
                                self.status_message.clear();
                            }
                        }
                        Err(message) => {
                            "sync.failed".clone_into(&mut self.status_id);
                            self.status_message = message;
                        }
                    }
                    self.refresh_can_sync();
                    self.sync_changed();
                }
            }
        }

        if disconnected && self.is_running {
            // The worker died without a Finished event: surface it.
            self.is_running = false;
            self.cancel_flag = None;
            self.events = None;
            "sync.failed".clone_into(&mut self.status_id);
            self.status_message = redact("sync worker disconnected");
            self.refresh_can_sync();
            self.sync_changed();
        }
    }

    /// Recomputes `canSync` from the shared state. QML calls this when
    /// `Settings.settingsChanged` fires (token/demo-mode changes).
    #[qslot]
    fn refresh(&mut self) {
        self.refresh_can_sync();
        self.sync_changed();
    }
}

impl SyncBackend {
    fn refresh_can_sync(&mut self) {
        let state = AppState::get();
        let prefs = state.prefs();
        let mock = std::env::var_os("ROWPLAY_SYNC_MOCK").is_some();
        self.can_sync = (state.has_token() || mock) && !prefs.demo_mode_enabled && !self.is_running;
    }
}

/// The worker body: blocking Concept2 sync off the Qt thread.
fn run_sync(
    token: Option<SecretToken>,
    cache: &Arc<dyn WorkoutCache>,
    cancel: &AtomicBool,
    sender: &Sender<SyncEvent>,
    invoker: &QmlMethodInvoker,
    counts: &Mutex<(i64, Option<String>)>,
) {
    let finish = |outcome: Result<FinishedSync, String>| {
        let _ = sender.send(SyncEvent::Finished { outcome });
        invoker.invoke_method("pumpEvents");
    };

    if let Some(token) = token {
        let client = match Concept2HttpClient::new(token) {
            Ok(client) => client,
            Err(error) => {
                finish(Err(redact(&error.to_string())));
                return;
            }
        };
        run_coordinator(&client, cache, cancel, sender, invoker, counts, finish);
    } else {
        // ROWPLAY_SYNC_MOCK: the deterministic mock serving the demo
        // library; exercises the full worker path without a token.
        let client = MockConcept2Client::new(rowplay_core::demo::demo_details());
        run_coordinator(&client, cache, cancel, sender, invoker, counts, finish);
    }
}

fn run_coordinator(
    client: &dyn rowplay_platform::concept2::Concept2Client,
    cache: &Arc<dyn WorkoutCache>,
    cancel: &AtomicBool,
    sender: &Sender<SyncEvent>,
    invoker: &QmlMethodInvoker,
    counts: &Mutex<(i64, Option<String>)>,
    finish: impl FnOnce(Result<FinishedSync, String>),
) {
    let coordinator = WorkoutSyncCoordinator::new(client, cache.as_ref());
    let started = std::time::Instant::now();
    let result = coordinator.sync_with(cancel, &mut |progress| {
        let text = progress_text(progress, started.elapsed().as_secs_f64());
        if sender
            .send(SyncEvent::Progress {
                completed: progress.completed,
                total: progress.total,
                text,
            })
            .is_ok()
        {
            invoker.invoke_method("pumpEvents");
        }
    });

    match result {
        Ok(outcome) => {
            if let Ok(workouts) = cache.list_workouts() {
                counts.lock().expect("sync counts").0 = workouts.len() as i64;
            }
            finish(Ok(FinishedSync {
                added: outcome.result.saved_count as i64,
                cancelled: outcome.cancelled,
            }));
        }
        Err(WorkoutSyncError::ClientFailed(message)) => finish(Err(redact(&message))),
        Err(error) => finish(Err(redact(&error.to_string()))),
    }
}

/// "completed/total · remaining" with the remaining time estimated from the
/// elapsed rate (core `fmt_time`; QML never formats numbers).
fn progress_text(progress: SyncProgress, elapsed_secs: f64) -> String {
    let counts = format!("{}/{}", progress.completed, progress.total);
    if progress.completed == 0 || progress.total <= progress.completed {
        return counts;
    }
    let per_item = elapsed_secs / progress.completed as f64;
    let remaining = per_item * (progress.total - progress.completed) as f64;
    format!(
        "{counts} · {}",
        rowplay_core::formatting::fmt_time(remaining, false)
    )
}

// Manual registration keeps the `RowPlay` URI (qt-bridges-notes #1).
impl QmlRegister for SyncBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Sync";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
