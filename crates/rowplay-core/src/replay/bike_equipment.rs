// SPDX-License-Identifier: GPL-3.0-or-later
//! Bike fit contract (Phase 7): the port of the web's `bikeRig.js`.
//!
//! Authored the way a bike is actually fitted: the frame comes from real
//! road geometry and the rider's hip is *derived* from leg reach against the
//! bottom bracket. Every intermediate is a named constant mirroring the web
//! module's derivation order so the golden corpus
//! (`replay-current-main-equipment.json`, `bike` tree) compares within
//! 1e-12. Pure geometry, Qt-free and I/O-free like the rest of
//! `rowplay-core`.

/// Femur length: `v4LeftLowerLeg` offset from `v4LeftUpperLeg` (m).
pub const ATHLETE_THIGH: f64 = 0.4915;
/// Tibia length: `v4LeftFoot` offset from `v4LeftLowerLeg` (m).
pub const ATHLETE_SHIN: f64 = 0.4794;
/// Ankle-to-sole drop (m).
pub const ATHLETE_SOLE_DROP: f64 = 0.055;
/// Pelvis root to femoral head: the femur starts 25 mm below `v4Hips` (m).
pub const HIP_ROOT_TO_FEMORAL_HEAD: f64 = 0.025;
/// Knee flexion at bottom dead centre: the Holmes road-fit window is 25–35°,
/// 30° is the middle (rad).
pub const KNEE_FLEXION_AT_BDC: f64 = 30.0 * std::f64::consts::PI / 180.0;
/// Wheel major radius, rim centreline (m).
pub const WHEEL_RADIUS: f64 = 0.31;
/// Radial thickness of the tyre shell (m).
pub const TYRE_TUBE: f64 = 0.025;
/// Bottom-bracket drop below the axle line (m).
pub const BB_DROP: f64 = 0.07;
/// Bottom-bracket height (m): the axle line minus the drop.
pub const BB_Y: f64 = 0.335 - BB_DROP;
/// Bottom-bracket fore-aft position (m).
pub const BB_Z: f64 = -0.05;
/// Crank arm length for a rider this size (m).
pub const CRANK_RADIUS: f64 = 0.1725;
/// Femoral-head setback behind the bottom bracket (m).
pub const PELVIS_SETBACK: f64 = 0.12;
/// Head tube angle from horizontal (rad, 73°).
pub const HEAD_ANGLE: f64 = 73.0 * std::f64::consts::PI / 180.0;
/// Head tube length (m).
pub const HEAD_TUBE_LENGTH: f64 = 0.155;
/// Frame stack from the bottom bracket (m).
pub const STACK: f64 = 0.575;
/// Frame reach from the bottom bracket (m).
pub const REACH: f64 = 0.385;
/// Fork offset ahead of the steering axis (m).
pub const FORK_RAKE: f64 = 0.05;
/// Chainstay length (m).
pub const CHAINSTAY: f64 = 0.41;
/// Ischial sit-bone surface relative to `v4Hips` (m).
pub const SIT_SURFACE_FROM_HIP_Y: f64 = -0.158;
/// Perineal centreline relative to the hips (m).
pub const PERINEUM_FROM_HIP_Y: f64 = -0.1952;
/// Soft cushion nestle, sinking the sit surface into the pad (m).
pub const SIT_NESTLE: f64 = 0.005;
/// Saddle performance pad half-height above the centre marker (m).
pub const SADDLE_PAD_HALF_HEIGHT: f64 = 0.022;
/// Sit-bone contact behind the pelvis root (m).
pub const SIT_CONTACT_Z_FROM_HIP: f64 = -0.115;
/// Seat-tube angle (rad, 73°).
pub const SEAT_TUBE_ANGLE: f64 = 73.0 * std::f64::consts::PI / 180.0;
/// Exposed seatpost between the seat cluster and the rail clamp (m).
pub const SEATPOST_EXPOSED: f64 = 0.06;
/// Saddle rail clamp offset ahead of the sit-bone origin (m).
pub const SADDLE_CLAMP_Z: f64 = 0.05;
/// Crank lateral offset (m).
pub const CRANK_LATERAL: f64 = 0.09;
/// Handlebar width: 40 cm bar, half-span (m).
pub const HANDLEBAR_GRIP_HALF_SPAN: f64 = 0.2;
/// Hood-body X rotation, shared with the visual hood mesh (rad).
pub const HOOD_ROTATION_X: f64 = -0.24;
/// Hood-body analytic contact radius: the rendered 36 mm half-section (m).
pub const HOOD_RADIUS: f64 = 0.018;
/// Hood thumb opposition hooking under the body (rad, thumb-root bone frame;
/// web `gripContractFor` bike branch, fitted band 1.52–1.60).
pub const HOOD_THUMB_OPPOSE: f64 = 1.56;

