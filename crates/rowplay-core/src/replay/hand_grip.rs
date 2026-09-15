// SPDX-License-Identifier: GPL-3.0-or-later
//! Geometry-closure hand grip (Phase 7): the port of the web's
//! `handGrip.ts`. Solves, once per hand at install time, the per-digit
//! flexion/opposition poses that close a hand's helper chain around a held
//! piece of equipment (a scull rubber with a thumb stop, a pole, a brake
//! hood), with signed contact reports.
//!
//! The solver is pure geometry over hand-local rest-space digit chains —
//! the chains are *input data* (collected from the V4 contract's helper rest
//! transforms by the view-model, or read from the parity fixture), so this
//! module stays Qt-free and I/O-free like the rest of `rowplay-core`.
//! Evaluation order mirrors the web module so the golden corpus
//! (`replay-current-main-grips.json`) compares within 1e-9.

use serde::Deserialize;

/// A point or direction in hand-local space.
pub type Vec3 = [f64; 3];
/// A rotation quaternion, `[x, y, z, w]`.
pub type Quat = [f64; 4];

/// Fitted hand curl axis (right hand; mirrors on Y/Z per side).
pub const HAND_CURL_AXIS: Vec3 = [-0.61, 0.16, 0.77];
/// Fitted SkiErg fist channel centre (right hand, hand-local).
pub const HAND_FIST_CENTRE: Vec3 = [0.0393, -0.0088, 0.0142];
/// Fitted fist channel radius: the circle the closed SkiErg fist's helpers
/// sit on around the 0.016 m rendered rubber.
pub const HAND_FIST_RADIUS: f64 = 0.0169;
/// The reference grip radius the fist was measured against.
pub const HAND_FIST_REFERENCE_GRIP_RADIUS: f64 = 0.016;
/// Authored palm-surface contact point (right hand) — the V4 contract's
/// `v4RightHand` contact offset, pinned here so the channel model and the
/// contact solver cannot drift onto two different palms.
pub const HAND_PALM_CONTACT: Vec3 = [0.08, -0.01, 0.035];
/// Outward palm normal measured from the shipped V4 bind geometry (right
/// hand; x mirrors per side). This is a separate measurement from
/// [`hand_palm_normal_in`] — the construction ray used to seat a cylinder
/// inside the hand — and describes the palm's true facing, which the wrist
/// tilt refinement preserves.
pub const HAND_PALM_NORMAL_OUT: Vec3 = [-0.1052, -0.8513, 0.514];
/// Hand long axis — wrist origin toward the middle-finger root.
pub const HAND_LONG_AXIS: Vec3 = [0.926, 0.105, 0.363];
/// Palm-cup carrying posture the grip channel was fitted under: rotation
/// about the `v4*Fingers` helper's local Y applied before curling.
pub const HAND_CLOSURE_CUP: f64 = 0.14;
/// Full-fist anatomical maxima (MCP ~90°, PIP ~110°, DIP ~80°).
pub const FINGER_STAGE_LIMITS: [f64; 3] = [1.57, 1.92, 1.4];
/// Thumb MCP/IP anatomical maxima (~60/80°).
pub const THUMB_STAGE_LIMITS: [f64; 3] = [1.0, 1.25, 1.35];
/// Digit-pad flesh calibrated from the rig's own approved envelope
/// (0.0169 − 0.016): a helper sits ~0.9 mm off the equipment it presses.
pub const DEFAULT_DIGIT_FLESH: f64 = HAND_FIST_RADIUS - HAND_FIST_REFERENCE_GRIP_RADIUS;
/// Axial allowance for a thumb pressing a flat handle end (the pad under the
/// estimated tip, not the tip itself).
pub const THUMB_END_PAD_ALLOWANCE: f64 = 0.0095;
/// Stage-flexion sweep resolution for emergence bracketing.
pub const CLOSURE_EMERGE_SAMPLES: u32 = 24;
/// Grid resolution of the opt-in final-pose enclosure search.
pub const WRAP_GRID_STEPS: u32 = 16;
/// Final helper points may compress into a held surface by at most 0.5 mm.
pub const WRAP_MAX_PENETRATION: f64 = 0.0005;
/// `contact` is a band around the surface, not an exact landing.
const CONTACT_BAND: f64 = 0.004;
/// Bisection iterations per stage refinement (28 ≈ limit/2^28 precision).
const BISECT_ITERATIONS: u32 = 28;

/// Inward palm normal of the right hand: palm contact toward the fitted
/// fist centre. Both endpoints are pinned measurements of the shipped rig.
pub fn hand_palm_normal_in() -> Vec3 {
    let direction = [
        HAND_FIST_CENTRE[0] - HAND_PALM_CONTACT[0],
        HAND_FIST_CENTRE[1] - HAND_PALM_CONTACT[1],
        HAND_FIST_CENTRE[2] - HAND_PALM_CONTACT[2],
    ];
    normalize(direction)
}

/// Distance from the palm skin to the surface of a held cylinder.
pub fn hand_grip_seat_flesh() -> f64 {
    distance(HAND_PALM_CONTACT, HAND_FIST_CENTRE) - HAND_FIST_RADIUS
}

