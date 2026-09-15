// SPDX-License-Identifier: GPL-3.0-or-later
//! Hand orientation and wrist budgets (Phase 7): the port of the web's
//! grip-frame orientation (`orientHandToGripChannel`,
//! `refineGripSpinForWrist`, `refineGripTiltForWrist` in `handGrip.ts`) and
//! the V4 controller's swing–twist wrist budget (`constrainWristFrame` plus
//! the SkiErg twist-keep and pronation-share rules in
//! `renderer3dV4Motion.ts`).
//!
//! Everything here is pure quaternion maths over the grip channel the
//! [`crate::replay::hand_grip`] module owns: orienting reads the channel,
//! the refinements spend its free spin on wrist flatness, and the budget
//! pass redistributes excess axial rotation into the forearm instead of
//! corkscrewing the wrist. The rest-frame axes a solve needs come from the
//! V4 contract at runtime (T6 wires them); the fixture corpus pins the
//! budgets through the equipment half of `equipment_contact_parity`, and the
//! unit tests below re-express the web `handGrip.test.ts` orientation suite.
//! Evaluation order mirrors the web modules.

use crate::models::Sport;
use crate::replay::hand_grip::{
    Quat, Vec3, add_scaled, axis_angle, dot, hand_channel_centre, hand_curl_axis_thumbward,
    hand_long_axis, hand_palm_normal_out, length, normalize, quat_inverse, quat_mul, quat_rotate,
    scale,
};

/// Wrist twist budget: pronation/supination share the wrist itself may carry
/// (rad, 75°). Beyond it rotation is redistributed into the forearm.
pub const WRIST_TWIST_BUDGET: f64 = 75.0 * std::f64::consts::PI / 180.0;
/// Wrist flexion/extension budget about the curl axis (rad, 150°).
pub const WRIST_FLEXION_BUDGET: f64 = 150.0 * std::f64::consts::PI / 180.0;
/// Wrist radial/ulnar deviation budget (rad, 150°).
pub const WRIST_DEVIATION_BUDGET: f64 = 150.0 * std::f64::consts::PI / 180.0;
/// SkiErg wrist twist keep-cap (rad, 30°): the ski clip's hands are authored
/// on the pole-grip branch, so the wrist keeps only this much and the
/// elbow-seam excess moves into shoulder internal rotation.
pub const SKI_WRIST_TWIST_KEEP: f64 = 30.0 * std::f64::consts::PI / 180.0;
/// Share of the residual SkiErg pronation carried as shoulder internal
/// rotation rather than forearm roll.
pub const SKI_PRONATION_SHOULDER_SHARE: f64 = 0.5;

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn quat_dot(left: Quat, right: Quat) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2] + left[3] * right[3]
}

/// Shortest-arc rotation taking one unit vector onto another (three.js
/// `setFromUnitVectors`, including the exactly-opposite fallback axis).
fn quat_from_unit_vectors(from: Vec3, to: Vec3) -> Quat {
    let r = dot(from, to) + 1.0;
    if r < f64::EPSILON {
        let fallback = if from[0].abs() > from[2].abs() {
            normalize([-from[1], from[0], 0.0])
        } else {
            normalize([0.0, -from[2], from[1]])
        };
        return [fallback[0], fallback[1], fallback[2], 0.0];
    }
    let axis = cross(from, to);
    normalize4([axis[0], axis[1], axis[2], r])
}

fn normalize4(quaternion: Quat) -> Quat {
    let len = (quat_dot(quaternion, quaternion)).sqrt();
    if len == 0.0 {
        return quaternion;
    }
    [
        quaternion[0] / len,
        quaternion[1] / len,
        quaternion[2] / len,
        quaternion[3] / len,
    ]
}

