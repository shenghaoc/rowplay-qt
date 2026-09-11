// SPDX-License-Identifier: GPL-3.0-or-later
//! Default ghost rival selection (web `replay/ghostPick.ts`).
//!
//! Studio adds `rankedGhostCandidates` with non-finite sanitizers and an
//! id tie-break; the web version is canonical here (stable sort, no extra
//! sanitizer) and Studio's hardening is recorded in `docs/source-map.md`.

use crate::analytics::distance_band;
use crate::models::{Sport, Workout};
use crate::replay::comparability::{
    ComparabilityAxis, ComparableContext, are_comparable, classify_axis,
};

/// Context for selecting a ghost rival workout (web `GhostPickContext`).
#[derive(Debug, Clone, PartialEq)]
pub struct GhostPickContext {
    /// Concept2 result id of the current workout.
    pub id: i64,
    /// Total distance in metres.
    pub distance: f64,
    /// Machine family.
    pub sport: Sport,
    /// Total elapsed seconds (for time-axis comparability).
    pub time: Option<f64>,
    /// Concept2 `workout_type` for axis classification (may be absent).
    pub workout_type: Option<String>,
}

fn comparable_of(workout: &Workout) -> ComparableContext {
    ComparableContext {
        sport: workout.sport,
        distance: workout.distance,
        time: workout.time,
        workout_type: workout.workout_type.clone(),
    }
}

fn comparable_of_current(current: &GhostPickContext) -> ComparableContext {
    ComparableContext {
        sport: current.sport,
        distance: current.distance,
        time: current.time.unwrap_or(0.0),
        workout_type: current.workout_type.clone(),
    }
}

fn num_cmp(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
}

/// Pick a meaningful default ghost rival (web `pickDefaultGhostCandidate`):
/// same comparability band, then closest metres (or closest elapsed time on
/// the time axis), then fastest pace, then most recent session. Ties keep the
/// input order (the web sort is stable).
#[must_use]
pub fn pick_default_ghost_candidate<'a>(
    candidates: &'a [Workout],
    current: &GhostPickContext,
) -> Option<&'a Workout> {
    let current_ctx = comparable_of_current(current);
    let pool: Vec<&Workout> = candidates
        .iter()
        .filter(|c| c.id != current.id && are_comparable(&current_ctx, &comparable_of(c)))
        .collect();
    if pool.is_empty() {
        return None;
    }

    // `pool` is already band-matched by areComparable. For time-axis pieces
    // the shared band is the duration band, so ranking by distance would
    // wrongly drop a comparable session covering a different distance; rank
    // those by closeness in elapsed time instead.
    if classify_axis(current.workout_type.as_deref()) == ComparabilityAxis::Time {
        let target = current.time.unwrap_or(0.0);
        let mut ranked = pool;
        ranked.sort_by(|a, b| {
            num_cmp((a.time - target).abs(), (b.time - target).abs())
                .then_with(|| num_cmp(a.pace, b.pace))
                .then_with(|| b.date.cmp(&a.date))
        });
        return ranked.first().copied();
    }

    let band = distance_band(current.distance);
    let in_band: Vec<&Workout> = pool
        .iter()
        .copied()
        .filter(|c| distance_band(c.distance).key == band.key)
        .collect();
    let mut ranked = if in_band.is_empty() { pool } else { in_band };
    ranked.sort_by(|a, b| {
        num_cmp(
            (a.distance - current.distance).abs(),
            (b.distance - current.distance).abs(),
        )
        .then_with(|| num_cmp(a.pace, b.pace))
        .then_with(|| b.date.cmp(&a.date))
    });
    ranked.first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(
        id: i64,
        distance: f64,
        pace: f64,
        date: &str,
        time: f64,
        workout_type: Option<&str>,
    ) -> Workout {
        let mut workout = Workout::new(id, date, Sport::Rower, distance, time, pace);
        workout.workout_type = workout_type.map(String::from);
        workout.has_stroke_data = true;
        workout
    }

    fn current(id: i64, distance: f64, time: f64, workout_type: Option<&str>) -> GhostPickContext {
        GhostPickContext {
            id,
            distance,
            sport: Sport::Rower,
            time: Some(time),
            workout_type: workout_type.map(String::from),
        }
    }

    #[test]
    fn prefers_same_distance_band_and_closest_metres() {
        let current = current(1, 2000.0, 480.0, Some("2000m test"));
        let candidates = vec![
            w(2, 5000.0, 110.0, "2026-01-01", 600.0, None),
            w(3, 2010.0, 115.0, "2026-02-01", 600.0, None),
            w(4, 10_000.0, 120.0, "2026-03-01", 600.0, None),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(3)
        );
    }

    #[test]
    fn excludes_the_current_workout_id() {
        let current = current(2, 2000.0, 480.0, None);
        let candidates = vec![
            w(2, 2000.0, 110.0, "2026-01-01", 600.0, None),
            w(3, 2005.0, 112.0, "2026-02-01", 600.0, None),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(3)
        );
    }

    #[test]
    fn returns_none_without_comparable_candidates() {
        let current = current(1, 2000.0, 480.0, None);
        let candidates = vec![
            w(2, 5000.0, 110.0, "2026-01-01", 600.0, None),
            w(3, 10_000.0, 120.0, "2026-03-01", 600.0, None),
        ];
        assert!(pick_default_ghost_candidate(&candidates, &current).is_none());
    }

    #[test]
    fn rejects_fixed_time_candidates_for_a_fixed_distance_current() {
        let current = current(1, 2000.0, 480.0, Some("2000m test"));
        let candidates = vec![
            w(2, 7500.0, 118.0, "2026-01-01", 1800.0, Some("JustRow")),
            w(3, 2010.0, 115.0, "2026-02-01", 600.0, None),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(3)
        );
    }

    #[test]
    fn picks_time_closest_candidate_for_time_axis_current() {
        let current = current(1, 7500.0, 1800.0, Some("JustRow"));
        let candidates = vec![
            w(2, 7200.0, 120.0, "2026-01-01", 1760.0, Some("JustRow")),
            w(3, 8000.0, 118.0, "2026-02-01", 1900.0, Some("JustRow")),
            w(4, 7800.0, 115.0, "2026-03-01", 1850.0, Some("JustRow")),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(2)
        );
    }

    #[test]
    fn breaks_an_equidistant_tie_by_fastest_pace() {
        let current = current(1, 2000.0, 480.0, None);
        let candidates = vec![
            w(2, 1950.0, 120.0, "2026-05-01", 600.0, None),
            w(3, 2050.0, 110.0, "2026-01-01", 600.0, None),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(3)
        );
    }

    #[test]
    fn breaks_a_distance_and_pace_tie_by_most_recent_date() {
        let current = current(1, 2000.0, 480.0, None);
        let candidates = vec![
            w(2, 1950.0, 115.0, "2026-01-01", 600.0, None),
            w(3, 2050.0, 115.0, "2026-05-01", 600.0, None),
        ];
        assert_eq!(
            pick_default_ghost_candidate(&candidates, &current).map(|c| c.id),
            Some(3)
        );
    }
}
