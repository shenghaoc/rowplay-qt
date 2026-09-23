// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Library` QML singleton: workout set, list query, sidebar list model,
//! dashboard bundle, selection and the detail-column navigation state.
//!
//! The sidebar list is a qtbridge `QListModel` (`Base = QListModel`): one
//! `SidebarRowItem` per visible row, bulk-swapped through `reset()` so a
//! 5,000-workout library re-filters without per-row QObjects (Phase 4 ground
//! rule 4). All display strings and derivations come from
//! `rowplay_viewmodel`; this adapter only shapes them into bridge types.

use std::collections::{BTreeSet, HashMap};

use qtbridge::QModelItem;
use qtbridge::qobject;
use qtbridge::qtbridge_interfaces::{QListModel, QListModelBase};
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::analytics::dashboard_summary;
use rowplay_core::models::{Sport, Workout, WorkoutDetail};
use rowplay_core::workout_query::{
    SortDir, WorkoutListQuery, filter_and_sort_workouts, pb_workout_ids,
};
use rowplay_platform::library::{WorkoutLibrarySource, load_library};
use rowplay_viewmodel::dashboard::{
    by_sport_series, distance_axis_label, pace_axis_labels, pace_chart_sport, pb_cards,
    recent_pace_domain, recent_pace_series, tiles,
};
use rowplay_viewmodel::library::{
    normalize_day_key, sidebar_rows, sort_field_id, sort_fields, toggle_sort,
};
use rowplay_viewmodel::nav::{DetailNavigationState, ReplayUnavailability};

use crate::backend::AppState;

/// Studio's dashboard pseudo-selection (`ContentView.dashboardSelectionID`).
const DASHBOARD_SELECTION: i64 = -1;

/// One sidebar row as the `ListView` sees it. Field names are the QML role
/// names (the `QModelItem` derive uses them verbatim).
#[derive(Clone, Default, QModelItem)]
pub struct SidebarRowItem {
    /// Concept2 result id.
    pub workout_id: i64,
    /// Row title (logbook workout type, else sport name).
    pub title: String,
    /// Locale short date.
    pub date_text: String,
    /// Monitor-local time of day.
    pub time_text: String,
    /// Distance in the preferred unit.
    pub distance_text: String,
    /// Pace.
    pub pace_text: String,
    /// Machine key (`rower` / `skierg` / `bike`).
    pub sport_key: String,
    /// Sport display name (trademark, untranslated).
    pub sport_name: String,
    /// Personal-best badge.
    pub is_pb: bool,
    /// `YYYY-MM-DD` day key.
    pub section: String,
    /// Locale day header.
    pub section_text: String,
    /// First row of its day section.
    pub is_section_start: bool,
    /// Screen-reader text.
    pub accessible_text: String,
}

/// Backend for the library sidebar, dashboard data and shell navigation.
pub struct LibraryBackend {
    details: Vec<WorkoutDetail>,
    /// Summary rows parallel to `details` (cheap clones) for the query engine.
    summaries: Vec<Workout>,
    query: WorkoutListQuery,
    pb_ids: BTreeSet<i64>,
    nav_state: DetailNavigationState,
    rows: Vec<SidebarRowItem>,
    selected_workout_id: i64,
    filtered_count: i64,
    total_count: i64,
    is_demo_library: bool,
    is_empty: bool,
    cache_error: String,
    sport_names: Vec<String>,
    sport_filter_index: i32,
    search_text: String,
    date_from: String,
    date_to: String,
    sort_field_index: i32,
    sort_ascending: bool,
    sort_field_ids: Vec<String>,
    // Dashboard bundle (Studio's library.filtered* properties).
    tiles_json: serde_json::Value,
    pb_cards_json: serde_json::Value,
    sport_bar_labels: Vec<String>,
    sport_bar_values: Vec<f64>,
    pace_series: Vec<f64>,
    pace_date_texts: Vec<String>,
    pace_domain_low: f64,
    pace_domain_high: f64,
    pace_axis_values: Vec<f64>,
    pace_axis_labels: Vec<String>,
    distance_axis: String,
    pace_sport_name: String,
    // Navigation.
    is_replay_presented: bool,
    replay_workout_id: i64,
    /// Why a replay request was refused: 0 = none, 1 = library loading,
    /// 2 = no selection, 3 = no stroke data, 4 = already presented
    /// (`ReplayUnavailability` as an int for the bridge).
    replay_block_reason: i32,
}