/// Orient a hand so its authored grip channel encloses the equipment: the
/// curl axis lands exactly on the thumb-ward signed shaft direction and the
/// remaining spin about the shaft resolves the channel onto the requested
/// side of the wrist (web `orientHandToGripChannel`).
///
/// `shaft_thumbward` is the equipment channel axis already signed from the
/// pinky side toward the thumb/index side; `roll_reference` is the desired
/// direction of (channel centre − wrist); `base` is the starting orientation
/// (the clip sample at runtime). Equipment roll about its own axis cancels
/// out — the handle feathers inside the fingers. Pass `roll_vector_local`
/// to resolve roll against the palm's true facing instead of the default
/// channel-centre construction ray.
#[must_use]
pub fn orient_hand_to_grip_channel(
    base: Quat,
    side: f64,
    radius: f64,
    shaft_thumbward: Vec3,
    roll_reference: Vec3,
    roll_vector_local: Option<Vec3>,
) -> Quat {
    let mut hand = base;
    let axis = quat_rotate(hand, hand_curl_axis_thumbward(side));
    let target = normalize(shaft_thumbward);
    hand = quat_mul(quat_from_unit_vectors(axis, target), hand);

    let mut local = match roll_vector_local {
        Some(vector) => normalize(vector),
        None => normalize(hand_channel_centre(radius, side)),
    };
    local = quat_rotate(hand, local);
    let mut reference = normalize(roll_reference);
    local = add_scaled(local, target, -dot(local, target));
    reference = add_scaled(reference, target, -dot(reference, target));
    if dot(local, local) > 1e-8 && dot(reference, reference) > 1e-8 {
        local = normalize(local);
        reference = normalize(reference);
        let cosine = dot(local, reference).clamp(-1.0, 1.0);
        // Both vectors are perpendicular to the shaft, so their signed angle
        // is an axial roll. atan2 keeps the exact 180° case on the shaft
        // axis; the unit-vector swing has no unique cross-axis there and can
        // destroy the alignment established above.
        let sine = dot(cross(local, reference), target);
        hand = quat_mul(axis_angle(target, sine.atan2(cosine)), hand);
    }
    hand
}

/// Spend the grip frame's free spin about the shaft on wrist flatness,
/// bounded so the sport-requested palm side stays authoritative (web
/// `refineGripSpinForWrist`). The channel axis is preserved exactly.
#[must_use]
pub fn refine_grip_spin_for_wrist(
    hand: Quat,
    side: f64,
    shaft_dir: Vec3,
    forearm_dir: Vec3,
    max_palm_deviation: f64,
) -> Quat {
    let shaft = normalize(shaft_dir);
    let mut long = quat_rotate(hand, hand_long_axis(side));
    long = add_scaled(long, shaft, -dot(long, shaft));
    let mut forearm = add_scaled(forearm_dir, shaft, -dot(forearm_dir, shaft));
    if dot(long, long) < 1e-8 || dot(forearm, forearm) < 1e-6 {
        return hand;
    }
    long = normalize(long);
    forearm = normalize(forearm);
    let angle = dot(cross(long, forearm), shaft).atan2(dot(long, forearm));
    let clamped = angle.clamp(-max_palm_deviation, max_palm_deviation);
    if clamped.abs() < 1e-6 {
        return hand;
    }
    quat_mul(axis_angle(shaft, clamped), hand)
}

/// Let a held shaft run diagonally across the palm to relieve remaining wrist
/// bend, rotating about the true palm normal so palm facing is preserved
/// (web `refineGripTiltForWrist`).
#[must_use]
pub fn refine_grip_tilt_for_wrist(
    hand: Quat,
    side: f64,
    forearm_dir: Vec3,
    comfort: f64,
    max_tilt: f64,
    strength: f64,
) -> Quat {
    let normal = normalize(quat_rotate(hand, hand_palm_normal_out(side)));
    let mut long = quat_rotate(hand, hand_long_axis(side));
    long = add_scaled(long, normal, -dot(long, normal));
    let mut forearm = add_scaled(forearm_dir, normal, -dot(forearm_dir, normal));
    if dot(long, long) < 1e-8 || dot(forearm, forearm) < 1e-6 {
        return hand;
    }
    long = normalize(long);
    forearm = normalize(forearm);
    let angle = dot(cross(long, forearm), normal).atan2(dot(long, forearm));
    let excess = (angle.abs() - comfort).max(0.0);
    let clamped = angle.signum() * excess.min(max_tilt) * strength.clamp(0.0, 1.0);
    if clamped.abs() < 1e-6 {
        return hand;
    }
    quat_mul(axis_angle(normal, clamped), hand)
}

