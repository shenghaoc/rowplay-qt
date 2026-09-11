// SPDX-License-Identifier: GPL-3.0-or-later
//! Durable user preferences. Never holds credentials.
//!
//! Production storage is [`FilePreferencesStore`]: a JSON file in the platform
//! configuration directory, written atomically (temporary file + rename) and
//! read tolerantly (a missing or corrupt file falls back to defaults).

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use rowplay_core::models::DistanceUnit;
use rowplay_core::privacy::{PrivacySafeLogger, redact};
use serde::{Deserialize, Serialize};

use crate::logging::logger;
use crate::paths;

/// Largest preferences file the store will read, in bytes.
///
/// The file holds a handful of scalars; anything larger is treated as corrupt
/// rather than parsed.
pub const MAX_PREFERENCES_BYTES: u64 = 64 * 1024;

/// Suffix appended to the target file name for the atomic-write temporary.
const TEMP_SUFFIX: &str = ".tmp";

/// Preferences persisted between launches.
///
/// Deliberately has no token field: the Concept2 token lives only in the
/// [`crate::token_store::TokenStore`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Preferences {
    /// Show deterministic demo workouts when the cache is empty.
    pub demo_mode_enabled: bool,
    /// Freeze articulation and camera smoothing in the replay.
    pub reduce_replay_motion: bool,
    /// Distance display unit.
    pub preferred_distance_unit: DistanceUnit,
    /// IANA home time zone for calendar bucketing.
    pub home_timezone: Option<String>,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            demo_mode_enabled: true,
            reduce_replay_motion: false,
            preferred_distance_unit: DistanceUnit::Metric,
            home_timezone: None,
        }
    }
}

/// Preference store failures.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PreferencesError {
    /// The store could not be read or written.
    #[error("preferences unavailable: {0}")]
    Unavailable(String),
    /// Stored data was corrupt; callers should fall back to defaults.
    #[error("preferences corrupt: {0}")]
    Corrupt(String),
}

/// Storage for [`Preferences`].
pub trait PreferencesStore: Send + Sync {
    /// Load preferences, or defaults when nothing is stored.
    fn load(&self) -> Result<Preferences, PreferencesError>;
    /// Persist preferences.
    fn save(&self, preferences: &Preferences) -> Result<(), PreferencesError>;
}

/// Process-local store for tests.
#[derive(Debug, Default)]
pub struct InMemoryPreferencesStore {
    value: Mutex<Option<Preferences>>,
}

impl PreferencesStore for InMemoryPreferencesStore {
    fn load(&self) -> Result<Preferences, PreferencesError> {
        Ok(self
            .value
            .lock()
            .expect("preferences lock")
            .clone()
            .unwrap_or_default())
    }

    fn save(&self, preferences: &Preferences) -> Result<(), PreferencesError> {
        *self.value.lock().expect("preferences lock") = Some(preferences.clone());
        Ok(())
    }
}

/// JSON-file preferences store.
///
/// Writes go to `<name>.tmp` in the same directory and are then renamed over
/// the target, so a crash mid-write can never leave a truncated file. Reads
/// ignore unknown fields and fall back to [`Preferences::default`] for a
/// missing, oversized or corrupt file — the app always starts.
pub struct FilePreferencesStore {
    path: PathBuf,
    logger: PrivacySafeLogger<'static>,
}

impl std::fmt::Debug for FilePreferencesStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilePreferencesStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl FilePreferencesStore {
    /// A store backed by `path`.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        FilePreferencesStore {
            path: path.into(),
            logger: logger("preferences"),
        }
    }

    /// A store at the platform's default configuration path.
    pub fn open_default() -> Result<Self, PreferencesError> {
        let path = paths::default_preferences_path()
            .map_err(|error| PreferencesError::Unavailable(redact(&error.to_string())))?;
        Ok(Self::new(path))
    }

    /// Path of the JSON file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The temporary file the atomic write goes through.
    fn temporary_path(&self) -> PathBuf {
        let name = self.path.file_name().map_or_else(
            || "preferences.json".into(),
            |name| name.to_string_lossy().into_owned(),
        );
        self.path.with_file_name(format!("{name}{TEMP_SUFFIX}"))
    }
}

impl PreferencesStore for FilePreferencesStore {
    fn load(&self) -> Result<Preferences, PreferencesError> {
        let metadata = match std::fs::metadata(&self.path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // First launch: defaults, not an error.
                self.logger.info(
                    "no stored preferences; using defaults",
                    &[&self.path.display() as &dyn std::fmt::Display],
                );
                return Ok(Preferences::default());
            }
            Err(error) => {
                return Err(PreferencesError::Unavailable(redact(&error.to_string())));
            }
        };
        if metadata.len() > MAX_PREFERENCES_BYTES {
            self.logger.warn(
                "stored preferences are implausibly large; using defaults",
                &[&metadata.len() as &dyn std::fmt::Display],
            );
            return Ok(Preferences::default());
        }