/// Outward palm normal of the shipped bind geometry (web
/// `handPalmNormalOut`): the measured [`HAND_PALM_NORMAL_OUT`], mirrored on
/// x per side and normalised. Distinct from the negated construction ray:
/// the authored hand bone's metacarpal arch puts the geometric palm facing
/// well off the palm-contact→fist-centre axis.
#[must_use]
pub fn hand_palm_normal_out(side: f64) -> Vec3 {
    let mirror = side.signum();
    let mirror = if mirror == 0.0 { 1.0 } else { mirror };
    normalize([
        mirror * HAND_PALM_NORMAL_OUT[0],
        HAND_PALM_NORMAL_OUT[1],
        HAND_PALM_NORMAL_OUT[2],
    ])
}

/// Hand-local centre of the channel that a cylinder of `radius` occupies
/// when held: the palm contact pushed inward by seat flesh plus the radius.
/// x mirrors by side.
#[must_use]
pub fn hand_channel_centre(radius: f64, side: f64) -> Vec3 {
    let seat = hand_grip_seat_flesh() + radius;
    let mirror = side.signum();
    let mirror = if mirror == 0.0 { 1.0 } else { mirror };
    [
        (HAND_PALM_CONTACT[0] + hand_palm_normal_in()[0] * seat) * mirror,
        HAND_PALM_CONTACT[1] + hand_palm_normal_in()[1] * seat,
        HAND_PALM_CONTACT[2] + hand_palm_normal_in()[2] * seat,
    ]
}

/// Hand-local curl axis, mirrored the way the shipped rig mirrors hands.
#[must_use]
pub fn hand_curl_axis(side: f64) -> Vec3 {
    let mirror = side.signum();
    let mirror = if mirror == 0.0 { 1.0 } else { mirror };
    normalize([
        HAND_CURL_AXIS[0],
        HAND_CURL_AXIS[1] * mirror,
        HAND_CURL_AXIS[2] * mirror,
    ])
}

/// Thumb-ward curl axis: the curl axis signed from the pinky side toward the
/// thumb/index side, so a stopped handle's thumb end is deterministic.
#[must_use]
pub fn hand_curl_axis_thumbward(side: f64) -> Vec3 {
    let mirror = side.signum();
    let mirror = if mirror == 0.0 { 1.0 } else { mirror };
    scale(hand_curl_axis(side), mirror)
}

/// Hand long axis — wrist origin toward the middle-finger root — measured
/// from the sealed V4 helper rest transforms (right hand; x mirrors).
#[must_use]
pub fn hand_long_axis(side: f64) -> Vec3 {
    let mirror = side.signum();
    let mirror = if mirror == 0.0 { 1.0 } else { mirror };
    normalize([
        mirror * HAND_LONG_AXIS[0],
        HAND_LONG_AXIS[1],
        HAND_LONG_AXIS[2],
    ])
}

// --- minimal vector / quaternion algebra ---------------------------------

