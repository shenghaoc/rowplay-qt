// SPDX-License-Identifier: GPL-3.0-or-later
//! Detail-column navigation state — a port of rowplay-studio's
//! `Views/DetailNavigationState.swift`.
//!
//! Keeping the route stack, the replay-availability policy and the
//! selection-reset behaviour in one pure value makes the sidebar/replay
//! interaction deterministic and directly testable, exactly like Studio.

use rowplay_core::models::WorkoutDetail;

/// A pushed route in the detail column.
///
/// Phase 4 only ever pushes [`Route::Replay`] (the replay screen itself is
/// Phase 5; the route exists so the navigation policy is already parity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    /// The replay screen for one workout.
    Replay {
        /// The workout to replay.
        workout_id: i64,
    },
}

/// Why the Replay command cannot run right now.
///
/// The menu item's disabled state and the runtime push consult the same
/// policy, so the enabled state a user sees always matches what invoking the
/// command actually does (Studio's invariant).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayUnavailability {
    /// The library is still loading or syncing.
    LibraryLoading,
    /// No workout is selected (the dashboard is showing).
    NoSelection,
    /// The selected workout has no stroke data.
    NoStrokeData,
    /// The replay screen is already on the stack.
    AlreadyPresented,
}

/// Navigation state for the detail column.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DetailNavigationState {
    /// The pushed route stack.
    pub path: Vec<Route>,
}

impl DetailNavigationState {
    /// A fresh state with an empty stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True while any route is presented.
    #[must_use]
    pub fn is_replay_presented(&self) -> bool {
        !self.path.is_empty()
    }

    /// Pushes the replay route for `workout_id` unconditionally.
    pub fn show_replay(&mut self, workout_id: i64) {
        self.path.push(Route::Replay { workout_id });
    }

    /// The reason the replay command is unavailable, or `None` when it can run.
    #[must_use]
    pub fn replay_unavailability(
        &self,
        selected_detail: Option<&WorkoutDetail>,
        is_library_loading: bool,
    ) -> Option<ReplayUnavailability> {
        if is_library_loading {
            return Some(ReplayUnavailability::LibraryLoading);
        }
        if self.is_replay_presented() {
            return Some(ReplayUnavailability::AlreadyPresented);
        }
        let Some(selected_detail) = selected_detail else {
            return Some(ReplayUnavailability::NoSelection);
        };
        if !selected_detail.workout.has_stroke_data {
            return Some(ReplayUnavailability::NoStrokeData);
        }
        None
    }

    /// Pushes the replay route for whichever workout is selected at the moment
    /// the command fires.
    ///
    /// The replay command must not bake a workout ID into UI closures: AppKit
    /// menu and toolbar bridging can keep dispatching an action captured
    /// during an earlier render pass, which replays the previously selected
    /// workout. Callers pass the freshly resolved selection instead. Returns
    /// the pushed route, or `None` when
    /// [`replay_unavailability`](Self::replay_unavailability) reports the
    /// command is unavailable.
    pub fn show_replay_for(
        &mut self,
        selected_detail: Option<&WorkoutDetail>,
        is_library_loading: bool,
    ) -> Option<Route> {
        if self
            .replay_unavailability(selected_detail, is_library_loading)
            .is_some()
        {
            return None;
        }
        let workout_id = selected_detail?.id();
        self.show_replay(workout_id);
        self.path.last().copied()
    }