        let bytes = match std::fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Err(PreferencesError::Unavailable(redact(&error.to_string())));
            }
        };
        let Ok(preferences) = serde_json::from_slice::<Preferences>(&bytes) else {
            // Never fatal, never echoed: the file may contain anything.
            self.logger.warn(
                "stored preferences could not be decoded; using defaults",
                &[&self.path.display() as &dyn std::fmt::Display],
            );
            return Ok(Preferences::default());
        };
        Ok(preferences)
    }

    fn save(&self, preferences: &Preferences) -> Result<(), PreferencesError> {
        let json = serde_json::to_vec_pretty(preferences)
            .map_err(|error| PreferencesError::Unavailable(redact(&error.to_string())))?;

        if let Some(parent) = self.path.parent() {
            paths::create_private_dir(parent)
                .map_err(|error| PreferencesError::Unavailable(redact(&error.to_string())))?;
        }

        let temporary = self.temporary_path();
        let write = || -> io::Result<()> {
            std::fs::write(&temporary, &json)?;
            paths::restrict_permissions(&temporary)?;
            std::fs::rename(&temporary, &self.path)
        };
        match write() {
            Ok(()) => Ok(()),
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                Err(PreferencesError::Unavailable(redact(&error.to_string())))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_demo_mode_on_and_metric() {
        let prefs = Preferences::default();
        assert!(prefs.demo_mode_enabled);
        assert!(!prefs.reduce_replay_motion);
        assert_eq!(prefs.preferred_distance_unit, DistanceUnit::Metric);
        assert_eq!(prefs.home_timezone, None);
    }

    #[test]
    fn serialises_without_any_token_field() {
        let prefs = Preferences {
            home_timezone: Some("Asia/Singapore".into()),
            ..Preferences::default()
        };
        let json = serde_json::to_string(&prefs).unwrap();
        assert!(!json.to_lowercase().contains("token"));
        let back: Preferences = serde_json::from_str(&json).unwrap();
        assert_eq!(back, prefs);
        // Unknown or missing fields fall back to defaults rather than failing.
        let partial: Preferences =
            serde_json::from_str(r#"{"preferredDistanceUnit":"imperial"}"#).unwrap();
        assert_eq!(partial.preferred_distance_unit, DistanceUnit::Imperial);
        assert!(partial.demo_mode_enabled);
    }

    #[test]
    fn in_memory_store_round_trips() {
        let store = InMemoryPreferencesStore::default();
        assert_eq!(store.load().unwrap(), Preferences::default());
        let prefs = Preferences {
            reduce_replay_motion: true,
            ..Preferences::default()
        };
        store.save(&prefs).unwrap();
        assert_eq!(store.load().unwrap(), prefs);
    }

    fn file_store() -> (tempfile::TempDir, FilePreferencesStore) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = FilePreferencesStore::new(dir.path().join("preferences.json"));
        (dir, store)
    }

    #[test]
    fn a_missing_file_yields_defaults_without_failing() {
        let (_dir, store) = file_store();
        assert_eq!(store.load().unwrap(), Preferences::default());
    }

    #[test]
    fn round_trips_through_the_json_file() {
        let (_dir, store) = file_store();
        let prefs = Preferences {
            demo_mode_enabled: false,
            reduce_replay_motion: true,
            preferred_distance_unit: DistanceUnit::Imperial,
            home_timezone: Some("Asia/Singapore".into()),
        };
        store.save(&prefs).unwrap();
        assert_eq!(store.load().unwrap(), prefs);

        // The temporary file is renamed away, and nothing token-shaped lands in
        // the file.
        assert!(!store.path().with_file_name("preferences.json.tmp").exists());
        let text = std::fs::read_to_string(store.path()).unwrap();
        assert!(!text.to_lowercase().contains("token"), "{text}");

        // A store built on the same path reads the same values.
        let reopened = FilePreferencesStore::new(store.path());
        assert_eq!(reopened.load().unwrap(), prefs);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let (dir, _store) = file_store();
        let path = dir.path().join("preferences.json");
        std::fs::write(
            &path,
            r#"{"demoModeEnabled":false,"futureOption":42,"nested":{"a":1}}"#,
        )
        .unwrap();
        let prefs = FilePreferencesStore::new(&path).load().unwrap();
        assert!(!prefs.demo_mode_enabled);
        assert_eq!(prefs.preferred_distance_unit, DistanceUnit::Metric);
        assert_eq!(prefs.home_timezone, None);
    }

    #[test]
    fn a_corrupt_or_oversized_file_yields_defaults() {
        let (dir, _store) = file_store();
        let path = dir.path().join("preferences.json");
        for content in ["{not json", "", "\u{0}\u{1}\u{2}", "[]", "null"] {
            std::fs::write(&path, content).unwrap();
            assert_eq!(
                FilePreferencesStore::new(&path).load().unwrap(),
                Preferences::default(),
                "{content:?} should fall back to defaults"
            );
        }

        std::fs::write(&path, "x".repeat(MAX_PREFERENCES_BYTES as usize + 1)).unwrap();
        assert_eq!(
            FilePreferencesStore::new(&path).load().unwrap(),
            Preferences::default()
        );
    }

    #[test]
    fn a_failed_write_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("blocker");
        std::fs::write(&blocker, b"not a directory").unwrap();
        let store = FilePreferencesStore::new(blocker.join("preferences.json"));
        assert!(store.save(&Preferences::default()).is_err());
        assert!(store.load().is_err());
    }

    #[test]
    fn saving_creates_the_directory_and_restricts_the_file_on_unix() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("config").join("rowplay-qt");
        let store = FilePreferencesStore::new(nested.join("preferences.json"));
        store.save(&Preferences::default()).unwrap();
        assert!(nested.join("preferences.json").is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            assert_eq!(
                std::fs::metadata(&nested).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                std::fs::metadata(nested.join("preferences.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn preferences_never_serialise_a_token_field() {
        let (_dir, store) = file_store();
        store.save(&Preferences::default()).unwrap();
        let text = std::fs::read_to_string(store.path()).unwrap();
        for forbidden in ["token", "secret", "bearer", "authorization"] {
            assert!(!text.to_lowercase().contains(forbidden), "{text}");
        }
    }
}
