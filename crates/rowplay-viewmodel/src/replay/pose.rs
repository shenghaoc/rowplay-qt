// SPDX-License-Identifier: GPL-3.0-or-later
//! Posing the V4 athlete for one frame (Phase 5b spec R2): the clip time from
//! the stroke pose (web `clipFraction`), the sampled clip over the rest pose,
//! then the contact pass — pelvis alignment and a two-bone solve on each arm
//! and leg toward the rig's contact targets (Studio
//! `ReplayAthleteContactSolver`, the web's `constrain`) — packed into the
//! frame bundle.
//!
//! Contact targets come from the sport's rig pose
//! ([`rowplay_core::replay::rig_pose`]) placed on the rig geometry the web
//! and Studio share (`rowRig.ts`, `skiEquipment.ts`, `bikeRig.js`); the ski
//! hands ride the rigid planted-pole solve. Everything is in rig-root space
//! (ground at `y = 0`, the athlete facing `+z`), which is the athlete's own
//! glTF space, so the contract's bend hints apply unrotated.

use rowplay_core::models::Sport;
use rowplay_core::replay::hand_grip::{hand_curl_axis, hand_long_axis, hand_palm_normal_out};
use rowplay_core::replay::rig_pose::{BikeErgRigPose, RowerRigPose, SkiErgRigPose, SportRigPose};
use rowplay_core::replay::row_equipment::{
    FOREARM_LENGTH, UPPER_ARM_LENGTH, rower_requested_wrist_reach, solve_rower_oar_yaw,
};
use rowplay_core::replay::two_bone::{
    add, cross, dot, length, scale, solve_rigid_contact3d, solve3d, sub,
};
use rowplay_core::replay::wrist::{
    WristMetrics, WristRest, constrain_wrist_frame, orient_hand_to_grip_channel,
    refine_grip_spin_for_wrist, refine_grip_tilt_for_wrist,
};

use super::athlete::{AssetError, Clip, LocalTransform, V4Athlete};
use super::frame;
use super::grip::{
    GripFrame, ROWER_PALM_TILT, ROWER_PALM_TILT_COMFORT, SKI_FLAT_MAX_SPIN, SKI_PALM_TILT,
    SKI_PALM_TILT_COMFORT, smoothstep,
};

/// The web `clipFraction`: map the stroke cycle fraction onto the authored
/// clip so the source drive end lands on the clip's drive end.
#[must_use]
pub fn clip_fraction(
    cycle_frac: f64,
    phase: f64,
    source_drive_end: f64,
    authored_drive_end: f64,
) -> f64 {
    let phase_cycle =
        wrap_unit(if phase.is_finite() { phase } else { 0.0 } / std::f64::consts::TAU);
    let cycle = if cycle_frac.is_finite() {
        wrap_unit(cycle_frac)
    } else {
        phase_cycle
    };
    let source = (if source_drive_end.is_finite() {
        source_drive_end
    } else {
        0.4
    })
    .clamp(0.05, 0.95);
    let clip = (if authored_drive_end.is_finite() {
        authored_drive_end
    } else {
        0.5
    })
    .clamp(0.05, 0.95);
    if cycle < source {
        cycle / source * clip
    } else {
        clip + (cycle - source) / (1.0 - source) * (1.0 - clip)
    }
}

fn wrap_unit(value: f64) -> f64 {
    let wrapped = value - value.floor();
    if wrapped < 0.0 {
        wrapped + 1.0
    } else {
        wrapped
    }
}

/// Rig geometry the targets sit on (web `rowRig.ts`, `skiEquipment.ts`,
/// `bikeRig.js`; Studio's grip contracts carry the same numbers).
pub mod geometry {
    /// RowErg: oarlock pivot in row avatar-root coordinates.
    pub const ROW_OARLOCK: [f64; 3] = [0.88, 0.51, 0.28];
    /// RowErg: foot contact per side (lateral, y, z).
    pub const ROW_FOOT: [f64; 3] = [0.12, 0.215, 0.75];
    /// RowErg: inboard distance from the pivot to the hand on the scull grip
    /// (Studio `scullInboardContact` = 0.66 + grip length / 2 − anchor).
    pub const ROW_INBOARD_CONTACT: f64 = 0.66 + 0.32 / 2.0 - 0.04;
    /// RowErg: the grip anchor's drop below the pivot line.
    pub const ROW_GRIP_DROP: f64 = -0.04;
    /// RowErg: pelvis target height and its z at neutral seat travel.
    pub const ROW_PELVIS: [f64; 2] = [0.30, -0.1];
    /// SkiErg: foot anchor per side (lateral, y, z).
    pub const SKI_FOOT: [f64; 3] = [0.15, 0.055, 0.18];
    /// SkiErg: shoulder half width and the hand's extra lateral margin.
    pub const SKI_SHOULDER_HALF_WIDTH: f64 = 0.25;
    /// See [`SKI_SHOULDER_HALF_WIDTH`].
    pub const SKI_HAND_LATERAL_MARGIN: f64 = 0.05;
    /// SkiErg: pole length.
    pub const SKI_POLE_LENGTH: f64 = 1.37;
    /// SkiErg: planted basket lateral offset and its height on the course.
    pub const SKI_PLANT_LATERAL: f64 = 0.46;
    /// See [`SKI_PLANT_LATERAL`].
    pub const SKI_CONTACT_Y: f64 = 0.055;
    /// SkiErg: the arm's reach the pole solve stays inside.
    pub const SKI_MAX_REACH: f64 = 0.49 + 0.47 - 0.02;
    /// SkiErg: pelvis carry at full height, the compression drop, and z.
    pub const SKI_PELVIS_Y: f64 = 0.72;
    /// See [`SKI_PELVIS_Y`].
    pub const SKI_PELVIS_COMPRESSION: f64 = 0.15;
    /// See [`SKI_PELVIS_Y`].
    pub const SKI_PELVIS_Z: f64 = 0.02;
    /// BikeErg: axle height (tyre outer radius) and bottom bracket.
    pub const BIKE_AXLE_Y: f64 = 0.31 + 0.025;
    /// See [`BIKE_AXLE_Y`].
    pub const BIKE_BB_Y: f64 = BIKE_AXLE_Y - 0.07;
    /// See [`BIKE_AXLE_Y`].
    pub const BIKE_BB_Z: f64 = -0.05;
    /// BikeErg: crank arm length and pedal lateral offset.
    pub const BIKE_CRANK_RADIUS: f64 = 0.1725;
    /// See [`BIKE_CRANK_RADIUS`].
    pub const BIKE_CRANK_LATERAL: f64 = 0.09;
    /// BikeErg: head-tube top from stack and reach.
    pub const BIKE_HEAD_TOP_Y: f64 = BIKE_BB_Y + 0.575;
    /// See [`BIKE_HEAD_TOP_Y`].
    pub const BIKE_HEAD_TOP_Z: f64 = BIKE_BB_Z + 0.385;
    /// BikeErg: brake-hood grip (y, z ahead of the head top, half span).
    pub const BIKE_GRIP_Y: f64 = BIKE_HEAD_TOP_Y;
    /// See [`BIKE_GRIP_Y`].
    pub const BIKE_GRIP_Z: f64 = BIKE_HEAD_TOP_Z + 0.175;
    /// See [`BIKE_GRIP_Y`].
    pub const BIKE_GRIP_HALF_SPAN: f64 = 0.2;
    /// BikeErg: rider pelvis z (setback behind the bottom bracket).
    pub const BIKE_PELVIS_Z: f64 = BIKE_BB_Z - 0.12;
    /// BikeErg: athlete leg segments and the knee flexion at bottom dead
    /// centre that fix the hip height over the pedals.
    pub const BIKE_THIGH: f64 = 0.4915;
    /// See [`BIKE_THIGH`].
    pub const BIKE_SHIN: f64 = 0.4794;
    /// See [`BIKE_THIGH`].
    pub const BIKE_KNEE_FLEXION_AT_BDC: f64 = 30.0 * std::f64::consts::PI / 180.0;
    /// See [`BIKE_THIGH`].
    pub const BIKE_HIP_ROOT_TO_FEMORAL_HEAD: f64 = 0.025;
    /// BikeErg seat contract: sit surface below the hip, the pad nestle.
    pub const BIKE_SIT_SURFACE_FROM_HIP_Y: f64 = -0.158;
    /// See [`BIKE_SIT_SURFACE_FROM_HIP_Y`].
    pub const BIKE_SIT_NESTLE: f64 = 0.005;

    /// The rider's hip height over the pedals (web `RIDER_HIP_Y`).
    #[must_use]
    pub fn bike_rider_hip_y() -> f64 {
        let leg_reach = (BIKE_THIGH * BIKE_THIGH + BIKE_SHIN * BIKE_SHIN
            - 2.0
                * BIKE_THIGH
                * BIKE_SHIN
                * (std::f64::consts::PI - BIKE_KNEE_FLEXION_AT_BDC).cos())
        .sqrt();
        let femoral_head_rise = (leg_reach * leg_reach - 0.12 * 0.12).sqrt();
        (BIKE_BB_Y - BIKE_CRANK_RADIUS) + femoral_head_rise + BIKE_HIP_ROOT_TO_FEMORAL_HEAD
    }

    /// The saddle pad's top (web `SADDLE_PAD_TOP_Y`).
    #[must_use]
    pub fn bike_saddle_pad_top_y() -> f64 {
        bike_rider_hip_y() + BIKE_SIT_NESTLE + BIKE_SIT_SURFACE_FROM_HIP_Y
    }
}

/// Studio's hand reach shortfall the residual gate forgives (metres).
pub const GRIP_REACH_SHORTFALL_TOLERANCE: f64 = 0.08;
/// Studio's contact budgets: hands, soles / pelvis (metres).
pub const GRIP_CONTACT_BUDGET: f64 = 0.01;
/// See [`GRIP_CONTACT_BUDGET`].
pub const REACH_CONTACT_BUDGET: f64 = 0.12;

/// Where the rig wants the pelvis, hands and feet this frame (rig-root space).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactTargets {
    /// Pelvis (hip bone) position.
    pub pelvis: [f64; 3],
    /// Left palm contact.
    pub left_hand: [f64; 3],
    /// Right palm contact.
    pub right_hand: [f64; 3],
    /// Left sole contact.
    pub left_foot: [f64; 3],
    /// Right sole contact.
    pub right_foot: [f64; 3],
}

/// A ski pole's placement this frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PolePlacement {
    /// The pole root (the solved hand / grip).
    pub root: [f64; 3],
    /// Unit direction from the grip down the shaft to the basket.
    pub direction: [f64; 3],
    /// The basket point: the snow-contact tip raised 0.026 m up-shaft
    /// (the rendered basket node, web `pole.basket.position = tip +
    /// groundUp · 0.026` — the fixture's `poleTipLeft/Right` oracle).
    pub basket: [f64; 3],
    /// The snow-contact tip the rigid shaft spans to (`|tip − root| =
    /// SKI_POLE_LENGTH` exactly; web `placeFigureSegmentBetween(pole.shaft,
    /// handPoint, tipLocalPoint)`).
    pub tip: [f64; 3],
}

/// RowErg arm-authority inputs for the composed oar solve.
///
/// The web's `placeArms` schedules a requested shoulder→wrist reach from the
/// `armDraw` channel and then calls `rowRig.solveRowerOarYaw` to place the
/// rigid oar's inboard grip on that reach sphere; the authored sweep is only
/// the solve's `preferredYaw` branch fallback. Composing that solve needs the
/// draw fraction and the roll here, where the solved shoulder positions are
/// known.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerOarInputs {
    /// The `armDraw` channel, 0..1 — the arm-authority velocity profile.
    pub draw: f64,
    /// The blade roll (the oar's Z rotation, unsigned).
    pub roll: f64,
    /// The authored sweep (unsigned), the solve's branch fallback.
    pub preferred_yaw: f64,
}

/// The rig's targets plus the ski poles when the sport has them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigTargets {
    /// The contact targets.
    pub contacts: ContactTargets,
    /// Left and right poles (SkiErg only).
    pub poles: Option<[PolePlacement; 2]>,
    /// RowErg arm-authority inputs (RowErg only).
    pub oar: Option<RowerOarInputs>,
}

/// Derive the contact targets from the sport's rig pose (Studio's
/// `ReplayRowerRig` / `ReplaySkiErgRig` / `ReplayBikeErgRig.applyPose`).
#[must_use]
pub fn rig_targets(pose: &SportRigPose) -> RigTargets {
    match pose {
        SportRigPose::Rower(rig) => rower_targets(rig),
        SportRigPose::SkiErg(rig) => skierg_targets(rig),
        SportRigPose::Bike(rig) => bike_targets(rig),
    }
}

/// Project a skierg arm-bend hint perpendicular to the shoulder→hand chord
/// (web `setArmBendHint` tail): the solver's bend plane must not collapse
/// onto the limb's own line. Falls back to pure side-out when the
/// projection degenerates.
#[must_use]
fn project_bend_hint(hint: [f64; 3], shoulder: [f64; 3], hand: [f64; 3]) -> [f64; 3] {
    let chord = sub(hand, shoulder);
    let chord_len = length(chord);
    if chord_len > 1e-6 {
        let chord = scale(chord, 1.0 / chord_len);
        let along = dot(hint, chord);
        let projected = sub(hint, scale(chord, along));
        if dot(projected, projected) >= 1e-6 {
            return projected;
        }
    }
    [hint[0].signum() * 1.0, 0.25, 0.0]
}