/// Authored rest frame a wrist-budget solve is measured against: the hand's
/// rest orientation plus its anatomical axes in the forearm-local frame. T6
/// collects these from the vendored V4 contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WristRest {
    /// The hand's rest orientation in the forearm-local frame.
    pub hand_rest: Quat,
    /// Forearm-local bone axis: twist about it is pronation/supination.
    pub bone_axis_local: Vec3,
    /// Forearm's own long axis (through elbow and wrist).
    pub forearm_axis_local: Vec3,
    /// Curl-axis direction: swing about it is flexion/extension.
    pub flex_axis_local: Vec3,
    /// Remaining swing axis: radial/ulnar deviation.
    pub deviation_axis_local: Vec3,
}

/// Per-hand wrist metrics after a constrained solve (radians, web
/// `ReplayV4WristMetrics`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WristMetrics {
    /// Pronation/supination the wrist itself carries relative to rest.
    pub twist: f64,
    /// Flexion/extension the wrist carries about the curl axis.
    pub flexion: f64,
    /// Radial/ulnar deviation the wrist carries.
    pub deviation: f64,
    /// Axial rotation redistributed into the forearm this frame.
    pub forearm_twist: f64,
    /// Swing the clamp removed (0 while the grip frame is anatomically fine).
    pub clamped_swing: f64,
    /// Twist the grip frame demanded *before* the budget clamp. Kept
    /// alongside the clamped value so a saturating budget can be told apart
    /// from an upstream frame asking for far more twist than it should: a
    /// request sitting just past the budget is a tight budget, while 120°
    /// or a sign flip near the catch is a frame bug the clamp is quietly
    /// rescuing.
    pub requested_twist: f64,
    /// Axial rotation redistributed into the humerus this frame (SkiErg
    /// shoulder share of the elbow-seam excess; 0 for the other sports).
    pub humerus_roll: f64,
}

/// A constrained wrist solve: the wrist's new local orientation, the forearm
/// share to post-multiply into the forearm's local orientation (with its
/// inverse pre-multiplied into the wrist already applied when the swing is
/// rebuilt, mirroring the web order), and the metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WristSolve {
    /// The hand's constrained local orientation.
    pub effector: Quat,
    /// Axial share for the forearm's local orientation (`middle * twist`).
    pub forearm_twist: Quat,
    /// Whether a forearm share was applied.
    pub redistributed: bool,
    /// Post-solve metrics.
    pub metrics: WristMetrics,
}

