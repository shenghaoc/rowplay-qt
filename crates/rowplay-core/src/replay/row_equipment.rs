// SPDX-License-Identifier: GPL-3.0-or-later
//! Rowing-shell equipment contract (Phase 7): the port of the web's
//! `rowRig.ts` contact landmarks and closed-chain solves.
//!
//! The physical landmarks of the authored shell (foot contact, stretcher,
//! scull grip, oarlock, elbow corridor/plane, draw flexions) plus the rigid
//! oar yaw solve and the law-of-cosines elbow conversions. Pure geometry,
//! Qt-free and I/O-free like the rest of `rowplay-core`. Evaluation order
//! mirrors the web module so the golden corpus
//! (`replay-current-main-equipment.json`) compares within 1e-12.

/// Foot contact: lateral offset, height, and stern-side position (m).
pub const FOOT_CONTACT: [f64; 3] = [0.12, 0.215, 0.75];
/// Stretcher board centre height/position (m).
pub const STRETCHER_CENTER_Y: f64 = 0.295;
/// Stretcher board centre fore-aft position (m).
pub const STRETCHER_CENTER_Z: f64 = 0.68;
/// Stretcher board rotation (rad, −48°).
pub const STRETCHER_BOARD_ROTATION: f64 = -48.0 * std::f64::consts::PI / 180.0;
/// Shoe pitch at the catch (rad, −35°).
pub const STRETCHER_SHOE_CATCH_PITCH: f64 = -35.0 * std::f64::consts::PI / 180.0;
/// Shoe pitch at the finish (rad, −42°).
pub const STRETCHER_SHOE_FINISH_PITCH: f64 = -42.0 * std::f64::consts::PI / 180.0;
/// Scull handle rubber radius (m) — the grip closure's equipment surface.
pub const SCULL_GRIP_RADIUS: f64 = 0.023;
/// Scull handle length (m).
pub const SCULL_GRIP_LENGTH: f64 = 0.32;
/// Palm anchor inboard of the flat thumb stop (m).
pub const SCULL_GRIP_ANCHOR_FROM_END: f64 = 0.04;
/// Scull-handle thumb opposition closing onto the rubber (rad, thumb-root
/// bone frame; web `gripContractFor` rower branch).
pub const SCULL_THUMB_OPPOSE: f64 = 0.3;
/// Oarlock pin position (m): lateral, height, stern-side.
pub const OARLOCK: [f64; 3] = [0.88, 0.51, 0.28];
/// Elbow corridor: maximum outboard displacement from the working plane (m).
pub const ELBOW_CORRIDOR_MAX_OUTBOARD: f64 = 0.11;
/// Elbow corridor: token inboard excursion toward the spine (m).
pub const ELBOW_CORRIDOR_MAX_INBOARD: f64 = 0.05;
/// Elbow corridor: how far the joint may trail the shoulder plane (m).
pub const ELBOW_CORRIDOR_MAX_BEHIND_SHOULDER: f64 = 0.19;
/// Elbow corridor: ceiling of outboard excursion over rearward travel while
/// drawing (recalibrated 0.5 → 0.7 with the flat-wrist sculling grip).
pub const ELBOW_CORRIDOR_MAX_OUTBOARD_PER_REARWARD: f64 = 0.7;
/// Interior elbow flexion at a full draw (rad).
pub const DRAW_FINISH_FLEXION: f64 = 2.46;
/// Soft elbow unlock held through the drive (rad from straight).
pub const DRAW_SOFT_FLEXION: f64 = 0.32;
/// Elbow-plane schedule: flexion below which the plane has no authority.
pub const ELBOW_PLANE_AUTHORITY_START: f64 = 0.14;
/// Elbow-plane schedule: flexion at which the drawn plane owns the joint.
pub const ELBOW_PLANE_AUTHORITY_FULL: f64 = 0.55;
/// Relaxed near-straight plane direction, athlete frame.
pub const ELBOW_PLANE_RELAXED: [f64; 3] = [0.32, -1.0, -0.12];
/// Drawn plane direction, athlete frame (two-bone branch hint).
pub const ELBOW_PLANE_DRAWN: [f64; 3] = [0.02, -0.72, -0.69];
/// Chord-frame outboard station of the drawn elbow on its circle.
pub const ELBOW_PLANE_DRAWN_OUTBOARD_WEIGHT: f64 = 0.24;
/// Chord-frame down station of the drawn elbow on its circle
/// (web `Math.sqrt(1 - 0.24 * 0.24)`; pinned here so the pair stays unit).
pub const ELBOW_PLANE_DRAWN_DOWN_WEIGHT: f64 = 0.970_772_887_960_927_8;
/// Authored oar yaw at the catch (rad, web `OAR_YAW_CATCH`): positive puts the
/// inboard grip ahead of the shoulders toward the stretcher.
pub const OAR_YAW_CATCH: f64 = 0.68;
/// Authored oar yaw at the finish (rad, web `OAR_DRAW_YAW`): the drive crosses
/// the pin normal and finishes on the athlete-facing side at the lower ribs.
pub const OAR_YAW_DRAW: f64 = -0.8;
/// Upper-arm length the web's procedural avatar reach is built from (m).
pub const UPPER_ARM_LENGTH: f64 = 0.39;
/// Forearm length the web's procedural avatar reach is built from (m).
pub const FOREARM_LENGTH: f64 = 0.38;