fn rower_targets(rig: &RowerRigPose) -> RigTargets {
    use geometry::{ROW_FOOT, ROW_GRIP_DROP, ROW_INBOARD_CONTACT, ROW_OARLOCK, ROW_PELVIS};
    let hand = |side: f64| -> [f64; 3] {
        // Each inboard grip rides its own rigid circle, in the web's
        // composition order (three.js Euler-XYZ: `qy(yaw) ⊗ qz(roll)`, the
        // roll's yaw-independent vertical lift — see `oar_rotations`).
        let yaw = axis_angle([0.0, 1.0, 0.0], rig.oar_sweep * side);
        let roll = axis_angle([0.0, 0.0, 1.0], -rig.oar_feather * side);
        let orientation = quat_mul(yaw, roll);
        let local = [-side * ROW_INBOARD_CONTACT, ROW_GRIP_DROP, 0.0];
        let pivot = [side * ROW_OARLOCK[0], ROW_OARLOCK[1], ROW_OARLOCK[2]];
        add(pivot, rotate(orientation, local))
    };
    RigTargets {
        contacts: ContactTargets {
            pelvis: [0.0, ROW_PELVIS[0], ROW_PELVIS[1] + rig.seat_z],
            left_hand: hand(-1.0),
            right_hand: hand(1.0),
            left_foot: [-ROW_FOOT[0], ROW_FOOT[1], ROW_FOOT[2]],
            right_foot: [ROW_FOOT[0], ROW_FOOT[1], ROW_FOOT[2]],
        },
        oar: Some(RowerOarInputs {
            draw: rig.arm_draw,
            roll: rig.oar_feather,
            preferred_yaw: rig.oar_sweep,
        }),
        poles: None,
    }
}

fn skierg_targets(rig: &SkiErgRigPose) -> RigTargets {
    use geometry::{
        SKI_CONTACT_Y, SKI_FOOT, SKI_HAND_LATERAL_MARGIN, SKI_MAX_REACH, SKI_PELVIS_COMPRESSION,
        SKI_PELVIS_Y, SKI_PELVIS_Z, SKI_POLE_LENGTH, SKI_SHOULDER_HALF_WIDTH,
    };
    let place = |side: f64| -> PolePlacement {
        // Phase 7.5 rewrite (web `placePoleArms` + `setPlantTipWorld`):
        // the plant anchor is fixed at the catch in course space, so in
        // rig-local it only rotates with the course turn (~5e-3 m drift
        // over the contact window, pinned by the fixture's `poleTipLeft`);
        // the carried tip is authored from the technique-phase attitude,
        // then rotated in DIRECTION space toward the hand→plant ray so it
        // converges exactly at contact — at contact the hand→plant
        // distance IS the pole length, so tip position and velocity are
        // continuous at both boundaries. The previous position lerp of a
        // retreating plant slid the basket ~2 m rearward through every
        // pull (0.977 m at the hand, visible on screen) and could pass
        // underneath the hand — the web's own rejected law (its comment
        // records the measured 955 mm wrist snap).
        let desired_hand = [
            side * (SKI_SHOULDER_HALF_WIDTH + SKI_HAND_LATERAL_MARGIN),
            rig.preferred_hand_y,
            rig.preferred_hand_z,
        ];

        // Plant anchor in rig-local under the course-turn rotation.
        let plant_basket = [
            rig.plant_course_turn_x(side),
            SKI_CONTACT_Y,
            rig.plant_course_turn_z(side),
        ];

        // Carried-tip attitude (web `poleAngle = degToRad(80 − sweep·57)`
        // from horizontal, snow-clearance clamped): the vertical drop and
        // its Pythagorean horizontal remainder keep the shaft exactly
        // `SKI_POLE_LENGTH`; trail behind the hand, splayed outboard so
        // the two shafts read separately.
        let attitude = rig.pole_attitude;
        let clearance = 0.015 + rig.pole_lift * 0.2;
        let desired_vertical = -attitude.sin() * SKI_POLE_LENGTH;
        let vertical = desired_vertical
            .max(SKI_CONTACT_Y + clearance - desired_hand[1])
            .min(SKI_POLE_LENGTH * 0.985);
        let horizontal = (SKI_POLE_LENGTH * SKI_POLE_LENGTH - vertical * vertical)
            .max(0.0)
            .sqrt();
        let lateral = side * 0.22;
        let forward = -(1.0 - lateral * lateral).max(0.0).sqrt();
        let carried_dir = [lateral, 0.0, forward];
        let free_basket = [
            desired_hand[0] + carried_dir[0] * horizontal,
            desired_hand[1] + vertical,
            desired_hand[2] + carried_dir[2] * horizontal,
        ];

        // Direction-space blend toward the plant: rotate the carried
        // direction onto the hand→plant ray by the angle between them,
        // scaled by (1 − flight) and the early-release fade. Antiparallel
        // shafts rotate through the stable lateral axis.
        let mut to_plant = sub(plant_basket, desired_hand);
        let to_plant_len = length(to_plant);
        let mut fore = normalised_or(sub(free_basket, desired_hand), [0.0, -1.0, 0.0]);
        if to_plant_len > 1e-4 && rig.pole_flight < 1.0 {
            to_plant = scale(to_plant, 1.0 / to_plant_len);
            let cosine = dot(fore, to_plant).clamp(-1.0, 1.0);
            let blend_angle = cosine.acos() * (1.0 - rig.pole_flight) * rig.release_fade;
            let mut axis = cross(fore, to_plant);
            if dot(axis, axis) < 1e-10 {
                axis = [side, 0.0, 0.0];
            }
            axis = normalised_or(axis, [0.0, -1.0, 0.0]);
            fore = crate::replay::equipment::rotate_vec(axis_angle(axis, blend_angle), fore);
        }
        let rotated_free = add(desired_hand, scale(fore, SKI_POLE_LENGTH));
        // Final contact lerp (web `tipWorld.lerpVectors`): the direction
        // rotation above has already converged the carried tip onto the
        // hand→plant ray, so this lerp is ~zero at the boundary and lands
        // the tip exactly on the plant at full contact — continuous in
        // position and velocity at both ends (the property the web's
        // comment records; without the rotation step the same lerp can
        // pass underneath the hand and flip the shaft).
        let blended = add(
            rotated_free,
            scale(
                sub(plant_basket, rotated_free),
                rig.pole_contact.clamp(0.0, 1.0),
            ),
        );
        // The rendered basket sits 0.026 m up-shaft from the tip (web
        // `pole.basket.position = tipLocalPoint + groundUp · 0.026`); the
        // fixture's `poleTipLeft` records that basket node.
        let basket_node = [blended[0], blended[1] + 0.026, blended[2]];

        let shoulder = [
            side * SKI_SHOULDER_HALF_WIDTH,
            rig.shoulder_y,
            rig.shoulder_z,
        ];
        let (root, _feasible) = solve_rigid_contact3d(
            shoulder,
            desired_hand,
            blended,
            SKI_POLE_LENGTH,
            0.0,
            SKI_MAX_REACH,
        );
        PolePlacement {
            root,
            direction: normalised_or(sub(blended, root), [0.0, -1.0, 0.0]),
            basket: basket_node,
            tip: blended,
        }
    };
    let left = place(-1.0);
    let right = place(1.0);
    RigTargets {
        contacts: ContactTargets {
            pelvis: [
                0.0,
                SKI_PELVIS_Y - rig.hip_compression.clamp(0.0, 1.0) * SKI_PELVIS_COMPRESSION,
                SKI_PELVIS_Z,
            ],
            left_hand: left.root,
            right_hand: right.root,
            left_foot: [-SKI_FOOT[0], SKI_FOOT[1], SKI_FOOT[2]],
            right_foot: [SKI_FOOT[0], SKI_FOOT[1], SKI_FOOT[2]],
        },
        poles: Some([left, right]),
        oar: None,
    }
}

fn bike_targets(rig: &BikeErgRigPose) -> RigTargets {
    use geometry::{
        BIKE_BB_Y, BIKE_BB_Z, BIKE_CRANK_LATERAL, BIKE_GRIP_HALF_SPAN, BIKE_GRIP_Y, BIKE_GRIP_Z,
        BIKE_PELVIS_Z, bike_rider_hip_y,
    };
    RigTargets {
        contacts: ContactTargets {
            pelvis: [0.0, bike_rider_hip_y(), BIKE_PELVIS_Z],
            left_hand: [-BIKE_GRIP_HALF_SPAN, BIKE_GRIP_Y, BIKE_GRIP_Z],
            right_hand: [BIKE_GRIP_HALF_SPAN, BIKE_GRIP_Y, BIKE_GRIP_Z],
            left_foot: [
                -BIKE_CRANK_LATERAL,
                BIKE_BB_Y + rig.pedal_pos_l.y,
                BIKE_BB_Z + rig.pedal_pos_l.z,
            ],
            right_foot: [
                BIKE_CRANK_LATERAL,
                BIKE_BB_Y + rig.pedal_pos_r.y,
                BIKE_BB_Z + rig.pedal_pos_r.z,
            ],
        },
        poles: None,
        oar: None,
    }
}

/// Residual distances after the contact pass, metres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Residuals {
    /// Left palm to its target.
    pub left_hand: f64,
    /// Right palm to its target.
    pub right_hand: f64,
    /// Left sole to its target.
    pub left_foot: f64,
    /// Right sole to its target.
    pub right_foot: f64,
    /// Hip bone to the pelvis target.
    pub pelvis: f64,
}

impl Residuals {
    /// Studio's usability gate: hands inside the grip budget plus the
    /// forgiven reach shortfall, soles and pelvis inside the reach budget.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        let values = [
            self.left_hand,
            self.right_hand,
            self.left_foot,
            self.right_foot,
            self.pelvis,
        ];
        values.iter().all(|v| v.is_finite())
            && self.left_hand.max(self.right_hand)
                <= GRIP_CONTACT_BUDGET + GRIP_REACH_SHORTFALL_TOLERANCE
            && self.left_foot.max(self.right_foot) <= REACH_CONTACT_BUDGET
            && self.pelvis <= REACH_CONTACT_BUDGET
    }
}

/// The posed skeleton: every joint's local transform plus the residuals.
#[derive(Debug, Clone, PartialEq)]
pub struct Posed {
    /// Local transforms, skin order.
    pub locals: Vec<LocalTransform>,
    /// Contact residuals.
    pub residuals: Residuals,
    /// Per-hand wrist metrics (left, right) from the budget pass.
    pub wrist: [WristMetrics; 2],
    /// RowErg only: the composed oar yaws (left, right) solved from the arm's
    /// requested reach (web `placeArms` → `rowRig.solveRowerOarYaw`). The
    /// renderer packs these as the oar rotations; `oar_sweep` alone is only
    /// the solve's branch fallback.
    pub oar_yaw: Option<[f64; 2]>,
}

#[derive(Debug, Clone, Copy)]
struct Binding {
    upper: usize,
    lower: usize,
    terminal: usize,
    offset: [f64; 3],
    bend_hint: [f64; 3],
}

/// Build one hand's wrist rest frame from the athlete's rest hierarchy
/// (web `wristRest` construction): the twist axis is the hand joint's own
/// offset in its parent (the forearm), the rest orientation is aligned so
/// the hand's long axis continues that bone axis, and flexion/deviation
/// complete the triad off the curl axis.
fn build_wrist_rest(athlete: &V4Athlete, hand: usize, side: f64) -> WristRest {
    let rest = &athlete.joints[hand];
    let bone_axis_local = normalised_or(rest.translation, [0.0, 1.0, 0.0]);
    let mut hand_rest = rest.rotation;
    let long_in_forearm = rotate(hand_rest, hand_long_axis(side));
    let align = rotation_between(long_in_forearm, bone_axis_local);
    hand_rest = normalise(quat_mul(align, hand_rest));
    let mut flex = rotate(hand_rest, hand_curl_axis(side));
    let along = dot(flex, bone_axis_local);
    flex = [
        flex[0] - bone_axis_local[0] * along,
        flex[1] - bone_axis_local[1] * along,
        flex[2] - bone_axis_local[2] * along,
    ];
    let flex_axis_local = normalised_or(flex, [1.0, 0.0, 0.0]);
    let dev = cross(bone_axis_local, flex_axis_local);
    let deviation_axis_local = normalised_or(dev, [0.0, 0.0, 1.0]);
    WristRest {
        hand_rest,
        bone_axis_local,
        forearm_axis_local: bone_axis_local,
        flex_axis_local,
        deviation_axis_local,
    }
}

/// The per-athlete solver plan: hierarchy order and the four contact chains.
#[derive(Debug, Clone)]
pub struct PoseSolver {
    parent: Vec<Option<usize>>,
    order: Vec<usize>,
    hips: usize,
    semantic: Vec<usize>,
    /// left hand, right hand, left foot, right foot.
    bindings: [Binding; 4],
    /// Wrist rest frames (left, right) the budget pass measures against.
    wrists: [WristRest; 2],
}

impl PoseSolver {
    /// Parent joint index (test/debug FK reconstruction).
    #[must_use]
    pub fn parent_of(&self, joint: usize) -> Option<usize> {
        self.parent.get(joint).copied().flatten()
    }

    /// The elbow (lower) joint index for hand binding `index`
    /// (test/debug FK reconstruction).
    #[must_use]
    pub fn elbow_index(&self, index: usize) -> usize {
        self.bindings[index].lower
    }