impl Default for LibraryBackend {
    fn default() -> Self {
        let sport_names = [Sport::Rower, Sport::Skierg, Sport::Bike]
            .iter()
            .map(|sport| sport.display_name().to_owned())
            .collect();
        let sort_field_ids = sort_fields()
            .iter()
            .map(|field| sort_field_id(*field).to_owned())
            .collect();
        let mut backend = LibraryBackend {
            details: Vec::new(),
            summaries: Vec::new(),
            query: WorkoutListQuery::default(),
            pb_ids: BTreeSet::new(),
            nav_state: DetailNavigationState::new(),
            rows: Vec::new(),
            selected_workout_id: DASHBOARD_SELECTION,
            filtered_count: 0,
            total_count: 0,
            is_demo_library: false,
            is_empty: true,
            cache_error: String::new(),
            sport_names,
            sport_filter_index: 0,
            search_text: String::new(),
            date_from: String::new(),
            date_to: String::new(),
            sort_field_index: 0,
            sort_ascending: false,
            sort_field_ids,
            tiles_json: serde_json::Value::Array(Vec::new()),
            pb_cards_json: serde_json::Value::Array(Vec::new()),
            sport_bar_labels: Vec::new(),
            sport_bar_values: Vec::new(),
            pace_series: Vec::new(),
            pace_date_texts: Vec::new(),
            pace_domain_low: -180.0,
            pace_domain_high: -60.0,
            pace_axis_values: Vec::new(),
            pace_axis_labels: Vec::new(),
            distance_axis: "km".to_owned(),
            pace_sport_name: Sport::Rower.display_name().to_owned(),
            is_replay_presented: false,
            replay_workout_id: DASHBOARD_SELECTION,
            replay_block_reason: 0,
        };
        backend.reload_data();
        backend
    }
}

/// The list-model half: `rows` is swapped wholesale in `rebuild`, then
/// `reset()` tells the view (the macro-generated `QListModelBase::reset`
/// wraps `reset_unnotified` in begin/endResetModel).
impl QListModel for LibraryBackend {
    type Item = SidebarRowItem;

    fn len(&self) -> usize {
        self.rows.len()
    }

    fn get(&self, index: usize) -> Option<&Self::Item> {
        self.rows.get(index)
    }

