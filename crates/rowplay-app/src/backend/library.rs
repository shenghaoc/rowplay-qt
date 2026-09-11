// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Library` QML singleton: workout set, list query, selection and the
//! detail-column navigation state.
//!
//! Phase 4a exposes the shell-level state (counts, filters, selection,
//! navigation routes); the sidebar's `QListModel` and the per-row display
//! strings land in 4b on top of `rowplay_viewmodel::library`.

use std::collections::BTreeSet;

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::models::{Sport, Workout, WorkoutDetail};
use rowplay_core::workout_query::{WorkoutListQuery, filter_and_sort_workouts, pb_workout_ids};
use rowplay_platform::library::{WorkoutLibrarySource, load_library};
use rowplay_viewmodel::nav::{DetailNavigationState, ReplayUnavailability};

use crate::backend::AppState;

/// Studio's dashboard pseudo-selection (`ContentView.dashboardSelectionID`).
const DASHBOARD_SELECTION: i64 = -1;

/// Backend for the library sidebar and shell navigation.
pub struct LibraryBackend {
    details: Vec<WorkoutDetail>,
    /// Summary rows parallel to `details` (cheap clones) for the query engine.
    summaries: Vec<Workout>,
    query: WorkoutListQuery,
    pb_ids: BTreeSet<i64>,
    nav_state: DetailNavigationState,
    selected_workout_id: i64,
    filtered_count: i64,
    total_count: i64,
    is_demo_library: bool,
    is_empty: bool,
    cache_error: String,
    sport_names: Vec<String>,
    sport_filter_index: i32,
    search_text: String,
    is_replay_presented: bool,
    replay_workout_id: i64,
    /// Why a replay request was refused: 0 = none, 1 = library loading,
    // 2 = no selection, 3 = no stroke data, 4 = already presented
    /// (`ReplayUnavailability` as an int for the bridge).
    replay_block_reason: i32,
}

impl Default for LibraryBackend {
    fn default() -> Self {
        let sport_names = [Sport::Rower, Sport::Skierg, Sport::Bike]
            .iter()
            .map(|sport| sport.display_name().to_owned())
            .collect();
        let mut backend = LibraryBackend {
            details: Vec::new(),
            summaries: Vec::new(),
            query: WorkoutListQuery::default(),
            pb_ids: BTreeSet::new(),
            nav_state: DetailNavigationState::new(),
            selected_workout_id: DASHBOARD_SELECTION,
            filtered_count: 0,
            total_count: 0,
            is_demo_library: false,
            is_empty: true,
            cache_error: String::new(),
            sport_names,
            sport_filter_index: 0,
            search_text: String::new(),
            is_replay_presented: false,
            replay_workout_id: DASHBOARD_SELECTION,
            replay_block_reason: 0,
        };
        backend.reload_data();
        backend
    }
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
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
        self.recompute_filtered();
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
        self.recompute_filtered();
        self.library_changed();
    }

    /// Pushes the replay route for the current selection, resolving the
    // selection at invocation time (Studio's stale-closure fix). Phase 5
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
        let _ = prefs;
        // Publish for the `Detail` singleton (one snapshot, two readers).
        state.set_details(self.details.clone());
        self.summaries = self.details.iter().map(|d| d.workout.clone()).collect();
        self.pb_ids = pb_workout_ids(&self.summaries, self.query.sport);
        self.total_count = self.summaries.len() as i64;
        self.is_empty = self.details.is_empty();
        self.recompute_filtered();
        // The selection may have vanished (e.g. cache replaced demo data).
        if self.selected_workout_id != DASHBOARD_SELECTION && self.selected_detail().is_none() {
            self.selected_workout_id = DASHBOARD_SELECTION;
        }
        self.nav_state.reset_for_selection_change();
        self.sync_nav();
    }

    fn recompute_filtered(&mut self) {
        let filtered = filter_and_sort_workouts(&self.summaries, &self.query, Some(&self.pb_ids));
        self.filtered_count = filtered.len() as i64;
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
