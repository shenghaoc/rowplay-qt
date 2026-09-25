// SPDX-License-Identifier: GPL-3.0-or-later
//! Live race-gap helpers for ghost-replay workflows
//! (web `replay/replayGap.ts`, Studio `ReplayRaceGap`).
//!
//! Positive gap means the player is ahead; negative means behind.
//! Approximate seconds use the player's current speed (500 / pace per 500 m).
//! Non-finite distances collapse to zero before the subtraction (Studio's
//! guard, which the golden gap fixture was verified against).

use crate::models::Stroke;
use crate::replay::engine::{Frame, sample_at};

/// Gap in metres between player and ghost (web `raceGapMetres`).
#[must_use]
pub fn race_gap_metres(player_d: f64, ghost_d: f64) -> f64 {
    let player = if player_d.is_finite() { player_d } else { 0.0 };
    let ghost = if ghost_d.is_finite() { ghost_d } else { 0.0 };
    player - ghost
}

/// Approximate time gap in seconds using the player's current pace (web
/// `raceGapSeconds`); 0 when the pace is non-positive or non-finite.
#[must_use]
pub fn race_gap_seconds(gap_m: f64, player_pace_per_500m: f64) -> f64 {
    if !gap_m.is_finite() {
        return 0.0;
    }
    if !player_pace_per_500m.is_finite() || player_pace_per_500m <= 0.0 {
        return 0.0;
    }
    let speed_mps = 500.0 / player_pace_per_500m;
    gap_m / speed_mps
}

/// Finish-time delta in seconds (player − ghost, web `finishDeltaSec`):
/// negative = player faster, positive = ghost faster.
#[must_use]
pub fn finish_delta_sec(player_strokes: &[Stroke], ghost_strokes: &[Stroke]) -> f64 {
    let (Some(player_last), Some(ghost_last)) = (player_strokes.last(), ghost_strokes.last())
    else {
        return 0.0;
    };
    player_last.t - ghost_last.t
}

/// Distance the ghost has covered at the moment the player crosses the
/// finish (web `ghostDistAtPlayerFinish`).
#[must_use]
pub fn ghost_dist_at_player_finish(ghost_strokes: &[Stroke], player_time: f64) -> f64 {
    sample_at(ghost_strokes, player_time).d
}

/// Distance the player has covered at the moment the ghost crosses the
/// finish (web `playerDistAtGhostFinish`).
#[must_use]
pub fn player_dist_at_ghost_finish(player_strokes: &[Stroke], ghost_time: f64) -> f64 {
    sample_at(player_strokes, ghost_time).d
}

/// Relative duration from the first to the last stroke timestamp
/// (Studio `ReplayRaceGap.relativeDuration`).
#[must_use]
pub fn relative_duration(strokes: &[Stroke]) -> f64 {
    match (strokes.first(), strokes.last()) {
        (Some(first), Some(last)) if first.t.is_finite() && last.t.is_finite() => {
            (last.t - first.t).max(0.0)
        }
        _ => 0.0,
    }
}

/// Convert elapsed replay time (relative to the first stroke) to an absolute
/// stroke timestamp, clamped to `[first.t, last.t]` (Studio
/// `ReplayRaceGap.absoluteTime`).
#[must_use]
pub fn absolute_time(elapsed: f64, strokes: &[Stroke]) -> f64 {
    let (Some(first), Some(last)) = (strokes.first(), strokes.last()) else {
        return 0.0;
    };
    if !first.t.is_finite() || !last.t.is_finite() {
        return 0.0;
    }
    let safe_elapsed = if elapsed.is_finite() {
        elapsed.max(0.0)
    } else {
        0.0
    };
    let duration = (last.t - first.t).max(0.0);
    first.t + safe_elapsed.min(duration)
}

/// Interpolated ghost frame at the player's current elapsed time
/// (Studio `ReplayRaceGap.ghostFrame`).
#[must_use]
pub fn ghost_frame(elapsed: f64, strokes: &[Stroke]) -> Frame {
    if strokes.is_empty() {
        return Frame::zero(0.0);
    }
    let abs_t = absolute_time(elapsed, strokes);
    sample_at(strokes, abs_t)
}

/// Ghost distance at the player's current elapsed time
/// (Studio `ReplayRaceGap.ghostDistance`).
#[must_use]
pub fn ghost_distance(elapsed: f64, strokes: &[Stroke]) -> f64 {
    ghost_frame(elapsed, strokes).d
}