pub(crate) fn sub(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

pub(crate) fn add(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

pub(crate) fn dot(left: Vec3, right: Vec3) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

pub(crate) fn length(vector: Vec3) -> f64 {
    dot(vector, vector).sqrt()
}

pub(crate) fn distance(left: Vec3, right: Vec3) -> f64 {
    length(sub(left, right))
}

pub(crate) fn scale(vector: Vec3, factor: f64) -> Vec3 {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

pub(crate) fn add_scaled(base: Vec3, direction: Vec3, factor: f64) -> Vec3 {
    [
        base[0] + direction[0] * factor,
        base[1] + direction[1] * factor,
        base[2] + direction[2] * factor,
    ]
}

pub(crate) fn normalize(vector: Vec3) -> Vec3 {
    let len = length(vector);
    if len == 0.0 {
        return vector;
    }
    scale(vector, 1.0 / len)
}

/// Quaternion for a rotation of `angle` about the unit-ish axis `axis`
/// (three.js `setFromAxisAngle` normalises the axis).
pub(crate) fn axis_angle(axis: Vec3, angle: f64) -> Quat {
    let axis = normalize(axis);
    let half = angle / 2.0;
    let s = half.sin();
    [axis[0] * s, axis[1] * s, axis[2] * s, half.cos()]
}

pub(crate) fn quat_mul(left: Quat, right: Quat) -> Quat {
    [
        left[3] * right[0] + left[0] * right[3] + left[1] * right[2] - left[2] * right[1],
        left[3] * right[1] - left[0] * right[2] + left[1] * right[3] + left[2] * right[0],
        left[3] * right[2] + left[0] * right[1] - left[1] * right[0] + left[2] * right[3],
        left[3] * right[3] - left[0] * right[0] - left[1] * right[1] - left[2] * right[2],
    ]
}

pub(crate) fn quat_conjugate(quaternion: Quat) -> Quat {
    [
        -quaternion[0],
        -quaternion[1],
        -quaternion[2],
        quaternion[3],
    ]
}

pub(crate) fn quat_inverse(quaternion: Quat) -> Quat {
    let norm_sq = quaternion[0] * quaternion[0]
        + quaternion[1] * quaternion[1]
        + quaternion[2] * quaternion[2]
        + quaternion[3] * quaternion[3];
    scale4(quat_conjugate(quaternion), 1.0 / norm_sq)
}

pub(crate) fn scale4(quaternion: Quat, factor: f64) -> Quat {
    [
        quaternion[0] * factor,
        quaternion[1] * factor,
        quaternion[2] * factor,
        quaternion[3] * factor,
    ]
}

pub(crate) fn quat_rotate(quaternion: Quat, vector: Vec3) -> Vec3 {
    // v' = q * v * q^-1 (expanded; equivalent to three's applyQuaternion).
    let [qx, qy, qz, qw] = quaternion;
    let ix = qw * vector[0] + qy * vector[2] - qz * vector[1];
    let iy = qw * vector[1] + qz * vector[0] - qx * vector[2];
    let iz = qw * vector[2] + qx * vector[1] - qy * vector[0];
    let iw = -qx * vector[0] - qy * vector[1] - qz * vector[2];
    [
        ix * qw + iw * -qx + iy * -qz - iz * -qy,
        iy * qw + iw * -qy + iz * -qx - ix * -qz,
        iz * qw + iw * -qz + ix * -qy - iy * -qx,
    ]
}

// --- solver data model ----------------------------------------------------

/// One joint of a digit chain, in hand-local rest space.
#[derive(Debug, Clone, PartialEq)]
pub struct DigitJoint {
    /// The helper's name (`v4LeftIndexProximal`, …).
    pub helper: String,
    /// Rest position relative to the hand bone.
    pub position: Vec3,
    /// Rest orientation relative to the hand bone.
    pub quaternion: Quat,
}

/// One digit's joint chain, proximal→distal, in hand-local rest space.
#[derive(Debug, Clone, PartialEq)]
pub struct HandDigitChain {
    /// `index` | `middle` | `ring` | `pinky` | `thumb`.
    pub digit: &'static str,
    /// Joints proximal→distal.
    pub joints: Vec<DigitJoint>,
    /// Estimated distal-tip length beyond the last joint (m).
    pub tip_length: f64,
    /// The `v4*Fingers` palm-cup helper the chain hangs under, when present
    /// (thumbs parent the hand directly and carry no cup node).
    pub cup_node: Option<DigitJoint>,
}

/// The held equipment: a cylinder/capsule of `radius` (the rendered rubber,
/// shaft or hood body — *not* the hand's fist radius), with an optional
/// signed axial flat-end coordinate for the thumb to press.
#[derive(Debug, Clone, PartialEq)]
pub struct GripSurface {
    /// Cylinder/capsule radius of the held equipment (m) — the rendered
    /// rubber, shaft or hood body, *not* the hand's fist radius. The solver
    /// adds the pad flesh itself, so passing the fist channel radius would
    /// double-count it and seat the axis too deep in the palm.
    pub radius: f64,
    /// Signed axial coordinate along the thumb-ward channel axis of a flat
    /// handle end for the thumb to press (`None` for continuous shafts).
    pub thumb_end_axial: Option<f64>,
}

/// Closure options, mirroring the web `HandGripClosureOptions`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosureOptions {
    /// −1 left hand, +1 right hand.
    pub side: f64,
    /// The held equipment's channel geometry.
    pub surface: GripSurface,
    /// Base opposition (local Z at the thumb root) bringing the thumb across.
    pub thumb_oppose: f64,
    /// Finger-pad flesh; defaults to [`DEFAULT_DIGIT_FLESH`].
    pub finger_flesh: Option<f64>,
    /// Thumb-pad flesh; defaults to radial [`DEFAULT_DIGIT_FLESH`], or
    /// [`THUMB_END_PAD_ALLOWANCE`] when pressing a flat handle end.
    pub thumb_flesh: Option<f64>,
    /// Solve all three bounded finger stages together for a final enclosure.
    pub wrap_finger_stages: bool,
}

/// One solved helper pose.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitStagePose {
    /// The helper joint's name (`v4LeftIndexProximal`, …).
    pub helper: String,
    /// Flexion about the helper's local X (radians, sign as the web solver).
    pub flex: f64,
    /// Opposition about the helper's local Z (thumb root only).
    pub oppose: f64,
}

/// One digit's contact report.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DigitContact {
    /// `index` | `middle` | `ring` | `pinky` | `thumb`.
    pub digit: String,
    /// Signed distance of the closest bone point to the equipment surface
    /// (negative = penetration).
    pub surface_distance: f64,
    /// Closest complete phalanx segment, for final-enclosure contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_surface_distance: Option<f64>,
    /// True when the digit stopped because it reached the surface, not its
    /// flex limit (a 4 mm band around the surface).
    pub contact: bool,
    /// Solved tip position in hand-local space.
    pub tip: Vec3,
}

/// The solved closure: per-helper poses (document order) + per-digit reports.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GripClosure {
    /// One pose per helper joint, in fixture/document order.
    pub poses: Vec<DigitStagePose>,
    /// One report per digit, in chain order.
    pub contacts: Vec<DigitContact>,
}

// --- solver internals ------------------------------------------------------

fn local_axis(index: usize) -> Vec3 {
    match index {
        0 => [1.0, 0.0, 0.0],
        1 => [0.0, 1.0, 0.0],
        _ => [0.0, 0.0, 1.0],
    }
}

fn side_sign(side: f64) -> f64 {
    let s = side.signum();
    if s == 0.0 { 1.0 } else { s }
}

/// Apply one solved digit-stage pose to a helper's rest orientation: the
/// live bone rotation the renderer holds (web `applyGripHelpers`). The
/// `v4*Fingers` cup helper holds the carrying posture the channel was fitted
/// in (rest × Y-rotation); every other helper composes rest × oppose (local
/// Z) × flex (local X). `side` signs the cup like the web (`-side * cup`).
#[must_use]
pub fn pose_digit_stage(rest: Quat, flex: f64, oppose: f64, side: f64, is_cup: bool) -> Quat {
    if is_cup {
        let sign = side_sign(side);
        return quat_mul(rest, axis_angle(local_axis(1), -sign * HAND_CLOSURE_CUP));
    }
    let secondary = axis_angle(local_axis(2), oppose);
    let curl = axis_angle(local_axis(0), -flex);
    quat_mul(rest, quat_mul(secondary, curl))
}

