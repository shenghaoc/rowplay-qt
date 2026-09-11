// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Detail` QML singleton: the selected workout's header, metric strip,
//! splits/intervals table, targets and stroke-analysis series.
//!
//! Every string and series is rendered by `rowplay_viewmodel::{detail,
//! strokes}` (Studio's statics ported); this adapter only shapes them into
//! bridge types. Stroke series are flat `[x0,y0,…]` vectors so Qt Graphs
//! loads them with one `replace` — never point by point.

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::models::WorkoutDetail;
use rowplay_viewmodel::detail::{
    header, metric_strip, split_column_ids, split_rows, splits_section_id, target_rows,
};
use rowplay_viewmodel::strokes::{
    STROKE_CHART_LIMIT, distance_axis_label, downsample_strokes, hr_series_segments, overview,
    pace_chart_domain, pace_series, power_series, rate_series, split_boundary_distances,
};

use crate::backend::AppState;

/// Studio's dashboard pseudo-selection (`ContentView.dashboardSelectionID`).
const DASHBOARD_SELECTION: i64 = -1;

/// Backend for the workout-detail screen.
pub struct DetailBackend {
    selected_workout_id: i64,
    has_selection: bool,
    // Header.
    workout_type: String,
    sport_name: String,
    date_text: String,
    time_text: String,
    source_text: String,
    is_interval: bool,
    has_comments: bool,
    comments: String,
    has_stroke_data: bool,
    header_accessible: String,
    // Metric strip / splits / targets (read-only JSON arrays).
    strip_json: serde_json::Value,
    splits_json: serde_json::Value,
    targets_json: serde_json::Value,
    splits_section_id: String,
    split_column_ids: Vec<String>,
    // Stroke analysis.
    has_strokes: bool,
    stroke_count: i64,
    split_count: i64,
    average_pace_text: String,
    average_watts_text: String,
    peak_watts_text: String,
    pace_series: Vec<f64>,
    power_series: Vec<f64>,
    rate_series: Vec<f64>,
    hr_segments_json: serde_json::Value,
    pace_domain_low: f64,
    pace_domain_high: f64,
    split_boundaries: Vec<f64>,
    distance_axis: String,
}

