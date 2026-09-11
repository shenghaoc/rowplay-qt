// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Detail` QML singleton: the selected workout's header data.
//!
//! Phase 4a exposes the shell-level minimum (selection, type, sport, date);
//! the metric strip, splits table and stroke-analysis series land in 4b on
//! top of `rowplay_viewmodel::detail` / `::strokes`.

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::models::WorkoutDetail;
use rowplay_viewmodel::dates::fmt_date;

use crate::backend::AppState;

/// Studio's dashboard pseudo-selection (`ContentView.dashboardSelectionID`).
const DASHBOARD_SELECTION: i64 = -1;

/// Backend for the workout-detail screen.
pub struct DetailBackend {
    selected_workout_id: i64,
    has_selection: bool,
    workout_type: String,
    sport_name: String,
    /// Locale short date, formatted by the view-model (never in QML).
    date_text: String,
    source_text: String,
    is_interval: bool,
    has_comments: bool,
    comments: String,
    has_stroke_data: bool,
}

impl Default for DetailBackend {
    fn default() -> Self {
        DetailBackend {
            selected_workout_id: DASHBOARD_SELECTION,
            has_selection: false,
            workout_type: String::new(),
            sport_name: String::new(),
            date_text: String::new(),
            source_text: String::new(),
            is_interval: false,
            has_comments: false,
            comments: String::new(),
            has_stroke_data: false,
        }
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
impl DetailBackend {
    qproperty!(
        "selectedWorkoutId",
        Member = selected_workout_id,
        Notify = detail_changed
    );
    qproperty!(
        "hasSelection",
        Member = has_selection,
        Notify = detail_changed
    );
    qproperty!(
        "workoutType",
        Member = workout_type,
        Notify = detail_changed
    );
    qproperty!("sportName", Member = sport_name, Notify = detail_changed);
    qproperty!("dateText", Member = date_text, Notify = detail_changed);
    qproperty!("sourceText", Member = source_text, Notify = detail_changed);
    qproperty!("isInterval", Member = is_interval, Notify = detail_changed);
    qproperty!(
        "hasComments",
        Member = has_comments,
        Notify = detail_changed
    );
    qproperty!("comments", Member = comments, Notify = detail_changed);
    qproperty!(
        "hasStrokeData",
        Member = has_stroke_data,
        Notify = detail_changed
    );

    #[qsignal]
    fn detail_changed(&mut self);

    /// Points the detail screen at a workout; `-1` clears it. QML calls this
    /// from `Library.selectionChanged` so both singletons stay in step.
    #[qslot]
    fn select_workout(&mut self, id: i64) {
        self.selected_workout_id = id;
        self.apply_selection();
        self.detail_changed();
    }

    /// Re-reads the shared snapshot (after `Library.reload()`).
    #[qslot]
    fn refresh(&mut self) {
        self.apply_selection();
        self.detail_changed();
    }
}

impl DetailBackend {
    fn apply_selection(&mut self) {
        let state = AppState::get();
        let details = state.details();
        let selected = details
            .iter()
            .find(|detail| detail.id() == self.selected_workout_id);
        self.has_selection = selected.is_some();
        let (prefs, language) = (state.prefs(), state.language());
        if let Some(detail) = selected {
            self.fill(detail, language, prefs.home_timezone.as_deref());
        } else {
            self.workout_type.clear();
            self.sport_name.clear();
            self.date_text.clear();
            self.source_text.clear();
            self.is_interval = false;
            self.has_comments = false;
            self.comments.clear();
            self.has_stroke_data = false;
        }
    }

    fn fill(
        &mut self,
        detail: &WorkoutDetail,
        language: rowplay_viewmodel::settings::Language,
        home_timezone: Option<&str>,
    ) {
        let workout = &detail.workout;
        self.workout_type = workout
            .workout_type
            .clone()
            .unwrap_or_else(|| "workout".to_owned());
        workout
            .sport
            .display_name()
            .clone_into(&mut self.sport_name);
        self.date_text = fmt_date(&workout.date, language, home_timezone);
        self.source_text = workout.source.clone().unwrap_or_default();
        self.is_interval = workout.is_interval;
        self.comments = workout.comments.clone().unwrap_or_default();
        self.has_comments = !self.comments.is_empty();
        self.has_stroke_data = workout.has_stroke_data;
    }
}

// Manual registration keeps the `RowPlay` URI (qt-bridges-notes #1).
impl QmlRegister for DetailBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Detail";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