/// Femoral-head-to-pedal distance at BDC, by the law of cosines (only
/// femur + tibia count; the sole drop buys no extra reach).
#[must_use]
pub fn leg_reach() -> f64 {
    (ATHLETE_THIGH * ATHLETE_THIGH + ATHLETE_SHIN * ATHLETE_SHIN
        - 2.0 * ATHLETE_THIGH * ATHLETE_SHIN * (std::f64::consts::PI - KNEE_FLEXION_AT_BDC).cos())
    .sqrt()
}

/// Bottom-bracket position (m).
#[must_use]
pub fn bottom_bracket() -> [f64; 3] {
    [0.0, BB_Y, BB_Z]
}

/// Pelvis fore-aft position: the femoral-head setback behind the bracket (m).
#[must_use]
pub fn pelvis_z() -> f64 {
    BB_Z - PELVIS_SETBACK
}

/// Axle height so the tyre outer shell rests on the ground plane (web
/// `bikeWheelAxleY`).
#[must_use]
pub fn wheel_axle_y() -> f64 {
    WHEEL_RADIUS + TYRE_TUBE
}

/// Front wheel axle fore-aft position (m).
#[must_use]
pub fn front_axle_z() -> f64 {
    head_top()[2] + (head_top()[1] - wheel_axle_y()) / HEAD_ANGLE.tan() + FORK_RAKE
}

/// Rear wheel axle fore-aft position (m).
#[must_use]
pub fn rear_axle_z() -> f64 {
    BB_Z - CHAINSTAY
}

/// Head tube top (m).
#[must_use]
pub fn head_top() -> [f64; 3] {
    [0.0, BB_Y + STACK, BB_Z + REACH]
}

/// Head tube bottom: the steerer leans back as it rises (m).
#[must_use]
pub fn head_bottom() -> [f64; 3] {
    let top = head_top();
    [
        0.0,
        top[1] - HEAD_TUBE_LENGTH,
        top[2] + HEAD_TUBE_LENGTH / HEAD_ANGLE.tan(),
    ]
}

/// Saddle centre (m): over the sit bones, pad top following the hip.
#[must_use]
pub fn saddle() -> [f64; 3] {
    [0.0, saddle_y(), saddle_z()]
}

/// Saddle centre height (m).
#[must_use]
pub fn saddle_y() -> f64 {
    saddle_pad_top_y() - SADDLE_PAD_HALF_HEIGHT
}

/// Saddle origin fore-aft: under the sit bones, not the pelvis root (m).
#[must_use]
pub fn saddle_z() -> f64 {
    pelvis_z() + SIT_CONTACT_Z_FROM_HIP
}

/// Pad top height (m): follows the hip so the sit surface lands on cushion.
#[must_use]
pub fn saddle_pad_top_y() -> f64 {
    rider_hip_y() + SIT_NESTLE + SIT_SURFACE_FROM_HIP_Y
}

/// Rail clamp position (m): under the saddle's mid-length.
#[must_use]
pub fn saddle_clamp() -> [f64; 3] {
    [
        0.0,
        saddle_y() - SADDLE_PAD_HALF_HEIGHT,
        saddle_z() + SADDLE_CLAMP_Z,
    ]
}

/// Seat-tube top (m): one exposed seatpost down the seat-tube axis.
#[must_use]
pub fn seat_cluster() -> [f64; 3] {
    let clamp = saddle_clamp();
    [
        0.0,
        clamp[1] - SEATPOST_EXPOSED * SEAT_TUBE_ANGLE.sin(),
        clamp[2] + SEATPOST_EXPOSED * SEAT_TUBE_ANGLE.cos(),
    ]
}

/// Stem clamp / bar centre (m).
#[must_use]
pub fn handlebar_base() -> [f64; 3] {
    let top = head_top();
    [0.0, top[1] + 0.015, top[2] + 0.09]
}

