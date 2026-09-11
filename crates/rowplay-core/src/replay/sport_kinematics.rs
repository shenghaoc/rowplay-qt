// SPDX-License-Identifier: GPL-3.0-or-later
//! Compatibility projections from the motion graph to the legacy compact 2D
//! channels (web `replay/sportKinematics.ts`).
//!
//! The motion graph is the single source of choreography; these names keep
//! Canvas-style consumers (and the golden 2D corpus) on the same poses while
//! they adopt the richer graph incrementally.

use crate::replay::motion_graph::{
    SKI_POLE_OFF_CYCLE, sample_bike_motion_graph, sample_rower_motion_graph,
    sample_skier_motion_graph,
};
use crate::replay::stroke_model::StrokePose;

/// Normalised rowing pose channels; main joints in 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerKinematics {
    /// Seat / leg drive, 0 at the catch and 1 fully driven back.
    pub leg_extension: f64,
    /// Torso swing, 0 body-over and 1 open at the finish.
    pub body_swing: f64,
    /// Handle draw, 0 long arms and 1 at the body.
    pub arm_draw: f64,
    /// Blade immersion envelope, 0..1.
    pub blade_depth: f64,
    /// Blade feather during recovery, 0..1.
    pub blade_feather: f64,
    /// Signed propulsion accent.
    pub surge: f64,
    /// Signed vertical accent.
    pub vertical: f64,
}

/// Normalised SkiErg pose channels; main joints in 0..1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierKinematics {
    /// Canonical full-cycle phase in `[0, 1)`.
    pub cycle: f64,
    /// Double-pole press, 0 high reach and 1 completed press.
    pub arm_press: f64,
    /// Forward hip hinge, 0..1.
    pub hip_hinge: f64,
    /// Knee compression, 0..1.
    pub knee_flex: f64,
    /// Pole plant envelope, 0..1.
    pub pole_contact: f64,
    /// Pole sweep travel, 0..1.
    pub pole_sweep: f64,
    /// Early flexion cue keeping the elbow on its anatomical branch.
    pub elbow_load: f64,
    /// Long-arm cue from late press through early recovery.
    pub arm_extension: f64,
    /// Lifted basket recovery cue; zero at pole-off and the next plant.
    pub pole_lift: f64,
    /// Free-flight weight, C2-eased away from and back to the snow anchor.
    pub pole_flight: f64,
    /// Recovery rebound accent, 0..1.
    pub rebound: f64,
    /// Propulsion accent, 0..1.
    pub surge: f64,
}

/// Phase-continuous SkiErg elbow-plane direction; semantic directions rather
/// than measured joint angles (web `SkierElbowDirection`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkierElbowDirection {
    /// Negative points down.
    pub vertical: f64,
    /// Negative points back.
    pub fore_aft: f64,
}

/// Bike crank angle and restrained secondary joint rotations, in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BikeKinematics {
    /// Crank angle in radians, `[0, 2π)`.
    pub crank_angle: f64,
    /// Restrained side-to-side torso cue (radians).
    pub torso_sway: f64,
    /// Restrained pelvis rock cue (radians).
    pub hip_rock: f64,
    /// Left ankle articulation (radians).
    pub ankle_pitch_left: f64,
    /// Right ankle articulation (radians).
    pub ankle_pitch_right: f64,
}

fn clamp_unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn secondary_scale(intensity: f64) -> f64 {
    let normalized = if intensity.is_finite() {
        clamp_unit(intensity)
    } else {
        0.5
    };
    // Logged effort may make secondary motion more legible, but must not
    // change the authored technique sequence, contact paths, or pose limits.
    0.9 + normalized * 0.1
}

/// Rower projection (web `solveRowerKinematics`).
#[must_use]
pub fn solve_rower_kinematics(pose: &StrokePose) -> RowerKinematics {
    let graph = sample_rower_motion_graph(pose);
    RowerKinematics {
        leg_extension: graph.body.leg_extension.value,
        body_swing: graph.body.spine_hinge.value,
        arm_draw: graph.body.arm_draw.value,
        blade_depth: graph.contacts.blade_water.value,
        blade_feather: graph.contacts.blade_feather.value,
        surge: graph.accents.surge.value,
        vertical: graph.accents.vertical.value,
    }
}

/// SkiErg projection; pole contact remains a C2 plant/release envelope
/// (web `solveSkierKinematics`).
#[must_use]
pub fn solve_skier_kinematics(pose: &StrokePose) -> SkierKinematics {
    let graph = sample_skier_motion_graph(pose);
    SkierKinematics {
        cycle: graph.timing.cycle,
        arm_press: graph.body.arm_press.value,
        hip_hinge: graph.body.pelvis_hinge.value,
        knee_flex: graph.body.knee_flex.value,
        pole_contact: graph.contacts.pole_plant.value,
        pole_sweep: graph.body.pole_sweep.value,
        elbow_load: graph.body.elbow_load.value,
        arm_extension: graph.body.arm_extension.value,
        pole_lift: graph.body.pole_lift.value,
        pole_flight: graph.body.pole_flight.value,
        rebound: graph.accents.rebound.value,
        surge: graph.accents.surge.value,
    }
}

