// SPDX-License-Identifier: GPL-3.0-or-later
//! Finished-race outcome calculation (Studio `ReplayRaceResult`).
//!
//! **Documented deviation (Studio source map):** distance-axis finish times
//! use the first *interpolated* crossing of the target distance rather than
//! the web's array-endpoint comparison — stricter, and it makes sparse traces
//! (one sample spanning the finish) decide correctly.

use crate::models::{Stroke, Workout};
use crate::replay::comparability::{ComparabilityAxis, classify_axis};
use crate::replay::engine::sample_at;
use crate::replay::race_gap::absolute_time;

/// Outcome of a completed replay race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaceOutcome {
    /// The player finished ahead.
    PlayerWon,
    /// The rival finished ahead.
    RivalWon,
    /// Within the axis tie tolerance.
    Tie,
}

/// Deterministic race result; every numeric field is finite and non-negative
/// (Studio `ReplayRaceResult`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaceResult {
    /// Who won.
    pub outcome: RaceOutcome,
    /// Which axis the race was decided on.
    pub axis: ComparabilityAxis,
    /// Absolute time margin in seconds when meaningful (distance axis).
    pub time_margin: Option<f64>,
    /// Absolute distance margin in metres.
    pub distance_margin: Option<f64>,
    /// True when the rival never reached a distance-axis target.
    pub rival_did_not_finish: bool,
    /// Player finish time (relative), when available.
    pub player_finish_time: Option<f64>,
    /// Rival finish time (relative), when available.
    pub rival_finish_time: Option<f64>,
    /// Player distance at the decision point.
    pub player_distance: Option<f64>,
    /// Rival distance at the decision point.
    pub rival_distance: Option<f64>,
}

fn sanitized_non_negative(value: Option<f64>) -> Option<f64> {
    match value {
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        _ => None,
    }
}

/// Distance-axis finish-time tie tolerance (seconds).
pub const DISTANCE_TIME_TIE_TOLERANCE: f64 = 0.05;
/// Time-axis distance tie tolerance (metres).
pub const TIME_DISTANCE_TIE_TOLERANCE: f64 = 0.5;

/// Compute a completed race result, or `None` when either trace has fewer
/// than two strokes or the player never finishes (Studio
/// `ReplayRaceResultCalculator.result`).
#[must_use]
pub fn race_result(
    player_strokes: &[Stroke],
    rival_strokes: &[Stroke],
    workout: &Workout,
) -> Option<RaceResult> {
    if player_strokes.len() < 2 || rival_strokes.len() < 2 {
        return None;
    }
    match classify_axis(workout.workout_type.as_deref()) {
        ComparabilityAxis::Distance => {
            distance_axis_result(player_strokes, rival_strokes, workout.distance)
        }
        ComparabilityAxis::Time => time_axis_result(player_strokes, rival_strokes, workout.time),
    }
}

/// First interpolated relative time at which the trace crosses
/// `target_distance` (linear between consecutive samples); `None` when the
/// trace never reaches the target.
#[must_use]
pub fn time_crossing_target(strokes: &[Stroke], target_distance: f64) -> Option<f64> {
    if !target_distance.is_finite() || target_distance <= 0.0 || strokes.len() < 2 {
        return None;
    }
    let origin_t = strokes[0].t;
    if !origin_t.is_finite() {
        return None;
    }

    // Already past or at the target at the first sample.
    if strokes[0].d.is_finite() && strokes[0].d >= target_distance {
        return Some(0.0);
    }

    for i in 1..strokes.len() {
        let prev = &strokes[i - 1];
        let curr = &strokes[i];
        if !prev.t.is_finite() || !curr.t.is_finite() || !prev.d.is_finite() || !curr.d.is_finite()
        {
            continue;
        }
        // Skip non-forward or invalid segments.
        if curr.t < prev.t {
            continue;
        }
        if prev.d < target_distance && curr.d >= target_distance {
            let delta = curr.d - prev.d;
            if !(delta.is_finite() && delta > 0.0) {
                continue;
            }
            let frac = ((target_distance - prev.d) / delta).clamp(0.0, 1.0);
            let absolute = prev.t + frac * (curr.t - prev.t);
            if !absolute.is_finite() {
                continue;
            }
            let relative = absolute - origin_t;
            if relative.is_finite() && relative >= 0.0 {
                return Some(relative);
            }
            return None;
        }
    }
    None
}