/// Brake-hood grip point: height, fore-aft, half-span (m).
#[must_use]
pub fn handlebar_grip() -> [f64; 3] {
    let top = head_top();
    [top[1], top[2] + 0.175, HANDLEBAR_GRIP_HALF_SPAN]
}

/// Crank lateral offset and arm length (m).
#[must_use]
pub fn crank() -> [f64; 2] {
    [CRANK_LATERAL, CRANK_RADIUS]
}

/// Hip / pelvis target height (m): leg reach against the bottom bracket.
#[must_use]
pub fn rider_hip_y() -> f64 {
    let bdc_pedal_y = BB_Y - CRANK_RADIUS;
    let femoral_head_rise = (leg_reach() * leg_reach() - PELVIS_SETBACK * PELVIS_SETBACK).sqrt();
    bdc_pedal_y + femoral_head_rise + HIP_ROOT_TO_FEMORAL_HEAD
}

/// Hip / pelvis target (m).
#[must_use]
pub fn rider_root() -> [f64; 3] {
    [0.0, rider_hip_y(), pelvis_z()]
}

/// Compact residual nestle of the procedural pelvis into the seat shell (m).
#[must_use]
pub fn rider_pelvis_offset() -> [f64; 3] {
    [0.0, 0.0, -0.005]
}

/// Measured ischial sit surface relative to `v4Hips` (m).
#[must_use]
pub fn rider_sit_surface_from_hip() -> [f64; 3] {
    [0.0, SIT_SURFACE_FROM_HIP_Y, SIT_CONTACT_Z_FROM_HIP]
}

/// Rider limb lengths, so procedural legs cannot drift from the V4 rig (m).
#[must_use]
pub fn rider_athlete() -> [f64; 3] {
    [ATHLETE_THIGH, ATHLETE_SHIN, ATHLETE_SOLE_DROP]
}

/// Authored sit-bone pad top Y in avatar-local space (web `bikeSaddleTopY`).
#[must_use]
pub fn saddle_top_y() -> f64 {
    saddle()[1] + SADDLE_PAD_HALF_HEIGHT
}

/// Knee flexion (radians away from a straight leg) with the crank at `angle`,
/// 0 at top dead centre increasing forward (web `bikeKneeFlexion`).
#[must_use]
pub fn knee_flexion(angle: f64) -> f64 {
    let bb = bottom_bracket();
    let pedal_y = bb[1] + CRANK_RADIUS * angle.cos();
    let pedal_z = bb[2] + CRANK_RADIUS * angle.sin();
    let root = rider_root();
    let hip_y = root[1] - HIP_ROOT_TO_FEMORAL_HEAD;
    let hip_z = root[2];
    let reach = (hip_y - pedal_y).hypot(hip_z - pedal_z);
    let cos_knee = (ATHLETE_THIGH * ATHLETE_THIGH + ATHLETE_SHIN * ATHLETE_SHIN - reach * reach)
        / (2.0 * ATHLETE_THIGH * ATHLETE_SHIN);
    std::f64::consts::PI - cos_knee.clamp(-1.0, 1.0).acos()
}

/// Hip Y placing `sitSurfaceFromHip` on the pad top minus soft nestle: the
/// single seating derivation (web `bikeRiderHipY`).
#[must_use]
pub fn derived_rider_hip_y() -> f64 {
    saddle_top_y() - SIT_NESTLE - SIT_SURFACE_FROM_HIP_Y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seating_derivation_is_self_consistent() {
        // The rig's authored hip is exactly the seating derivation: height
        // tweaks cannot drift the rider through the cushion.
        assert!((rider_hip_y() - derived_rider_hip_y()).abs() < 1e-12);
    }

    #[test]
    fn axle_height_rests_the_tread_on_the_ground() {
        assert!((wheel_axle_y() - (WHEEL_RADIUS + TYRE_TUBE)).abs() < 1e-15);
        assert!((BB_Y - (wheel_axle_y() - BB_DROP)).abs() < 1e-15);
    }

    #[test]
    fn bdc_flexion_is_the_road_fit_window() {
        assert!((knee_flexion(std::f64::consts::PI) - KNEE_FLEXION_AT_BDC).abs() < 1e-9);
    }

    #[test]
    fn knee_bends_further_past_top_dead_centre() {
        let tdc = knee_flexion(0.0);
        assert!(tdc > KNEE_FLEXION_AT_BDC);
        assert!(knee_flexion(std::f64::consts::PI / 2.0) > KNEE_FLEXION_AT_BDC);
    }
}
