// SPDX-License-Identifier: GPL-3.0-or-later
//! The web's course loop and sport motion profiles (Phase 5b spec R3.4;
//! `renderer3d.ts` `placeAvatar`, `SPORT_PROFILES`; `renderer3dAvatarKit.ts`
//! `COURSE_LOOP_METERS`).
//!
//! The live athlete rides a circle whose circumference is one kilometre:
//! `a = metres / 1000 · 2π`, position `(r·sin a, 0, r·cos a)`, unit tangent
//! `(cos a, −sin a)`. The rig yaws to the tangent — rowing shells travel
//! bow-first while the rower faces the stern, so the row rig turns a further
//! π. The profile accents (bob, surge, roll) come straight from the motion
//! graph's accent channels scaled by the sport's amplitudes. Venue geometry
//! is Phase 6; this is pure motion.

use rowplay_core::models::Sport;

/// One lap of the visual loop, metres (web `COURSE_LOOP_METERS`).
pub const COURSE_LOOP_METERS: f64 = 1000.0;
/// Radius of the live lane, metres (web `CourseRenderer3D.loopRadius`).
pub const LIVE_LOOP_RADIUS: f64 = 30.0;
/// Radius of the comparison lane, metres (web `ghostRadius`; Phase 5c).
pub const GHOST_LOOP_RADIUS: f64 = 26.0;

/// The per-sport accent amplitudes of the web's `SPORT_PROFILES`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SportProfile {
    /// Vertical bob amplitude, metres, applied to the accent channel.
    pub bob_amp: f64,
    /// Longitudinal surge amplitude, metres.
    pub surge_amp: f64,
    /// Whether the rig rolls (the shell rocks; skis and bikes do not).
    pub roll: bool,
}

/// The web's `SPORT_PROFILES` accents per sport.
#[must_use]
pub fn profile(sport: Sport) -> SportProfile {
    match sport {
        Sport::Rower => SportProfile {
            bob_amp: 0.13,
            surge_amp: 0.48,
            roll: true,
        },
        Sport::Skierg => SportProfile {
            bob_amp: 0.08,
            surge_amp: 1.45,
            roll: false,
        },
        Sport::Bike => SportProfile {
            bob_amp: 0.03,
            surge_amp: 0.0,
            roll: false,
        },
    }
}

/// Where the rig root sits on the loop and which way it faces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    /// World x, metres.
    pub x: f64,
    /// World z, metres.
    pub z: f64,
    /// Unit tangent x (direction of increasing distance).
    pub tx: f64,
    /// Unit tangent z.
    pub tz: f64,
    /// Rig yaw about +Y, radians (`atan2(tx, tz)`, plus π for the row shell).
    pub yaw: f64,
}

/// Place the rig root `metres` along the loop of `radius` (web `placeAvatar`).
///
/// Non-finite distances place the rig at the start line.
#[must_use]
pub fn place(sport: Sport, metres: f64, radius: f64) -> Placement {
    let metres = if metres.is_finite() { metres } else { 0.0 };
    let a = metres / COURSE_LOOP_METERS * std::f64::consts::TAU;
    let (sin, cos) = a.sin_cos();
    let tx = cos;
    let tz = -sin;
    let mut yaw = tx.atan2(tz);
    if sport == Sport::Rower {
        yaw += std::f64::consts::PI;
    }
    Placement {
        x: radius * sin,
        z: radius * cos,
        tx,
        tz,
        yaw,
    }
}

/// The rig's per-frame accents (web `placeAvatar`: bob, surge, roll).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Accents {
    /// Vertical offset of the rig group, metres.
    pub bob: f64,
    /// Longitudinal offset of the rig group, metres (the shell checks at the
    /// catch and runs out through the drive; negative along the rower's
    /// local z, positive for the other sports).
    pub surge: f64,
    /// Roll about the rig's local z, radians.
    pub roll: f64,
}

/// The motion-graph accent channel values the profile scales.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AccentCues {
    /// Vertical cue: the rower's `accents.vertical`, the skier's
    /// `accents.rebound`, zero for the bike.
    pub vertical: f64,
    /// The `accents.surge` channel (zero for the bike).
    pub surge: f64,
}

/// Compose the accents (web `placeAvatar` lines 2574–2588).
///
/// `anim_phase` is the renderer's free-running clock (advanced by
/// [`advance_anim_phase`]) and `cadence` the current stroke rate; reduce
/// motion zeroes everything.
#[must_use]
pub fn accents(
    sport: Sport,
    cues: AccentCues,
    anim_phase: f64,
    cadence: f64,
    reduce_motion: bool,
) -> Accents {
    let profile = profile(sport);
    if reduce_motion {
        return Accents::default();
    }
    let bob = if profile.bob_amp == 0.0 {
        0.0
    } else {
        cues.vertical * profile.bob_amp
    };
    let surge = if profile.surge_amp == 0.0 {
        0.0
    } else {
        cues.surge * profile.surge_amp
    };
    let surge = if sport == Sport::Rower { -surge } else { surge };
    let roll = if profile.roll {
        let ambient = (anim_phase + cadence * 0.05).sin() * 0.035;
        let stroke = cues.surge * 0.045;
        ambient + stroke
    } else {
        0.0
    };
    Accents {
        bob: finite(bob),
        surge: finite(surge),
        roll: finite(roll),
    }
}