    fn reset_unnotified(&mut self) {
        // `rebuild` already swapped the data; nothing left to do.
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase, Base = QListModel)]
impl LibraryBackend {
    qproperty!(
        "selectedWorkoutId",
        Member = selected_workout_id,
        Notify = selection_changed
    );
    qproperty!(
        "filteredCount",
        Member = filtered_count,
        Notify = library_changed
    );
    qproperty!("totalCount", Member = total_count, Notify = library_changed);
    qproperty!(
        "isDemoLibrary",
        Member = is_demo_library,
        Notify = library_changed
    );
    // No workouts at all; the shell shows the empty state when this is true
    // *and* demo mode is off (Studio's `library.isEmpty && !demoModeEnabled`).
    qproperty!("isEmpty", Member = is_empty, Notify = library_changed);
    // Redacted cache failure text ("" when healthy); never silently demo.
    qproperty!("cacheError", Member = cache_error, Notify = library_changed);
    // Sport display names (trademark terms, untranslated); index 0 of the
    // picker is the localised "All" entry, which QML prepends.
    qproperty!("sportNames", Member = sport_names, Constant);
    qproperty!(
        "sportFilterIndex",
        Member = sport_filter_index,
        Notify = library_changed
    );
    qproperty!("searchText", Member = search_text, Notify = library_changed);
    qproperty!("dateFrom", Member = date_from, Notify = library_changed);
    qproperty!("dateTo", Member = date_to, Notify = library_changed);
    qproperty!(
        "sortFieldIndex",
        Member = sort_field_index,
        Notify = library_changed
    );
    qproperty!(
        "sortAscending",
        Member = sort_ascending,
        Notify = library_changed
    );
    // Locale ids for the sort menu (web `workoutList.sort*`).
    qproperty!("sortFieldIds", Member = sort_field_ids, Constant);
    qproperty!("tilesJson", Member = tiles_json, Notify = library_changed);
    qproperty!(
        "pbCardsJson",
        Member = pb_cards_json,
        Notify = library_changed
    );
    qproperty!(
        "sportBarLabels",
        Member = sport_bar_labels,
        Notify = library_changed
    );
    qproperty!(
        "sportBarValues",
        Member = sport_bar_values,
        Notify = library_changed
    );
    // Flat [x0,y0,x1,y1,…] pace series for Qt Graphs `replace`.
    qproperty!("paceSeries", Member = pace_series, Notify = library_changed);
    // Locale short date per pace point (x-axis tick labels).
    qproperty!(
        "paceDateTexts",
        Member = pace_date_texts,
        Notify = library_changed
    );
    qproperty!(
        "paceDomainLow",
        Member = pace_domain_low,
        Notify = library_changed
    );
    qproperty!(
        "paceDomainHigh",
        Member = pace_domain_high,
        Notify = library_changed
    );
    qproperty!(
        "paceAxisValues",
        Member = pace_axis_values,
        Notify = library_changed
    );
    qproperty!(
        "paceAxisLabels",
        Member = pace_axis_labels,
        Notify = library_changed
    );
    // Unit symbol for the distance axes ("km" / "mi").
    qproperty!(
        "distanceAxis",
        Member = distance_axis,
        Notify = library_changed
    );
    qproperty!(
        "paceSportName",
        Member = pace_sport_name,
        Notify = library_changed
    );
    qproperty!(
        "isReplayPresented",
        Member = is_replay_presented,
        Notify = selection_changed
    );
    qproperty!(
        "replayWorkoutId",
        Member = replay_workout_id,
        Notify = selection_changed
    );
    qproperty!(
        "replayBlockReason",
        Member = replay_block_reason,
        Notify = selection_changed
    );

    #[qsignal]
    fn library_changed(&mut self);
    #[qsignal]
    fn selection_changed(&mut self);

    /// Re-reads the cache (or demo data) — Studio's "Reload Workout Library".
    #[qslot]
    fn reload(&mut self) {
        self.reload_data();
        self.notify_model_reset();
        self.library_changed();
        self.selection_changed();
    }

    /// Selects a workout; `-1` returns to the dashboard. Selecting resets the
    /// route stack like Studio's `onChange(of: selectedWorkoutID)`.
    #[qslot]
    fn select_workout(&mut self, id: i64) {
        if self.selected_workout_id == id {
            return;
        }
        self.selected_workout_id = id;
        self.nav_state.reset_for_selection_change();
        self.sync_nav();
        self.selection_changed();
    }

    /// Escape key / hidden Studio button: back to the dashboard.
    #[qslot]
    fn clear_selection(&mut self) {
        self.select_workout(DASHBOARD_SELECTION);
    }

    /// 0 = All, then one index per `sportNames` entry.
    #[qslot]
    fn set_sport_filter(&mut self, index: i32) {
        self.sport_filter_index = index;
        self.query.sport = match index {
            1 => Some(Sport::Rower),
            2 => Some(Sport::Skierg),
            3 => Some(Sport::Bike),
            _ => None,
        };
        // Studio recomputes the PB set when the sport filter changes.
        self.pb_ids = pb_workout_ids(&self.summaries, self.query.sport);
        self.rebuild();
        self.notify_model_reset();
        self.library_changed();
    }

    #[qslot]
    fn set_search_text(&mut self, text: String) {
        self.search_text.clone_from(&text);
        self.query.q = if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
        self.rebuild();
        self.notify_model_reset();
        self.library_changed();
    }

    /// Date-range filter; empty strings clear a bound. Returns false when a
    /// non-empty value is not a `YYYY-MM-DD` key (the field marks itself
    /// invalid; the bound keeps its previous value).
    #[qslot]
    fn set_date_range(&mut self, from: String, to: String) -> bool {
        let (Some(from_key), Some(to_key)) = (normalize_day_key(&from), normalize_day_key(&to))
        else {
            return false;
        };
        self.query.date_from = from_key;
        self.query.date_to = to_key;
        // Consume the raw field text for the echo properties.
        self.date_from = from;
        self.date_to = to;
        self.rebuild();
        self.notify_model_reset();
        self.library_changed();
        true
    }