/// Swing–twist wrist budget measured against the authored rest pose (web
/// `constrainWristFrame`).
///
/// The hand's local rotation decomposes about the forearm-local bone axis
/// into twist (pronation/supination) and swing (flexion/deviation). Twist
/// beyond the budget rotates the forearm about its own long axis instead —
/// neither joint moves, so every joint position and the world grip frame
/// stay exact while the visible twist is shared the way a pronating forearm
/// shares it. Swing beyond its budget is clamped outright: anatomy wins over
/// perfect channel alignment. Returns `None` if the solve goes non-finite.
#[must_use]
pub fn constrain_wrist_frame(effector: Quat, rest: &WristRest, sport: Sport) -> Option<WristSolve> {
    // Deviation from rest expressed in the forearm frame: current = D · rest.
    let delta = quat_mul(effector, quat_inverse(rest.hand_rest));
    let dot_twist = delta[0] * rest.bone_axis_local[0]
        + delta[1] * rest.bone_axis_local[1]
        + delta[2] * rest.bone_axis_local[2];
    // Signed twist about the bone axis; 2·atan2(dot, w) is exact for the
    // normalised (dot·â, w) projection. Wrap to the short representation.
    let mut twist_angle = 2.0 * dot_twist.atan2(delta[3]);
    if twist_angle > std::f64::consts::PI {
        twist_angle -= 2.0 * std::f64::consts::PI;
    } else if twist_angle < -std::f64::consts::PI {
        twist_angle += 2.0 * std::f64::consts::PI;
    }
    let twist = axis_angle(rest.bone_axis_local, twist_angle);
    let swing = quat_mul(delta, quat_inverse(twist));

    let twist_budget = match sport {
        Sport::Skierg => SKI_WRIST_TWIST_KEEP,
        Sport::Rower | Sport::Bike => WRIST_TWIST_BUDGET,
    };
    let kept_twist = twist_angle.clamp(-twist_budget, twist_budget);
    let excess = twist_angle - kept_twist;
    let redistributed = excess.abs() > 1e-5;
    let forearm_twist = axis_angle(rest.forearm_axis_local, excess);
    // The forearm's long axis passes through both the elbow and the wrist,
    // so rotating the forearm about it and counter-rotating the hand keeps
    // the world grip frame bit-exact.
    let mut constrained = effector;
    if redistributed {
        constrained = quat_mul(quat_inverse(forearm_twist), effector);
    }

    // Split the swing into flexion about the curl axis and deviation about
    // the remaining axis; each clamps on its own budget.
    let swing_half = swing[3].abs().clamp(0.0, 1.0).acos();
    let swing_full = 2.0 * swing_half;
    let swing_sin = (1.0 - swing[3] * swing[3]).max(0.0).sqrt();
    let (mut flexion, mut deviation) = (0.0, 0.0);
    if swing_sin > 1e-7 {
        let flip = if swing[3] >= 0.0 { 1.0 } else { -1.0 };
        let scale_factor = swing_full / swing_sin * flip;
        let v = [
            swing[0] * scale_factor,
            swing[1] * scale_factor,
            swing[2] * scale_factor,
        ];
        flexion = v[0] * rest.flex_axis_local[0]
            + v[1] * rest.flex_axis_local[1]
            + v[2] * rest.flex_axis_local[2];
        deviation = v[0] * rest.deviation_axis_local[0]
            + v[1] * rest.deviation_axis_local[1]
            + v[2] * rest.deviation_axis_local[2];
    }
    let kept_flexion = flexion.clamp(-WRIST_FLEXION_BUDGET, WRIST_FLEXION_BUDGET);
    let kept_deviation = deviation.clamp(-WRIST_DEVIATION_BUDGET, WRIST_DEVIATION_BUDGET);
    let clamped_swing = (flexion - kept_flexion).hypot(deviation - kept_deviation);
    if clamped_swing > 1e-5 {
        // Rebuild the in-budget swing and recompose the hand; the recompose
        // replaces the counter-rotation above, so restore the forearm share.
        let axis = add_scaled(
            scale(rest.flex_axis_local, kept_flexion),
            rest.deviation_axis_local,
            kept_deviation,
        );
        let kept_angle = length(axis);
        let kept_swing = if kept_angle > 1e-7 {
            axis_angle(scale(axis, 1.0 / kept_angle), kept_angle)
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        let kept_twist_quat = axis_angle(rest.bone_axis_local, kept_twist);
        constrained = quat_mul(quat_mul(kept_swing, kept_twist_quat), rest.hand_rest);
        if redistributed {
            constrained = quat_mul(quat_inverse(forearm_twist), constrained);
        }
    }

    if !constrained.iter().all(|c| c.is_finite()) || !forearm_twist.iter().all(|c| c.is_finite()) {
        return None;
    }
    Some(WristSolve {
        effector: constrained,
        forearm_twist,
        redistributed,
        metrics: WristMetrics {
            twist: kept_twist,
            flexion: kept_flexion,
            deviation: kept_deviation,
            forearm_twist: excess,
            clamped_swing,
            requested_twist: twist_angle,
            humerus_roll: 0.0,
        },
    })
}

/// Post-solve shoulder share of the SkiErg pronation (web
/// `distributeSkiElbowTwist`): the elbow-seam `excess_twist` rolls into the
/// humerus about its own long axis — shoulder internal rotation — while the
/// forearm's world orientation is restored. Returns the humerus roll, or 0
/// below the web's 1e-6 threshold. The bone re-targeting itself rides the
/// V4 controller (T6); this pins the portable arithmetic.
#[must_use]
pub fn ski_humerus_roll(excess_twist: f64) -> f64 {
    let roll = excess_twist * SKI_PRONATION_SHOULDER_SHARE;
    if roll.abs() < 1e-6 { 0.0 } else { roll }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::hand_grip::{hand_channel_centre, hand_curl_axis_thumbward};

    /// Shortest-arc angle between two orientations (three.js `angleTo`).
    fn quat_angle(left: Quat, right: Quat) -> f64 {
        2.0 * quat_dot(left, right).abs().clamp(-1.0, 1.0).acos()
    }

    const SCULL_RADIUS: f64 = 0.023;

    fn shaft() -> Vec3 {
        normalize([0.2, 0.05, 0.97])
    }

    fn roll() -> Vec3 {
        normalize([0.1, -1.0, 0.05])
    }

    fn orient() -> Quat {
        orient_hand_to_grip_channel(
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            SCULL_RADIUS,
            shaft(),
            roll(),
            None,
        )
    }

    #[test]
    fn curl_axis_lands_on_the_shaft() {
        let applied = quat_rotate(orient(), hand_curl_axis_thumbward(1.0));
        assert!(dot(applied, shaft()) > 0.9999);
    }

    #[test]
    fn channel_sits_on_the_requested_side() {
        let mut channel = quat_rotate(orient(), hand_channel_centre(SCULL_RADIUS, 1.0));
        let shaft_dir = shaft();
        channel = normalize(add_scaled(channel, shaft_dir, -dot(channel, shaft_dir)));
        let mut reference = roll();
        reference = normalize(add_scaled(reference, shaft_dir, -dot(reference, shaft_dir)));
        assert!(dot(channel, reference) > 0.9999);
    }

    #[test]
    fn equipment_roll_cancels_about_the_shaft() {
        let squared = orient();
        let feathered = orient_hand_to_grip_channel(
            axis_angle(shaft(), 0.72),
            1.0,
            SCULL_RADIUS,
            shaft(),
            roll(),
            None,
        );
        assert!(quat_dot(squared, feathered).abs() > 1.0 - 1e-9);
    }

    #[test]
    fn orientation_is_deterministic() {
        let (first, second) = (orient(), orient());
        assert!(quat_dot(first, second).abs() > 1.0 - 1e-12);
    }

    #[test]
    fn opposite_roll_keeps_shaft_alignment() {
        let aligned = quat_from_unit_vectors(hand_curl_axis_thumbward(1.0), shaft());
        let mut channel = quat_rotate(aligned, hand_channel_centre(SCULL_RADIUS, 1.0));
        let shaft_dir = shaft();
        channel = normalize(add_scaled(channel, shaft_dir, -dot(channel, shaft_dir)));
        let opposite = scale(channel, -1.0);
        let solved = orient_hand_to_grip_channel(
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            SCULL_RADIUS,
            shaft_dir,
            opposite,
            None,
        );
        let applied_axis = quat_rotate(solved, hand_curl_axis_thumbward(1.0));
        let mut applied_channel = quat_rotate(solved, hand_channel_centre(SCULL_RADIUS, 1.0));
        applied_channel = normalize(add_scaled(
            applied_channel,
            shaft_dir,
            -dot(applied_channel, shaft_dir),
        ));
        assert!(dot(applied_axis, shaft_dir) > 0.9999);
        assert!(dot(applied_channel, opposite) > 0.9999);
    }

    #[test]
    fn spin_relief_is_bounded_and_keeps_the_axis() {
        let shaft_dir = [0.0, 0.0, 1.0];
        let before = orient_hand_to_grip_channel(
            [0.0, 0.0, 0.0, 1.0],
            1.0,
            SCULL_RADIUS,
            shaft_dir,
            [0.0, -1.0, 0.0],
            None,
        );
        let after = refine_grip_spin_for_wrist(before, 1.0, shaft_dir, [0.0, 1.0, 0.0], 0.2);
        let rotation = quat_angle(before, after);
        assert!(rotation > 0.0);
        assert!(rotation <= 0.2 + 1e-9);
        let axis = quat_rotate(after, hand_curl_axis_thumbward(1.0));
        assert!(dot(axis, shaft_dir) > 0.9999);
    }

    #[test]
    fn tilt_relief_preserves_palm_facing() {
        let before = hand_palm_normal_out(1.0);
        let solved =
            refine_grip_tilt_for_wrist([0.0, 0.0, 0.0, 1.0], 1.0, [0.0, 1.0, 0.0], 0.0, 0.3, 1.0);
        assert!(quat_angle(solved, [0.0, 0.0, 0.0, 1.0]) > 0.0);
        let after = quat_rotate(solved, hand_palm_normal_out(1.0));
        // Bound the cosine shortfall, not the acos angle: acos amplifies
        // 1-ULP rotation noise near 1 into ~1e-8 rad of apparent turn.
        assert!(dot(before, after) > 1.0 - 1e-12);
    }

    fn rest_frame() -> WristRest {
        WristRest {
            hand_rest: [0.0, 0.0, 0.0, 1.0],
            bone_axis_local: [0.0, 1.0, 0.0],
            forearm_axis_local: [0.0, 1.0, 0.0],
            flex_axis_local: [1.0, 0.0, 0.0],
            deviation_axis_local: [0.0, 0.0, 1.0],
        }
    }

    #[test]
    fn twist_beyond_budget_moves_into_the_forearm() {
        // 100° of twist about the bone axis: the wrist keeps 75°, the
        // forearm carries 25°.
        let rest = rest_frame();
        let effector = axis_angle([0.0, 1.0, 0.0], 100.0 * std::f64::consts::PI / 180.0);
        let solved = constrain_wrist_frame(effector, &rest, Sport::Rower).expect("finite");
        assert!(solved.redistributed);
        assert!((solved.metrics.twist - WRIST_TWIST_BUDGET).abs() < 1e-9);
        assert!((solved.metrics.forearm_twist - 25.0 * std::f64::consts::PI / 180.0).abs() < 1e-9);
        assert_eq!(solved.metrics.clamped_swing, 0.0);
    }

    #[test]
    fn ski_keep_is_tighter_than_the_generic_budget() {
        let rest = rest_frame();
        let effector = axis_angle([0.0, 1.0, 0.0], 100.0 * std::f64::consts::PI / 180.0);
        let solved = constrain_wrist_frame(effector, &rest, Sport::Skierg).expect("finite");
        assert!((solved.metrics.twist - SKI_WRIST_TWIST_KEEP).abs() < 1e-9);
        assert!((solved.metrics.forearm_twist - 70.0 * std::f64::consts::PI / 180.0).abs() < 1e-9);
    }

    #[test]
    fn in_budget_pose_passes_through_untouched() {
        let rest = rest_frame();
        let effector = axis_angle([0.0, 1.0, 0.0], 0.3);
        let solved = constrain_wrist_frame(effector, &rest, Sport::Bike).expect("finite");
        assert!(!solved.redistributed);
        for (axis, expected) in effector.iter().enumerate() {
            assert!((solved.effector[axis] - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn swing_beyond_budget_clamps_on_its_own_axis() {
        // 170° of pure flexion: kept at the 150° budget with the excess
        // reported, while a healthy 55° grip extension is never touched.
        let rest = rest_frame();
        let over = axis_angle([1.0, 0.0, 0.0], 170.0 * std::f64::consts::PI / 180.0);
        let solved = constrain_wrist_frame(over, &rest, Sport::Rower).expect("finite");
        assert!((solved.metrics.flexion - WRIST_FLEXION_BUDGET).abs() < 1e-9);
        assert!(solved.metrics.clamped_swing > 0.0);
        let healthy = axis_angle([1.0, 0.0, 0.0], 55.0 * std::f64::consts::PI / 180.0);
        let kept = constrain_wrist_frame(healthy, &rest, Sport::Rower).expect("finite");
        assert_eq!(kept.metrics.clamped_swing, 0.0);
    }

    #[test]
    fn ski_shoulder_share_is_half_the_seam_excess() {
        assert_eq!(ski_humerus_roll(0.0), 0.0);
        assert!((ski_humerus_roll(0.4) - 0.2).abs() < 1e-15);
        assert_eq!(ski_humerus_roll(1e-7), 0.0);
    }
}
