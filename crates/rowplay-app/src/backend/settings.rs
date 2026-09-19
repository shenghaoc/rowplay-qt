// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Settings` QML singleton: preferences, the keyring token flag and the
//! picker option lists.
//!
//! Privacy: the Concept2 token is written straight into a `SecretToken` and
//! the OS keychain; QML only ever sees `hasToken`. The password field's text
//! arrives as a slot argument and is dropped here — it is never stored in a
//! QML-readable property, echoed back, or logged.

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::models::DistanceUnit;
use rowplay_platform::token_store::SecretToken;
use rowplay_viewmodel::settings::{Language, timezone_options, unit_options};

use crate::backend::AppState;

/// Backend for the settings screen (Studio's `SettingsView` state).
pub struct SettingsBackend {
    has_token: bool,
    demo_mode_enabled: bool,
    reduce_replay_motion: bool,
    distance_unit_index: i32,
    language_index: i32,
    language_code: String,
    language_codes: Vec<String>,
    language_names: Vec<String>,
    unit_labels: Vec<String>,
    timezone_values: Vec<String>,
    timezone_labels: Vec<String>,
    timezone_group_ids: Vec<String>,
    timezone_group_starts: Vec<i32>,
    home_timezone_index: i32,
    status_text_id: String,
    gate_mode: bool,
    screenshot_dir: String,
    color_scheme_override: String,
    sync_mock_mode: bool,
    /// Comma-separated `Singleton.member` pairs the gate probes (from
    /// `ROWPLAY_GATE_MEMBER_CHECK`); empty outside the gate.
    gate_member_check: String,
    /// Replay quality tier index (0 Low, 1 Medium, 2 High, 3 Ultra).
    quality_index: i32,
    /// Quality tier labels for the Settings picker.
    quality_labels: Vec<String>,
    /// Bench mode: run 600-tick measurements per sport per tier.
    bench_mode: bool,
    /// Phase-shot mode: capture catch / mid-drive / finish / mid-recovery
    /// per sport with the ghost loaded (T8 visual baseline).
    phase_shots: bool,
    /// Close-up twins for the phase shots: a second, torso-and-hands
    /// framing per capture (the wrist/posture judgement shots, T8).
    phase_closeups: bool,
    /// Quit after this many rendered frames of the shell (`0` = never).
    /// Read in every build, unlike the gate hooks: a packaged binary has
    /// no other way to prove it starts, and an early quit is the one
    /// override that cannot alter or expose data.
    exit_after_frames: i32,
}