    /// Studio's sort menu: same field flips direction, new field starts
    /// descending (pace/time ascending).
    #[qslot]
    fn toggle_sort(&mut self, field_index: i32) {
        let fields = sort_fields();
        let Some(field) = fields.get(field_index as usize) else {
            return;
        };
        self.query = toggle_sort(&self.query, *field);
        self.sort_field_index = field_index;
        self.sort_ascending = self.query.dir == SortDir::Asc;
        self.rebuild();
        self.notify_model_reset();
        self.library_changed();
    }

    /// Preference changes (unit/language/timezone) re-render every string.
    #[qslot]
    fn refresh(&mut self) {
        self.rebuild();
        self.notify_model_reset();
        self.library_changed();
    }

    /// Pushes the replay route for the current selection, resolving the
    /// selection at invocation time (Studio's stale-closure fix). Phase 5
    /// renders the route; the policy is already parity.
    #[qslot]
    fn request_replay(&mut self, is_library_loading: bool) {
        let detail = self.selected_detail();
        let reason = self
            .nav_state
            .replay_unavailability(detail, is_library_loading);
        let workout_id = detail.map(WorkoutDetail::id);
        self.replay_block_reason = match reason {
            None => {
                // The policy check passed: push by id so the detail borrow is
                // released before nav_state is mutated again.
                if let Some(id) = workout_id {
                    self.nav_state.show_replay(id);
                }
                0
            }
            Some(ReplayUnavailability::LibraryLoading) => 1,
            Some(ReplayUnavailability::NoSelection) => 2,
            Some(ReplayUnavailability::NoStrokeData) => 3,
            Some(ReplayUnavailability::AlreadyPresented) => 4,
        };
        self.sync_nav();
        self.selection_changed();
    }

    /// Pops the replay route (back navigation).
    #[qslot]
    fn close_replay(&mut self) {
        self.nav_state.reset_for_selection_change();
        self.sync_nav();
        self.selection_changed();
    }
}

impl LibraryBackend {
    fn reload_data(&mut self) {
        let state = AppState::get();
        let prefs = state.prefs();
        self.cache_error = state.cache_error.clone().unwrap_or_default();
        match load_library(state.cache.as_ref(), prefs.demo_mode_enabled) {
            Ok(snapshot) => {
                self.is_demo_library = snapshot.source == WorkoutLibrarySource::Demo;
                self.details = snapshot.details;
                if self.is_demo_library {
                    // Studio: a demo reload reselects the default demo workout.
                    self.selected_workout_id = rowplay_core::demo::DEFAULT_WORKOUT_ID;
                }
            }
            Err(error) => {
                // Cache failures never silently fall back to demo data.
                self.cache_error = rowplay_core::privacy::redact(&error.to_string());
                self.details.clear();
                self.is_demo_library = false;
            }
        }
        // Publish for the `Detail` singleton (one snapshot, two readers).
        state.set_details(self.details.clone());
        self.summaries = self.details.iter().map(|d| d.workout.clone()).collect();
        self.pb_ids = pb_workout_ids(&self.summaries, self.query.sport);
        self.total_count = self.summaries.len() as i64;
        self.is_empty = self.details.is_empty();
        // The selection may have vanished (e.g. cache replaced demo data).
        if self.selected_workout_id != DASHBOARD_SELECTION && self.selected_detail().is_none() {
            self.selected_workout_id = DASHBOARD_SELECTION;
        }
        self.nav_state.reset_for_selection_change();
        self.sync_nav();
        self.rebuild();
    }