/// Distance at a relative elapsed time from the first stroke timestamp
/// (Studio `ReplayRaceResultCalculator.distanceAtRelativeTime`).
#[must_use]
pub fn distance_at_relative_time(strokes: &[Stroke], relative_time: f64) -> f64 {
    if strokes.is_empty() {
        return 0.0;
    }
    let safe_elapsed = if relative_time.is_finite() {
        relative_time.max(0.0)
    } else {
        0.0
    };
    let absolute = absolute_time(safe_elapsed, strokes);
    let d = sample_at(strokes, absolute).d;
    if d.is_finite() && d >= 0.0 { d } else { 0.0 }
}

fn distance_axis_result(
    player_strokes: &[Stroke],
    rival_strokes: &[Stroke],
    target_distance: f64,
) -> Option<RaceResult> {
    if !target_distance.is_finite() || target_distance <= 0.0 {
        return None;
    }
    let player_finish = time_crossing_target(player_strokes, target_distance)?;
    let Some(rival_finish) = time_crossing_target(rival_strokes, target_distance) else {
        // Rival DNF — player wins with the shortfall at the player's finish.
        let rival_dist = distance_at_relative_time(rival_strokes, player_finish);
        return Some(RaceResult {
            outcome: RaceOutcome::PlayerWon,
            axis: ComparabilityAxis::Distance,
            time_margin: None,
            distance_margin: sanitized_non_negative(Some(target_distance - rival_dist.max(0.0))),
            rival_did_not_finish: true,
            player_finish_time: Some(player_finish),
            rival_finish_time: None,
            player_distance: Some(target_distance),
            rival_distance: Some(rival_dist),
        });
    };

    let delta = player_finish - rival_finish;
    let abs_delta = delta.abs();

    if abs_delta <= DISTANCE_TIME_TIE_TOLERANCE {
        return Some(RaceResult {
            outcome: RaceOutcome::Tie,
            axis: ComparabilityAxis::Distance,
            time_margin: Some(0.0),
            distance_margin: Some(0.0),
            rival_did_not_finish: false,
            player_finish_time: Some(player_finish),
            rival_finish_time: Some(rival_finish),
            player_distance: Some(target_distance),
            rival_distance: Some(target_distance),
        });
    }

    if delta < 0.0 {
        // Player faster: shortfall is the rival's distance at the player's finish.
        let rival_at_player_finish = distance_at_relative_time(rival_strokes, player_finish);
        Some(RaceResult {
            outcome: RaceOutcome::PlayerWon,
            axis: ComparabilityAxis::Distance,
            time_margin: Some(abs_delta),
            distance_margin: sanitized_non_negative(Some(
                target_distance - rival_at_player_finish.max(0.0),
            )),
            rival_did_not_finish: false,
            player_finish_time: Some(player_finish),
            rival_finish_time: Some(rival_finish),
            player_distance: Some(target_distance),
            rival_distance: Some(rival_at_player_finish),
        })
    } else {
        // Rival faster.
        let player_at_rival_finish = distance_at_relative_time(player_strokes, rival_finish);
        Some(RaceResult {
            outcome: RaceOutcome::RivalWon,
            axis: ComparabilityAxis::Distance,
            time_margin: Some(abs_delta),
            distance_margin: sanitized_non_negative(Some(
                target_distance - player_at_rival_finish.max(0.0),
            )),
            rival_did_not_finish: false,
            player_finish_time: Some(player_finish),
            rival_finish_time: Some(rival_finish),
            player_distance: Some(player_at_rival_finish),
            rival_distance: Some(target_distance),
        })
    }
}

