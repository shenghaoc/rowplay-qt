// SPDX-License-Identifier: GPL-3.0-or-later
//! Personal best detection at standard Concept2 distances.
//!
//! Port of the web app's `distancePBs`, `pbWorkoutIds` and `detectNewPBs` from
//! `src/lib/analytics.ts`. Personal bests are tracked **per sport** at each of
//! the seven standard distances, matching a workout within ±2% so a "2000m"
//! piece logged as 2003 m still counts. The sport-filtered id variant used by
//! the workout list lives in [`crate::workout_query::pb_workout_ids`].
//!
//! rowplay-studio also tracked 42 195 m and picked the fastest workout across
//! sports when unfiltered; the web app is canonical, so neither is kept.

use std::collections::BTreeSet;

use crate::models::{Sport, Workout};

/// Standard erg distances we track records for, in metres.
pub const STANDARD_DISTANCES: [f64; 7] = [500.0, 1000.0, 2000.0, 5000.0, 6000.0, 10000.0, 21097.0];

/// Distance matching tolerance (±2%).
pub const DISTANCE_TOLERANCE: f64 = 0.02;

/// Fastest workout at one standard distance for one sport.
#[derive(Debug, Clone, PartialEq)]
pub struct PersonalBest {
    /// Standard distance in metres.
    pub distance: f64,
    /// Elapsed seconds.
    pub time: f64,
    /// Average pace (sec/500 m).
    pub pace: f64,
    /// Logbook date of the workout.
    pub date: String,
    /// Machine family.
    pub sport: Sport,
}

/// Whether `distance` counts as the standard `target` (±2%).
#[must_use]
pub fn matches_standard_distance(distance: f64, target: f64) -> bool {
    (distance - target).abs() <= target * DISTANCE_TOLERANCE
}

/// The standard distance a workout distance matches, if any (Studio `standardDistance(matching:)`).
#[must_use]
pub fn standard_distance_matching(distance: f64) -> Option<f64> {
    STANDARD_DISTANCES
        .into_iter()
        .find(|target| matches_standard_distance(distance, *target))
}

/// Groups workouts by sport in first-seen order (the web app's `Map` iteration order).
fn by_sport(workouts: &[Workout]) -> Vec<(Sport, Vec<&Workout>)> {
    let mut groups: Vec<(Sport, Vec<&Workout>)> = Vec::new();
    for workout in workouts {
        match groups.iter_mut().find(|(sport, _)| *sport == workout.sport) {
            Some((_, list)) => list.push(workout),
            None => groups.push((workout.sport, vec![workout])),
        }
    }
    groups
}

/// For every sport and standard distance, the fastest matching workout. Ties
/// keep the first-seen workout (strict `<` comparison, like the web app).
fn best_per_sport_and_distance(workouts: &[Workout]) -> Vec<(f64, &Workout)> {
    let mut out = Vec::new();
    for (_, sport_workouts) in by_sport(workouts) {
        for target in STANDARD_DISTANCES {
            let mut best: Option<&Workout> = None;
            let mut min_time = f64::INFINITY;
            for w in &sport_workouts {
                if w.time > 0.0
                    && matches_standard_distance(w.distance, target)
                    && w.time < min_time
                {
                    best = Some(w);
                    min_time = w.time;
                }
            }
            if let Some(best) = best {
                out.push((target, best));
            }
        }
    }
    out
}

/// Fastest time for each standard distance the athlete has completed, per sport.
#[must_use]
pub fn distance_pbs(workouts: &[Workout]) -> Vec<PersonalBest> {
    best_per_sport_and_distance(workouts)
        .into_iter()
        .map(|(distance, best)| PersonalBest {
            distance,
            time: best.time,
            pace: best.pace,
            date: best.date.clone(),
            sport: best.sport,
        })
        .collect()
}

/// Ids of the workouts that hold a personal best at any standard distance for
/// their sport.
#[must_use]
pub fn pb_workout_ids(workouts: &[Workout]) -> BTreeSet<i64> {
    best_per_sport_and_distance(workouts)
        .into_iter()
        .map(|(_, best)| best.id)
        .collect()
}