    /// Recomputes the filtered rows and the dashboard bundle from the
    /// current summaries/query/preferences. Pure data: no Qt notifications.
    fn rebuild(&mut self) {
        let state = AppState::get();
        let prefs = state.prefs();
        let language = state.language();
        let unit = prefs.preferred_distance_unit;
        let tz = prefs.home_timezone.as_deref();

        let rendered = sidebar_rows(
            &self.summaries,
            &self.query,
            &self.pb_ids,
            unit,
            language,
            tz,
        );
        self.filtered_count = rendered.len() as i64;
        self.rows = rendered
            .into_iter()
            .map(|row| SidebarRowItem {
                workout_id: row.id,
                title: row.title,
                date_text: row.date_text,
                time_text: row.time_text,
                distance_text: row.distance_text,
                pace_text: row.pace_text,
                sport_key: row.sport_key.to_owned(),
                sport_name: row.sport_name.to_owned(),
                is_pb: row.is_pb,
                section: row.section,
                section_text: row.section_text,
                is_section_start: row.is_section_start,
                accessible_text: row.accessible_text,
            })
            .collect();

        // Dashboard bundle over the *filtered* set (Studio's filtered*
        // properties feed DashboardView the same way).
        let filtered: Vec<Workout> =
            filter_and_sort_workouts(&self.summaries, &self.query, Some(&self.pb_ids))
                .into_iter()
                .cloned()
                .collect();
        let summary = dashboard_summary(&filtered);
        self.tiles_json = serde_json::to_value(
            tiles(&summary, unit)
                .into_iter()
                .map(|tile| {
                    serde_json::json!({
                        "labelId": tile.label_id,
                        "valueText": tile.value_text,
                        "role": tile.role.index(),
                        "accessibleValue": tile.accessible_value,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();
        self.pb_cards_json = serde_json::to_value(
            pb_cards(&filtered, self.query.sport, unit, language, tz)
                .into_iter()
                .map(|card| {
                    serde_json::json!({
                        "id": card.id,
                        "label": card.label,
                        "sportName": card.sport_name,
                        "sportKey": card.sport_key,
                        "timeText": card.time_text,
                        "paceText": card.pace_text,
                        "dateText": card.date_text,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();

        let bars = by_sport_series(&summary, unit);
        self.sport_bar_labels = bars.iter().map(|bar| bar.sport_name.to_owned()).collect();
        self.sport_bar_values = bars.iter().map(|bar| bar.value).collect();
        distance_axis_label(unit).clone_into(&mut self.distance_axis);

        let pace_sport = pace_chart_sport(&filtered, self.query.sport);
        pace_sport
            .display_name()
            .clone_into(&mut self.pace_sport_name);
        let pace_points = recent_pace_series(&filtered, pace_sport, language, tz);
        self.pace_series = pace_points
            .iter()
            .flat_map(|point| [point.x, point.y])
            .collect();
        // One locale date per point: the chart's X range and its tick labels
        // both come from this list (it was never filled, so the axis spanned
        // only the first point and showed no dates).
        self.pace_date_texts = pace_points
            .iter()
            .map(|point| point.date_text.clone())
            .collect();
        let domain = recent_pace_domain(&filtered);
        self.pace_domain_low = domain.0;
        self.pace_domain_high = domain.1;
        let axis = pace_axis_labels(domain, 4);
        self.pace_axis_values = axis.iter().map(|(value, _)| *value).collect();
        self.pace_axis_labels = axis.into_iter().map(|(_, label)| label).collect();
    }

    /// `reset()` panics when the QObject is not attached yet (the singleton
    /// is constructed during QML type resolution, before any view binds);
    /// data-only rebuilds skip the notification then.
    fn notify_model_reset(&mut self) {
        use qtbridge::QObjectHolder;
        if self.try_get_rust_proxy_ptr().is_some() {
            self.reset();
        }
    }

    fn selected_detail(&self) -> Option<&WorkoutDetail> {
        if self.selected_workout_id == DASHBOARD_SELECTION {
            return None;
        }
        self.details
            .iter()
            .find(|detail| detail.id() == self.selected_workout_id)
    }

    fn sync_nav(&mut self) {
        self.is_replay_presented = self.nav_state.is_replay_presented();
        self.replay_workout_id = match self.nav_state.path.last() {
            Some(rowplay_viewmodel::nav::Route::Replay { workout_id }) => *workout_id,
            None => DASHBOARD_SELECTION,
        };
    }
}

// Manual registration keeps the `RowPlay` URI (qt-bridges-notes #1).
impl QmlRegister for LibraryBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Library";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
