// SPDX-License-Identifier: GPL-3.0-or-later
//! Equipment motion and placement for the scene (Phase 5b spec R3): the
//! per-frame rotations of the oar rigs, ski poles, cranks and wheels as
//! ready-to-assign quaternions, the static instance placements of the
//! repeated templates, and the bounds fit that seats the V3 leaf shells
//! (blade, pole parts) in their runtime slots (Studio `attachFittedVisual`,
//! the web's authored-leaf slots).
//!
//! Angles come in as the rig pose's radians; everything QML receives is a
//! quaternion, a position, a scale or degrees, so no rotation is composed
//! in QML.

use std::f64::consts::PI;

use rowplay_core::replay::two_bone::{add, cross, dot, length, scale, sub};
use serde_json::{Value, json};

use super::anchors::{OARLOCK_PIVOT, SKI_ANCHOR};
use super::pose::geometry::{
    BIKE_AXLE_Y, BIKE_BB_Z, BIKE_HEAD_TOP_Y, BIKE_HEAD_TOP_Z, SKI_POLE_LENGTH,
};

/// A mesh's `(min, max)` POSITION bounds, metres.
pub type Bounds = ([f64; 3], [f64; 3]);

/// The web row avatar's blade placement along the oar: `blade.position.set(
/// side * 1.82, -0.06, 0)` in the oar frame whose origin is the oarlock.
pub const BLADE_OFFSET: [f64; 3] = [1.82, -0.06, 0.0];
/// Head-tube angle of the bike frame (web `HEAD_ANGLE`, 73°).
const BIKE_HEAD_ANGLE: f64 = 73.0 * PI / 180.0;
/// Fork rake and chainstay (web `FORK_RAKE`, `CHAINSTAY`).
const BIKE_FORK_RAKE: f64 = 0.05;
const BIKE_CHAINSTAY: f64 = 0.41;
/// The web's fist grip radius the pole grip leaf is fitted to
/// (`HAND_FIST_REFERENCE_GRIP_RADIUS`).
pub const POLE_GRIP_RADIUS: f64 = 0.016;
/// Pole grip length (Studio `gripLength`).
pub const POLE_GRIP_LENGTH: f64 = 0.15;

/// Front axle z on the bike frame (web `FRONT_AXLE_Z`).
#[must_use]
pub fn bike_front_axle_z() -> f64 {
    BIKE_HEAD_TOP_Z + (BIKE_HEAD_TOP_Y - BIKE_AXLE_Y) / BIKE_HEAD_ANGLE.tan() + BIKE_FORK_RAKE
}

/// Rear axle z on the bike frame (web `REAR_AXLE_Z`).
#[must_use]
pub fn bike_rear_axle_z() -> f64 {
    BIKE_BB_Z - BIKE_CHAINSTAY
}

/// A quaternion `x, y, z, w` for a rotation about a unit axis.
#[must_use]
pub fn axis_angle(axis: [f64; 3], angle: f64) -> [f64; 4] {
    let half = if angle.is_finite() { angle * 0.5 } else { 0.0 };
    let s = half.sin();
    normalise([axis[0] * s, axis[1] * s, axis[2] * s, half.cos()])
}

/// Quaternion product `a · b` (apply `b`, then `a`).
#[must_use]
pub fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let av = [a[0], a[1], a[2]];
    let bv = [b[0], b[1], b[2]];
    let v = add(add(scale(bv, a[3]), scale(av, b[3])), cross(av, bv));
    normalise([v[0], v[1], v[2], a[3] * b[3] - dot(av, bv)])
}

fn normalise(q: [f64; 4]) -> [f64; 4] {
    let magnitude = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if !magnitude.is_finite() || magnitude < 1e-9 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [
        q[0] / magnitude,
        q[1] / magnitude,
        q[2] / magnitude,
        q[3] / magnitude,
    ]
}

/// Rotate vector `v` by unit quaternion `q` (Hamilton product shortcut).
#[must_use]
pub fn rotate_vec(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u = [q[0], q[1], q[2]];
    let t = scale(cross(u, v), 2.0);
    add(add(v, scale(t, q[3])), cross(u, t))
}