/// Apply the carrying cup to a finger chain: conjugate every joint by the
/// cup rotation about the `v4*Fingers` node — exactly the composition the
/// renderer applies to the live helper. Chains without a cup node (thumbs,
/// minimal test rigs) pass through untouched.
fn cup_chain(chain: &HandDigitChain, side: f64) -> HandDigitChain {
    let Some(cup) = &chain.cup_node else {
        return chain.clone();
    };
    let roll = axis_angle(local_axis(1), -side_sign(side) * HAND_CLOSURE_CUP);
    let conjugate = quat_mul(quat_mul(cup.quaternion, roll), quat_inverse(cup.quaternion));
    let joints = chain
        .joints
        .iter()
        .map(|joint| DigitJoint {
            helper: joint.helper.clone(),
            position: add(
                quat_rotate(conjugate, sub(joint.position, cup.position)),
                cup.position,
            ),
            quaternion: quat_mul(conjugate, joint.quaternion),
        })
        .collect();
    HandDigitChain {
        digit: chain.digit,
        joints,
        tip_length: chain.tip_length,
        cup_node: chain.cup_node.clone(),
    }
}

/// Forward kinematics of one digit chain for candidate stage flexions.
/// `points[stage]` is the flexed joint origin, `points[len]` the tip; stages
/// beyond the chain keep their rest pose. Writes into `points` (len = joints
/// + 1) so the hot path allocates nothing.
fn digit_points(chain: &HandDigitChain, flexions: &[f64; 3], oppose: f64, points: &mut [Vec3]) {
    let mut parent_position = chain.joints[0].position;
    let mut parent_quaternion = chain.joints[0].quaternion;
    let oppose_rot = axis_angle(local_axis(2), oppose);
    let stage_rot = axis_angle(local_axis(0), -flexions[0]);
    parent_quaternion = quat_mul(quat_mul(parent_quaternion, oppose_rot), stage_rot);
    points[0] = parent_position;
    for stage in 1..chain.joints.len() {
        let joint = &chain.joints[stage];
        let previous = &chain.joints[stage - 1];
        // The child's rest offset/orientation relative to its parent joint.
        let local_position = quat_rotate(
            quat_inverse(previous.quaternion),
            sub(joint.position, previous.position),
        );
        let local_quaternion = quat_mul(quat_inverse(previous.quaternion), joint.quaternion);
        let world_position = add(
            quat_rotate(parent_quaternion, local_position),
            parent_position,
        );
        let mut world_quaternion = quat_mul(parent_quaternion, local_quaternion);
        let stage_rot = axis_angle(local_axis(0), -flexions[stage]);
        world_quaternion = quat_mul(world_quaternion, stage_rot);
        points[stage] = world_position;
        parent_position = world_position;
        parent_quaternion = world_quaternion;
    }
    let tip_offset = quat_rotate(parent_quaternion, [0.0, chain.tip_length, 0.0]);
    points[chain.joints.len()] = add(parent_position, tip_offset);
}

/// Radial distance from a point to the grip axis through `centre`.
fn distance_to_axis(point: Vec3, centre: Vec3, axis: Vec3) -> f64 {
    let delta = sub(point, centre);
    let along = dot(delta, axis);
    (dot(delta, delta) - along * along).max(0.0).sqrt()
}

/// Minimum radial distance from a complete phalanx segment to the grip axis.
fn segment_distance_to_axis(start: Vec3, end: Vec3, centre: Vec3, axis: Vec3) -> f64 {
    let s = add_scaled(sub(start, centre), axis, -dot(sub(start, centre), axis));
    let mut d = sub(end, centre);
    d = add_scaled(d, axis, -dot(d, axis));
    d = sub(d, s);
    let length_sq = dot(d, d);
    let along = if length_sq <= f64::EPSILON {
        0.0
    } else {
        (-dot(s, d) / length_sq).clamp(0.0, 1.0)
    };
    length(add_scaled(s, d, along))
}

/// Signed axial coordinate of a point along the grip axis from `centre`.
fn axial_coordinate(point: Vec3, centre: Vec3, axis: Vec3) -> f64 {
    dot(sub(point, centre), axis)
}

/// Clearance of the constrained points for one candidate stage flexion
/// state: min over the constrained points of the signed distance to the
/// surface (radial, or axial for a thumb pressing a flat handle end).
#[allow(clippy::too_many_arguments)]
fn stage_clearance(
    chain: &HandDigitChain,
    flexions: &[f64; 3],
    oppose: f64,
    thumb_end: bool,
    surface: &GripSurface,
    flesh: f64,
    centre: Vec3,
    axis: Vec3,
    first_collision_point: usize,
    points: &mut [Vec3; 4],
) -> f64 {
    digit_points(chain, flexions, oppose, points);
    let mut nearest = f64::INFINITY;
    for point in &points[first_collision_point..=chain.joints.len()] {
        let value = if thumb_end {
            axial_coordinate(*point, centre, axis) - surface.thumb_end_axial.unwrap() - flesh
        } else {
            distance_to_axis(*point, centre, axis) - surface.radius - flesh
        };
        nearest = nearest.min(value);
    }
    nearest
}