impl Default for SettingsBackend {
    fn default() -> Self {
        let state = AppState::get();
        let prefs = state.prefs();

        let languages = Language::all();
        let language = Language::from_preference(prefs.language.as_deref());
        let language_index = languages
            .iter()
            .position(|candidate| *candidate == language)
            .unwrap_or(0) as i32;

        let unit_index = unit_options()
            .iter()
            .position(|(unit, _)| *unit == prefs.preferred_distance_unit)
            .unwrap_or(0) as i32;

        let groups = timezone_options();
        let mut timezone_values = Vec::new();
        let mut timezone_labels = Vec::new();
        let mut timezone_group_ids = Vec::new();
        let mut timezone_group_starts = Vec::new();
        for group in &groups {
            timezone_group_ids.push(group.group_id.to_owned());
            timezone_group_starts.push(timezone_values.len() as i32);
            for option in &group.options {
                timezone_values.push(option.value.to_owned());
                timezone_labels.push(option.label.to_owned());
            }
        }
        // Index 0 is the "UTC (default)" entry (settings.timezoneUtcDefault in
        // QML); curated zones start at 1.
        let home_timezone_index = prefs
            .home_timezone
            .as_deref()
            .and_then(|stored| timezone_values.iter().position(|zone| zone == stored))
            .map_or(0, |index| index as i32 + 1);

        SettingsBackend {
            has_token: state.has_token(),
            demo_mode_enabled: prefs.demo_mode_enabled,
            reduce_replay_motion: prefs.reduce_replay_motion,
            distance_unit_index: unit_index,
            language_index,
            language_code: language.as_str().to_owned(),
            language_codes: languages.iter().map(|l| l.as_str().to_owned()).collect(),
            language_names: languages.iter().map(|l| l.endonym().to_owned()).collect(),
            unit_labels: unit_options()
                .iter()
                .map(|(_, label)| (*label).to_owned())
                .collect(),
            timezone_values,
            timezone_labels,
            timezone_group_ids,
            timezone_group_starts,
            home_timezone_index,
            status_text_id: String::new(),
            // Debug-only like the other hooks: in a release build this
            // variable would make the app walk its screens and quit.
            gate_mode: crate::backend::test_env("ROWPLAY_SMOKE_GATE").is_some(),
            screenshot_dir: std::env::var("ROWPLAY_SMOKE_SCREENSHOT_DIR").unwrap_or_default(),
            color_scheme_override: std::env::var("ROWPLAY_FORCE_COLOR_SCHEME").unwrap_or_default(),
            sync_mock_mode: crate::backend::test_env("ROWPLAY_SYNC_MOCK").is_some(),
            gate_member_check: crate::backend::test_env("ROWPLAY_GATE_MEMBER_CHECK")
                .unwrap_or_default(),
            quality_index: i32::from(prefs.replay_quality.unwrap_or(1)),
            quality_labels: vec![
                "Low".to_owned(),
                "Medium".to_owned(),
                "High".to_owned(),
                "Ultra".to_owned(),
            ],
            bench_mode: crate::backend::test_env("ROWPLAY_REPLAY_BENCH").is_some(),
            phase_shots: crate::backend::test_env("ROWPLAY_PHASE_SHOTS").is_some(),
            phase_closeups: crate::backend::test_env("ROWPLAY_PHASE_CLOSEUPS").is_some(),
            // Deliberately plain `env::var`, not `test_env`: the packaged
            // launch check (tools/package/*) runs the *release* bundle and
            // needs it to exit on its own. Documented in AGENTS.md next to
            // ROWPLAY_DATA_DIR and ROWPLAY_FORCE_COLOR_SCHEME.
            exit_after_frames: std::env::var("ROWPLAY_EXIT_AFTER_FRAMES")
                .ok()
                .and_then(|value| value.parse::<i32>().ok())
                .filter(|frames| *frames > 0)
                .unwrap_or(0),
        }
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
impl SettingsBackend {
    qproperty!("hasToken", Member = has_token, Notify = settings_changed);
    qproperty!(
        "demoModeEnabled",
        Member = demo_mode_enabled,
        Notify = settings_changed
    );
    qproperty!(
        "reduceReplayMotion",
        Member = reduce_replay_motion,
        Notify = settings_changed
    );
    qproperty!(
        "distanceUnitIndex",
        Member = distance_unit_index,
        Notify = settings_changed
    );
    qproperty!(
        "languageIndex",
        Member = language_index,
        Notify = settings_changed
    );
    // The active locale code; `Main.qml` binds `Qt.uiLanguage` to it.
    qproperty!(
        "languageCode",
        Member = language_code,
        Notify = settings_changed
    );
    qproperty!("languageCodes", Member = language_codes, Constant);
    qproperty!("languageNames", Member = language_names, Constant);
    qproperty!("unitLabels", Member = unit_labels, Constant);
    qproperty!("timezoneValues", Member = timezone_values, Constant);
    qproperty!("timezoneLabels", Member = timezone_labels, Constant);
    // Locale message ids for the three group headers, parallel to
    // `timezoneGroupStarts`.
    qproperty!("timezoneGroupIds", Member = timezone_group_ids, Constant);
    // First option index of each group (one extra entry: the list length).
    qproperty!(
        "timezoneGroupStarts",
        Member = timezone_group_starts,
        Constant
    );
    // 0 = UTC default, n+1 = `timezoneValues[n]`.
    qproperty!(
        "homeTimezoneIndex",
        Member = home_timezone_index,
        Notify = settings_changed
    );
    // Locale message id of the last save/disconnect outcome ("" = none).
    qproperty!(
        "statusTextId",
        Member = status_text_id,
        Notify = settings_changed
    );
    // True under the CI runtime-error gate (`ROWPLAY_SMOKE_GATE=1`).
    qproperty!("gateMode", Member = gate_mode, Constant);
    // Where the gate saves per-screen PNGs ("" = off).
    qproperty!("screenshotDir", Member = screenshot_dir, Constant);
    // Test/CI override for the palette: "", "dark" or "light"
    // (ROWPLAY_FORCE_COLOR_SCHEME); empty follows the system.
    qproperty!(
        "colorSchemeOverride",
        Member = color_scheme_override,
        Constant
    );
    // True under ROWPLAY_SYNC_MOCK=1: syncs run against the deterministic
    // MockConcept2Client (demo details) instead of the live Logbook, with no
    // token required. Lets CI exercise the whole worker-thread sync path.
    qproperty!("syncMockMode", Member = sync_mock_mode, Constant);
    // The gate's singleton-property checklist (see Main.qml and
    // crates/rowplay-app/tests/qml_runtime_gate.rs): a missing qtbridge
    // property reads as `undefined` with no QML error, so the runtime gate
    // probes every member QML references and fails on any miss.
    qproperty!("gateMemberCheck", Member = gate_member_check, Constant);
    // Replay quality: 0 Low, 1 Medium, 2 High, 3 Ultra.
    qproperty!(
        "qualityIndex",
        Member = quality_index,
        Notify = settings_changed
    );
    qproperty!("qualityLabels", Member = quality_labels, Constant);
    qproperty!("benchMode", Member = bench_mode, Constant);
    // Phase-shot mode (ROWPLAY_PHASE_SHOTS): stroke-phase captures per
    // sport for the T8 visual baseline.
    qproperty!("phaseShots", Member = phase_shots, Constant);
    // Close-up twins for the phase shots (ROWPLAY_PHASE_CLOSEUPS): each
    // phase grab also saves a torso-and-hands framing (the wrist/posture
    // judgement captures; T8).
    qproperty!("phaseCloseups", Member = phase_closeups, Constant);
    // Packaged-launch probe (ROWPLAY_EXIT_AFTER_FRAMES): Main.qml drives
    // this many frames, then quits with status 0. Release-safe by design;
    // see the field comment.
    qproperty!("exitAfterFrames", Member = exit_after_frames, Constant);

    /// Emitted after any preference or token flag changed.
    #[qsignal]
    fn settings_changed(&mut self);

    #[qslot]
    fn set_demo_mode_enabled(&mut self, enabled: bool) {
        self.apply(|prefs| prefs.demo_mode_enabled = enabled);
        self.demo_mode_enabled = enabled;
        self.settings_changed();
    }

    #[qslot]
    fn set_reduce_replay_motion(&mut self, enabled: bool) {
        self.apply(|prefs| prefs.reduce_replay_motion = enabled);
        self.reduce_replay_motion = enabled;
        self.settings_changed();
    }

    #[qslot]
    fn set_distance_unit_index(&mut self, index: i32) {
        let options = unit_options();
        let Some((unit, _)) = options.get(index as usize) else {
            return;
        };
        let unit = *unit;
        self.apply(|prefs| prefs.preferred_distance_unit = unit);
        self.distance_unit_index = index;
        self.settings_changed();
    }

    #[qslot]
    fn set_language_index(&mut self, index: i32) {
        let languages = Language::all();
        let Some(language) = languages.get(index as usize) else {
            return;
        };
        let code = language.as_str();
        self.apply(|prefs| prefs.language = Some(code.to_owned()));
        self.language_index = index;
        code.clone_into(&mut self.language_code);
        self.settings_changed();
    }

    #[qslot]
    fn set_home_timezone_index(&mut self, index: i32) {
        let zone = if index <= 0 {
            None
        } else {
            self.timezone_values.get(index as usize - 1).cloned()
        };
        let zone_clone = zone.clone();
        self.apply(move |prefs| prefs.home_timezone = zone_clone);
        self.home_timezone_index = index;
        self.settings_changed();
    }

    /// Stores the token in the OS keychain. The raw string exists only as
    // this slot's argument; it is dropped here and never echoed back,
    /// logged or exposed as a property.
    #[qslot]
    fn save_token(&mut self, raw: String) {
        let state = AppState::get();
        let outcome = match SecretToken::new(&raw) {
            Err(_) => "token.rejected".to_owned(),
            Ok(token) => match state.token_store.save(&token) {
                Ok(()) => {
                    self.has_token = state.refresh_has_token();
                    // The web shows no "saved" toast: the header switching to
                    // the Log-out control is the confirmation. Same here.
                    String::new()
                }
                Err(_) => "token.rejected".to_owned(),
            },
        };
        drop(raw); // the slot argument is the only Rust-side copy; consume it
        self.status_text_id = outcome;
        self.settings_changed();
    }

    /// Removes the stored token (the web header's Log out).
    #[qslot]
    fn clear_token(&mut self) {
        let state = AppState::get();
        self.status_text_id = match state.token_store.clear() {
            Ok(()) => {
                self.has_token = state.refresh_has_token();
                String::new()
            }
            Err(_) => "token.rejected".to_owned(),
        };
        self.settings_changed();
    }

    #[qslot]
    fn set_quality_index(&mut self, index: i32) {
        let clamped = index.clamp(0, 3);
        self.apply(|prefs| prefs.replay_quality = Some(clamped as u8));
        self.quality_index = clamped;
        self.settings_changed();
    }

    /// Removes every cached workout (web `settings.deleteAction`; also used
    /// by the CI gate to leave no mock-synced data behind).
    #[qslot]
    fn clear_cached_workouts(&mut self) {
        let state = AppState::get();
        self.status_text_id = match state.cache.clear() {
            Ok(()) => "settings.deleteDone".to_owned(),
            Err(_) => "settings.deleteFailed".to_owned(),
        };
        self.settings_changed();
    }
}

impl SettingsBackend {
    /// The active distance unit, for other backends (4b detail formatting).
    #[allow(dead_code)]
    #[must_use]
    pub fn unit(&self) -> DistanceUnit {
        unit_options()
            .get(self.distance_unit_index as usize)
            .map_or(DistanceUnit::Metric, |(unit, _)| *unit)
    }

    fn apply(&mut self, update: impl FnOnce(&mut rowplay_platform::preferences::Preferences)) {
        if let Err(message) = AppState::get().update_prefs(update) {
            eprintln!("could not persist preferences: {message}");
            "settings.deleteFailed".clone_into(&mut self.status_text_id);
        }
    }
}

// qtbridge derives the module URI from the Cargo package name; the manual
// impl keeps the QML-facing name `RowPlay` (qt-bridges-notes #1).
impl QmlRegister for SettingsBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Settings";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
