// SPDX-License-Identifier: GPL-3.0-or-later
//! Process-wide composition root for the QML backends.
//!
//! qtbridge creates QML singletons through `Default::default()` when QML first
//! resolves them, so the platform services they share cannot be handed in
//! through a constructor: they live in one lazily-initialised `AppState`
//! (Studio's `Concept2SyncController` + `AppPreferences` + library wiring,
//! which Phase 3 deferred to this layer).
//!
//! Privacy invariants: the Concept2 token never crosses the Qt bridge — only
//! the `has_token` flag does — and every user-data string that can reach QML
//! or a log line goes through `rowplay_core::privacy::redact`.

pub mod detail;
pub mod library;
pub mod settings;
pub mod sync;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use rowplay_core::models::WorkoutDetail;
use rowplay_core::privacy::redact;
use rowplay_platform::preferences::{
    FilePreferencesStore, InMemoryPreferencesStore, Preferences, PreferencesStore,
};
use rowplay_platform::token_store::{InMemoryTokenStore, KeyringTokenStore, TokenStore};
use rowplay_platform::workout_cache::{InMemoryWorkoutCache, SqliteWorkoutCache, WorkoutCache};
use rowplay_viewmodel::settings::Language;

static APP_STATE: OnceLock<AppState> = OnceLock::new();

/// The services shared by every QML backend singleton.
pub struct AppState {
    /// Durable preferences (atomic JSON file; in-memory when unwritable).
    pub prefs_store: Arc<dyn PreferencesStore>,
    /// OS keychain token store (in-memory mock when the keyring is unusable).
    pub token_store: Arc<dyn TokenStore>,
    /// Workout cache; `cache_error` explains why it is unusable, if it is.
    pub cache: Arc<dyn WorkoutCache>,
    /// Redacted reason the cache could not be opened (`None` when healthy).
    pub cache_error: Option<String>,
    /// Live preferences copy, mutated only on the Qt thread.
    prefs: Mutex<Preferences>,
    /// Mirrors `token_store.load()` so QML only ever sees a bool.
    has_token: AtomicBool,
    /// The loaded workout library, shared by the `Library` and `Detail`
    /// singletons (written by `Library.reload`, read by `Detail`).
    details: Mutex<Arc<Vec<WorkoutDetail>>>,
}

impl AppState {
    /// The process-wide state, opening every service on first use.
    pub fn get() -> &'static AppState {
        APP_STATE.get_or_init(AppState::open)
    }

    fn open() -> AppState {
        let prefs_store: Arc<dyn PreferencesStore> = match FilePreferencesStore::open_default() {
            Ok(store) => Arc::new(store),
            Err(error) => {
                eprintln!(
                    "preferences unavailable ({}); using an in-memory store",
                    redact(&error.to_string())
                );
                Arc::new(InMemoryPreferencesStore::default())
            }
        };
        let prefs = prefs_store.load().unwrap_or_else(|error| {
            eprintln!(
                "preferences unreadable ({}); using defaults",
                redact(&error.to_string())
            );
            Preferences::default()
        });

        let token_store: Arc<dyn TokenStore> = match KeyringTokenStore::new() {
            Ok(store) => Arc::new(store),
            Err(error) => {
                eprintln!(
                    "OS keychain unavailable ({}); tokens will not persist",
                    redact(&error.to_string())
                );
                Arc::new(InMemoryTokenStore::default())
            }
        };
        let has_token = matches!(token_store.load(), Ok(Some(_)));

        let (cache, cache_error): (Arc<dyn WorkoutCache>, Option<String>) =
            match SqliteWorkoutCache::open_default() {
                Ok(store) => (Arc::new(store), None),
                Err(error) => {
                    let message = redact(&error.to_string());
                    eprintln!("workout cache unavailable ({message}); workouts will not persist");
                    (Arc::new(InMemoryWorkoutCache::default()), Some(message))
                }
            };

        AppState {
            prefs_store,
            token_store,
            cache,
            cache_error,
            prefs: Mutex::new(prefs),
            has_token: AtomicBool::new(has_token),
            details: Mutex::new(Arc::new(Vec::new())),
        }
    }

    /// The current preferences snapshot.
    #[must_use]
    pub fn prefs(&self) -> Preferences {
        self.prefs.lock().expect("preferences lock").clone()
    }

    /// Applies `update` to the live preferences and persists them.
    pub fn update_prefs(&self, update: impl FnOnce(&mut Preferences)) -> Result<(), String> {
        let next = {
            let mut prefs = self.prefs.lock().expect("preferences lock");
            update(&mut prefs);
            prefs.clone()
        };
        self.prefs_store
            .save(&next)
            .map_err(|error| redact(&error.to_string()))
    }

    /// Whether a Concept2 token is stored. QML only ever sees this flag.
    #[must_use]
    pub fn has_token(&self) -> bool {
        self.has_token.load(Ordering::Relaxed)
    }

    /// Re-reads the keyring and returns the new flag.
    pub fn refresh_has_token(&self) -> bool {
        let has = matches!(self.token_store.load(), Ok(Some(_)));
        self.has_token.store(has, Ordering::Relaxed);
        has
    }

    /// The active UI language (English fallback like the web).
    #[must_use]
    pub fn language(&self) -> Language {
        Language::from_preference(self.prefs().language.as_deref())
    }

    /// The current library snapshot (cheap `Arc` clone).
    #[must_use]
    pub fn details(&self) -> Arc<Vec<WorkoutDetail>> {
        Arc::clone(&self.details.lock().expect("details lock"))
    }

    /// Publishes a freshly loaded library snapshot.
    pub fn set_details(&self, details: Vec<WorkoutDetail>) {
        *self.details.lock().expect("details lock") = Arc::new(details);
    }
}