/// The ghost's own elapsed time (from its first stroke) at the web's
/// sampling instant: the web samples the ghost at the player's absolute
/// frame time (`sampleAt(ghostStrokes, f.t)`), which is the player's first
/// stroke plus `player_elapsed`. Studio's [`ghost_frame`] samples at the
/// ghost's first stroke plus `player_elapsed` instead; the two agree only
/// when both workouts start at the same stroke time. The result is not
/// clamped: seeking a [`crate::replay::engine::ReplayState`] to it clamps
/// to the ghost's span, which holds its first or last stroke like the
/// web's `sampleAt`.
#[must_use]
pub fn ghost_elapsed_at(
    player_elapsed: f64,
    player_strokes: &[Stroke],
    ghost_strokes: &[Stroke],
) -> f64 {
    let first = |strokes: &[Stroke]| {
        strokes
            .first()
            .map(|s| s.t)
            .filter(|t| t.is_finite())
            .unwrap_or(0.0)
    };
    let elapsed = if player_elapsed.is_finite() {
        player_elapsed
    } else {
        0.0
    };
    first(player_strokes) + elapsed - first(ghost_strokes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strokes(times: &[f64], dists: &[f64]) -> Vec<Stroke> {
        times
            .iter()
            .zip(dists)
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

    #[test]
    fn race_gap_metres_signs() {
        assert!((race_gap_metres(250.0, 230.0) - 20.0).abs() < 1e-9);
        assert!((race_gap_metres(210.0, 250.0) - (-40.0)).abs() < 1e-9);
        assert_eq!(race_gap_metres(500.0, 500.0), 0.0);
    }

    #[test]
    fn race_gap_seconds_conversion() {
        assert!((race_gap_seconds(20.0, 120.0) - 4.8).abs() < 1e-9);
        assert_eq!(race_gap_seconds(20.0, 0.0), 0.0);
        assert!((race_gap_seconds(-20.0, 120.0) - (-4.8)).abs() < 1e-9);
    }

    #[test]
    fn finish_delta_sec_signs_and_empties() {
        let player = strokes(&[0.0, 480.0], &[0.0, 2000.0]);
        let ghost = strokes(&[0.0, 495.0], &[0.0, 2000.0]);
        assert!((finish_delta_sec(&player, &ghost) - (-15.0)).abs() < 1e-9);
        let player = strokes(&[0.0, 500.0], &[0.0, 2000.0]);
        let ghost = strokes(&[0.0, 490.0], &[0.0, 2000.0]);
        assert!((finish_delta_sec(&player, &ghost) - 10.0).abs() < 1e-9);
        let s = strokes(&[0.0, 480.0], &[0.0, 2000.0]);
        assert_eq!(finish_delta_sec(&s, &s), 0.0);
        assert_eq!(finish_delta_sec(&[], &[]), 0.0);
        assert_eq!(finish_delta_sec(&s, &[]), 0.0);
    }

    #[test]
    fn ghost_and_player_distance_at_finish() {
        let ghost = strokes(&[0.0, 100.0, 200.0], &[0.0, 500.0, 1000.0]);
        assert!((ghost_dist_at_player_finish(&ghost, 150.0) - 750.0).abs() < 1e-9);
        let ghost = strokes(&[0.0, 100.0], &[0.0, 1000.0]);
        assert_eq!(ghost_dist_at_player_finish(&ghost, 200.0), 1000.0);
        let player = strokes(&[0.0, 100.0, 200.0], &[0.0, 500.0, 1000.0]);
        assert!((player_dist_at_ghost_finish(&player, 50.0) - 250.0).abs() < 1e-9);
    }

    #[test]
    fn relative_helpers_handle_non_zero_origins() {
        let ghost = strokes(&[5.0, 15.0, 25.0], &[50.0, 150.0, 250.0]);
        assert_eq!(relative_duration(&ghost), 20.0);
        // elapsed 15 clamps to the 5..25 origin window → t = 20 → d = 200.
        assert!((ghost_distance(15.0, &ghost) - 200.0).abs() < 1e-9);
        assert!((absolute_time(0.0, &ghost) - 5.0).abs() < 1e-9);
        assert!((absolute_time(1e9, &ghost) - 25.0).abs() < 1e-9);
        assert_eq!(relative_duration(&[]), 0.0);
        assert_eq!(absolute_time(10.0, &[]), 0.0);
        let empty = ghost_frame(10.0, &[]);
        assert_eq!(empty.d, 0.0);
    }

    #[test]
    fn ghost_elapsed_at_follows_the_players_absolute_time() {
        // The player starts at 2 s and the ghost at 1 s: 4 s into the
        // player's replay the web samples both at t = 6, which is 5 s into
        // the ghost's own replay.
        let player = strokes(&[2.0, 12.0], &[10.0, 110.0]);
        let ghost = strokes(&[1.0, 11.0], &[0.0, 100.0]);
        let elapsed = ghost_elapsed_at(4.0, &player, &ghost);
        assert!((elapsed - 5.0).abs() < 1e-12);
        let web = sample_at(&ghost, 2.0 + 4.0).d;
        assert!((absolute_time(elapsed, &ghost) - 6.0).abs() < 1e-12);
        assert!((sample_at(&ghost, absolute_time(elapsed, &ghost)).d - web).abs() < 1e-12);
        // Studio's elapsed-time sampling reads the ghost a second early:
        // 10 m short at its 10 m/s.
        assert!((web - ghost_distance(4.0, &ghost) - 10.0).abs() < 1e-12);
        // Same first stroke: the two agree.
        let same = strokes(&[2.0, 12.0], &[0.0, 100.0]);
        assert!((ghost_elapsed_at(4.0, &player, &same) - 4.0).abs() < 1e-12);
        // A ghost that starts later reads a negative elapsed time, which a
        // seek clamps to its first stroke, where `sampleAt` holds too.
        let late = strokes(&[5.0, 15.0], &[0.0, 100.0]);
        assert!((ghost_elapsed_at(1.0, &player, &late) - (-2.0)).abs() < 1e-12);
        // Non-finite or missing inputs read as zero.
        assert!((ghost_elapsed_at(f64::NAN, &player, &ghost) - 1.0).abs() < 1e-12);
        assert!((ghost_elapsed_at(4.0, &[], &ghost) - 3.0).abs() < 1e-12);
        assert!((ghost_elapsed_at(4.0, &player, &[]) - 6.0).abs() < 1e-12);
    }
}