    /// Build the plan for `athlete` (hierarchy order, contact chains).
    ///
    /// # Errors
    /// A contact bone without two ancestors, or a broken hierarchy.
    pub fn new(athlete: &V4Athlete) -> Result<Self, AssetError> {
        let parent: Vec<Option<usize>> = athlete.joints.iter().map(|joint| joint.parent).collect();
        let mut order = Vec::with_capacity(parent.len());
        let mut queue: Vec<usize> = (0..parent.len()).filter(|&i| parent[i].is_none()).collect();
        let mut cursor = 0;
        while cursor < queue.len() {
            let index = queue[cursor];
            cursor += 1;
            order.push(index);
            queue.extend((0..parent.len()).filter(|&child| parent[child] == Some(index)));
        }
        if order.len() != parent.len() {
            return Err(AssetError::Athlete("cyclic joint hierarchy".to_owned()));
        }
        let hips = *athlete
            .semantic
            .first()
            .ok_or_else(|| AssetError::Athlete("no semantic bones".to_owned()))?;
        let binding = |role: &str, hint: [f64; 3]| -> Result<Binding, AssetError> {
            let (bone, _, offset) = athlete
                .contacts
                .iter()
                .find(|(_, contact_role, _)| contact_role == role)
                .ok_or_else(|| AssetError::Athlete(format!("no {role} contact")))?;
            let terminal = athlete
                .joint_index(bone)
                .ok_or_else(|| AssetError::Athlete(format!("{role} names unknown bone {bone}")))?;
            let lower = parent[terminal]
                .ok_or_else(|| AssetError::Athlete(format!("{role} chain too short at {bone}")))?;
            let upper = parent[lower].ok_or_else(|| {
                AssetError::Athlete(format!("{role} chain too short above {bone}"))
            })?;
            Ok(Binding {
                upper,
                lower,
                terminal,
                offset: *offset,
                bend_hint: hint,
            })
        };
        // Studio's anatomical bend planes, remapped from its +90° X workspace
        // back into glTF space ((x, y, z)_ws → (x, z, −y)): elbows out, back
        // and slightly down; knees out and forward.
        let bindings = [
            binding("left-hand", [-0.65, -0.22, -0.70])?,
            binding("right-hand", [0.65, -0.22, -0.70])?,
            binding("left-foot", [-0.12, 0.18, 0.90])?,
            binding("right-foot", [0.12, 0.18, 0.90])?,
        ];
        Ok(PoseSolver {
            parent,
            order,
            hips,
            semantic: athlete.semantic.clone(),
            bindings,
            wrists: [
                build_wrist_rest(athlete, bindings[0].terminal, -1.0),
                build_wrist_rest(athlete, bindings[1].terminal, 1.0),
            ],
        })
    }

    /// Pose the athlete: sample `clip` at the stroke's clip time, align the
    /// pelvis, close the four contacts (Studio `solve`, web `constrain`),
    /// then replace the clip's authored wrist orientation with the grip
    /// channel frame (web `orientHandToGripChannel` + the sport refinements)
    /// under the wrist budgets, and re-close the hands onto their targets.
    #[must_use]
    pub fn pose(
        &self,
        athlete: &V4Athlete,
        sport: Sport,
        clip: &Clip,
        clip_time: f64,
        targets: &ContactTargets,
        frames: &[GripFrame; 2],
        oar: Option<RowerOarInputs>,
    ) -> Posed {
        let mut targets = *targets;
        let mut work = Workspace::new(athlete.sample(clip, clip_time), &self.parent, &self.order);
        // Clip snapshot of both forearm locals: the shoulder-share pass
        // measures the elbow-seam excess against these after the solves.
        let snapshot = [
            work.locals[self.bindings[0].lower].rotation,
            work.locals[self.bindings[1].lower].rotation,
        ];

        // Row's long reach benefits from an initial arm clearance solve
        // before the pelvis is placed; its clamp results are throwaway.
        if sport == Sport::Rower {
            for (binding, target) in [
                (self.bindings[0], targets.left_hand),
                (self.bindings[1], targets.right_hand),
            ] {
                let target = work.clamp_hand_target(&binding, target).0;
                work.solve_limb(&binding, target);
            }
        }

        // Translation-only pelvis alignment: move the root joint so the hip
        // bone lands on its target; the authored hip pitch/roll stay.
        let hips_world = work.world_pos[self.hips];
        let shift = sub(targets.pelvis, hips_world);
        work.locals[self.hips].translation = add(work.locals[self.hips].translation, shift);
        work.recompute_subtree(self.hips);
        if sport == Sport::Bike {
            // Seat contract: keep the posterior on the pad rather than
            // sinking through it (lift only, never drag down).
            let sit_y = work.world_pos[self.hips][1] + geometry::BIKE_SIT_SURFACE_FROM_HIP_Y;
            let lift = geometry::bike_saddle_pad_top_y() - geometry::BIKE_SIT_NESTLE - sit_y;
            if lift > 1e-5 && lift <= REACH_CONTACT_BUDGET {
                work.locals[self.hips].translation[1] += lift;
                work.recompute_subtree(self.hips);
            }
        }

        // RowErg arm-authority composition (web `placeArms`): the `armDraw`
        // channel schedules the requested shoulder→wrist reach, and the rigid
        // oar's yaw is solved to put the inboard grip on that reach sphere.
        // The authored sweep is only the solve's branch fallback, so the hand
        // targets must be recomputed from the solved yaw before the arm
        // solves below — otherwise the arms chase a grip the web never
        // renders (the port fell ~0.13 m short at the forward catch).
        let mut oar_yaw = None;
        if sport == Sport::Rower {
            if let Some(oar) = oar {
                let mut solved = [0.0f64; 2];
                for (index, side) in [(0usize, -1.0f64), (1usize, 1.0f64)] {
                    let binding = self.bindings[index];
                    let shoulder = work.world_pos[binding.upper];
                    let elbow = work.world_pos[binding.lower];
                    let wrist = work.point(binding.offset, binding.terminal);
                    // The skeleton's reach, as the web reads it from the V4
                    // refinement (`armReaches`).
                    let arm_reach = length(sub(elbow, shoulder)) + length(sub(wrist, elbow));
                    let share = UPPER_ARM_LENGTH / (UPPER_ARM_LENGTH + FOREARM_LENGTH);
                    let upper = arm_reach * share;
                    let requested = rower_requested_wrist_reach(oar.draw, upper, arm_reach - upper);
                    let yaw = solve_rower_oar_yaw(
                        shoulder,
                        side * geometry::ROW_OARLOCK[0],
                        geometry::ROW_OARLOCK[1],
                        geometry::ROW_OARLOCK[2],
                        -side * geometry::ROW_INBOARD_CONTACT,
                        -side * oar.roll,
                        requested,
                        side * oar.preferred_yaw,
                        true,
                    );
                    solved[index] = yaw;
                    // Rebuild this side's grip target from the solved yaw
                    // (same rigid circle `rower_targets` uses, but with the
                    // solved sweep instead of the authored one) — the web's
                    // Euler-XYZ order (yaw ⊗ roll), whose yaw-independent
                    // roll lift `solve_rower_oar_yaw` assumes.
                    let yaw_q = axis_angle([0.0, 1.0, 0.0], yaw);
                    let roll_q = axis_angle([0.0, 0.0, 1.0], -side * oar.roll);
                    let orientation = quat_mul(yaw_q, roll_q);
                    let local = [
                        -side * geometry::ROW_INBOARD_CONTACT,
                        geometry::ROW_GRIP_DROP,
                        0.0,
                    ];
                    let pivot = [
                        side * geometry::ROW_OARLOCK[0],
                        geometry::ROW_OARLOCK[1],
                        geometry::ROW_OARLOCK[2],
                    ];
                    let target = add(pivot, rotate(orientation, local));
                    if index == 0 {
                        targets.left_hand = target;
                    } else {
                        targets.right_hand = target;
                    }
                }
                oar_yaw = Some(solved);
            }
        }

        let mut effective = targets;
        // Structural segment lengths per hand chain, measured once per
        // frame from the clip-sampled pose (web `correctContactChain`):
        // consumed unchanged by every solve pass below so the reach
        // boundary cannot shift between passes (the alternating-pass
        // re-measure was the elbow-branch snap at near-extension).
        let measured_hands = [
            (
                length(sub(
                    work.world_pos[self.bindings[0].lower],
                    work.world_pos[self.bindings[0].upper],
                )),
                length(sub(
                    work.point(self.bindings[0].offset, self.bindings[0].terminal),
                    work.world_pos[self.bindings[0].lower],
                )),
            ),
            (
                length(sub(
                    work.world_pos[self.bindings[1].lower],
                    work.world_pos[self.bindings[1].upper],
                )),
                length(sub(
                    work.point(self.bindings[1].offset, self.bindings[1].terminal),
                    work.world_pos[self.bindings[1].lower],
                )),
            ),
        ];
        let roles = [
            (0, targets.left_hand, true),
            (1, targets.right_hand, true),
            (2, targets.left_foot, false),
            (3, targets.right_foot, false),
        ];
        for (index, target, is_hand) in roles {
            let binding = self.bindings[index];
            let solve_target = if is_hand {
                let (clamped, forgiven) = work.clamp_hand_target(&binding, target);
                if forgiven {
                    if index == 0 {
                        effective.left_hand = clamped;
                    } else {
                        effective.right_hand = clamped;
                    }
                }
                clamped
            } else {
                target
            };
            // SkiErg hands solve against the per-phase bend plane (web
            // `setArmBendHint`): the phase hint projected perpendicular to
            // the shoulder→hand chord, with the web's pure-side-out
            // fallback if that degenerates. The static Studio hint flips
            // the elbow branch where it crosses the chord under the fixed
            // plant (measured 1.74 rad wrist-demand snap at cyc 0.2745).
            if is_hand && sport == Sport::Skierg && index < frames.len() {
                let hint = project_bend_hint(
                    frames[index].ski_bend_hint,
                    work.world_pos[binding.upper],
                    solve_target,
                );
                work.solve_limb_measured(
                    &binding,
                    solve_target,
                    hint,
                    Some(measured_hands[index]),
                    false,
                );
            } else {
                work.solve_limb(&binding, solve_target);
            }
        }

        // Terminal orientation: the hands adopt the full grip-channel frame
        // (the clip's authored wrist orientation is replaced for the current
        // sport), refined per sport and held inside the wrist budgets; the
        // position pass afterwards re-closes the palm with the solved frame.
        // A second alternating pass does not converge further (measured:
        // identical worst residuals) — the extreme-phase shortfall is a
        // reach limit, inside the usability budget, not an iteration count.
        let mut wrist = [WristMetrics::default(), WristMetrics::default()];
        if sport == Sport::Skierg {
            // The refinements' forearm input is captured ONCE per frame
            // (after the first position solve), matching the web: the
            // procedural `placePoleArms` reads the pole-solved
            // `handPoint − elbowPoint` of its own pass — a stable per-frame
            // input — while the V4 loop's orient never re-reads the live
            // forearm. Reading the live pose inside the loop instead fed
            // the refinements their own previous output (the loop rotates
            // the wrist, the solve re-aims the elbow, the refinement sees
            // a different forearm...), and the wrist frame oscillated
            // between two solutions pass-to-pass (measured: `desired`
            // alternating 2 rad within one frame at cyc ≈ 0.26).
            let stable_forearms = [
                normalised_or(
                    sub(
                        work.world_pos[self.bindings[0].terminal],
                        work.world_pos[self.bindings[0].lower],
                    ),
                    [0.0, -1.0, 0.0],
                ),
                normalised_or(
                    sub(
                        work.world_pos[self.bindings[1].terminal],
                        work.world_pos[self.bindings[1].lower],
                    ),
                    [0.0, -1.0, 0.0],
                ),
            ];
            // Web V4 `correctContactChain` (renderer3dV4Motion), arm path,
            // in the web's exact order:
            //
            // 1. `softOrientEffector` once — establish the grip frame.
            // 2. Six position-only passes with `preserve_terminal` (the
            //    web's `solvePositionTowardTarget` counter-rotation): the
            //    two-bone geometry closes on the contact *before* any
            //    orient re-set. The counter-rotation holds the hand's
            //    world frame through the parent swings, so these passes
            //    move only the elbow/forearm — they cannot snap the hand.
            // 3. Four alternating orient → solve passes, position LAST —
            //    "a trailing orientation set rotates the contact offset
            //    and reopens the gap it just closed". The pole-led grip
            //    frame moves the palm centimetres relative to the clip
            //    wrist, so frame and position must be closed alternately.
            //
            // The port previously went straight into the alternating loop
            // (no settling passes): at the near-extension boundary (cyc ≈
            // 0.2645) the pass-1 orient saw an unsettled geometry whose
            // contact offset pointed out of family, and the loop landed in
            // a different convergence basin than the neighbouring frames —
            // the hand snapped 0.096 m / 1.30 rad in one 1/2000-cycle step
            // while the elbow stayed smooth. The web's own history has the
            // same class of bug: switching solver precision per phase
            // "produced a genuine discontinuity (measured 0.17 m … hand
            // jump in one sample)"; their fix — always converge fully, the
            // same structure this port now matches.
            for (index, side) in [(0, -1.0), (1, 1.0)] {
                let binding = self.bindings[index];
                let frame = frames[index];
                wrist[index] = self.orient_hand_with_forearm(
                    &mut work,
                    sport,
                    binding,
                    side,
                    &frame,
                    Some(stable_forearms[index]),
                );
            }
            for _ in 0..6 {
                for (index, target) in [(0, effective.left_hand), (1, effective.right_hand)] {
                    let hint = project_bend_hint(
                        frames[index].ski_bend_hint,
                        work.world_pos[self.bindings[index].upper],
                        target,
                    );
                    work.solve_limb_measured(
                        &self.bindings[index],
                        target,
                        hint,
                        Some(measured_hands[index]),
                        true,
                    );
                }
            }
            for _pass in 0..4 {
                for (index, side) in [(0, -1.0), (1, 1.0)] {
                    let binding = self.bindings[index];
                    let frame = frames[index];
                    wrist[index] = self.orient_hand_with_forearm(
                        &mut work,
                        sport,
                        binding,
                        side,
                        &frame,
                        Some(stable_forearms[index]),
                    );
                }
                for (index, target) in [(0, effective.left_hand), (1, effective.right_hand)] {
                    let hint = project_bend_hint(
                        frames[index].ski_bend_hint,
                        work.world_pos[self.bindings[index].upper],
                        target,
                    );
                    work.solve_limb_measured(
                        &self.bindings[index],
                        target,
                        hint,
                        Some(measured_hands[index]),
                        true,
                    );
                }
            }
        } else {
            for (index, side) in [(0, -1.0), (1, 1.0)] {
                let binding = self.bindings[index];
                let frame = frames[index];
                wrist[index] = self.orient_hand(&mut work, sport, binding, side, &frame);
            }
            for (index, target) in [(0, effective.left_hand), (1, effective.right_hand)] {
                work.solve_limb(&self.bindings[index], target);
            }
        }
        if sport == Sport::Skierg {
            for (index, side) in [(0, -1.0), (1, 1.0)] {
                let binding = self.bindings[index];
                wrist[index].humerus_roll =
                    self.distribute_ski_elbow_twist(&mut work, snapshot[index], binding, side);
            }
        }

        let residual = |binding: &Binding, target: [f64; 3]| -> f64 {
            let actual = work.point(binding.offset, binding.terminal);
            let value = length(sub(actual, target));
            if value.is_finite() {
                value
            } else {
                f64::INFINITY
            }
        };
        let residuals = Residuals {
            left_hand: residual(&self.bindings[0], effective.left_hand),
            right_hand: residual(&self.bindings[1], effective.right_hand),
            left_foot: residual(&self.bindings[2], effective.left_foot),
            right_foot: residual(&self.bindings[3], effective.right_foot),
            pelvis: length(sub(work.world_pos[self.hips], targets.pelvis)),
        };
        for local in &mut work.locals {
            local.rotation = normalise(local.rotation);
            for value in local.translation.iter_mut().chain(local.scale.iter_mut()) {
                if !value.is_finite() {
                    *value = 0.0;
                }
            }
        }
        Posed {
            oar_yaw,
            locals: work.locals,
            residuals,
            wrist,
        }
    }

