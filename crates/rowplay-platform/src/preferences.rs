// SPDX-License-Identifier: GPL-3.0-or-later
//! Durable user preferences. Never holds credentials.

use std::sync::Mutex;

use rowplay_core::models::DistanceUnit;
use serde::{Deserialize, Serialize};

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
}