/// Close every digit of one hand around an equipment surface.
///
/// The capsule axis runs through `hand_channel_centre(radius)` along the
/// hand's curl axis, thumb-ward positive; finger chains are first posed into
/// the [`HAND_CLOSURE_CUP`] carrying posture the channel was fitted in. Each
/// stage flexes until a constrained bone point reaches the surface
/// (radius + pad flesh); penetration is minimised, not forbidden — read the
/// contact report's `surface_distance` rather than assuming non-negative.
/// An install-time solve: cache the returned poses, never call per frame.
#[must_use]
pub fn solve_hand_grip_closure(chains: &[HandDigitChain], options: &ClosureOptions) -> GripClosure {
    let finger_flesh = options.finger_flesh.unwrap_or(DEFAULT_DIGIT_FLESH);
    let posed: Vec<HandDigitChain> = chains
        .iter()
        .map(|chain| cup_chain(chain, options.side))
        .collect();
    let mut axis = hand_curl_axis(options.side);
    // Thumb-ward sign: the axis must point from the pinky side toward the
    // index/thumb side so `thumb_end_axial` has one meaning on both hands.
    let index = posed.iter().find(|chain| chain.digit == "index");
    let pinky = posed.iter().find(|chain| chain.digit == "pinky");
    if let (Some(index), Some(pinky)) = (index, pinky) {
        let delta = sub(index.joints[0].position, pinky.joints[0].position);
        if dot(delta, axis) < 0.0 {
            axis = scale(axis, -1.0);
        }
    }
    let centre = hand_channel_centre(options.surface.radius, options.side);

    let mut closure = GripClosure::default();
    let mut points = [[0.0; 3]; 4];

    for chain in &posed {
        let is_thumb = chain.digit == "thumb";
        let limits = if is_thumb {
            THUMB_STAGE_LIMITS
        } else {
            FINGER_STAGE_LIMITS
        };
        let oppose = if is_thumb {
            side_sign(options.side) * options.thumb_oppose
        } else {
            0.0
        };
        let mut flexions = [0.0_f64; 3];
        let thumb_end = is_thumb && options.surface.thumb_end_axial.is_some();
        let flesh = if is_thumb {
            options.thumb_flesh.unwrap_or(if thumb_end {
                THUMB_END_PAD_ALLOWANCE
            } else {
                DEFAULT_DIGIT_FLESH
            })
        } else {
            finger_flesh
        };

        // A thumb lies nearly along the shaft it opposes: in radial mode only
        // the pad tip is the opposing contact. Fingers wrap across the shaft
        // and constrain every downstream point.
        let first_collision_point = |stage: usize| {
            if is_thumb && !thumb_end {
                chain.joints.len()
            } else {
                stage + 1
            }
        };
        macro_rules! clearance {
            ($stage:expr) => {
                stage_clearance(
                    chain,
                    &flexions,
                    oppose,
                    thumb_end,
                    &options.surface,
                    flesh,
                    centre,
                    axis,
                    first_collision_point($stage),
                    &mut points,
                )
            };
        }

        let mut wrapped_solved = false;
        if options.wrap_finger_stages && !is_thumb {
            // A palm-supported hood needs a final-pose enclosure, not the
            // first point reached while closing: search the three bounded
            // joint ranges together so the chain can land on the far side
            // with every phalanx segment outside the equipment.
            let mut best_score = f64::NEG_INFINITY;
            let mut best_flexions: Option<[f64; 3]> = None;
            for proximal in 0..=WRAP_GRID_STEPS {
                flexions[0] = (limits[0] * f64::from(proximal)) / f64::from(WRAP_GRID_STEPS);
                for intermediate in 0..=WRAP_GRID_STEPS {
                    flexions[1] =
                        (limits[1] * f64::from(intermediate)) / f64::from(WRAP_GRID_STEPS);
                    for distal in 0..=WRAP_GRID_STEPS {
                        flexions[2] = (limits[2] * f64::from(distal)) / f64::from(WRAP_GRID_STEPS);
                        digit_points(chain, &flexions, oppose, &mut points);
                        let mut nearest = f64::INFINITY;
                        for point in &points[1..=chain.joints.len()] {
                            nearest = nearest.min(
                                distance_to_axis(*point, centre, axis)
                                    - options.surface.radius
                                    - flesh,
                            );
                        }
                        for segment in 1..chain.joints.len() {
                            nearest = nearest.min(
                                segment_distance_to_axis(
                                    points[segment],
                                    points[segment + 1],
                                    centre,
                                    axis,
                                ) - options.surface.radius
                                    - flesh,
                            );
                        }
                        let tip_distance =
                            distance_to_axis(points[chain.joints.len()], centre, axis)
                                - options.surface.radius
                                - flesh;
                        if nearest < -WRAP_MAX_PENETRATION
                            || nearest.abs() >= CONTACT_BAND
                            || tip_distance.abs() >= CONTACT_BAND
                        {
                            continue;
                        }
                        let wrap = f64::from(proximal + intermediate + distal)
                            / f64::from(WRAP_GRID_STEPS);
                        let score = wrap - 12.0 * (nearest.abs() + tip_distance.abs());
                        if score > best_score {
                            best_score = score;
                            best_flexions = Some(flexions);
                        }
                    }
                }
            }
            if let Some(best) = best_flexions {
                flexions = best;
                wrapped_solved = true;
            }
        }

        let first_point = first_collision_point(0);
        if wrapped_solved {
            record_digit(
                chain,
                &flexions,
                oppose,
                thumb_end,
                first_point,
                &options.surface,
                flesh,
                centre,
                axis,
                true,
                &mut points,
                &mut closure,
            );
            continue;
        }

        for stage in 0..chain.joints.len() {
            let limit = limits.get(stage).copied().unwrap_or(1.0);
            flexions[stage] = 0.0;
            if clearance!(stage) > 0.0 {
                // Outside the surface: close until the first constrained bone
                // point lands on it.
                flexions[stage] = limit;
                if clearance!(stage) > 0.0 {
                    // Fully flexed still clears: scan for a mid-range touch
                    // (a non-monotone sweep can dip onto the surface and
                    // recede again), then bisect the approach.
                    let mut best = limit;
                    let mut best_clearance = clearance!(stage);
                    let mut touched: Option<f64> = None;
                    let mut before = 0.0;
                    for sample in 0..CLOSURE_EMERGE_SAMPLES {
                        let candidate =
                            (limit * f64::from(sample)) / f64::from(CLOSURE_EMERGE_SAMPLES);
                        flexions[stage] = candidate;
                        let value = clearance!(stage);
                        if value <= 0.0 {
                            touched = Some(candidate);
                            break;
                        }
                        before = candidate;
                        if value < best_clearance {
                            best_clearance = value;
                            best = candidate;
                        }
                    }
                    match touched {
                        None => {
                            // No touch anywhere in range: a non-terminal
                            // finger stage still curls fully (its wrap is
                            // what brings the next segment around the
                            // shaft); the tip stage — and every stage of a
                            // radial thumb — holds the closest approach.
                            let hold_closest =
                                (is_thumb && !thumb_end) || stage == chain.joints.len() - 1;
                            flexions[stage] = if hold_closest { best } else { limit };
                            continue;
                        }
                        Some(touched) => {
                            let mut low = before;
                            let mut high = touched;
                            for _ in 0..BISECT_ITERATIONS {
                                let middle = (low + high) / 2.0;
                                flexions[stage] = middle;
                                if clearance!(stage) > 0.0 {
                                    low = middle;
                                } else {
                                    high = middle;
                                }
                            }
                            flexions[stage] = low;
                            continue;
                        }
                    }
                }
                // Fully flexed lands on the surface: bisect [0, limit] for the
                // first contact.
                let mut low = 0.0;
                let mut high = limit;
                for _ in 0..BISECT_ITERATIONS {
                    let middle = (low + high) / 2.0;
                    flexions[stage] = middle;
                    if clearance!(stage) > 0.0 {
                        low = middle;
                    } else {
                        high = middle;
                    }
                }
                flexions[stage] = low;
                continue;
            }

            // The stage starts *inside* the surface: close further until the
            // segment emerges onto the far side (clearance is not monotonic
            // across that sweep, so sample first, then bisect the bracket).
            let mut previous = 0.0;
            let mut emerged: Option<f64> = None;
            for sample in 1..=CLOSURE_EMERGE_SAMPLES {
                let candidate = (limit * f64::from(sample)) / f64::from(CLOSURE_EMERGE_SAMPLES);
                flexions[stage] = candidate;
                if clearance!(stage) > 0.0 {
                    emerged = Some(candidate);
                    break;
                }
                previous = candidate;
            }
            match emerged {
                None => {
                    // No pose in anatomical range clears: hold the flexion
                    // that penetrates least so the report is the honest best
                    // this anatomy can reach.
                    let mut best = 0.0;
                    let mut best_clearance = f64::NEG_INFINITY;
                    for sample in 0..=CLOSURE_EMERGE_SAMPLES {
                        let candidate =
                            (limit * f64::from(sample)) / f64::from(CLOSURE_EMERGE_SAMPLES);
                        flexions[stage] = candidate;
                        let value = clearance!(stage);
                        if value > best_clearance {
                            best_clearance = value;
                            best = candidate;
                        }
                    }
                    flexions[stage] = best;
                }
                Some(emerged) => {
                    let mut low = previous;
                    let mut high = emerged;
                    for _ in 0..BISECT_ITERATIONS {
                        let middle = (low + high) / 2.0;
                        flexions[stage] = middle;
                        if clearance!(stage) > 0.0 {
                            high = middle;
                        } else {
                            low = middle;
                        }
                    }
                    flexions[stage] = high;
                }
            }
        }
        record_digit(
            chain,
            &flexions,
            oppose,
            thumb_end,
            first_point,
            &options.surface,
            flesh,
            centre,
            axis,
            false,
            &mut points,
            &mut closure,
        );
    }
    closure
}