/// Compute a blade's position (xyz) from an oarlock position and oar
/// quaternion: `oarlock + oarQuat × BLADE_OFFSET`.
#[must_use]
pub fn blade_position(oarlock: [f64; 3], oar_quat: [f64; 4]) -> [f64; 3] {
    add(oarlock, rotate_vec(oar_quat, BLADE_OFFSET))
}

/// Compute a pole leaf's final position: `polePos + poleRot × leafFitPos`.
#[must_use]
pub fn pole_leaf_position(
    pole_pos: [f64; 3],
    pole_rot: [f64; 4],
    leaf_fit_pos: [f64; 3],
) -> [f64; 3] {
    add(pole_pos, rotate_vec(pole_rot, leaf_fit_pos))
}

/// The oar-rig instance rotations `[left, right]` for a sweep and feather.
///
/// The web drives `oar.group.rotation` as three.js Euler(0, yaw, roll) in
/// the default XYZ order (renderer3dRowAvatar `placeOars`), i.e. the group
/// quaternion is `qy(yaw) ⊗ qz(roll)` — the roll is applied **first**, in
/// the oar's local frame, so its vertical lift (`inboard · sin(roll)`) is
/// yaw-independent. `ROWER_OARLOCK`'s comment names the coupling this buys:
/// "the drive-side roll that buries the spoon just below the water then
/// puts the handles at the drive height". Composing `qz ⊗ qy` instead
/// scales the lift by `cos(yaw)` — up to 6× short at the finish — and was
/// measured 0.18 m off the web's rendered grip at the catch
/// (`replay-rig-phase-parity.json` `handTargets`, rowplay#199).
///
/// The authored template is the right oar; the left instance carries the
/// README's π mirror first (the web mirrors only the visual child
/// `oarVisual`, leaving the group quaternion pure — folding the mirror
/// into the instance after the yaw·roll pair is the same transform for
/// the port's single-template composition).
#[must_use]
pub fn oar_rotations(sweep: f64, feather: f64) -> [[f64; 4]; 2] {
    let side = |sign: f64| -> [f64; 4] {
        let yaw = axis_angle([0.0, 1.0, 0.0], sweep * sign);
        let roll = axis_angle([0.0, 0.0, 1.0], -feather * sign);
        let mirror = if sign < 0.0 {
            axis_angle([0.0, 1.0, 0.0], PI)
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        quat_mul(quat_mul(yaw, roll), mirror)
    };
    [side(-1.0), side(1.0)]
}

/// The oar-rig instance rotations `[left, right]` for a per-side solved yaw
/// pair (the arm-authority solve's output) and a shared feather.
///
/// Same rigid-circle construction as [`oar_rotations`], but each side takes
/// its own solved yaw (the web solves per side from that side's shoulder).
/// `yaws` is `[left, right]`, already signed.
#[must_use]
pub fn oar_rotations_from_yaws(yaws: [f64; 2], feather: f64) -> [[f64; 4]; 2] {
    let side = |sign: f64, yaw_angle: f64| -> [f64; 4] {
        let yaw = axis_angle([0.0, 1.0, 0.0], yaw_angle);
        let roll = axis_angle([0.0, 0.0, 1.0], -feather * sign);
        let mirror = if sign < 0.0 {
            axis_angle([0.0, 1.0, 0.0], PI)
        } else {
            [0.0, 0.0, 0.0, 1.0]
        };
        quat_mul(quat_mul(yaw, roll), mirror)
    };
    [side(-1.0, yaws[0]), side(1.0, yaws[1])]
}

/// The blade's roll about the shaft in degrees (web `oar.blade.rotation.x =
/// (1 − bladeFeather) · π/2`): squared through the drive, flat in recovery.
#[must_use]
pub fn blade_roll_degrees(blade_feather: f64) -> f64 {
    let feather = if blade_feather.is_finite() {
        blade_feather.clamp(0.0, 1.0)
    } else {
        0.0
    };
    (1.0 - feather) * 90.0
}

/// A pole's rotation from its authored down-the-shaft axis `(0, −1, 0)` to
/// the solved shaft direction (Studio `pole.root.orientation`).
#[must_use]
pub fn pole_rotation(direction: [f64; 3]) -> [f64; 4] {
    rotation_between([0.0, -1.0, 0.0], direction)
}

/// Crank rotation about local X (web `cranks.orientation`).
#[must_use]
pub fn crank_rotation(angle: f64) -> [f64; 4] {
    axis_angle([1.0, 0.0, 0.0], angle)
}

/// Wheel rotation about local X (web `for (const w of wheels) w.rotation.x`).
#[must_use]
pub fn wheel_rotation(angle: f64) -> [f64; 4] {
    axis_angle([1.0, 0.0, 0.0], angle)
}

/// The rig root's yaw about Y as a quaternion.
#[must_use]
pub fn yaw_rotation(yaw: f64) -> [f64; 4] {
    axis_angle([0.0, 1.0, 0.0], yaw)
}

/// The rig group's roll about its local Z as a quaternion.
#[must_use]
pub fn roll_rotation(roll: f64) -> [f64; 4] {
    axis_angle([0.0, 0.0, 1.0], roll)
}

/// The rotation taking `source` onto `target` (shortest arc), identity when
/// either is degenerate.
#[must_use]
pub fn rotation_between(source: [f64; 3], target: [f64; 3]) -> [f64; 4] {
    let ls = length(source);
    let lt = length(target);
    if !ls.is_finite() || !lt.is_finite() || ls < 1e-9 || lt < 1e-9 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let from = scale(source, 1.0 / ls);
    let to = scale(target, 1.0 / lt);
    let scalar = dot(from, to).clamp(-1.0, 1.0);
    if scalar > 0.999_99 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    if scalar < -0.999_99 {
        let basis = if from[0].abs() < from[1].abs() && from[0].abs() < from[2].abs() {
            [1.0, 0.0, 0.0]
        } else if from[1].abs() < from[2].abs() {
            [0.0, 1.0, 0.0]
        } else {
            [0.0, 0.0, 1.0]
        };
        let axis = cross(from, basis);
        return axis_angle(scale(axis, 1.0 / length(axis)), PI);
    }
    let c = cross(from, to);
    normalise([c[0], c[1], c[2], 1.0 + scalar])
}

/// A leaf shell's fit into a target box (Studio `attachFittedVisual`): the
/// scale that maps the mesh extents onto `target_size` and the position that
/// puts the scaled mesh centre at `target_center`. `None` for degenerate
/// bounds.
#[must_use]
pub fn leaf_fit(
    bounds: Bounds,
    target_size: [f64; 3],
    target_center: [f64; 3],
) -> Option<([f64; 3], [f64; 3])> {
    let (min, max) = bounds;
    let extents = sub(max, min);
    if extents.iter().any(|e| !e.is_finite() || *e <= 1e-6) {
        return None;
    }
    let scale = [
        target_size[0] / extents[0],
        target_size[1] / extents[1],
        target_size[2] / extents[2],
    ];
    let center = scale_components(add(min, max), [0.5; 3]);
    let position = sub(target_center, scale_components(center, scale));
    Some((scale, position))
}

fn scale_components(v: [f64; 3], s: [f64; 3]) -> [f64; 3] {
    [v[0] * s[0], v[1] * s[1], v[2] * s[2]]
}

/// A pole leaf's fit: scale and position within the pole's local frame.
#[derive(Debug, Clone, Copy)]
pub struct PoleLeafFit {
    /// Scale to apply to the mesh.
    pub scale: [f64; 3],
    /// Position offset within the pole's local frame (before pole rotation).
    pub position: [f64; 3],
}

/// The three pole leaf fits (shaft, grip, basket) for the current pack.
/// Returns `None` entries for leaves whose bounds are unavailable.
#[must_use]
pub fn pole_leaf_fits(leaf_bounds: &dyn Fn(&str) -> Option<Bounds>) -> [Option<PoleLeafFit>; 3] {
    let compute = |slot: &str, size: [f64; 3], center: [f64; 3]| -> Option<PoleLeafFit> {
        let bounds = leaf_bounds(slot)?;
        let (scale, position) = leaf_fit(bounds, size, center)?;
        Some(PoleLeafFit { scale, position })
    };
    [
        compute(
            "equipment:ski:pole-shaft",
            [0.018, SKI_POLE_LENGTH, 0.018],
            [0.0, -SKI_POLE_LENGTH / 2.0, 0.0],
        ),
        compute(
            "equipment:ski:pole-grip",
            [
                POLE_GRIP_RADIUS * 2.0,
                POLE_GRIP_LENGTH,
                POLE_GRIP_RADIUS * 2.0,
            ],
            [0.0, -POLE_GRIP_LENGTH / 2.0, 0.0],
        ),
        compute(
            "equipment:ski:pole-basket",
            [0.056, 0.014, 0.056],
            [0.0, -SKI_POLE_LENGTH, 0.0],
        ),
    ]
}

/// The static equipment layout for QML (`Replay.equipmentLayout`): instance
/// positions for the repeated templates, the blade offset under each oar,
/// and the pole leaf fits (given the leaf bounds by slot).
#[must_use]
pub fn layout_json(leaf_bounds: &dyn Fn(&str) -> Option<Bounds>) -> Value {
    let fit = |slot: &str, size: [f64; 3], center: [f64; 3]| -> Value {
        match leaf_bounds(slot).and_then(|bounds| leaf_fit(bounds, size, center)) {
            Some((scale, position)) => {
                json!({ "slot": slot, "scale": scale, "position": position })
            }
            None => json!({ "slot": slot, "scale": [1.0, 1.0, 1.0], "position": [0.0, 0.0, 0.0] }),
        }
    };
    json!({
        "oarInstances": [
            [-OARLOCK_PIVOT[0], OARLOCK_PIVOT[1], OARLOCK_PIVOT[2]],
            [OARLOCK_PIVOT[0], OARLOCK_PIVOT[1], OARLOCK_PIVOT[2]],
        ],
        "bladeOffset": BLADE_OFFSET,
        "skiInstances": [
            [-SKI_ANCHOR[0], SKI_ANCHOR[1], SKI_ANCHOR[2]],
            [SKI_ANCHOR[0], SKI_ANCHOR[1], SKI_ANCHOR[2]],
        ],
        "wheelInstances": [
            [0.0, BIKE_AXLE_Y, bike_front_axle_z()],
            [0.0, BIKE_AXLE_Y, bike_rear_axle_z()],
        ],
        "poleLength": SKI_POLE_LENGTH,
        "poleLeaves": [
            fit("equipment:ski:pole-shaft", [0.018, SKI_POLE_LENGTH, 0.018], [0.0, -SKI_POLE_LENGTH / 2.0, 0.0]),
            fit("equipment:ski:pole-grip", [POLE_GRIP_RADIUS * 2.0, POLE_GRIP_LENGTH, POLE_GRIP_RADIUS * 2.0], [0.0, -POLE_GRIP_LENGTH / 2.0, 0.0]),
            fit("equipment:ski:pole-basket", [0.056, 0.014, 0.056], [0.0, -SKI_POLE_LENGTH, 0.0]),
        ],
        // Constant scales for the three pole leaves, for QML to set on the
        // leaf nodes (the per-frame positions come from the frame bundle).
        "poleLeafScales": pole_leaf_scale_json(leaf_bounds),
    })
}

fn pole_leaf_scale_json(leaf_bounds: &dyn Fn(&str) -> Option<Bounds>) -> Value {
    let fits = pole_leaf_fits(leaf_bounds);
    let s = |i: usize| -> Value {
        match fits[i] {
            Some(ref f) => json!(f.scale),
            None => json!([1.0, 1.0, 1.0]),
        }
    };
    json!([s(0), s(1), s(2)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
        let u = [q[0], q[1], q[2]];
        let t = scale(cross(u, v), 2.0);
        add(add(v, scale(t, q[3])), cross(u, t))
    }

    #[test]
    fn the_right_oar_sweeps_and_feathers_and_the_left_mirrors() {
        let [left, right] = oar_rotations(0.0, 0.0);
        assert_eq!(right, [0.0, 0.0, 0.0, 1.0]);
        // The left instance turns the authored (+x outboard) oar to −x.
        let tip = rotate(left, [1.72, 0.0, 0.0]);
        assert!(
            (tip[0] + 1.72).abs() < 1e-9 && tip[2].abs() < 1e-9,
            "{tip:?}"
        );
        let [_, swept] = oar_rotations(0.5, 0.0);
        let tip = rotate(swept, [1.0, 0.0, 0.0]);
        // A positive sweep about +Y carries the +x tip toward −z.
        assert!((tip[0] - 0.5_f64.cos()).abs() < 1e-9 && (tip[2] + 0.5_f64.sin()).abs() < 1e-9);
        let [left_swept, _] = oar_rotations(0.5, 0.0);
        let left_tip = rotate(left_swept, [1.0, 0.0, 0.0]);
        // The left oar mirrors: its outboard tip (now at −x) sweeps toward −z too.
        assert!(
            (left_tip[0] + 0.5_f64.cos()).abs() < 1e-9
                && (left_tip[2] + 0.5_f64.sin()).abs() < 1e-9,
            "{left_tip:?}"
        );
        let [_, feathered] = oar_rotations(0.0, 0.2);
        let grip = rotate(feathered, [-0.83, 0.0, 0.0]);
        assert!(
            grip[1] > 0.0,
            "a positive feather lifts the inboard grip: {grip:?}"
        );
    }

    /// The yaw and roll composed **together** follow the web's three.js
    /// Euler-XYZ law: an oar-local X vector lands at
    /// `(cos(roll)·cos(yaw), sin(roll), −cos(roll)·sin(yaw))` — the roll's
    /// vertical lift is yaw-independent. The tests above pin yaw and roll
    /// in isolation (which is how the wrong order survived); this one pins
    /// their product, the case the rig-phase `handTargets` fixture found
    /// 0.181 m of error in (rowplay#199).
    #[test]
    fn a_swept_and_feathered_oar_follows_the_web_euler_xyz_composition() {
        // Catch: mid-stroke yaw and drive-side roll together.
        let [_, right] = oar_rotations(1.2, 0.28);
        let tip = rotate(right, [1.0, 0.0, 0.0]);
        let (yaw, roll) = (1.2f64, -0.28f64);
        assert!(
            (tip[0] - roll.cos() * yaw.cos()).abs() < 1e-12
                && (tip[1] - roll.sin()).abs() < 1e-12
                && (tip[2] + roll.cos() * yaw.sin()).abs() < 1e-12,
            "right oar local +X must land at (cos·cos, sin, −cos·sin): {tip:?}"
        );
        // Under the old order (`qz ⊗ qy`) the lift is `cos(yaw)·sin(roll)`
        // — 0.24 m short at this yaw; the identity above distinguishes
        // the orders whenever both angles are non-zero.
        let wrong_lift = yaw.cos() * roll.sin();
        assert!(
            (tip[1] - wrong_lift).abs() > 1e-3,
            "the assertion must distinguish the composition orders"
        );
        // The left instance mirrors: template +X (outboard for the right
        // oar) carries the π mirror, so the rotated template +X equals the
        // web law evaluated on the left's side-authored local −X — every
        // component picks up the mirror's −1.
        let [left, _] = oar_rotations(1.2, 0.28);
        let left_tip = rotate(left, [1.0, 0.0, 0.0]);
        let (yaw, roll) = (-1.2f64, 0.28f64);
        assert!(
            (left_tip[0] + roll.cos() * yaw.cos()).abs() < 1e-12
                && (left_tip[1] + roll.sin()).abs() < 1e-12
                && (left_tip[2] - roll.cos() * yaw.sin()).abs() < 1e-12,
            "left oar template +X must follow the mirrored law: {left_tip:?}"
        );
    }

    #[test]
    fn blade_roll_squares_in_the_drive() {
        assert_eq!(blade_roll_degrees(0.0), 90.0);
        assert_eq!(blade_roll_degrees(1.0), 0.0);
        assert_eq!(blade_roll_degrees(0.5), 45.0);
        assert_eq!(blade_roll_degrees(f64::NAN), 90.0);
    }

    #[test]
    fn poles_cranks_and_wheels_turn_about_their_axes() {
        let pole = pole_rotation([0.0, -1.0, 0.0]);
        assert_eq!(pole, [0.0, 0.0, 0.0, 1.0]);
        let tilted = pole_rotation([0.0, -0.8, 0.6]);
        let down = rotate(tilted, [0.0, -1.0, 0.0]);
        assert!(
            (down[1] + 0.8).abs() < 1e-9 && (down[2] - 0.6).abs() < 1e-9,
            "{down:?}"
        );
        let crank = crank_rotation(std::f64::consts::FRAC_PI_2);
        let pedal = rotate(crank, [0.0, 0.1725, 0.0]);
        assert!(
            pedal[1].abs() < 1e-9 && (pedal[2] - 0.1725).abs() < 1e-9,
            "{pedal:?}"
        );
        assert_eq!(wheel_rotation(0.0), [0.0, 0.0, 0.0, 1.0]);
        let flipped = rotation_between([0.0, 1.0, 0.0], [0.0, -1.0, 0.0]);
        let v = rotate(flipped, [0.0, 1.0, 0.0]);
        assert!((v[1] + 1.0).abs() < 1e-9);
        assert_eq!(
            rotation_between([0.0; 3], [1.0, 0.0, 0.0]),
            [0.0, 0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn a_leaf_fit_maps_its_bounds_onto_the_target_box() {
        let bounds = ([-1.0, -2.0, -0.5], [1.0, 2.0, 0.5]);
        let (scale, position) =
            leaf_fit(bounds, [0.018, 1.37, 0.018], [0.0, -0.685, 0.0]).expect("fit");
        assert!((scale[0] - 0.009).abs() < 1e-12 && (scale[1] - 0.3425).abs() < 1e-12);
        assert_eq!(position, [0.0, -0.685, 0.0]);
        let off = ([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let (scale, position) = leaf_fit(off, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]).expect("fit");
        assert_eq!(scale, [0.5, 0.5, 0.5]);
        assert_eq!(position, [-0.5, -0.5, -0.5]);
        assert!(leaf_fit(([0.0; 3], [0.0; 3]), [1.0; 3], [0.0; 3]).is_none());
    }

    #[test]
    fn the_layout_places_the_repeated_templates_from_the_readme_and_bike_rig() {
        let layout = layout_json(&|slot| {
            (slot == "equipment:ski:pole-shaft").then_some(([-0.5, -0.5, -0.5], [0.5, 0.5, 0.5]))
        });
        let near = |value: &Value, expected: [f64; 3]| {
            let got: Vec<f64> = value
                .as_array()
                .expect("triple")
                .iter()
                .map(|v| v.as_f64().expect("number"))
                .collect();
            got.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6)
        };
        assert!(near(&layout["oarInstances"][1], [0.88, 0.51, 0.28]));
        assert!(near(&layout["skiInstances"][0], [-0.15, 0.0, 0.16]));
        let front = layout["wheelInstances"][0][2].as_f64().expect("front z");
        let rear = layout["wheelInstances"][1][2].as_f64().expect("rear z");
        assert!(
            (front - 0.5394).abs() < 1e-3 && (rear + 0.46).abs() < 1e-9,
            "{front} {rear}"
        );
        assert!(near(&layout["wheelInstances"][0], [0.0, 0.335, front]));
        assert_eq!(
            layout["poleLeaves"][0]["slot"],
            json!("equipment:ski:pole-shaft")
        );
        assert!((layout["poleLeaves"][0]["scale"][1].as_f64().expect("scale") - 1.37).abs() < 1e-9);
        // An unknown leaf keeps the identity fit rather than failing.
        assert_eq!(layout["poleLeaves"][2]["scale"], json!([1.0, 1.0, 1.0]));
        assert_eq!(layout["bladeOffset"], json!([1.82, -0.06, 0.0]));
    }
}