    /// Orient one hand onto its grip frame with the sport's refinements,
    /// then hold the wrist inside its budgets (web `softOrientEffector` +
    /// `constrainWristFrame`). Returns the solve metrics.
    fn orient_hand(
        &self,
        work: &mut Workspace<'_>,
        sport: Sport,
        binding: Binding,
        side: f64,
        frame: &GripFrame,
    ) -> WristMetrics {
        self.orient_hand_with_forearm(work, sport, binding, side, frame, None)
    }

    /// [`Self::orient_hand`] with a per-frame-stable forearm direction for
    /// the wrist refinements (the skierg alternating loop passes the
    /// captured first-solve direction; `None` reads the live pose for the
    /// single-pass sports).
    fn orient_hand_with_forearm(
        &self,
        work: &mut Workspace<'_>,
        sport: Sport,
        binding: Binding,
        side: f64,
        frame: &GripFrame,
        stable_forearm: Option<[f64; 3]>,
    ) -> WristMetrics {
        let elbow = work.world_pos[binding.lower];
        let hand = work.world_pos[binding.terminal];
        let forearm = stable_forearm.unwrap_or_else(|| {
            let f = sub(hand, elbow);
            let l = length(f);
            if l.is_finite() && l > 1e-6 {
                scale(f, 1.0 / l)
            } else {
                [0.0, -1.0, 0.0]
            }
        });
        let roll_local = if frame.palm_roll {
            Some(hand_palm_normal_out(side))
        } else {
            None
        };
        let mut desired = orient_hand_to_grip_channel(
            frame.base,
            side,
            frame.radius,
            frame.shaft_thumbward,
            frame.roll_reference,
            roll_local,
        );
        let shaft = frame.shaft_thumbward;
        let across = |direction: [f64; 3]| -> f64 {
            let along = dot(direction, shaft);
            length(sub(direction, scale(shaft, along)))
        };
        match sport {
            Sport::Skierg => {
                let weight = smoothstep(across(forearm), 0.12, 0.35);
                if weight > 1e-4 {
                    desired = refine_grip_spin_for_wrist(
                        desired,
                        side,
                        shaft,
                        forearm,
                        SKI_FLAT_MAX_SPIN * weight,
                    );
                }
                desired = refine_grip_tilt_for_wrist(
                    desired,
                    side,
                    forearm,
                    SKI_PALM_TILT_COMFORT,
                    SKI_PALM_TILT,
                    1.0,
                );
            }
            Sport::Rower => {
                let weight = frame.flat_window * smoothstep(across(forearm), 0.12, 0.3);
                if weight > 1e-4 {
                    desired = flat_wrist_roll(desired, side, shaft, forearm, weight);
                }
                desired = refine_grip_tilt_for_wrist(
                    desired,
                    side,
                    forearm,
                    ROWER_PALM_TILT_COMFORT,
                    ROWER_PALM_TILT,
                    weight,
                );
            }
            Sport::Bike => {}
        }
        let parent_world = work.world_rot[binding.lower];
        let local = normalise(quat_mul(conjugate(parent_world), desired));
        let previous = work.locals[binding.terminal].rotation;
        work.locals[binding.terminal].rotation = local;
        work.recompute_subtree(binding.terminal);
        let rest = &self.wrists[usize::from(side >= 0.0)];
        let Some(solved) = constrain_wrist_frame(local, rest, sport) else {
            work.locals[binding.terminal].rotation = previous;
            work.recompute_subtree(binding.terminal);
            return WristMetrics::default();
        };
        if solved.redistributed {
            work.locals[binding.lower].rotation = normalise(quat_mul(
                work.locals[binding.lower].rotation,
                solved.forearm_twist,
            ));
        }
        work.locals[binding.terminal].rotation = solved.effector;
        work.recompute_subtree(binding.lower);
        solved.metrics
    }

    /// Post-solve shoulder share of the SkiErg pronation (web
    /// `distributeSkiElbowTwist`): after the passes settle, the forearm's
    /// local rotation relative to the humerus carries whatever axial twist
    /// the hold demanded beyond the wrist keep. Measured against the clip
    /// snapshot's forearm local, the excess rolls into the humerus about its
    /// own long axis — shoulder internal rotation — and the forearm's world
    /// orientation is restored, which restores every descendant including
    /// the solved grip hand. Joint positions and the hand frame are
    /// bit-exact; only the seam distribution changes. Returns the humerus
    /// roll (0 below the web's 1e-6 threshold).
    fn distribute_ski_elbow_twist(
        &self,
        work: &mut Workspace<'_>,
        snapshot_middle: [f64; 4],
        binding: Binding,
        side: f64,
    ) -> f64 {
        let rest = &self.wrists[usize::from(side >= 0.0)];
        work.split_seam_excess(&binding, snapshot_middle, rest.forearm_axis_local)
    }

    /// The wrist rest frame for one side (forearm-local anatomical axes).
    #[must_use]
    pub fn wrist_rest(&self, side: f64) -> WristRest {
        self.wrists[usize::from(side >= 0.0)]
    }

    /// Write the 19 semantic joints (translation xyz, rotation xyzw) into
    /// `frame` at the layout's joint block.
    pub fn pack(&self, posed: &Posed, frame: &mut [f32]) {
        for (slot, &joint) in self.semantic.iter().enumerate().take(frame::JOINT_COUNT) {
            let at = frame::joint_offset(slot);
            if at + frame::JOINT_STRIDE > frame.len() {
                break;
            }
            let local = &posed.locals[joint];
            for (i, value) in local.translation.iter().enumerate() {
                frame[at + i] = *value as f32;
            }
            for (i, value) in local.rotation.iter().enumerate() {
                frame[at + 3 + i] = *value as f32;
            }
        }
    }
}

/// Rower flat-wrist roll (web `placeArms`): an explicit extra rotation about
/// the shaft laying the hand's long axis onto the solved forearm —
/// Concept2's "wrists should be flat" made literal. The weight fades through
/// the feather window (the caller folds the window in) and guards the ±π
/// wrap, so the applied roll goes smoothly to zero on both sides of it.
fn flat_wrist_roll(
    hand: [f64; 4],
    side: f64,
    shaft: [f64; 3],
    forearm: [f64; 3],
    weight: f64,
) -> [f64; 4] {
    let project = |direction: [f64; 3]| -> [f64; 3] {
        let along = dot(direction, shaft);
        sub(direction, scale(shaft, along))
    };
    let long = project(rotate(hand, hand_long_axis(side)));
    let fore = project(forearm);
    if dot(long, long) <= 1e-8 || dot(fore, fore) <= 1e-8 {
        return hand;
    }
    let long = scale(long, 1.0 / length(long));
    let fore = scale(fore, 1.0 / length(fore));
    let cosine = dot(long, fore).clamp(-1.0, 1.0);
    let sine = dot(cross(long, fore), shaft);
    let angle = sine.atan2(cosine);
    let wrap_guard = smoothstep(std::f64::consts::PI - angle.abs(), 0.15, 0.6);
    let applied = angle * weight * wrap_guard;
    if applied.abs() < 1e-9 {
        return hand;
    }
    normalise(quat_mul(axis_angle(shaft, applied), hand))
}

/// Mutable world-transform workspace over the local pose (Studio
/// `ReplayAthleteMatrixWorkspace`): changing a joint recomputes only its
/// subtree.
struct Workspace<'a> {
    locals: Vec<LocalTransform>,
    parent: &'a [Option<usize>],
    order: &'a [usize],
    world_pos: Vec<[f64; 3]>,
    world_rot: Vec<[f64; 4]>,
    world_scale: Vec<[f64; 3]>,
}

impl<'a> Workspace<'a> {
    fn new(locals: Vec<LocalTransform>, parent: &'a [Option<usize>], order: &'a [usize]) -> Self {
        let n = locals.len();
        let mut work = Workspace {
            locals,
            parent,
            order,
            world_pos: vec![[0.0; 3]; n],
            world_rot: vec![[0.0, 0.0, 0.0, 1.0]; n],
            world_scale: vec![[1.0; 3]; n],
        };
        for &index in order {
            work.recompute(index);
        }
        work
    }

    fn recompute(&mut self, index: usize) {
        let local = self.locals[index];
        if let Some(parent) = self.parent[index] {
            let ps = self.world_scale[parent];
            let scaled = [
                local.translation[0] * ps[0],
                local.translation[1] * ps[1],
                local.translation[2] * ps[2],
            ];
            self.world_pos[index] = add(
                self.world_pos[parent],
                rotate(self.world_rot[parent], scaled),
            );
            self.world_rot[index] = normalise(quat_mul(self.world_rot[parent], local.rotation));
            self.world_scale[index] = [
                ps[0] * local.scale[0],
                ps[1] * local.scale[1],
                ps[2] * local.scale[2],
            ];
        } else {
            self.world_pos[index] = local.translation;
            self.world_rot[index] = normalise(local.rotation);
            self.world_scale[index] = local.scale;
        }
    }

    fn recompute_subtree(&mut self, root: usize) {
        // The evaluation order lists parents before children, so a single
        // pass over the suffix starting at `root` covers its subtree.
        let start = self.order.iter().position(|&i| i == root).unwrap_or(0);
        let mut in_subtree = vec![false; self.locals.len()];
        in_subtree[root] = true;
        for &index in &self.order[start..] {
            if index == root || self.parent[index].is_some_and(|p| in_subtree[p]) {
                in_subtree[index] = true;
                self.recompute(index);
            }
        }
    }

    fn point(&self, offset: [f64; 3], joint: usize) -> [f64; 3] {
        let s = self.world_scale[joint];
        let scaled = [offset[0] * s[0], offset[1] * s[1], offset[2] * s[2]];
        add(self.world_pos[joint], rotate(self.world_rot[joint], scaled))
    }

    /// Clamp a hand target into the arm's reach (web parity: best-effort
    /// constrain). Returns the target to solve toward and whether the
    /// shortfall is small enough to be forgiven by the residual gate.
    fn clamp_hand_target(&self, binding: &Binding, target: [f64; 3]) -> ([f64; 3], bool) {
        let shoulder = self.world_pos[binding.upper];
        let elbow = self.world_pos[binding.lower];
        let contact = self.point(binding.offset, binding.terminal);
        let reach = (length(sub(elbow, shoulder)) + length(sub(contact, elbow))) * 0.998;
        let delta = sub(target, shoulder);
        let distance = length(delta);
        if distance > reach && distance > 1e-6 && reach > 1e-5 {
            let clamped = add(shoulder, scale(delta, reach / distance));
            return (clamped, distance - reach <= GRIP_REACH_SHORTFALL_TOLERANCE);
        }
        (target, false)
    }

    fn solve_limb(&mut self, binding: &Binding, target: [f64; 3]) {
        self.solve_limb_hinted(binding, target, binding.bend_hint);
    }

    /// [`Self::solve_limb`] with a per-phase bend hint. The skierg arm
    /// pass recomputes the bend plane from the solved target and the
    /// technique phase (web `setArmBendHint` + its recompute pass): a
    /// static hint runs near the shoulder→hand chord through parts of the
    /// cycle and flips the elbow branch (the web's own measured 0.19 m
    /// elbow jumps; its fix — a geometry-conditioned lateral floor of at
    /// least ~0.24 through every phase — is the law implemented here).
    fn solve_limb_hinted(&mut self, binding: &Binding, target: [f64; 3], hint: [f64; 3]) {
        self.solve_limb_measured(binding, target, hint, None, false);
    }