#[allow(clippy::too_many_arguments)]
fn record_digit(
    chain: &HandDigitChain,
    flexions: &[f64; 3],
    oppose: f64,
    thumb_end: bool,
    first_point: usize,
    surface: &GripSurface,
    flesh: f64,
    centre: Vec3,
    axis: Vec3,
    report_segments: bool,
    points: &mut [Vec3; 4],
    closure: &mut GripClosure,
) {
    digit_points(chain, flexions, oppose, points);
    let mut surface_distance = f64::INFINITY;
    // Same constrained-point rule as the solver: a radial thumb's only
    // opposing contact is its pad tip; fingers constrain every downstream
    // point.
    for point in &points[first_point..=chain.joints.len()] {
        let value = if thumb_end {
            axial_coordinate(*point, centre, axis) - surface.thumb_end_axial.unwrap() - flesh
        } else {
            distance_to_axis(*point, centre, axis) - surface.radius - flesh
        };
        surface_distance = surface_distance.min(value);
    }
    let mut segment_surface_distance = None;
    if report_segments {
        let mut segment_nearest = f64::INFINITY;
        for segment in 1..chain.joints.len() {
            segment_nearest = segment_nearest.min(
                segment_distance_to_axis(points[segment], points[segment + 1], centre, axis)
                    - surface.radius
                    - flesh,
            );
        }
        segment_surface_distance = Some(segment_nearest);
    }
    closure.contacts.push(DigitContact {
        digit: chain.digit.to_owned(),
        surface_distance,
        segment_surface_distance,
        contact: surface_distance.abs() < CONTACT_BAND,
        tip: points[chain.joints.len()],
    });
    for (stage, joint) in chain.joints.iter().enumerate() {
        closure.poses.push(DigitStagePose {
            helper: joint.helper.clone(),
            flex: flexions[stage],
            oppose: if stage == 0 { oppose } else { 0.0 },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal, synthetic finger chain: three joints in a straight line
    /// along +Y (the tip direction), curling about -X in the solver's sign
    /// convention. Two of these stand in for index/pinky so the thumb-ward
    /// axis signing has a line to read.
    fn chain(digit: &'static str, root: Vec3) -> HandDigitChain {
        let joints = (0..3)
            .map(|stage| DigitJoint {
                helper: format!("test:{digit}:{stage}"),
                position: [root[0], root[1] + 0.02 * f64::from(stage), root[2]],
                quaternion: [0.0, 0.0, 0.0, 1.0],
            })
            .collect();
        HandDigitChain {
            digit,
            joints,
            tip_length: 0.018,
            cup_node: None,
        }
    }

    fn chains_with_thumb() -> Vec<HandDigitChain> {
        vec![
            chain("index", [0.03, 0.0, 0.0]),
            chain("middle", [0.0, 0.0, 0.0]),
            chain("pinky", [-0.03, 0.0, 0.0]),
            chain("thumb", [0.03, -0.04, 0.0]),
        ]
    }

    fn options(radius: f64, thumb_end_axial: Option<f64>) -> ClosureOptions {
        ClosureOptions {
            side: 1.0,
            surface: GripSurface {
                radius,
                thumb_end_axial,
            },
            thumb_oppose: 0.3,
            finger_flesh: None,
            thumb_flesh: None,
            wrap_finger_stages: false,
        }
    }

    #[test]
    fn stage_application_composes_rest_oppose_flex() {
        let rest = [0.0, 0.0, 0.0, 1.0];
        // Identity rest: oppose then flex about local Z then X.
        let posed = pose_digit_stage(rest, 0.5, 0.25, 1.0, false);
        let expected = quat_mul(
            axis_angle(local_axis(2), 0.25),
            axis_angle(local_axis(0), -0.5),
        );
        for axis in 0..4 {
            assert!((posed[axis] - expected[axis]).abs() < 1e-15);
        }
        // The cup holds the carrying posture, ignoring flex/oppose.
        let cup = pose_digit_stage(rest, 0.5, 0.25, 1.0, true);
        let expected_cup = axis_angle(local_axis(1), -HAND_CLOSURE_CUP);
        for axis in 0..4 {
            assert!((cup[axis] - expected_cup[axis]).abs() < 1e-15);
        }
        // Cup mirrors by side.
        let cup_left = pose_digit_stage(rest, 0.5, 0.25, -1.0, true);
        assert!((cup_left[1] + expected_cup[1]).abs() < 1e-15);
        assert!((cup_left[3] - expected_cup[3]).abs() < 1e-15);
    }

    #[test]
    fn channel_geometry_matches_its_own_constants() {
        // channelCentre(HAND_FIST_RADIUS) reproduces the pinned fist centre.
        let centre = hand_channel_centre(HAND_FIST_RADIUS, 1.0);
        for axis in 0..3 {
            assert!((centre[axis] - HAND_FIST_CENTRE[axis]).abs() < 1e-12);
        }
        // Seat flesh is the palm-to-fist distance minus the channel radius.
        assert!(
            (hand_grip_seat_flesh()
                - (distance(HAND_PALM_CONTACT, HAND_FIST_CENTRE) - HAND_FIST_RADIUS))
                .abs()
                < 1e-15
        );
        // Palm normals: the inward construction ray is unit length, and the
        // outward facing is the shipped rig's own measured normal (not the
        // negated ray — the metacarpal arch puts true facing well off axis).
        let inward = hand_palm_normal_in();
        assert!((length(inward) - 1.0).abs() < 1e-12);
        for side in [1.0, -1.0] {
            let outward = hand_palm_normal_out(side);
            assert!((length(outward) - 1.0).abs() < 1e-12);
            let mirror = side.signum();
            let raw = [
                mirror * HAND_PALM_NORMAL_OUT[0],
                HAND_PALM_NORMAL_OUT[1],
                HAND_PALM_NORMAL_OUT[2],
            ];
            let raw_len = length(raw);
            for axis in 0..3 {
                assert!((outward[axis] - raw[axis] / raw_len).abs() < 1e-15);
            }
        }
    }

    #[test]
    fn curl_axis_mirrors_by_side() {
        let right = hand_curl_axis(1.0);
        let left = hand_curl_axis(-1.0);
        assert!((right[0] - left[0]).abs() < 1e-12);
        assert!((right[1] + left[1]).abs() < 1e-12);
    }

    #[test]
    fn every_solved_flexion_stays_within_anatomical_limits() {
        // Whatever the geometry, no stage may be driven past its anatomical
        // maximum (fingers 1.57/1.92/1.40 rad, thumb 1.0/1.25/1.35). Reach
        // outcomes for real hands are pinned by `grip_closure_parity` against
        // the fixture; this guards the bound itself.
        let closure = solve_hand_grip_closure(&chains_with_thumb(), &options(0.02, None));
        for pose in &closure.poses {
            let is_thumb = pose.helper.starts_with("test:thumb");
            let limits = if is_thumb {
                THUMB_STAGE_LIMITS
            } else {
                FINGER_STAGE_LIMITS
            };
            let stage: usize = pose.helper.rsplit(':').next().unwrap().parse().unwrap();
            assert!(
                pose.flex >= 0.0 && pose.flex <= limits[stage] + 1e-12,
                "{} flex {} outside 0..{}",
                pose.helper,
                pose.flex,
                limits[stage]
            );
        }
    }

    #[test]
    fn every_contact_report_is_finite() {
        let closure = solve_hand_grip_closure(&chains_with_thumb(), &options(0.02, None));
        assert_eq!(closure.contacts.len(), 4, "one report per chain");
        for contact in &closure.contacts {
            assert!(
                contact.surface_distance.is_finite(),
                "{} distance",
                contact.digit
            );
            for axis in 0..3 {
                assert!(
                    contact.tip[axis].is_finite(),
                    "{} tip axis {axis}",
                    contact.digit
                );
            }
        }
    }

    #[test]
    fn an_unreachably_thin_shaft_leaves_the_tip_short() {
        // A one-micron shaft cannot be gripped by these stubby chains: the
        // tip holds its closest approach and reports a clear (non-contact)
        // standoff rather than a fabricated landing.
        let closure = solve_hand_grip_closure(&chains_with_thumb(), &options(0.0001, None));
        let index = closure
            .contacts
            .iter()
            .find(|c| c.digit == "index")
            .expect("index report");
        assert!(
            index.surface_distance > CONTACT_BAND || !index.contact,
            "an unreachable surface must not report contact, got {}",
            index.surface_distance
        );
    }

    #[test]
    fn flesh_offset_seats_the_axis() {
        // Doubling the finger flesh pushes the reported standoff out by the
        // same amount: the solver adds the flesh to the surface radius, so a
        // larger pad rests further from the shaft.
        let thin = solve_hand_grip_closure(&chains_with_thumb(), &options(0.02, None));
        let mut fat_options = options(0.02, None);
        fat_options.finger_flesh = Some(DEFAULT_DIGIT_FLESH + 0.005);
        let fat = solve_hand_grip_closure(&chains_with_thumb(), &fat_options);
        let thin_index = thin.contacts.iter().find(|c| c.digit == "index").unwrap();
        let fat_index = fat.contacts.iter().find(|c| c.digit == "index").unwrap();
        assert!(
            fat_index.surface_distance > thin_index.surface_distance,
            "a thicker pad should stand further off ({} vs {})",
            fat_index.surface_distance,
            thin_index.surface_distance
        );
    }

    #[test]
    fn thumb_end_press_uses_the_pad_allowance() {
        // With a flat handle end the thumb presses axially; its report is an
        // axial distance and the radial pad flesh no longer applies.
        let closure = solve_hand_grip_closure(&chains_with_thumb(), &options(0.02, Some(0.01)));
        let thumb = closure
            .contacts
            .iter()
            .find(|c| c.digit == "thumb")
            .unwrap();
        assert!(thumb.surface_distance.is_finite());
        // The thumb root is the only opposed helper.
        let root = closure
            .poses
            .iter()
            .find(|pose| pose.helper == "test:thumb:0")
            .unwrap();
        assert!((root.oppose.abs() - 0.3).abs() < 1e-12);
    }

    #[test]
    fn wrap_mode_never_reports_beyond_the_penetration_bound() {
        // When the final-enclosure search finds a bounded pose it emits a
        // segment report and that report obeys WRAP_MAX_PENETRATION; when it
        // does not (this stub geometry cannot reach), it falls back to the
        // sequential solver with no segment report. Both are honest — what
        // must never happen is a segment report outside the bound.
        let mut wrap_options = options(0.02, None);
        wrap_options.wrap_finger_stages = true;
        let closure = solve_hand_grip_closure(&chains_with_thumb(), &wrap_options);
        for contact in closure.contacts.iter().filter(|c| c.digit != "thumb") {
            if let Some(segment) = contact.segment_surface_distance {
                assert!(
                    segment > -WRAP_MAX_PENETRATION - 1e-12,
                    "{} segment penetrates {segment} beyond the wrap bound",
                    contact.digit
                );
            }
        }
    }

    #[test]
    fn closure_is_deterministic() {
        let first = solve_hand_grip_closure(&chains_with_thumb(), &options(0.023, Some(0.04)));
        let second = solve_hand_grip_closure(&chains_with_thumb(), &options(0.023, Some(0.04)));
        assert_eq!(first, second);
    }
}