    /// Drops every pushed route (Studio: selection changes reset the stack).
    pub fn reset_for_selection_change(&mut self) {
        self.path.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use rowplay_core::demo::demo_details;
    use rowplay_core::models::Sport;

    use super::*;

    /// Demo lookup mirroring Studio's `WorkoutLibrary.detail(id:)`.
    fn demo_by_id() -> HashMap<i64, WorkoutDetail> {
        demo_details().into_iter().map(|d| (d.id(), d)).collect()
    }

    /// Demo IDs used by the Studio repro: 2000m test (rower), 1000m SkiErg,
    /// 8000m BikeErg and the stroke-less 5000m steady row.
    const ROWER_WORKOUT_ID: i64 = 1001;
    const SKIERG_WORKOUT_ID: i64 = 1003;
    const BIKE_WORKOUT_ID: i64 = 1004;
    const STROKELESS_WORKOUT_ID: i64 = 9001;

    #[test]
    fn replay_action_routes_selected_workout() {
        let mut navigation = DetailNavigationState::new();
        navigation.show_replay(42);
        assert_eq!(navigation.path, [Route::Replay { workout_id: 42 }]);
    }

    /// The core Studio repro: a command closure created while workout A was
    /// selected must still replay workout B after the selection changes,
    /// because stale closures are exactly what bridged menu/toolbar actions
    /// dispatch. Re-expressed from `ReplayNavigationTests`.
    #[test]
    fn stale_replay_command_targets_newly_selected_workout() {
        let demo = demo_by_id();
        let mut navigation = DetailNavigationState::new();
        let mut selected: Option<i64> = Some(ROWER_WORKOUT_ID);
        // The "stale command" exists before the loop, while the rower test is
        // selected — it must not capture that id.
        assert!(demo.contains_key(&selected.unwrap_or_default()));

        let selection_sequence = [
            (SKIERG_WORKOUT_ID, Sport::Skierg),
            (BIKE_WORKOUT_ID, Sport::Bike),
            (ROWER_WORKOUT_ID, Sport::Rower),
        ];
        for (workout_id, sport) in selection_sequence {
            selected = Some(workout_id);
            // Mirrors ContentView's onChange(of: selectedWorkoutID).
            navigation.reset_for_selection_change();

            // The "stale closure": resolves the live selection when invoked.
            let detail = selected.and_then(|id| demo.get(&id));
            navigation.show_replay_for(detail, false);

            assert_eq!(
                navigation.path,
                [Route::Replay { workout_id }],
                "replay command must target the newly selected workout"
            );
            assert_eq!(demo[&workout_id].workout.sport, sport);
        }
    }

    #[test]
    fn replay_command_returns_pushed_route_for_current_selection() {
        let demo = demo_by_id();
        let mut navigation = DetailNavigationState::new();
        let route = navigation.show_replay_for(demo.get(&SKIERG_WORKOUT_ID), false);
        assert_eq!(
            route,
            Some(Route::Replay {
                workout_id: SKIERG_WORKOUT_ID
            })
        );
        assert_eq!(
            navigation.path,
            [Route::Replay {
                workout_id: SKIERG_WORKOUT_ID
            }]
        );
    }

    /// The enabled state the user sees must match what invoking does.
    #[test]
    fn replay_unavailability_matches_push_outcome() {
        type Case<'a> = (
            &'a str,
            Option<&'a WorkoutDetail>,
            bool,
            &'a DetailNavigationState,
            Option<ReplayUnavailability>,
        );

        let demo = demo_by_id();
        let mut presented = DetailNavigationState::new();
        presented.show_replay(1);

        let cases: [Case; 5] = [
            (
                "available",
                demo.get(&ROWER_WORKOUT_ID),
                false,
                &DetailNavigationState::new(),
                None,
            ),
            (
                "loading",
                demo.get(&ROWER_WORKOUT_ID),
                true,
                &DetailNavigationState::new(),
                Some(ReplayUnavailability::LibraryLoading),
            ),
            (
                "already presented",
                demo.get(&ROWER_WORKOUT_ID),
                false,
                &presented,
                Some(ReplayUnavailability::AlreadyPresented),
            ),
            (
                "no selection",
                None,
                false,
                &DetailNavigationState::new(),
                Some(ReplayUnavailability::NoSelection),
            ),
            (
                "no stroke data",
                demo.get(&STROKELESS_WORKOUT_ID),
                false,
                &DetailNavigationState::new(),
                Some(ReplayUnavailability::NoStrokeData),
            ),
        ];

        for (name, detail, loading, state, expected) in cases {
            assert_eq!(
                state.replay_unavailability(detail, loading),
                expected,
                "case {name}"
            );
            let mut state = state.clone();
            let pushed = state.show_replay_for(detail, loading);
            assert_eq!(
                pushed.is_some(),
                expected.is_none(),
                "push outcome must match the policy for {name}"
            );
        }
    }

    #[test]
    fn selection_change_resets_the_stack() {
        let mut navigation = DetailNavigationState::new();
        navigation.show_replay(7);
        assert!(navigation.is_replay_presented());
        navigation.reset_for_selection_change();
        assert!(!navigation.is_replay_presented());
        assert!(navigation.path.is_empty());
    }
}