/// The web's requested shoulder→wrist reach for a given `armDraw`
/// (`renderer3dRowAvatar.ts` `requestedRowerWristReach`).
///
/// The armDraw channel is the **only velocity profile in the arm chain**: it
/// schedules the elbow's interior flexion affinely from the soft long-arm
/// unlock ([`DRAW_SOFT_FLEXION`]) to the measured production finish fold
/// ([`DRAW_FINISH_FLEXION`]), the law of cosines converts that flexion into a
/// reach, and the rigid-oar solve places the handle on that shrinking sphere.
/// The web subtracts a 2 mm grip-contact bias. This is the arm-authority
/// schedule: the oar yaw is solved to *meet* this reach, not the reverse.
#[must_use]
pub fn rower_requested_wrist_reach(arm_draw: f64, upper_arm: f64, forearm: f64) -> f64 {
    let draw = clamp(arm_draw, 0.0, 1.0);
    let flexion = DRAW_SOFT_FLEXION + draw * (DRAW_FINISH_FLEXION - DRAW_SOFT_FLEXION);
    rower_reach_for_flexion(flexion, upper_arm, forearm) - 0.002
}

/// The web's authored oar yaw for a draw fraction (`OAR_YAW_CATCH +
/// draw·(OAR_YAW_DRAW − OAR_YAW_CATCH)`): the staged branch the reach solve
/// falls back to and wraps onto, not the rendered yaw.
#[must_use]
pub fn rower_authored_oar_yaw(arm_draw: f64) -> f64 {
    let draw = clamp(arm_draw, 0.0, 1.0);
    OAR_YAW_CATCH + draw * (OAR_YAW_DRAW - OAR_YAW_CATCH)
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

/// Continuous yaw branch on a rigid oar's inboard circle satisfying a
/// requested shoulder-to-grip reach (web `solveRowerOarYaw`).
///
/// `shoulder` is the joint position; the pin, signed inboard lever,
/// blade-depth roll, requested reach, staged `preferred_yaw` and the
/// reach-boundary override mirror the web signature. Returns the staged yaw
/// whenever the shoulder can already reach it, else the physical root whose
/// inboard grip sits farther toward the stretcher, wrapped onto the staged
/// branch so the oar never takes a ±2π jump.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn solve_rower_oar_yaw(
    shoulder: [f64; 3],
    pin_x: f64,
    pin_y: f64,
    pin_z: f64,
    signed_inboard: f64,
    blade_roll: f64,
    requested_reach: f64,
    preferred_yaw: f64,
    force_reach_boundary: bool,
) -> f64 {
    let pin_delta = [
        pin_x - shoulder[0],
        pin_y - shoulder[1],
        pin_z - shoulder[2],
    ];
    // Three's XYZ Euler order sends an oar-local X vector to
    // (cos(roll)cos(yaw), sin(roll), −cos(roll)sin(yaw)).
    let roll_cos = blade_roll.cos();
    let roll_sin = blade_roll.sin();
    let projected_x = pin_delta[0] * roll_cos;
    let projected_z = -pin_delta[2] * roll_cos;
    let amplitude = projected_x.hypot(projected_z);
    if amplitude < 1e-8 || signed_inboard.abs() < 1e-8 {
        return preferred_yaw;
    }
    let base_distance_squared = pin_delta[0] * pin_delta[0]
        + pin_delta[1] * pin_delta[1]
        + pin_delta[2] * pin_delta[2]
        + signed_inboard * signed_inboard;
    let preferred_distance_squared = base_distance_squared
        + 2.0
            * signed_inboard
            * (projected_x * preferred_yaw.cos()
                + projected_z * preferred_yaw.sin()
                + pin_delta[1] * roll_sin);
    if !force_reach_boundary
        && preferred_distance_squared <= requested_reach * requested_reach + 1e-5
    {
        return preferred_yaw;
    }
    let cosine = clamp(
        (requested_reach * requested_reach
            - base_distance_squared
            - 2.0 * signed_inboard * pin_delta[1] * roll_sin)
            / (2.0 * signed_inboard * amplitude),
        -1.0,
        1.0,
    );
    let center = projected_z.atan2(projected_x);
    let offset = cosine.acos();
    let first = center + offset;
    let second = center - offset;
    // The catch is the root whose inboard grip sits farther toward the
    // stretcher (+Z).
    let first_grip_z = pin_z - signed_inboard * roll_cos * first.sin();
    let second_grip_z = pin_z - signed_inboard * roll_cos * second.sin();
    let selected = if first_grip_z >= second_grip_z {
        first
    } else {
        second
    };
    preferred_yaw
        + (selected - preferred_yaw)
            .sin()
            .atan2((selected - preferred_yaw).cos())
}