/// Advance the renderer's free-running phase (web `renderer3d.ts` line
/// 2731: `animPhase += (2.4 + spm / 13) · dt` while playing and not reduced).
#[must_use]
pub fn advance_anim_phase(
    anim_phase: f64,
    spm: f64,
    dt: f64,
    playing: bool,
    reduce_motion: bool,
) -> f64 {
    if !playing || reduce_motion || !dt.is_finite() || dt <= 0.0 {
        return anim_phase;
    }
    let spm = if spm.is_finite() { spm } else { 0.0 };
    anim_phase + (2.4 + spm / 13.0) * dt
}

fn finite(value: f64) -> f64 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    #[test]
    fn a_kilometre_is_one_lap_and_the_tangent_follows_the_circle() {
        let start = place(Sport::Skierg, 0.0, LIVE_LOOP_RADIUS);
        assert!((start.x).abs() < 1e-12 && (start.z - 30.0).abs() < 1e-12);
        assert!((start.tx - 1.0).abs() < 1e-12 && start.tz.abs() < 1e-12);
        assert!((start.yaw - FRAC_PI_2).abs() < 1e-12);
        let quarter = place(Sport::Skierg, 250.0, LIVE_LOOP_RADIUS);
        assert!((quarter.x - 30.0).abs() < 1e-9 && quarter.z.abs() < 1e-9);
        assert!(quarter.tx.abs() < 1e-9 && (quarter.tz + 1.0).abs() < 1e-9);
        let lap = place(Sport::Skierg, 1000.0, LIVE_LOOP_RADIUS);
        assert!((lap.x - start.x).abs() < 1e-9 && (lap.z - start.z).abs() < 1e-9);
        for metres in [0.0, 137.5, 600.0, 999.9] {
            let p = place(Sport::Bike, metres, LIVE_LOOP_RADIUS);
            assert!((p.tx * p.tx + p.tz * p.tz - 1.0).abs() < 1e-12);
            assert!((p.x * p.x + p.z * p.z - 900.0).abs() < 1e-9);
        }
    }

    #[test]
    fn the_row_shell_travels_bow_first() {
        let ski = place(Sport::Skierg, 100.0, LIVE_LOOP_RADIUS);
        let row = place(Sport::Rower, 100.0, LIVE_LOOP_RADIUS);
        assert!(((row.yaw - ski.yaw) - PI).abs() < 1e-12);
        assert_eq!((row.x, row.z), (ski.x, ski.z));
        assert_eq!(
            place(Sport::Rower, f64::NAN, 30.0),
            place(Sport::Rower, 0.0, 30.0)
        );
    }

    #[test]
    fn accents_scale_the_cues_by_the_sport_profile() {
        let cues = AccentCues {
            vertical: 0.5,
            surge: 1.0,
        };
        let row = accents(Sport::Rower, cues, 0.0, 30.0, false);
        assert!((row.bob - 0.065).abs() < 1e-12);
        assert!((row.surge + 0.48).abs() < 1e-12, "rower surge is negative");
        assert!((row.roll - ((1.5_f64).sin() * 0.035 + 0.045)).abs() < 1e-12);
        let ski = accents(Sport::Skierg, cues, 0.0, 30.0, false);
        assert!((ski.bob - 0.04).abs() < 1e-12);
        assert!((ski.surge - 1.45).abs() < 1e-12);
        assert_eq!(ski.roll, 0.0);
        let bike = accents(Sport::Bike, cues, 0.0, 30.0, false);
        assert!((bike.bob - 0.015).abs() < 1e-12);
        assert_eq!(bike.surge, 0.0);
        assert_eq!(
            accents(Sport::Rower, cues, 3.0, 30.0, true),
            Accents::default()
        );
    }

    #[test]
    fn the_anim_phase_only_advances_while_playing() {
        assert_eq!(advance_anim_phase(1.0, 26.0, 0.1, false, false), 1.0);
        assert_eq!(advance_anim_phase(1.0, 26.0, 0.1, true, true), 1.0);
        let advanced = advance_anim_phase(1.0, 26.0, 0.1, true, false);
        assert!((advanced - (1.0 + (2.4 + 2.0) * 0.1)).abs() < 1e-12);
        assert_eq!(
            advance_anim_phase(1.0, f64::NAN, 0.5, true, false),
            1.0 + 2.4 * 0.5
        );
    }
}