fn time_axis_result(
    player_strokes: &[Stroke],
    rival_strokes: &[Stroke],
    target_duration: f64,
) -> Option<RaceResult> {
    if !target_duration.is_finite() || target_duration <= 0.0 {
        return None;
    }
    if player_strokes.len() < 2 || rival_strokes.len() < 2 {
        return None;
    }
    let player_dist = distance_at_relative_time(player_strokes, target_duration);
    let rival_dist = distance_at_relative_time(rival_strokes, target_duration);
    let delta = player_dist - rival_dist;
    let abs_delta = delta.abs();

    if abs_delta <= TIME_DISTANCE_TIE_TOLERANCE {
        return Some(RaceResult {
            outcome: RaceOutcome::Tie,
            axis: ComparabilityAxis::Time,
            time_margin: None,
            distance_margin: Some(0.0),
            rival_did_not_finish: false,
            player_finish_time: Some(target_duration),
            rival_finish_time: Some(target_duration),
            player_distance: Some(player_dist),
            rival_distance: Some(rival_dist),
        });
    }

    Some(RaceResult {
        outcome: if delta > 0.0 {
            RaceOutcome::PlayerWon
        } else {
            RaceOutcome::RivalWon
        },
        axis: ComparabilityAxis::Time,
        time_margin: None,
        distance_margin: Some(abs_delta),
        rival_did_not_finish: false,
        player_finish_time: Some(target_duration),
        rival_finish_time: Some(target_duration),
        player_distance: Some(player_dist),
        rival_distance: Some(rival_dist),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Sport;

    fn strokes(pairs: &[(f64, f64)]) -> Vec<Stroke> {
        pairs
            .iter()
            .map(|(t, d)| Stroke {
                t: *t,
                d: *d,
                pace: 120.0,
                spm: 28.0,
                hr: None,
                watts: 200.0,
                raw_t: None,
                raw_d: None,
            })
            .collect()
    }

    fn workout(distance: f64, time: f64, workout_type: Option<&str>) -> Workout {
        let mut w = Workout::new(
            1,
            "2026-01-01 00:00:00",
            Sport::Rower,
            distance,
            time,
            120.0,
        );
        w.workout_type = workout_type.map(String::from);
        w
    }

    #[test]
    fn crossing_interpolates_between_samples() {
        let trace = strokes(&[(0.0, 0.0), (480.0, 2000.0)]);
        assert!((time_crossing_target(&trace, 1000.0).unwrap_or(-1.0) - 240.0).abs() < 1e-3);
        // Already at target on the first sample.
        assert_eq!(time_crossing_target(&trace, -5.0), None);
        let at_start = strokes(&[(0.0, 500.0), (10.0, 600.0)]);
        assert_eq!(time_crossing_target(&at_start, 400.0), Some(0.0));
        // Never reaches.
        let short = strokes(&[(0.0, 0.0), (240.0, 1000.0)]);
        assert_eq!(time_crossing_target(&short, 2000.0), None);
        // Non-finite target.
        assert_eq!(time_crossing_target(&trace, f64::NAN), None);
    }

    #[test]
    fn crossing_uses_relative_time_from_non_zero_origins() {
        let trace = strokes(&[(1000.0, 0.0), (1100.0, 500.0)]);
        assert!((time_crossing_target(&trace, 500.0).unwrap_or(-1.0) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn distance_axis_player_and_rival_wins() {
        let player = strokes(&[(0.0, 0.0), (480.0, 2000.0)]);
        let rival = strokes(&[(0.0, 0.0), (520.0, 2000.0)]);
        let result = race_result(&player, &rival, &workout(2000.0, 480.0, None)).unwrap();
        assert_eq!(result.outcome, RaceOutcome::PlayerWon);
        assert!((result.time_margin.unwrap() - 40.0).abs() < 1e-9);
        assert!(!result.rival_did_not_finish);

        let result = race_result(&rival, &player, &workout(2000.0, 520.0, None)).unwrap();
        assert_eq!(result.outcome, RaceOutcome::RivalWon);
        assert!((result.time_margin.unwrap() - 40.0).abs() < 1e-9);
    }

    #[test]
    fn distance_axis_tie_within_tolerance() {
        let player = strokes(&[(0.0, 0.0), (240.02, 1000.0)]);
        let rival = strokes(&[(0.0, 0.0), (240.0, 1000.0)]);
        let result = race_result(&player, &rival, &workout(1000.0, 240.0, None)).unwrap();
        assert_eq!(result.outcome, RaceOutcome::Tie);
        assert_eq!(result.time_margin, Some(0.0));
    }

    #[test]
    fn sparse_traces_interpolate_the_finish() {
        let player = strokes(&[(0.0, 0.0), (480.0, 2000.0)]);
        let rival = strokes(&[(0.0, 0.0), (600.0, 2000.0)]);
        let result = race_result(&player, &rival, &workout(1000.0, 480.0, None)).unwrap();
        assert_eq!(result.outcome, RaceOutcome::PlayerWon);
        assert!((result.player_finish_time.unwrap() - 240.0).abs() < 0.05);
        assert!((result.rival_finish_time.unwrap() - 300.0).abs() < 0.05);
        assert!((result.time_margin.unwrap() - 60.0).abs() < 0.05);
    }

    #[test]
    fn rival_dnf_reports_shortfall() {
        let player = strokes(&[(0.0, 0.0), (480.0, 2000.0)]);
        let rival = strokes(&[(0.0, 0.0), (240.0, 1000.0)]);
        let result = race_result(&player, &rival, &workout(2000.0, 480.0, None)).unwrap();
        assert_eq!(result.outcome, RaceOutcome::PlayerWon);
        assert!(result.rival_did_not_finish);
        assert!((result.distance_margin.unwrap() - 1000.0).abs() < 0.5);
    }

    #[test]
    fn player_dnf_yields_no_result() {
        let player = strokes(&[(0.0, 0.0), (240.0, 1000.0)]);
        let rival = strokes(&[(0.0, 0.0), (480.0, 2000.0)]);
        assert!(race_result(&player, &rival, &workout(2000.0, 480.0, None)).is_none());
        // Short traces.
        let one = strokes(&[(0.0, 0.0)]);
        assert!(race_result(&one, &rival, &workout(2000.0, 480.0, None)).is_none());
        // Non-finite target.
        assert!(race_result(&rival, &rival, &workout(f64::NAN, 480.0, None)).is_none());
    }

    #[test]
    fn time_axis_compares_distances_at_target_duration() {
        let player = strokes(&[(0.0, 0.0), (300.0, 1500.0)]);
        let rival = strokes(&[(0.0, 0.0), (300.0, 1250.0)]);
        let result = race_result(
            &player,
            &rival,
            &workout(1500.0, 300.0, Some("FixedTimeIntervals")),
        )
        .unwrap();
        assert_eq!(result.outcome, RaceOutcome::PlayerWon);
        assert_eq!(result.time_margin, None);
        assert!((result.distance_margin.unwrap() - 250.0).abs() < 0.5);

        let result =
            race_result(&rival, &player, &workout(1250.0, 300.0, Some("JustRow"))).unwrap();
        assert_eq!(result.outcome, RaceOutcome::RivalWon);

        let a = strokes(&[(0.0, 0.0), (180.0, 750.2)]);
        let b = strokes(&[(0.0, 0.0), (180.0, 750.0)]);
        let result =
            race_result(&a, &b, &workout(750.2, 180.0, Some("FixedTimeIntervals"))).unwrap();
        assert_eq!(result.outcome, RaceOutcome::Tie);
    }
}