impl Default for DetailBackend {
    fn default() -> Self {
        DetailBackend {
            selected_workout_id: DASHBOARD_SELECTION,
            has_selection: false,
            workout_type: String::new(),
            sport_name: String::new(),
            date_text: String::new(),
            time_text: String::new(),
            source_text: String::new(),
            is_interval: false,
            has_comments: false,
            comments: String::new(),
            has_stroke_data: false,
            header_accessible: String::new(),
            strip_json: serde_json::Value::Array(Vec::new()),
            splits_json: serde_json::Value::Array(Vec::new()),
            targets_json: serde_json::Value::Array(Vec::new()),
            splits_section_id: "replay.splitBreakdown".to_owned(),
            split_column_ids: split_column_ids()
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
            has_strokes: false,
            stroke_count: 0,
            split_count: 0,
            average_pace_text: String::new(),
            average_watts_text: String::new(),
            peak_watts_text: String::new(),
            pace_series: Vec::new(),
            power_series: Vec::new(),
            rate_series: Vec::new(),
            hr_segments_json: serde_json::Value::Array(Vec::new()),
            pace_domain_low: -180.0,
            pace_domain_high: -60.0,
            split_boundaries: Vec::new(),
            distance_axis: "km".to_owned(),
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
    // Locale short date, formatted by the view-model (never in QML).
    qproperty!("dateText", Member = date_text, Notify = detail_changed);
    qproperty!("timeText", Member = time_text, Notify = detail_changed);
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
    qproperty!(
        "headerAccessible",
        Member = header_accessible,
        Notify = detail_changed
    );
    qproperty!("stripJson", Member = strip_json, Notify = detail_changed);
    qproperty!("splitsJson", Member = splits_json, Notify = detail_changed);
    qproperty!(
        "targetsJson",
        Member = targets_json,
        Notify = detail_changed
    );
    qproperty!(
        "splitsSectionId",
        Member = splits_section_id,
        Notify = detail_changed
    );
    qproperty!("splitColumnIds", Member = split_column_ids, Constant);
    qproperty!("hasStrokes", Member = has_strokes, Notify = detail_changed);
    qproperty!(
        "strokeCount",
        Member = stroke_count,
        Notify = detail_changed
    );
    qproperty!("splitCount", Member = split_count, Notify = detail_changed);
    qproperty!(
        "averagePaceText",
        Member = average_pace_text,
        Notify = detail_changed
    );
    qproperty!(
        "averageWattsText",
        Member = average_watts_text,
        Notify = detail_changed
    );
    qproperty!(
        "peakWattsText",
        Member = peak_watts_text,
        Notify = detail_changed
    );
    // Flat [x0,y0,…] series in chart units (km/mi), downsampled to 500 points.
    qproperty!("paceSeries", Member = pace_series, Notify = detail_changed);
    qproperty!(
        "powerSeries",
        Member = power_series,
        Notify = detail_changed
    );
    qproperty!("rateSeries", Member = rate_series, Notify = detail_changed);
    qproperty!(
        "hrSegmentsJson",
        Member = hr_segments_json,
        Notify = detail_changed
    );
    qproperty!(
        "paceDomainLow",
        Member = pace_domain_low,
        Notify = detail_changed
    );
    qproperty!(
        "paceDomainHigh",
        Member = pace_domain_high,
        Notify = detail_changed
    );
    qproperty!(
        "splitBoundaries",
        Member = split_boundaries,
        Notify = detail_changed
    );
    qproperty!(
        "distanceAxis",
        Member = distance_axis,
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

    /// Re-reads the shared snapshot (after `Library.reload()`) and re-renders
    /// with the current preferences (after unit/language/timezone changes).
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
        match selected {
            Some(detail) => {
                let prefs = state.prefs();
                let language = state.language();
                let tz = prefs.home_timezone.as_deref();
                let unit = prefs.preferred_distance_unit;
                self.fill(detail, language, tz, unit);
            }
            None => self.clear(),
        }
    }

    fn fill(
        &mut self,
        detail: &WorkoutDetail,
        language: rowplay_viewmodel::settings::Language,
        home_timezone: Option<&str>,
        unit: rowplay_core::models::DistanceUnit,
    ) {
        let workout = &detail.workout;
        let header = header(detail, language, home_timezone);
        self.workout_type = header.title;
        header.sport_name.clone_into(&mut self.sport_name);
        self.date_text = header.date_text;
        self.time_text = header.time_text;
        self.source_text = header.source_text;
        self.is_interval = header.is_interval;
        self.comments = header.comments;
        self.has_comments = !self.comments.is_empty();
        self.has_stroke_data = workout.has_stroke_data;
        self.header_accessible = header.accessible_text;

        self.strip_json = serde_json::to_value(
            metric_strip(detail, unit)
                .into_iter()
                .map(|metric| {
                    serde_json::json!({
                        "labelId": metric.label_id,
                        "valueText": metric.value_text,
                        "role": metric.role.index(),
                        "accessibleLabelId": metric.accessible_label_id,
                        "accessibleValue": metric.accessible_value,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        self.splits_json = serde_json::to_value(
            split_rows(detail, unit)
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "index": row.index,
                        "distanceText": row.distance_text,
                        "timeText": row.time_text,
                        "paceText": row.pace_text,
                        "cadenceText": row.cadence_text,
                        "powerText": row.power_text,
                        "hrText": row.hr_text,
                        "isRest": row.is_rest,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        self.targets_json = serde_json::to_value(
            target_rows(detail)
                .into_iter()
                .map(|row| {
                    serde_json::json!({
                        "labelId": row.label_id,
                        "valueText": row.value_text,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        splits_section_id(detail.workout.is_interval).clone_into(&mut self.splits_section_id);

        // Stroke analysis over the downsampled chart sample (Studio's cache).
        let sampled = downsample_strokes(&detail.strokes, STROKE_CHART_LIMIT);
        let summary = overview(&detail.strokes, &detail.splits);
        self.has_strokes = summary.has_strokes;
        self.stroke_count = summary.count;
        self.split_count = summary.split_count;
        self.average_pace_text = summary.average_pace_text;
        self.average_watts_text = summary.average_watts_text;
        self.peak_watts_text = summary.peak_watts_text;
        self.pace_series = pace_series(&sampled, unit);
        self.power_series = power_series(&sampled, unit);
        self.rate_series = rate_series(&sampled, unit);
        self.hr_segments_json =
            serde_json::to_value(hr_series_segments(&sampled, unit)).unwrap_or_default();
        let domain = pace_chart_domain(&detail.strokes);
        self.pace_domain_low = domain.0;
        self.pace_domain_high = domain.1;
        self.split_boundaries = split_boundary_distances(&detail.splits, unit);
        distance_axis_label(unit).clone_into(&mut self.distance_axis);
    }

    fn clear(&mut self) {
        self.workout_type.clear();
        self.sport_name.clear();
        self.date_text.clear();
        self.time_text.clear();
        self.source_text.clear();
        self.is_interval = false;
        self.has_comments = false;
        self.comments.clear();
        self.has_stroke_data = false;
        self.header_accessible.clear();
        self.strip_json = serde_json::Value::Array(Vec::new());
        self.splits_json = serde_json::Value::Array(Vec::new());
        self.targets_json = serde_json::Value::Array(Vec::new());
        "replay.splitBreakdown".clone_into(&mut self.splits_section_id);
        self.has_strokes = false;
        self.stroke_count = 0;
        self.split_count = 0;
        self.average_pace_text.clear();
        self.average_watts_text.clear();
        self.peak_watts_text.clear();
        self.pace_series.clear();
        self.power_series.clear();
        self.rate_series.clear();
        self.hr_segments_json = serde_json::Value::Array(Vec::new());
        self.pace_domain_low = -180.0;
        self.pace_domain_high = -60.0;
        self.split_boundaries.clear();
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