/// PBs that improved (or are new) after a sync or data refresh.
#[must_use]
pub fn detect_new_pbs(before: &[PersonalBest], after: &[PersonalBest]) -> Vec<PersonalBest> {
    after
        .iter()
        .filter(|pb| {
            let previous = before
                .iter()
                .find(|prev| prev.sport == pb.sport && prev.distance == pb.distance);
            previous.is_none_or(|prev| pb.time < prev.time - 0.001)
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demo::mock_workouts;

    fn workout(id: i64, sport: Sport, distance: f64, time: f64) -> Workout {
        Workout::new(
            id,
            "2026-05-27 06:12:00",
            sport,
            distance,
            time,
            time / (distance / 500.0),
        )
    }

    #[test]
    fn finds_fastest_standard_distances_in_demo_history() {
        let pbs = distance_pbs(&mock_workouts());
        let two_k = pbs.iter().find(|p| p.distance == 2000.0).unwrap();
        assert!(two_k.time > 0.0 && two_k.pace > 0.0);
    }

    #[test]
    fn returns_fastest_per_distance_and_respects_tolerance() {
        let pbs = distance_pbs(&[
            workout(1, Sport::Rower, 2000.0, 480.0),
            workout(2, Sport::Rower, 2000.0, 420.0),
            workout(3, Sport::Rower, 5000.0, 1200.0),
        ]);
        assert_eq!(
            pbs.iter().find(|p| p.distance == 2000.0).map(|p| p.time),
            Some(420.0)
        );
        let pbs = distance_pbs(&[
            workout(1, Sport::Rower, 2003.0, 420.0),
            workout(2, Sport::Rower, 1997.0, 430.0),
        ]);
        assert_eq!(
            pbs.iter().find(|p| p.distance == 2000.0).map(|p| p.time),
            Some(420.0)
        );
        assert!(distance_pbs(&[workout(1, Sport::Rower, 2000.0, 0.0)]).is_empty());
    }

    #[test]
    fn pbs_are_tracked_per_sport_like_the_web_app() {
        let workouts = [
            workout(1, Sport::Rower, 2000.0, 420.0),
            workout(2, Sport::Skierg, 2000.0, 400.0),
        ];
        let pbs = distance_pbs(&workouts);
        assert_eq!(pbs.len(), 2);
        assert_eq!(pbs[0].sport, Sport::Rower);
        assert_eq!(pbs[1].sport, Sport::Skierg);
        assert_eq!(pb_workout_ids(&workouts), BTreeSet::from([1, 2]));
    }

    #[test]
    fn covers_every_standard_distance() {
        let workouts = [
            workout(1, Sport::Rower, 500.0, 90.0),
            workout(2, Sport::Rower, 1000.0, 200.0),
            workout(3, Sport::Rower, 2000.0, 420.0),
            workout(4, Sport::Rower, 5000.0, 1200.0),
            workout(5, Sport::Rower, 6000.0, 1500.0),
            workout(6, Sport::Rower, 10000.0, 2520.0),
            workout(7, Sport::Rower, 21097.0, 5400.0),
            workout(8, Sport::Rower, 42195.0, 12000.0),
        ];
        let pbs = distance_pbs(&workouts);
        assert_eq!(pbs.len(), 7, "the marathon is not a web standard distance");
        assert_eq!(
            pbs.iter()
                .map(|p| p.distance)
                .collect::<Vec<_>>()
                .as_slice(),
            &STANDARD_DISTANCES
        );
    }

    #[test]
    fn pb_workout_ids_cases() {
        let ids = pb_workout_ids(&[
            workout(1, Sport::Rower, 2000.0, 480.0),
            workout(2, Sport::Rower, 2000.0, 420.0),
            workout(3, Sport::Rower, 5000.0, 1200.0),
        ]);
        assert_eq!(ids, BTreeSet::from([2, 3]));
        assert!(pb_workout_ids(&[]).is_empty());
        let tie = pb_workout_ids(&[
            workout(1, Sport::Rower, 2000.0, 420.0),
            workout(2, Sport::Rower, 2000.0, 420.0),
        ]);
        assert_eq!(tie, BTreeSet::from([1]), "ties keep the first-seen workout");
    }

    #[test]
    fn standard_distance_matching_cases() {
        assert_eq!(standard_distance_matching(2003.0), Some(2000.0));
        assert_eq!(standard_distance_matching(1700.0), None);
        assert_eq!(standard_distance_matching(42195.0), None);
        assert!(matches_standard_distance(2020.0, 2000.0));
        assert!(!matches_standard_distance(2100.0, 2000.0));
    }

    #[test]
    fn detect_new_pbs_reports_improvements_only() {
        let before = distance_pbs(&[workout(1, Sport::Rower, 2000.0, 420.0)]);
        let after = distance_pbs(&[
            workout(1, Sport::Rower, 2000.0, 420.0),
            workout(2, Sport::Rower, 2000.0, 415.0),
            workout(3, Sport::Rower, 5000.0, 1200.0),
        ]);
        let new_pbs = detect_new_pbs(&before, &after);
        assert_eq!(
            new_pbs.iter().map(|p| p.distance).collect::<Vec<_>>(),
            vec![2000.0, 5000.0]
        );
        let unchanged = detect_new_pbs(&before, &before);
        assert!(unchanged.is_empty());
    }
}