    /// [`Self::solve_limb_hinted`] with fixed structural segment lengths.
    /// The web's V4 `correctContactChain` measures `proximalLength` /
    /// `distalLength` once per correction, from the pre-solve pose, and
    /// both its reach clamp and its two-bone solve consume the same pair;
    /// the follow-up passes reuse them unchanged. Re-measuring inside the
    /// solve (the pre-fix port) read the *just-solved* pose — where the
    /// wrist's oriented contact offset has already been re-aimed — so the
    /// distal length alternated between passes (measured 0.449/0.372 m)
    /// and the reach boundary shifted frame-to-frame, snapping the elbow
    /// branch at the near-extension singularity (a 2.0 rad twist-demand
    /// jump at cyc ≈ 0.2645 in the dense guard).
    fn solve_limb_measured(
        &mut self,
        binding: &Binding,
        target: [f64; 3],
        hint: [f64; 3],
        measured: Option<(f64, f64)>,
        preserve_terminal: bool,
    ) {
        let root = self.world_pos[binding.upper];
        let joint = self.world_pos[binding.lower];
        let contact = self.point(binding.offset, binding.terminal);
        let (first, second) =
            measured.unwrap_or_else(|| (length(sub(joint, root)), length(sub(contact, joint))));
        if !first.is_finite() || !second.is_finite() || first <= 1e-5 || second <= 1e-5 {
            return; // degenerate limb: keep the clip pose
        }
        let solution = solve3d(root, target, first, second, hint);
        // Preserve the effector's world orientation across the parent-bone
        // swings (web `solvePositionTowardTarget`'s counter-rotation):
        // grip-frame hands need the wrist frame chosen by `orient_hand` to
        // survive the forearm solve, "so the palm offset remains stable;
        // the next pass can then close the rigid grip instead of chasing a
        // contact point that moves with every IK pass" — the chasing is
        // what flipped the forearm across the shaft at near-extension.
        // `preserve_terminal` is set only inside the skierg alternating
        // orient/solve loop; the generic rower/bike path keeps the clip
        // wrist until `orient_hand` replaces it, as before.
        let preserved = self.world_rot[binding.terminal];
        self.aim_joint(binding.upper, root, joint, solution.joint);
        let new_joint = self.world_pos[binding.lower];
        let current_contact = self.point(binding.offset, binding.terminal);
        self.aim_joint(binding.lower, new_joint, current_contact, solution.end);
        if preserve_terminal {
            self.set_world_rotation(binding.terminal, preserved);
        }
    }

    /// Set a joint's world rotation, preserving its local translation
    /// (web `setBoneWorldQuaternion`).
    fn set_world_rotation(&mut self, joint: usize, world: [f64; 4]) {
        let parent_world = self.parent[joint].map_or([0.0, 0.0, 0.0, 1.0], |p| self.world_rot[p]);
        let local = normalise(quat_mul(conjugate(parent_world), world));
        if local.iter().all(|v| v.is_finite()) {
            self.locals[joint].rotation = local;
            self.recompute_subtree(joint);
        }
    }

    /// Split the elbow-seam twist excess between humerus and forearm (web
    /// `distributeSkiElbowTwist` core): roll the humerus about its own long
    /// axis by half the snapshot-relative excess and restore the forearm's
    /// world orientation, preserving every descendant. Returns the humerus
    /// roll, or 0 when the excess is below threshold or the axis is
    /// degenerate (locals untouched in both cases).
    fn split_seam_excess(
        &mut self,
        binding: &Binding,
        snapshot_middle: [f64; 4],
        forearm_axis: [f64; 3],
    ) -> f64 {
        use rowplay_core::replay::wrist::ski_humerus_roll;
        let delta = quat_mul(
            conjugate(snapshot_middle),
            self.locals[binding.lower].rotation,
        );
        let dot_twist =
            delta[0] * forearm_axis[0] + delta[1] * forearm_axis[1] + delta[2] * forearm_axis[2];
        let mut excess = 2.0 * dot_twist.atan2(delta[3]);
        if excess > std::f64::consts::PI {
            excess -= 2.0 * std::f64::consts::PI;
        } else if excess < -std::f64::consts::PI {
            excess += 2.0 * std::f64::consts::PI;
        }
        let roll = ski_humerus_roll(excess);
        if roll == 0.0 {
            return 0.0;
        }
        let mut axis = sub(self.world_pos[binding.lower], self.world_pos[binding.upper]);
        if dot(axis, axis) <= 1e-16 {
            return 0.0;
        }
        axis = scale(axis, 1.0 / length(axis));
        let saved_middle_world = self.world_rot[binding.lower];
        let parent_world =
            self.parent[binding.upper].map_or([0.0, 0.0, 0.0, 1.0], |p| self.world_rot[p]);
        let rolled_upper_world = normalise(quat_mul(
            axis_angle(axis, roll),
            self.world_rot[binding.upper],
        ));
        let upper_local = normalise(quat_mul(conjugate(parent_world), rolled_upper_world));
        if !upper_local.iter().all(|v| v.is_finite()) {
            return 0.0;
        }
        self.locals[binding.upper].rotation = upper_local;
        self.recompute_subtree(binding.upper);
        let restored_middle_local = normalise(quat_mul(
            conjugate(self.world_rot[binding.upper]),
            saved_middle_world,
        ));
        if !restored_middle_local.iter().all(|v| v.is_finite()) {
            return 0.0;
        }
        self.locals[binding.lower].rotation = restored_middle_local;
        self.recompute_subtree(binding.lower);
        roll
    }

    fn aim_joint(
        &mut self,
        joint: usize,
        source_start: [f64; 3],
        source_end: [f64; 3],
        target_end: [f64; 3],
    ) {
        let source = sub(source_end, source_start);
        let destination = sub(target_end, source_start);
        if length(source) <= 1e-5 || length(destination) <= 1e-5 {
            return;
        }
        let delta = rotation_between(source, destination);
        let desired_world = quat_mul(delta, self.world_rot[joint]);
        let parent_world = self.parent[joint].map_or([0.0, 0.0, 0.0, 1.0], |p| self.world_rot[p]);
        let local = normalise(quat_mul(conjugate(parent_world), desired_world));
        if local.iter().all(|v| v.is_finite()) {
            self.locals[joint].rotation = local;
            self.recompute_subtree(joint);
        }
    }
}

// Quaternion helpers, `x, y, z, w`.

fn axis_angle(axis: [f64; 3], angle: f64) -> [f64; 4] {
    let half = angle * 0.5;
    let s = half.sin();
    normalise([axis[0] * s, axis[1] * s, axis[2] * s, half.cos()])
}

fn quat_mul(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let av = [a[0], a[1], a[2]];
    let bv = [b[0], b[1], b[2]];
    let v = add(add(scale(bv, a[3]), scale(av, b[3])), cross(av, bv));
    [v[0], v[1], v[2], a[3] * b[3] - dot(av, bv)]
}

fn conjugate(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

fn rotate(q: [f64; 4], v: [f64; 3]) -> [f64; 3] {
    let u = [q[0], q[1], q[2]];
    let t = scale(cross(u, v), 2.0);
    add(add(v, scale(t, q[3])), cross(u, t))
}

/// The rotation taking unit `source` onto unit `target` (Studio `rotation(from:to:)`).
fn rotation_between(source: [f64; 3], target: [f64; 3]) -> [f64; 4] {
    let from = scale(source, 1.0 / length(source));
    let to = scale(target, 1.0 / length(target));
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
        let axis = scale(axis, 1.0 / length(axis));
        return axis_angle(axis, std::f64::consts::PI);
    }
    let c = cross(from, to);
    normalise([c[0], c[1], c[2], 1.0 + scalar])
}

fn normalise(q: [f64; 4]) -> [f64; 4] {
    let magnitude = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if !magnitude.is_finite() || magnitude < 1e-6 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [
        q[0] / magnitude,
        q[1] / magnitude,
        q[2] / magnitude,
        q[3] / magnitude,
    ]
}

