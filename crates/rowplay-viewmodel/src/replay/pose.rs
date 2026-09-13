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
use rowplay_core::replay::rig_pose::{BikeErgRigPose, RowerRigPose, SkiErgRigPose, SportRigPose};
use rowplay_core::replay::two_bone::{
    add, cross, dot, length, scale, solve_rigid_contact3d, solve3d, sub,
};

use super::athlete::{AssetError, Clip, LocalTransform, V4Athlete};
use super::frame;

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
    /// The basket point.
    pub basket: [f64; 3],
}

/// The rig's targets plus the ski poles when the sport has them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigTargets {
    /// The contact targets.
    pub contacts: ContactTargets,
    /// Left and right poles (SkiErg only).
    pub poles: Option<[PolePlacement; 2]>,
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

fn rower_targets(rig: &RowerRigPose) -> RigTargets {
    use geometry::{ROW_FOOT, ROW_GRIP_DROP, ROW_INBOARD_CONTACT, ROW_OARLOCK, ROW_PELVIS};
    let hand = |side: f64| -> [f64; 3] {
        // Each inboard grip rides its own rigid circle: yaw about Y by the
        // sweep, roll about Z by the feather, both mirrored per side.
        let yaw = axis_angle([0.0, 1.0, 0.0], rig.oar_sweep * side);
        let roll = axis_angle([0.0, 0.0, 1.0], -rig.oar_feather * side);
        let orientation = quat_mul(roll, yaw);
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
        poles: None,
    }
}

fn skierg_targets(rig: &SkiErgRigPose) -> RigTargets {
    use geometry::{
        SKI_CONTACT_Y, SKI_FOOT, SKI_HAND_LATERAL_MARGIN, SKI_MAX_REACH, SKI_PELVIS_COMPRESSION,
        SKI_PELVIS_Y, SKI_PELVIS_Z, SKI_PLANT_LATERAL, SKI_POLE_LENGTH, SKI_SHOULDER_HALF_WIDTH,
    };
    let place = |side: f64| -> PolePlacement {
        // The preferred hand rides the web's shoulder-arc path; free-flight
        // baskets swing behind the carried hand at an exact pole length and
        // the contact channel rotates that toward the course-space plant.
        let desired_hand = [
            side * (SKI_SHOULDER_HALF_WIDTH + SKI_HAND_LATERAL_MARGIN),
            rig.preferred_hand_y,
            rig.preferred_hand_z,
        ];
        let carried = normalised_or(
            [
                side * 0.12,
                -rig.pole_rotation.cos().max(0.18),
                rig.pole_rotation.sin(),
            ],
            [0.0, -1.0, 0.0],
        );
        let free_basket = add(desired_hand, scale(carried, SKI_POLE_LENGTH));
        let plant_basket = [side * SKI_PLANT_LATERAL, SKI_CONTACT_Y, rig.plant_basket_z];
        let blended = add(
            free_basket,
            scale(
                sub(plant_basket, free_basket),
                rig.pole_contact.clamp(0.0, 1.0),
            ),
        );
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
            basket: blended,
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
}

#[derive(Debug, Clone, Copy)]
struct Binding {
    upper: usize,
    lower: usize,
    terminal: usize,
    offset: [f64; 3],
    bend_hint: [f64; 3],
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
}

impl PoseSolver {
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
        })
    }

    /// Pose the athlete: sample `clip` at the stroke's clip time, align the
    /// pelvis and close the four contacts (Studio `solve`, web `constrain`).
    #[must_use]
    pub fn pose(
        &self,
        athlete: &V4Athlete,
        sport: Sport,
        clip: &Clip,
        clip_time: f64,
        targets: &ContactTargets,
    ) -> Posed {
        let mut work = Workspace::new(athlete.sample(clip, clip_time), &self.parent, &self.order);

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

        let mut effective = *targets;
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
            work.solve_limb(&binding, solve_target);
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
            locals: work.locals,
            residuals,
        }
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
        let root = self.world_pos[binding.upper];
        let joint = self.world_pos[binding.lower];
        let contact = self.point(binding.offset, binding.terminal);
        let first = length(sub(joint, root));
        let second = length(sub(contact, joint));
        if !first.is_finite() || !second.is_finite() || first <= 1e-5 || second <= 1e-5 {
            return; // degenerate limb: keep the clip pose
        }
        let solution = solve3d(root, target, first, second, binding.bend_hint);
        self.aim_joint(binding.upper, root, joint, solution.joint);
        let new_joint = self.world_pos[binding.lower];
        let current_contact = self.point(binding.offset, binding.terminal);
        self.aim_joint(binding.lower, new_joint, current_contact, solution.end);
        // The terminal keeps the clip's authored wrist/ankle orientation.
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
                    (length(sub(pole.basket, pole.root)) - 1.37).abs() < 1e-9,
                    "rigid pole"
                );
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
            let mut worst = Residuals::default();
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
                worst.left_hand = worst.left_hand.max(r.left_hand);
                worst.right_hand = worst.right_hand.max(r.right_hand);
                worst.left_foot = worst.left_foot.max(r.left_foot);
                worst.right_foot = worst.right_foot.max(r.right_foot);
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
            eprintln!("{sport:?} worst residuals: {worst:?}");
        }
    }

    #[test]
    fn a_broken_contract_chain_is_a_named_error() {
        let mut athlete = vendored();
        athlete.contacts[0].0 = "v4Hips".to_owned();
        let error = PoseSolver::new(&athlete).expect_err("hips have no two ancestors");
        assert!(error.to_string().contains("left-hand"), "{error}");
    }
}