/// Shoulder→wrist reach that yields `flexion` for the given segment pair
/// (web `rowerReachForFlexion`, law of cosines).
#[must_use]
pub fn rower_reach_for_flexion(flexion: f64, upper_arm: f64, forearm: f64) -> f64 {
    let clamped = clamp(flexion, 0.0, std::f64::consts::PI - 1e-3);
    (upper_arm * upper_arm + forearm * forearm + 2.0 * upper_arm * forearm * clamped.cos())
        .max(0.0)
        .sqrt()
}

/// Elbow flexion (radians away from a straight arm) for a given reach
/// (web `rowerElbowFlexion`).
#[must_use]
pub fn rower_elbow_flexion(chord_length: f64, upper_arm: f64, forearm: f64) -> f64 {
    let clamped = clamp(
        chord_length,
        (upper_arm - forearm).abs() + 1e-6,
        upper_arm + forearm - 1e-6,
    );
    let cos_interior = (upper_arm * upper_arm + forearm * forearm - clamped * clamped)
        / (2.0 * upper_arm * forearm);
    std::f64::consts::PI - clamp(cos_interior, -1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawn_weights_form_a_unit_pair() {
        let norm = (ELBOW_PLANE_DRAWN_OUTBOARD_WEIGHT * ELBOW_PLANE_DRAWN_OUTBOARD_WEIGHT
            + ELBOW_PLANE_DRAWN_DOWN_WEIGHT * ELBOW_PLANE_DRAWN_DOWN_WEIGHT)
            .sqrt();
        assert!((norm - 1.0).abs() < 1e-15);
    }

    #[test]
    fn straight_arm_reaches_full_extension() {
        let reach = rower_reach_for_flexion(0.0, 0.49, 0.47);
        assert!((reach - 0.96).abs() < 1e-12);
        // The elbow solve clamps the chord 1 µm short of full extension to
        // stay clear of the straight-arm singularity, so the recovered
        // flexion is small but nonzero — exactly like the web's clamp.
        assert!(rower_elbow_flexion(reach, 0.49, 0.47) < 1e-2);
    }

    #[test]
    fn flexion_and_reach_round_trip() {
        for flexion in [0.3, 0.9, 1.8, 2.46] {
            let reach = rower_reach_for_flexion(flexion, 0.49, 0.47);
            let back = rower_elbow_flexion(reach, 0.49, 0.47);
            assert!((back - flexion).abs() < 1e-9, "{flexion}: {back}");
        }
    }

    #[test]
    fn degenerate_oar_falls_back_to_staged_yaw() {
        assert_eq!(
            solve_rower_oar_yaw([0.0, 0.0, 0.0], 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, -1.1, false),
            -1.1
        );
    }

    #[test]
    fn reachable_staged_yaw_passes_through() {
        // Shoulder at the pin with a generous reach: the staged yaw holds.
        let yaw = solve_rower_oar_yaw(
            [0.88, 0.51, 0.28],
            0.88,
            0.51,
            0.28,
            0.78,
            0.0,
            2.0,
            -1.1,
            false,
        );
        assert_eq!(yaw, -1.1);
    }
}