fn normalised_or(v: [f64; 3], fallback: [f64; 3]) -> [f64; 3] {
    let len = length(v);
    if len.is_finite() && len > 1e-9 {
        scale(v, 1.0 / len)
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rowplay_core::replay::motion_graph;
    use rowplay_core::replay::rig_pose::solve_rig_pose;
    use rowplay_core::replay::stroke_model::fallback_stroke_pose;

    fn vendored() -> V4Athlete {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay");
        let bytes = std::fs::read(dir.join("rowplay-athlete-v4.glb")).expect("athlete pack");
        let contract = std::fs::read_to_string(dir.join("rowplay-athlete-v4.contract.json"))
            .expect("contract");
        super::super::athlete::read_v4(&bytes, &contract).expect("V4 pack")
    }

    fn sport_name(sport: Sport) -> &'static str {
        match sport {
            Sport::Rower => "rower",
            Sport::Skierg => "skierg",
            Sport::Bike => "bike",
        }
    }

    /// Rebuild a [`StrokePose`] from the fixture's pose echo (the same
    /// field-for-field reconstruction core's parity harness uses).
    fn stroke_pose_from_echo(
        echo: &serde_json::Value,
    ) -> rowplay_core::replay::stroke_model::StrokePose {
        let f = |key: &str| {
            echo[key]
                .as_f64()
                .unwrap_or_else(|| panic!("pose.{key} missing"))
        };
        rowplay_core::replay::stroke_model::StrokePose {
            index: echo["index"].as_u64().expect("pose.index") as usize,
            phase: f("phase"),
            warped_phase: f("warpedPhase"),
            cycle_frac: f("cycleFrac"),
            drive_frac: f("driveFrac"),
            drive: echo["drive"].as_bool().expect("pose.drive"),
            drive_progress: f("driveProgress"),
            recovery_progress: f("recoveryProgress"),
            stroke_seconds: f("strokeSeconds"),
            stroke_meters: f("strokeMeters"),
            rate: f("rate"),
            watts: f("watts"),
            intensity: f("intensity"),
            amplitude: f("amplitude"),
            fatigue: f("fatigue"),
            real: echo["real"].as_bool().expect("pose.real"),
        }
    }

    #[test]
    fn clip_fraction_matches_the_web_mapping() {
        assert!((clip_fraction(0.19, 0.0, 0.38, 0.38) - 0.19).abs() < 1e-12);
        assert!((clip_fraction(0.5, 0.0, 0.4, 0.5) - (0.5 + 0.1 / 0.6 * 0.5)).abs() < 1e-12);
        assert!((clip_fraction(0.2, 0.0, 0.4, 0.5) - 0.25).abs() < 1e-12);
        // Without a cycle fraction the phase (radians) wraps into the cycle.
        assert!((clip_fraction(f64::NAN, std::f64::consts::PI, 0.5, 0.5) - 0.5).abs() < 1e-12);
        assert!((clip_fraction(1.25, 0.0, 0.5, 0.5) - 0.25).abs() < 1e-12);
        assert!(
            (clip_fraction(0.5, 0.0, 0.0, 1.0) - (0.95 + (0.5 - 0.05) / 0.95 * 0.05)).abs() < 1e-12
        );
    }

    #[test]
    fn rower_targets_mirror_and_feet_stay_on_the_stretcher() {
        let pose = fallback_stroke_pose(Sport::Rower, 1.0, 28.0);
        let rig = solve_rig_pose(Sport::Rower, &pose, 0.0, false);
        let targets = rig_targets(&rig);
        let c = targets.contacts;
        assert!(
            (c.left_hand[0] + c.right_hand[0]).abs() < 1e-9
                && (c.left_hand[1] - c.right_hand[1]).abs() < 1e-9
        );
        assert_eq!(c.left_foot, [-0.12, 0.215, 0.75]);
        assert!(
            c.pelvis[1] == 0.30
                && (c.pelvis[2]
                    - (-0.1
                        + match rig {
                            SportRigPose::Rower(r) => r.seat_z,
                            _ => 0.0,
                        }))
                .abs()
                    < 1e-12
        );
        // With no sweep the hands sit inboard of the oarlocks, rolled only by
        // the neutral feather (−0.06 rad) about the pivot's z axis.
        let neutral = rig_targets(&rowplay_core::replay::rig_pose::reduced_pose(Sport::Rower));
        let hand = neutral.contacts.right_hand;
        let pivot = [0.88, 0.51, 0.28];
        let arm = (geometry::ROW_INBOARD_CONTACT.powi(2) + geometry::ROW_GRIP_DROP.powi(2)).sqrt();
        assert!((length(sub(hand, pivot)) - arm).abs() < 1e-9);
        assert!((hand[2] - 0.28).abs() < 1e-9, "a z-axis roll keeps z");
        assert!(
            hand[0] > 0.0 && hand[0] < 0.2,
            "inboard of the oarlock: {}",
            hand[0]
        );
        assert!((hand[1] - (0.51 - 0.04)).abs() < 0.06);
        assert!(targets.poles.is_none());
    }

    #[test]
    fn ski_poles_keep_their_length_and_the_hand_stays_in_reach() {
        for phase in [0.0, 0.7, 1.5, 3.0, 4.6, 6.0] {
            let pose = fallback_stroke_pose(Sport::Skierg, phase, 34.0);
            let rig = solve_rig_pose(Sport::Skierg, &pose, 0.0, false);
            let targets = rig_targets(&rig);
            let [left, right] = targets.poles.expect("poles");
            for (side, pole) in [(-1.0, left), (1.0, right)] {
                assert!(
                    (length(sub(pole.tip, pole.root)) - 1.37).abs() < 1e-9,
                    "rigid pole"
                );
                assert!(pole.basket[1] > pole.tip[1], "basket sits up-shaft");
                assert!((length(pole.direction) - 1.0).abs() < 1e-9);
                assert!(pole.root[0] * side > 0.0, "hand on its own side");
                let SportRigPose::SkiErg(ski) = rig else {
                    panic!()
                };
                let shoulder = [side * 0.25, ski.shoulder_y, ski.shoulder_z];
                assert!(length(sub(pole.root, shoulder)) <= geometry::SKI_MAX_REACH + 1e-6);
            }
            assert_eq!(targets.contacts.left_hand, left.root);
        }
    }

    /// The solved RowErg oar yaw against the web's **rendered** yaw, across
    /// every fixture sample (AGENTS.md: record what the web renders, not how
    /// it computes).
    ///
    /// `replay-rig-phase-parity.json` records, per sample, the arguments the
    /// web's `solveRowerOarYaw` received (shoulder→pin delta, signed inboard
    /// lever, blade roll, requested reach, preferred yaw) and the yaw the
    /// avatar actually rendered. The generator re-solves from its own
    /// recording and refuses to write the fixture unless it reproduces the
    /// rendered value, so these inputs are trustworthy. Feeding them through
    /// the same solver the pose pass calls and matching the rendered yaw is
    /// what pins the composition to the web rather than to itself.
    #[test]
    fn the_composed_oar_yaw_matches_the_web_rendered_yaw() {
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-rig-phase-parity.json").expect("fixture");
        let samples = fixture["samples"].as_array().expect("samples");
        let mut checked = 0;
        for sample in samples {
            if sample["sport"].as_str() != Some("rower") {
                continue;
            }
            let solve = &sample["rig"]["oarSolve"];
            for side in solve["sides"].as_array().expect("sides") {
                let number = |key: &str| -> f64 {
                    side[key]
                        .as_f64()
                        .unwrap_or_else(|| panic!("oarSolve.{key} missing"))
                };
                let pin = side["pinDelta"].as_array().expect("pinDelta");
                let pin = |index: usize| pin[index].as_f64().expect("pinDelta value");
                let yaw = rowplay_core::replay::row_equipment::solve_rower_oar_yaw(
                    [0.0, 0.0, 0.0],
                    pin(0),
                    pin(1),
                    pin(2),
                    number("inboard"),
                    number("roll"),
                    number("requestedReach"),
                    number("preferredYaw"),
                    true,
                );
                let rendered = number("renderedYaw");
                let delta = (yaw - rendered).rem_euclid(std::f64::consts::TAU);
                let delta = delta.min(std::f64::consts::TAU - delta).abs();
                assert!(
                    delta <= 1e-9,
                    "phase {}: solved yaw {yaw} != web rendered {rendered} (delta {delta})",
                    sample["phaseIndex"]
                );
                checked += 1;
            }
        }
        assert_eq!(
            checked, 256,
            "the rower fixture carries 128 samples × 2 sides"
        );
    }

    /// The web's composed oar quaternion law, against the rendered hand
    /// targets the new `handTargets` field records (rowplay#199):
    /// `oar.group.quaternion` is three.js Euler(0, yaw, roll) in the
    /// default XYZ order — `qy(yaw) ⊗ qz(roll)` — acting on the
    /// side-signed `handleAnchor.position` (`-side · 0.78, -0.04, 0`),
    /// pivoted at `side · ROWER_OARLOCK`. The port's
    /// `oar_rotations_from_yaws` must reproduce the same grip point from
    /// the recorded solved yaw and feather, through its own template
    /// mirror composition, in rig space.
    ///
    /// Measured before any port change (the fixed order's first run): the
    /// current `qz(roll) ⊗ (qy(yaw) ⊗ mirror)` composition is off by up
    /// to 0.181 m at the catch — the web's `ROWER_OARLOCK` comment names
    /// the coupling it breaks ("the drive-side roll … puts the handles at
    /// the drive height"): under XYZ the roll's vertical lift
    /// (`0.78 · sin(roll)`) is yaw-independent, while the port's order
    /// scales it by `cos(yaw)`, collapsing the grip up to 6× at the
    /// finish where cos(1.4) ≈ 0.17.
    #[test]
    fn the_composed_oar_grip_target_matches_the_web_hand_target() {
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-rig-phase-parity.json").expect("fixture");
        let samples = fixture["samples"].as_array().expect("samples");
        let mut worst = 0.0f64;
        let mut worst_at = String::new();
        let mut checked = 0;
        for sample in samples {
            if sample["sport"].as_str() != Some("rower") {
                continue;
            }
            let yaws = [
                sample["rig"]["oarSolve"]["sides"][0]["renderedYaw"]
                    .as_f64()
                    .expect("left renderedYaw"),
                sample["rig"]["oarSolve"]["sides"][1]["renderedYaw"]
                    .as_f64()
                    .expect("right renderedYaw"),
            ];
            let feather = sample["rig"]["oarSolve"]["sides"][0]["roll"]
                .as_f64()
                .expect("roll");
            // The recorded roll is the per-side `oar.rotation.z` already
            // signed (`-side · dip`); `oar_rotations_from_yaws` takes the
            // unsigned feather, so take the magnitude (the fixture records
            // exact per-side mirrors).
            let feather = feather.abs();
            let rotations = crate::replay::equipment::oar_rotations_from_yaws(yaws, feather);
            for (index, side) in [(0usize, -1.0f64), (1usize, 1.0f64)] {
                let orientation = rotations[index];
                // The template anchor is right-authored and un-signed; the
                // left instance's π mirror flips it (signing here as well
                // would double-apply — the trap `grip.rs` documents).
                let local = [-geometry::ROW_INBOARD_CONTACT, geometry::ROW_GRIP_DROP, 0.0];
                let pivot = [
                    side * geometry::ROW_OARLOCK[0],
                    geometry::ROW_OARLOCK[1],
                    geometry::ROW_OARLOCK[2],
                ];
                let grip = add(
                    pivot,
                    crate::replay::equipment::rotate_vec(orientation, local),
                );
                let key = if side < 0.0 { "left" } else { "right" };
                let recorded = sample["rig"]["handTargets"][key]
                    .as_array()
                    .unwrap_or_else(|| panic!("handTargets.{key} missing"));
                let value = |i: usize| recorded[i].as_f64().expect("handTargets component");
                let delta = length(sub(grip, [value(0), value(1), value(2)]));
                if delta > worst {
                    worst = delta;
                    worst_at = format!(
                        "{} phase {} {}",
                        sample["sweep"].as_str().unwrap_or("?"),
                        sample["phaseIndex"],
                        key
                    );
                }
                checked += 1;
            }
        }
        assert_eq!(
            checked, 256,
            "the rower fixture carries 128 samples × 2 sides"
        );
        assert!(
            worst <= 1e-9,
            "composed grip target diverges from the web hand target: worst {worst} at {worst_at}"
        );
    }

    /// The skierg pole-contact solve against the web's post-solve hand
    /// targets (`v4HandTargets` exposes `arm.handTarget` after
    /// `placePoleArms`'s contact pass overwrites it with the solved hand —
    /// renderer3dSkiAvatar.ts:1079). The comparable port quantity is
    /// `rig_targets().contacts.left_hand` (the port's own
    /// `solve_rigid_contact3d` output, asserted equal to `left.root` in
    /// `ski_poles_keep_their_length_and_the_hand_stays_in_reach`), not
    /// core's authored `preferred_hand_*` arc.
    ///
    /// This is the comparison the coverage map's None table said needed
    /// "either a generator that also records `arm.handTarget`" — rowplay#199
    /// shipped exactly that surface.
    ///
    /// Windows (first-run measurements, before any port change):
    ///
    /// - **Recovery** (`SKI_POLE_OFF_CYCLE < cyc < APPROACH_START`):
    ///   machine epsilon (6.6e-16) — the shared Bezier law.
    /// - **Contact** (`cyc ≤ 0.29`): worst **0.977 m** — the documented
    ///   `plant_basket_z` model divergence (docs/source-map.md, the
    ///   "basket drifts ~2 m rearward" defect): the web pins the tip at
    ///   rig-local (−0.46, 0.081, 0.24) through the whole contact while
    ///   the port's plant retreats with travel, dragging its solved hand
    ///   aft. Tracked as the source-map's scoped follow-up (per-frame IK
    ///   + blend rewrite + visual verification); pinned here once fixed.
    /// - **Approach** (`cyc ≥ 0.88`): worst **0.033 m** — the web's
    ///   release-fade + attitude-blended carry ramp toward the next
    ///   plant, which the port's simplified carry direction does not
    ///   model. Same follow-up.
    ///
    /// The contact/approach deltas are measured and asserted against
    /// documented expectations (not retired): the assertion fails loudly
    /// if the divergence *grows* or the recovery law regresses.
    #[test]
    fn ski_flight_pole_direction_matches_the_web() {
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-rig-phase-parity.json").expect("fixture");
        let samples = fixture["samples"].as_array().expect("samples");
        let mut worst = 0.0f64;
        let mut checked = 0usize;
        for sample in samples {
            if sample["sport"].as_str() != Some("skierg") {
                continue;
            }
            let pose = stroke_pose_from_echo(&sample["pose"]);
            if !(0.6..0.8).contains(&pose.cycle_frac) {
                continue;
            }
            let meters = sample["rig"]["meters"].as_f64().expect("meters");
            let SportRigPose::SkiErg(ski) = solve_rig_pose(Sport::Skierg, &pose, meters, false)
            else {
                panic!("sport mismatch")
            };
            let poles = rig_targets(&SportRigPose::SkiErg(ski))
                .poles
                .expect("poles");
            let pole = poles[0];
            // The fixture records the basket node; the shaft thumbward
            // direction is (hand − basket) normalised. Compare against the
            // port's placement `direction` (tip→root) — same ray.
            let rec_hand = sample["rig"]["targets"]["leftHand"]
                .as_array()
                .expect("hand");
            let rec_basket = sample["rig"]["poleTipLeft"].as_array().expect("basket");
            let hand = [
                rec_hand[0].as_f64().unwrap(),
                rec_hand[1].as_f64().unwrap(),
                rec_hand[2].as_f64().unwrap(),
            ];
            let basket = [
                rec_basket[0].as_f64().unwrap(),
                rec_basket[1].as_f64().unwrap(),
                rec_basket[2].as_f64().unwrap(),
            ];
            let web_shaft = normalised_or(sub(hand, basket), [0.0, -1.0, 0.0]);
            let port_shaft = normalised_or(sub(pole.root, pole.tip), [0.0, -1.0, 0.0]);
            let delta = length(sub(web_shaft, port_shaft));
            worst = worst.max(delta);
            checked += 1;
        }
        assert!(checked > 0, "no flight-window samples found");
        // Web parity: the carried pole attitude must match the web's
        // recorded shaft direction through the flight window.
        assert!(
            worst < 2e-2,
            "flight pole direction diverges from the web by {worst}"
        );
        println!("flight shaft worst delta: {worst:.6} over {checked} samples");
    }

    #[test]
    fn the_skierg_solved_hand_matches_the_web_post_pole_solve_hand_target() {
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-rig-phase-parity.json").expect("fixture");
        let samples = fixture["samples"].as_array().expect("samples");
        // Windowed worst-case, matching the windows the web's own
        // `placePoleArms` pipeline moves through (contact ≤ 0.29, recovery,
        // approach ≥ 0.88) — the failure modes differ per window.
        // Windowed worst-case, matching the windows the web's own
        // `placePoleArms` pipeline moves through — the failure modes
        // differ per window.
        let mut worst = [0.0f64; 3];
        let mut checked = 0usize;
        let mut window_counts = [0usize; 3];
        for sample in samples {
            if sample["sport"].as_str() != Some("skierg") {
                continue;
            }
            let pose = stroke_pose_from_echo(&sample["pose"]);
            let meters = sample["rig"]["meters"].as_f64().expect("meters");
            let rig = solve_rig_pose(Sport::Skierg, &pose, meters, false);
            let targets = rig_targets(&rig);
            for (key, solved) in [
                ("left", targets.contacts.left_hand),
                ("right", targets.contacts.right_hand),
            ] {
                let recorded = sample["rig"]["handTargets"][key]
                    .as_array()
                    .unwrap_or_else(|| panic!("handTargets.{key} missing"));
                let value = |i: usize| recorded[i].as_f64().expect("handTargets component");
                let delta = length(sub(solved, [value(0), value(1), value(2)]));
                let cycle = pose.cycle_frac;
                let window = if cycle <= motion_graph::SKI_POLE_OFF_CYCLE {
                    0
                } else if cycle >= motion_graph::SKI_POLE_APPROACH_START_CYCLE {
                    2
                } else {
                    1
                };
                worst[window] = worst[window].max(delta);
                window_counts[window] += 1;
                checked += 1;
            }
        }
        assert_eq!(
            checked, 256,
            "the skierg fixture carries 128 samples × 2 sides"
        );
        assert!(window_counts.iter().all(|c| *c > 0));
        assert!(
            worst[1] <= 1e-6,
            "skierg recovery Bezier diverges from the web: worst {}",
            worst[1]
        );
        // The contact and approach windows were bounded at 0.98 / 0.04 m
        // against the documented `plant_basket_z` divergence while the
        // rewrite was pending (Phase 7.5). The rewrite landed: the web's
        // plant is fixed in rig-local through contact (course-turn drift
        // of ~5e-3 m over the window) and the carried tip converges on it
        // in direction space, so both windows must now agree to
        // geometry-grade tolerance. The named residuals are the parts of
        // the web pipeline the viewmodel target layer deliberately does
        // not model (the V4 fixed-point grip passes and the course-frame
        // carry beyond the lateral splay term), not plant drift.
        assert!(
            worst[0] <= 1e-2,
            "skierg contact diverges from the web beyond the named residuals: {}",
            worst[0]
        );
        assert!(
            worst[2] <= 1e-2,
            "skierg approach diverges from the web beyond the named residuals: {}",
            worst[2]
        );
        println!(
            "skierg hand-target worst: contact={:.6} recovery={:.6} approach={:.6}",
            worst[0], worst[1], worst[2]
        );
    }

    /// The bike bar contact against the web's static handlebar anchor
    /// (`v4HandTargets` for bike is the brake-hood contact anchor in
    /// rider-local, constant across the cycle).
    #[test]
    fn the_bike_bar_contact_matches_the_web_handlebar_anchor() {
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-rig-phase-parity.json").expect("fixture");
        let samples = fixture["samples"].as_array().expect("samples");
        let mut worst = 0.0f64;
        let mut checked = 0;
        for sample in samples {
            if sample["sport"].as_str() != Some("bike") {
                continue;
            }
            let pose = stroke_pose_from_echo(&sample["pose"]);
            let meters = sample["rig"]["meters"].as_f64().expect("meters");
            let rig = solve_rig_pose(Sport::Bike, &pose, meters, false);
            let targets = rig_targets(&rig);
            for (key, solved) in [
                ("left", targets.contacts.left_hand),
                ("right", targets.contacts.right_hand),
            ] {
                let recorded = sample["rig"]["handTargets"][key]
                    .as_array()
                    .unwrap_or_else(|| panic!("handTargets.{key} missing"));
                let value = |i: usize| recorded[i].as_f64().expect("handTargets component");
                let delta = length(sub(solved, [value(0), value(1), value(2)]));
                worst = worst.max(delta);
                checked += 1;
            }
        }
        assert_eq!(
            256, checked,
            "the bike fixture carries 128 samples × 2 sides"
        );
        assert!(
            worst <= 1e-6,
            "bike bar contact diverges from the web handlebar anchor: worst {worst}"
        );
    }

    /// The composed RowErg arm-authority chain: the pose pass solves each
    /// oar's yaw to place the inboard grip at the requested shoulder→wrist
    /// reach, and the returned yaws differ from the authored sweep wherever
    /// the arm cannot reach that sweep (web `placeArms`). At the forward
    /// catch the authored sweep is unreachable, so the solved yaw must move
    /// the grip *toward* the shoulder.
    #[test]
    fn the_rower_pose_composes_the_arm_authority_oar_solve() {
        let athlete = vendored();
        let Ok(solver) = PoseSolver::new(&athlete) else {
            panic!("solver");
        };
        let clip = athlete.clip_for("rower").expect("row clip");
        // A catch-ish pose: low draw, so the authored sweep is far forward.
        let mut pose = fallback_stroke_pose(Sport::Rower, 0.0, 26.0);
        pose.cycle_frac = 0.02;
        let rig = solve_rig_pose(Sport::Rower, &pose, 0.0, false);
        let targets = rig_targets(&rig);
        let SportRigPose::Rower(rower) = rig else {
            panic!()
        };
        let frames = crate::replay::grip::grip_frames(
            Sport::Rower,
            &rig,
            None,
            crate::replay::grip::warped_cycle(pose.warped_phase),
        );
        let posed = solver.pose(
            &athlete,
            Sport::Rower,
            clip,
            0.0,
            &targets.contacts,
            &frames,
            targets.oar,
        );
        let yaws = posed.oar_yaw.expect("the rower pose composes oar yaws");
        assert!(
            yaws.iter().all(|y| y.is_finite()),
            "oar yaws finite: {yaws:?}"
        );
        // The reach identity is the contract: the solved grip sits on the
        // requested reach sphere around the shoulder.
        for (index, side) in [(0usize, -1.0f64), (1usize, 1.0f64)] {
            let hand = if index == 0 {
                targets.contacts.left_hand
            } else {
                targets.contacts.right_hand
            };
            let pivot = [
                side * geometry::ROW_OARLOCK[0],
                geometry::ROW_OARLOCK[1],
                geometry::ROW_OARLOCK[2],
            ];
            // The grip rides the oar's rigid inboard circle: the lever is the
            // inboard contact offset plus the −0.04 drop, rotated (length is
            // rotation-invariant, so the yaw drops out and this is constant).
            let grip_lever = length(sub(hand, pivot));
            let expected = geometry::ROW_INBOARD_CONTACT.hypot(geometry::ROW_GRIP_DROP);
            assert!(
                (grip_lever - expected).abs() < 1e-9,
                "side {side}: the grip stays on the rigid inboard circle ({grip_lever} != {expected})"
            );
        }
        // At draw 0 the authored sweep is +0.68; the picked branch must not
        // have jumped to the far side of the circle.
        assert!(
            (yaws[1] - rower.oar_sweep).abs() < std::f64::consts::PI,
            "right yaw {} vs authored {}",
            yaws[1],
            rower.oar_sweep
        );
    }

    #[test]
    fn bike_feet_ride_the_pedals_and_the_pelvis_sits_over_the_bracket() {
        let pose = fallback_stroke_pose(Sport::Bike, 2.0, 85.0);
        let rig = solve_rig_pose(Sport::Bike, &pose, 12.0, false);
        let targets = rig_targets(&rig);
        let SportRigPose::Bike(bike) = rig else {
            panic!()
        };
        let c = targets.contacts;
        assert!((c.left_foot[1] - (geometry::BIKE_BB_Y + bike.pedal_pos_l.y)).abs() < 1e-12);
        assert!((c.right_foot[2] - (geometry::BIKE_BB_Z + bike.pedal_pos_r.z)).abs() < 1e-12);
        assert_eq!(c.left_foot[0], -0.09);
        assert!((c.pelvis[1] - geometry::bike_rider_hip_y()).abs() < 1e-12);
        assert!(c.pelvis[1] > 1.0 && c.pelvis[1] < 1.1, "{}", c.pelvis[1]);
        assert_eq!(
            c.left_hand,
            [-0.2, geometry::BIKE_GRIP_Y, geometry::BIKE_GRIP_Z]
        );
        assert!(
            (geometry::BIKE_GRIP_Y - 0.84).abs() < 1e-12
                && (geometry::BIKE_GRIP_Z - 0.51).abs() < 1e-12
        );
    }

    #[test]
    fn the_contact_pass_lands_the_pelvis_and_closes_the_contacts_for_every_sport() {
        let athlete = vendored();
        let solver = PoseSolver::new(&athlete).expect("plan");
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let clip = athlete.clip_for(sport_name(sport)).expect("clip");
            for step in 0..40 {
                let phase = f64::from(step) / 40.0 * std::f64::consts::TAU;
                let stroke = fallback_stroke_pose(sport, phase, 30.0);
                let rig = solve_rig_pose(sport, &stroke, f64::from(step) * 3.0, false);
                let targets = rig_targets(&rig).contacts;
                let fraction = clip_fraction(
                    stroke.cycle_frac,
                    stroke.phase,
                    stroke.drive_frac,
                    clip.drive_end,
                );
                let posed = solver.pose(
                    &athlete,
                    sport,
                    clip,
                    fraction * f64::from(clip.duration),
                    &targets,
                    &crate::replay::grip::grip_frames(
                        sport,
                        &rig,
                        rig_targets(&rig).poles,
                        crate::replay::grip::warped_cycle(stroke.warped_phase),
                    ),
                    rig_targets(&rig).oar,
                );
                assert_eq!(posed.locals.len(), 51);
                for local in &posed.locals {
                    let q = local.rotation;
                    let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
                    assert!((len - 1.0).abs() < 1e-9);
                    assert!(local.translation.iter().all(|v| v.is_finite()));
                }
                let r = posed.residuals;
                assert!(
                    r.pelvis < 1e-6,
                    "{sport:?} step {step}: pelvis {}",
                    r.pelvis
                );
                assert!(r.is_usable(), "{sport:?} step {step}: {r:?}");
                let mut frame = frame::empty_frame();
                solver.pack(&posed, &mut frame);
                assert!(
                    frame[frame::joint_offset(0)..frame::EQUIPMENT]
                        .iter()
                        .all(|v| v.is_finite())
                );
                // The hips (semantic 0) carry the pelvis target after alignment.
                let hips = posed.locals[athlete.semantic[0]];
                assert!(
                    (hips.translation[1] - f64::from(frame[frame::joint_offset(0) + 1])).abs()
                        < 1e-6
                );
            }
        }
    }

    #[test]
    fn grip_orientation_replaces_the_clip_wrist_within_budgets() {
        use crate::replay::grip::{grip_frames, warped_cycle};
        use rowplay_core::replay::wrist::{
            SKI_WRIST_TWIST_KEEP, WRIST_DEVIATION_BUDGET, WRIST_FLEXION_BUDGET, WRIST_TWIST_BUDGET,
        };
        let athlete = vendored();
        let solver = PoseSolver::new(&athlete).expect("plan");
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let clip = athlete.clip_for(sport_name(sport)).expect("clip");
            let twist_budget = match sport {
                Sport::Skierg => SKI_WRIST_TWIST_KEEP,
                Sport::Rower | Sport::Bike => WRIST_TWIST_BUDGET,
            };
            for step in 0..40 {
                let phase = f64::from(step) / 40.0 * std::f64::consts::TAU;
                let stroke = fallback_stroke_pose(sport, phase, 30.0);
                let rig = solve_rig_pose(sport, &stroke, f64::from(step) * 3.0, false);
                let targets = rig_targets(&rig);
                let frames = grip_frames(
                    sport,
                    &rig,
                    targets.poles,
                    warped_cycle(stroke.warped_phase),
                );
                let clip_time = clip_fraction(
                    stroke.cycle_frac,
                    stroke.phase,
                    stroke.drive_frac,
                    clip.drive_end,
                ) * f64::from(clip.duration);
                let posed = solver.pose(
                    &athlete,
                    sport,
                    clip,
                    clip_time,
                    &targets.contacts,
                    &frames,
                    targets.oar,
                );
                // The grip frame replaces the clip's authored wrist: the hand
                // locals differ from the raw clip sample at the same time.
                let sampled = athlete.sample(clip, clip_time);
                for (index, terminal) in [
                    (0, solver.bindings[0].terminal),
                    (1, solver.bindings[1].terminal),
                ] {
                    let posed_q = posed.locals[terminal].rotation;
                    let clip_q = sampled[terminal].rotation;
                    let dot = (posed_q[0] * clip_q[0]
                        + posed_q[1] * clip_q[1]
                        + posed_q[2] * clip_q[2]
                        + posed_q[3] * clip_q[3])
                        .abs();
                    assert!(
                        dot < 1.0 - 1e-6,
                        "{sport:?} step {step} hand {index}: orientation unchanged"
                    );
                    let metrics = posed.wrist[index];
                    assert!(
                        metrics.twist.abs() <= twist_budget + 1e-9,
                        "{sport:?} step {step} hand {index}: twist {}",
                        metrics.twist
                    );
                    assert!(
                        metrics.flexion.abs() <= WRIST_FLEXION_BUDGET + 1e-9
                            && metrics.deviation.abs() <= WRIST_DEVIATION_BUDGET + 1e-9,
                        "{sport:?} step {step} hand {index}: swing {},{}",
                        metrics.flexion,
                        metrics.deviation
                    );
                    assert!(metrics.clamped_swing >= 0.0);
                    assert!(metrics.forearm_twist.is_finite());
                }
                assert!(posed.residuals.is_usable(), "{sport:?} step {step}");
            }
        }
    }

    fn solver_parent(solver: &PoseSolver, joint: usize) -> Option<usize> {
        solver.parent_of(joint)
    }

    /// Smallest signed angular difference, wrapped to (-PI, PI].
    fn wrap_signed(delta: f64) -> f64 {
        use std::f64::consts::{PI, TAU};
        let w = (delta + PI).rem_euclid(TAU) - PI;
        if w == -PI { PI } else { w }
    }

    #[test]
    fn wrap_signed_covers_the_near_and_far_sides_of_the_circle() {
        use std::f64::consts::{PI, TAU};
        assert!((wrap_signed(0.1) - 0.1).abs() < 1e-12);
        assert!((wrap_signed(TAU - 0.1) - (-0.1)).abs() < 1e-12);
        assert!((wrap_signed(-TAU + 0.1) - 0.1).abs() < 1e-12);
        assert_eq!(wrap_signed(PI), PI);
    }

    /// SkiErg pre-clamp twist demand saturates at the 30° keep budget.
    ///
    /// Measured (2000 samples/cycle): max `|requested_twist|` =
    /// `SKI_WRIST_TWIST_KEEP + ~1.4e-15` at step 222 — not "inside with
    /// headroom", and not a vacuous post-clamp check (`requested_twist` is
    /// the pre-clamp `twist_angle`). The flat-wrist law drives the wrist
    /// *to* the keep cap; the sub-ULP excess is below the redistribution
    /// threshold, so the forearm share stays dust and `engaged` is false.
    fn assert_skierg_twist_saturates_at_keep(max_abs_requested: f64, engaged: bool) {
        use rowplay_core::replay::wrist::SKI_WRIST_TWIST_KEEP;
        assert!(
            (max_abs_requested - SKI_WRIST_TWIST_KEEP).abs() < 1e-12,
            "skierg pre-clamp demand must saturate at the keep budget \
             (got {max_abs_requested}, keep {SKI_WRIST_TWIST_KEEP})"
        );
        assert!(
            !engaged,
            "skierg: saturating at the keep budget must not redistribute \
             into the forearm"
        );
    }

    #[test]
    fn skierg_saturation_assert_bites_on_over_budget_demand() {
        use rowplay_core::replay::wrist::SKI_WRIST_TWIST_KEEP;
        let over = std::panic::catch_unwind(|| {
            assert_skierg_twist_saturates_at_keep(SKI_WRIST_TWIST_KEEP + 0.01, false);
        });
        assert!(
            over.is_err(),
            "demand past the keep budget must fail the saturation assert"
        );
        let redistributed = std::panic::catch_unwind(|| {
            assert_skierg_twist_saturates_at_keep(SKI_WRIST_TWIST_KEEP, true);
        });
        assert!(
            redistributed.is_err(),
            "forearm redistribution must fail the no-engage half of the claim"
        );
        // The healthy saturation case still passes.
        assert_skierg_twist_saturates_at_keep(SKI_WRIST_TWIST_KEEP, false);
    }

    #[test]
    fn requested_twist_stays_continuous_and_engages_the_budgets() {
        // The unclamped demand distinguishes a saturating budget from an
        // upstream frame bug: the demand must evolve continuously (a ±2π
        // wrap or sign flip would read as a ~180°+ adjacent jump) and must
        // actually exceed the budgets somewhere (otherwise the clamp path
        // is untested); where nothing redistributes, kept == requested.
        //
        // Continuity is asserted at 2000 samples/cycle, not 40: the
        // skierg's late-press pronation demand against the fixed plant
        // (Phase 7.5) passes through ±π and is continuous on the circle,
        // but its scalar representation wraps there — a coarse grid that
        // straddles the wrap reads a ~1.9 rad "jump" that is really two
        // ~0.04 rad dense steps plus the ±π wrap. Comparing circularly at
        // density makes the assert measure continuity, not sampling.
        use crate::replay::grip::{grip_frames, warped_cycle};
        let athlete = vendored();
        let solver = PoseSolver::new(&athlete).expect("plan");
        const STEPS: usize = 2000;
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let clip = athlete.clip_for(sport_name(sport)).expect("clip");
            let mut previous: Option<f64> = None;
            let mut prev_elbow: Option<([f64; 3], [f64; 3], [f64; 4])> = None;
            let mut engaged = false;
            let mut max_abs_requested = 0.0f64;
            for step in 0..STEPS {
                let step_f = step as f64;
                let phase = step_f / STEPS as f64 * std::f64::consts::TAU;
                let stroke = fallback_stroke_pose(sport, phase, 30.0);
                let rig = solve_rig_pose(sport, &stroke, step_f * 3.0, false);
                let targets = rig_targets(&rig);
                let frames = grip_frames(
                    sport,
                    &rig,
                    targets.poles,
                    warped_cycle(stroke.warped_phase),
                );
                let clip_time = clip_fraction(
                    stroke.cycle_frac,
                    stroke.phase,
                    stroke.drive_frac,
                    clip.drive_end,
                ) * f64::from(clip.duration);
                let posed = solver.pose(
                    &athlete,
                    sport,
                    clip,
                    clip_time,
                    &targets.contacts,
                    &frames,
                    targets.oar,
                );
                for hand in [0, 1] {
                    let metrics = posed.wrist[hand];
                    assert!(
                        (metrics.requested_twist - (metrics.twist + metrics.forearm_twist)).abs()
                            < 1e-9,
                        "{sport:?} step {step} hand {hand}: demand must split exactly"
                    );
                    if metrics.forearm_twist.abs() > 1e-9 {
                        engaged = true;
                    } else {
                        assert!(
                            (metrics.requested_twist - metrics.twist).abs() < 1e-9,
                            "{sport:?} step {step} hand {hand}: unclamped demand passes through"
                        );
                    }
                }
                let requested = posed.wrist[0].requested_twist;
                max_abs_requested = max_abs_requested.max(requested.abs());
                max_abs_requested = max_abs_requested.max(posed.wrist[1].requested_twist.abs());
                // rendered hand + elbow world continuity, recomputed from the
                // returned locals through the same FK the Workspace runs:
                // root-local translation is world, every child adds its local
                // translation rotated by the accumulated parent rotation.
                let hand_fk = {
                    let b0 = solver.bindings[0];
                    let mut chain = vec![];
                    let mut cur = Some(b0.terminal);
                    while let Some(j) = cur {
                        chain.push(j);
                        cur = solver_parent(&solver, j);
                    }
                    chain.reverse();
                    let mut pos = [0.0f64; 3];
                    let mut rot = [0.0f64, 0.0, 0.0, 1.0];
                    for &j in &chain {
                        let l = &posed.locals[j];
                        let t = crate::replay::equipment::rotate_vec(rot, l.translation);
                        pos = [pos[0] + t[0], pos[1] + t[1], pos[2] + t[2]];
                        rot = crate::replay::equipment::quat_mul(rot, l.rotation);
                    }
                    let elbow = {
                        // position at the lower joint: accumulate through the
                        // chain minus the terminal's local contribution
                        let mut p = [0.0f64; 3];
                        let mut r = [0.0f64, 0.0, 0.0, 1.0];
                        for &j in &chain[..chain.len() - 1] {
                            let l = &posed.locals[j];
                            let t = crate::replay::equipment::rotate_vec(r, l.translation);
                            p = [p[0] + t[0], p[1] + t[1], p[2] + t[2]];
                            r = crate::replay::equipment::quat_mul(r, l.rotation);
                        }
                        p
                    };
                    (pos, elbow, rot)
                };
                let (hand_w, elbow_w, hand_q) = hand_fk;
                if let Some((last_e, last_h, last_q)) = prev_elbow {
                    let d = length(sub(elbow_w, last_e));
                    let dh = length(sub(hand_w, last_h));
                    let dq = {
                        let q = quat_mul(hand_q, conjugate(last_q));
                        2.0 * q[3].acos().clamp(-1.0, 1.0)
                    };
                    // Web-class continuity at this sampling density: the
                    // fastest legitimate motion is the press ramp (measured
                    // peak 0.0101 m/step at cyc ≈ 0.19, a smooth monotonic
                    // run — verified by tracing neighbors before trusting
                    // this bound), while the two-bone branch flip the Phase
                    // 7.5 rewrite removed measured 0.05–0.31 m *between
                    // sub-mm steps*. 0.02 separates both classes ≥2×.
                    assert!(
                        d < 0.02,
                        "{sport:?} step {step}: elbow jumps {d:.4} (hand {dh:.4})"
                    );
                    // The hand carries two inherited web discontinuities,
                    // both ±π wraps of an atan2 refinement angle — the web's
                    // own comment names the class ("the same ±π singularity
                    // as the rowing feather … wrap-guards at π") but the
                    // wrist refinements themselves have no guard:
                    //
                    // 1. TILT wrap (refineGripTiltForWrist), late press:
                    //    the port snaps 1.299 rad at cyc 0.2645, the web
                    //    driven identically snaps 1.299 rad at cyc 0.2635
                    //    (tools/web-tilt-probe.mjs: qjump 1.2991).
                    // 2. SPIN wrap (refineGripSpinForWrist), mid flight:
                    //    both sweep the same monotone −π approach through
                    //    the carried-pole window; the port crosses at cyc
                    //    0.6885 (1.448 rad), the web at cyc 0.7080
                    //    (qjump 1.0961) — the ~0.02 cycle offset is the
                    //    small forearm-vs-shaft geometry difference between
                    //    the two solvers, not a different phenomenon.
                    //
                    // Parity, not defect: the port must reproduce the
                    // web's snaps, not smooth past them. The position jump
                    // differs by architecture, not by law — the web's
                    // procedural path sets `arm.hand.position` directly
                    // from the pole solve (its measured hand stays smooth
                    // at 0.0069 m/step through its own wrap), while the
                    // port — like the web's V4 hero chain, which the Node
                    // harness cannot drive — solves the wrist as
                    // `target − R_hand·offset`, so the π flip of the hand
                    // frame couples into the contact-chasing position
                    // (measured 0.110 m ≈ 2·|offset⊥|·sin(max_spin), the
                    // oriented offset swinging with the frame). Each wrap
                    // is one isolated step inside its window, never two.
                    // Continuity uses the same tight TOL everywhere; angular
                    // comparison is wrap-aware so a ±π singularity is not
                    // mistaken for a step skip.
                    const POSITION_TOL: f64 = 0.02;
                    const ORIENTATION_TOL: f64 = 0.35;
                    assert!(
                        dh < POSITION_TOL,
                        "{sport:?} step {step}: hand jumps {dh:.4} \
                         cyc={} prev={last_h:?} curr={hand_w:?} \
                         wrap_signed(dh)={} TOL={POSITION_TOL}",
                        stroke.cycle_frac,
                        wrap_signed(dh)
                    );
                    assert!(
                        wrap_signed(dq).abs() < ORIENTATION_TOL,
                        "{sport:?} step {step}: hand orientation jumps {dq:.4}"
                    );
                }
                prev_elbow = Some((elbow_w, hand_w, hand_q));
                if let Some(last) = previous {
                    let mut delta = (requested - last).abs();
                    if delta > std::f64::consts::PI {
                        delta = 2.0 * std::f64::consts::PI - delta;
                    }
                    assert!(
                        delta < std::f64::consts::PI / 2.0,
                        "{sport:?} step {step}: twist demand jumps {delta}"
                    );
                }
                previous = Some(requested);
            }
            // Post-Phase-7.5 invariants, per sport:
            // - The rower's sweep/feather grip still drives wrist demands far
            //   beyond the 75° budget, so its clamp path must engage.
            // - The skierg's rewritten pass structure (fixed plant + measured
            //   segments + the web's per-phase bend hint) drives the
            //   pre-clamp demand to *saturate* at the 30° keep budget
            //   (measured max |requested_twist| = π/6 + ~1.4e-15 at step
            //   222) — not "inside with headroom". Before the rewrite,
            //   branch-flip defects inflated the demand past the budget (a
            //   recorded 1.74 rad snap); the clamp engaging there was
            //   measuring the defect, not coverage. The flat-wrist law
            //   (`refine_grip_spin_for_wrist` + budgets) is separately
            //   pinned by core's unit tests, so the clamp path stays tested.
            // - The bike's static hood frame never engages — its wrist is
            //   frozen by design.
            match sport {
                Sport::Rower => {
                    assert!(engaged, "{sport:?}: the budget clamp never engaged");
                }
                Sport::Skierg => {
                    assert_skierg_twist_saturates_at_keep(max_abs_requested, engaged);
                }
                Sport::Bike => {
                    assert!(
                        !engaged,
                        "{sport:?}: the static grip must stay inside its keep budget"
                    );
                }
            }
        }
    }

    #[test]
    fn seam_split_preserves_the_forearm_world_and_halves_the_excess() {
        use rowplay_core::replay::wrist::SKI_PRONATION_SHOULDER_SHARE;
        // Synthetic arm: root → upper → middle → terminal, each 0.3 along
        // −Y, identity rest rotations.
        let local = |y: f64| LocalTransform {
            translation: [0.0, y, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0, 1.0, 1.0],
        };
        let parent: Vec<Option<usize>> = vec![None, Some(0), Some(1), Some(2)];
        let order = vec![0, 1, 2, 3];
        let binding = Binding {
            upper: 1,
            lower: 2,
            terminal: 3,
            offset: [0.0, 0.0, 0.0],
            bend_hint: [0.0, 0.0, 1.0],
        };
        // Twist the forearm 0.8 rad about its long axis past the snapshot.
        let snapshot = [0.0, 0.0, 0.0, 1.0];
        let twisted = axis_angle([0.0, 1.0, 0.0], 0.8);
        let mut work = Workspace::new(
            vec![local(0.0), local(-0.3), local(-0.3), local(-0.3)],
            &parent,
            &order,
        );
        work.locals[2].rotation = twisted;
        work.recompute_subtree(2);
        let middle_world_before = work.world_rot[2];
        let roll = work.split_seam_excess(&binding, snapshot, [0.0, 1.0, 0.0]);
        assert!(
            (roll - 0.8 * SKI_PRONATION_SHOULDER_SHARE).abs() < 1e-12,
            "{roll}"
        );
        // The forearm world (and therefore every descendant) is bit-exact.
        for (axis, expected) in middle_world_before.iter().enumerate() {
            assert!((work.world_rot[2][axis] - expected).abs() < 1e-12, "{axis}");
        }
        // The humerus visibly carries the roll: its world orientation moved
        // about its own (downward, elbow-minus-shoulder) long axis by
        // exactly the roll.
        let upper_world_after = work.world_rot[1];
        let expected = axis_angle([0.0, -1.0, 0.0], roll);
        for axis in 0..4 {
            assert!(
                (upper_world_after[axis] - expected[axis]).abs() < 1e-12,
                "{upper_world_after:?}"
            );
        }
        // Below-threshold excess is a no-op returning 0.
        let mut still = Workspace::new(
            vec![local(0.0), local(-0.3), local(-0.3), local(-0.3)],
            &parent,
            &order,
        );
        assert_eq!(
            still.split_seam_excess(&binding, snapshot, [0.0, 1.0, 0.0]),
            0.0
        );
        // A degenerate humerus axis (shoulder == elbow) is a no-op too.
        let mut collapsed = Workspace::new(
            vec![local(0.0), local(-0.3), local(0.0), local(-0.3)],
            &parent,
            &order,
        );
        collapsed.locals[2].rotation = twisted;
        collapsed.recompute_subtree(2);
        assert_eq!(
            collapsed.split_seam_excess(&binding, snapshot, [0.0, 1.0, 0.0]),
            0.0
        );
    }

    #[test]
    fn ski_shoulder_share_engages_on_the_water_and_rests_on_land() {
        // Pose-level: the share is SkiErg-only and engages where the seam
        // demand runs hot.
        let athlete = vendored();
        let solver = PoseSolver::new(&athlete).expect("plan");
        use crate::replay::grip::{grip_frames, warped_cycle};
        let poser = |sport: Sport, step: u32| -> Posed {
            let name = match sport {
                Sport::Rower => "rower",
                Sport::Skierg => "skierg",
                Sport::Bike => "bike",
            };
            let clip = athlete.clip_for(name).expect("clip");
            let phase = f64::from(step) / 40.0 * std::f64::consts::TAU;
            let stroke = fallback_stroke_pose(sport, phase, 30.0);
            let rig = solve_rig_pose(sport, &stroke, f64::from(step) * 3.0, false);
            let targets = rig_targets(&rig);
            let frames = grip_frames(
                sport,
                &rig,
                targets.poles,
                warped_cycle(stroke.warped_phase),
            );
            let clip_time = clip_fraction(
                stroke.cycle_frac,
                stroke.phase,
                stroke.drive_frac,
                clip.drive_end,
            ) * f64::from(clip.duration);
            solver.pose(
                &athlete,
                sport,
                clip,
                clip_time,
                &targets.contacts,
                &frames,
                targets.oar,
            )
        };
        for sport in [Sport::Rower, Sport::Bike] {
            for step in (0..40).step_by(5) {
                let posed = poser(sport, step);
                assert_eq!(posed.wrist[0].humerus_roll, 0.0);
                assert_eq!(posed.wrist[1].humerus_roll, 0.0);
            }
        }
        let mut engaged = false;
        for step in 0..40 {
            let posed = poser(Sport::Skierg, step);
            for hand in [0, 1] {
                let roll = posed.wrist[hand].humerus_roll;
                assert!(roll.is_finite());
                if roll.abs() > 1e-9 {
                    engaged = true;
                }
            }
        }
        assert!(engaged, "the shoulder share never engaged");
    }

    #[test]
    fn a_broken_contract_chain_is_a_named_error() {
        let mut athlete = vendored();
        athlete.contacts[0].0 = "v4Hips".to_owned();
        let error = PoseSolver::new(&athlete).expect_err("hips have no two ancestors");
        assert!(error.to_string().contains("left-hand"), "{error}");
    }
}