/// Resolve the shared down-elbow → press → recovery SkiErg elbow sequence
/// (web `solveSkierElbowDirection`).
///
/// The bend vector stays on one continuous sagittal arc: at the plant the
/// elbows hang below the up-forward shoulder→hand chord, the press swings
/// them aft early, and the recovery retraces that same arc instead of closing
/// a full circle over the top (which degenerated the bend plane).
#[must_use]
pub fn solve_skier_elbow_direction(kinematics: &SkierKinematics) -> SkierElbowDirection {
    let sweep = clamp_unit(kinematics.pole_sweep);
    // vertical = cos(angle), foreAft = sin(angle): 0 is straight up, +π/2 is
    // horizontal-forward. Plant: down, tilted slightly forward; pole-off:
    // down-back.
    const PLANT_ANGLE: f64 = 2.9;
    const POLE_OFF_ANGLE: f64 = std::f64::consts::PI + 1.1;
    // Swing aft EARLY: completing the swing in the first 45% of the sweep
    // keeps the hint a quarter-turn off the chord through the deep load.
    let press_sweep = clamp_unit(sweep / 0.45);
    let press_ease = press_sweep * press_sweep * (3.0 - 2.0 * press_sweep);
    let angle = if kinematics.cycle <= SKI_POLE_OFF_CYCLE {
        PLANT_ANGLE + press_ease * (POLE_OFF_ANGLE - PLANT_ANGLE)
    } else {
        // Recovery retraces the press arc as sweep decays 1 → 0.
        POLE_OFF_ANGLE + (1.0 - sweep) * (PLANT_ANGLE - POLE_OFF_ANGLE)
    };
    SkierElbowDirection {
        vertical: angle.cos(),
        fore_aft: angle.sin(),
    }
}

/// BikeErg projection (web `solveBikeKinematics`). The graph's larger
/// coordinate-neutral values are projected into the equipment-safe contact
/// envelope instead of making the rider thrash.
#[must_use]
pub fn solve_bike_kinematics(pose: &StrokePose) -> BikeKinematics {
    let graph = sample_bike_motion_graph(pose);
    let effort = secondary_scale(pose.intensity);
    BikeKinematics {
        crank_angle: graph.crank.angle,
        torso_sway: graph.body.torso_sway.value * 0.34 * effort,
        hip_rock: graph.body.pelvis_rock.value * 0.3 * effort,
        ankle_pitch_left: -0.05 + graph.left_pedal.ankle_flex.value * 0.3,
        ankle_pitch_right: -0.05 + graph.right_pedal.ankle_flex.value * 0.3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Sport;

    fn pose(sport: Sport, cycle: f64) -> StrokePose {
        StrokePose {
            index: 0,
            phase: cycle * std::f64::consts::TAU,
            warped_phase: 0.0,
            cycle_frac: cycle,
            drive_frac: match sport {
                Sport::Bike => 0.5,
                Sport::Skierg => 0.34,
                Sport::Rower => 0.38,
            },
            drive: false,
            drive_progress: 0.0,
            recovery_progress: 0.0,
            stroke_seconds: 2.0,
            stroke_meters: 10.0,
            rate: 30.0,
            watts: 200.0,
            intensity: 0.5,
            amplitude: 1.0,
            fatigue: 0.0,
            real: true,
        }
    }

    #[test]
    fn rower_projection_stays_in_authored_ranges() {
        for i in 0..=100 {
            let k = solve_rower_kinematics(&pose(Sport::Rower, f64::from(i) / 100.0));
            assert!((0.0..=1.0).contains(&k.leg_extension));
            assert!((0.0..=1.0).contains(&k.body_swing));
            assert!((-1.0..=1.0).contains(&k.arm_draw));
            assert!((0.0..=1.0).contains(&k.blade_depth));
        }
    }

    #[test]
    fn skier_projection_carries_the_cycle() {
        let k = solve_skier_kinematics(&pose(Sport::Skierg, 0.37));
        assert!((k.cycle - 0.37).abs() < 1e-12);
        assert!(k.pole_contact >= 0.0 && k.pole_contact <= 1.5);
    }

    #[test]
    fn bike_projection_scales_secondary_motion_with_effort() {
        let mut low = pose(Sport::Bike, 0.25);
        low.intensity = 0.0;
        let mut high = pose(Sport::Bike, 0.25);
        high.intensity = 1.0;
        let kl = solve_bike_kinematics(&low);
        let kh = solve_bike_kinematics(&high);
        assert!(kh.torso_sway.abs() > kl.torso_sway.abs());
        assert_eq!(kl.crank_angle, kh.crank_angle);
        // Ankle pitch is restrained around the neutral -0.05 offset.
        assert!(kl.ankle_pitch_left <= -0.05 + 0.3 * 0.55 + 1e-9);
    }

    #[test]
    fn ski_elbow_direction_retraces_one_arc() {
        let mut k = solve_skier_kinematics(&pose(Sport::Skierg, 0.05));
        let plant = solve_skier_elbow_direction(&k);
        // Down and tilted slightly forward (~166° from vertical) at the plant.
        assert!(plant.vertical < 0.0);
        assert!(plant.fore_aft > 0.0);

        k.cycle = SKI_POLE_OFF_CYCLE;
        k.pole_sweep = 1.0;
        let pole_off = solve_skier_elbow_direction(&k);
        // Down-back at pole-off.
        assert!(pole_off.vertical < 0.0);
        assert!(pole_off.fore_aft < 0.0);

        // Continuity at the pole-off boundary: press end == recovery start.
        k.cycle = SKI_POLE_OFF_CYCLE + 1e-9;
        k.pole_sweep = 1.0 - 1e-9;
        let recovery_start = solve_skier_elbow_direction(&k);
        assert!((recovery_start.vertical - pole_off.vertical).abs() < 1e-6);
        assert!((recovery_start.fore_aft - pole_off.fore_aft).abs() < 1e-6);
    }
}
