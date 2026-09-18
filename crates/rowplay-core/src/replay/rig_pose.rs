// SPDX-License-Identifier: GPL-3.0-or-later
//! Sport rig poses: where the seat, handle, oars, poles, pedals and wheels
//! are for a stroke pose, plus the athlete's joint angles (Studio
//! `ReplayRigPose.swift`, `ReplayRigPoseSolver`).
//!
//! The web renderer derives the same quantities inside its procedural
//! per-sport avatars (`renderer3dRowAvatar.ts`, `renderer3dSkiAvatar.ts`,
//! `renderer3dBikeAvatar.ts`); Studio's solver is its documented
//! renderer-neutral port and the source of the calibration ranges below.
//! All phase sequencing comes from [`super::motion_graph`]; these are static
//! rig-space ranges, not another movement model. Every output is finite and
//! bounded; angles are radians, lengths metres, coordinates rig-root
//! relative (ground at `y = 0`, travel along `+z`).

use crate::models::Sport;
use crate::replay::motion_graph::{
    SKI_POLE_APPROACH_START_CYCLE, SKI_POLE_OFF_CYCLE, sample_bike_motion_graph,
    sample_rower_motion_graph, sample_skier_motion_graph,
};
use crate::replay::stroke_model::StrokePose;

/// Common torso / head / limb joint targets shared by all sports (Studio
/// `ReplayAthleteJointPose`). Positive rotations follow the right-hand rule
/// around the joint's local axis.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AthleteJointPose {
    /// Torso lean forward (positive) / backward (negative) from vertical.
    pub torso_lean: f64,
    /// Torso lateral tilt (positive = right).
    pub torso_tilt: f64,
    /// Head pitch (positive = look down).
    pub head_pitch: f64,
    /// Left shoulder flexion (positive = arm forward).
    pub shoulder_flex_l: f64,
    /// Right shoulder flexion.
    pub shoulder_flex_r: f64,
    /// Left elbow flexion (positive = bend).
    pub elbow_flex_l: f64,
    /// Right elbow flexion.
    pub elbow_flex_r: f64,
    /// Left hip flexion (positive = knee toward chest).
    pub hip_flex_l: f64,
    /// Right hip flexion.
    pub hip_flex_r: f64,
    /// Left knee flexion (positive = bend).
    pub knee_flex_l: f64,
    /// Right knee flexion.
    pub knee_flex_r: f64,
    /// Left ankle dorsiflexion (positive = toes up).
    pub ankle_dorsi_l: f64,
    /// Right ankle dorsiflexion.
    pub ankle_dorsi_r: f64,
}

impl AthleteJointPose {
    /// Neutral rest pose with every joint at zero.
    pub const NEUTRAL: AthleteJointPose = AthleteJointPose {
        torso_lean: 0.0,
        torso_tilt: 0.0,
        head_pitch: 0.0,
        shoulder_flex_l: 0.0,
        shoulder_flex_r: 0.0,
        elbow_flex_l: 0.0,
        elbow_flex_r: 0.0,
        hip_flex_l: 0.0,
        hip_flex_r: 0.0,
        knee_flex_l: 0.0,
        knee_flex_r: 0.0,
        ankle_dorsi_l: 0.0,
        ankle_dorsi_r: 0.0,
    };
}

/// RowErg rig pose (Studio `ReplayRowerRigPose`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowerRigPose {
    /// Common athlete joint angles.
    pub joints: AthleteJointPose,
    /// Seat Z offset from neutral (negative = toward the stern / catch).
    pub seat_z: f64,
    /// Handle height.
    pub handle_y: f64,
    /// Handle forward/back position.
    pub handle_z: f64,
    /// Handle rotation (recovery feather).
    pub handle_rot_x: f64,
    /// Oar sweep angle (positive = toward the bow).
    pub oar_sweep: f64,
    /// Oar feather angle (Z rotation for blade bury / feather).
    pub oar_feather: f64,
    /// The motion graph's `blade_feather` contact channel, 0..1 (the web
    /// squares the spoon with `(1 − bladeFeather) · π/2` about the shaft).
    pub blade_feather: f64,
    /// The motion graph's `arm_draw` channel, 0..1 — the arm-authority
    /// velocity profile. The web schedules the requested shoulder→wrist reach
    /// from it and solves the oar yaw to meet that reach
    /// (`renderer3dRowAvatar.ts` `placeArms` + `rowRig.solveRowerOarYaw`), so
    /// the pose layer needs it to compose the solve; `oar_sweep` is only the
    /// solve's `preferredYaw` branch fallback.
    pub arm_draw: f64,
}

/// SkiErg rig pose (Studio `ReplaySkiErgRigPose`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkiErgRigPose {
    /// Common athlete joint angles.
    pub joints: AthleteJointPose,
    /// Hip compression, 0 = tall, 1 = fully compressed.
    pub hip_compression: f64,
    /// Pelvis carry height in rig root space (web
    /// `renderer3dSkiAvatar.animate` `upper.position.y` — the pinned
    /// `SKI_STANDING_PELVIS_Y - kneeFlex·0.11 + rebound·0.045`).
    pub pelvis_y: f64,
    /// Pelvis forward carry (`upper.position.z` = `hipHinge · 0.055`).
    pub pelvis_z: f64,
    /// Pelvis local counter-tilt in the upper frame (web `hips.rotation.x`
    /// = `-hipHinge · SKI_PELVIS_COUNTER_TILT 0.14`). Applied on top of
    /// the torso hinge, this keeps the pelvis less pitched forward than
    /// the torso — the same connected-spine cue the web comments name.
    pub hip_counter_tilt: f64,
    /// Pole rotation angle.
    pub pole_rotation: f64,
    /// The carried pole's free-flight attitude, radians from horizontal
    /// (web `poleAngle = degToRad(80 − poleSweep·57)`): steep ~80° at the
    /// plant, ~23° at pole-off. Drives the free-tip vertical/horizontal
    /// split in `pose::skierg_targets`, replacing the Studio
    /// `pole_rotation` approximation now that the fixture pins real pole
    /// tips.
    pub pole_attitude: f64,
    /// 0..1 closure of the basket-to-course contact channel.
    pub pole_contact: f64,
    /// The pole's free-flight attitude term (web `poleLift`): snow
    /// clearance above the plant height, fading in after pole-off.
    pub pole_lift: f64,
    /// 0..1 release: 1 while the pole is in free flight outside the early
    /// release fade (web `poleFlight`).
    pub pole_flight: f64,
    /// Raw `poleSweep` motion channel (0..1), for the per-phase arm-bend
    /// hint (web `solveSkierElbowDirection`).
    pub pole_sweep: f64,
    /// Raw `elbowLoad` motion channel (0..1), for the hint's lateral floor.
    pub elbow_load: f64,
    /// Cycle fraction within the current stroke, for the hint's press /
    /// recovery branch (web `solveSkierElbowDirection` keys on `cycle`).
    pub cycle_frac: f64,
    /// 1 inside the early-release window after pole-off (cyc in
    /// `[SKI_POLE_OFF_CYCLE, SKI_POLE_OFF_CYCLE + 0.05)`) where the dead
    /// plant's attitude influence fades out; 1 elsewhere. Multiplies the
    /// direction blend so the carried shaft releases from the retired
    /// plant smoothly (web `releaseFade`).
    pub release_fade: f64,
    /// Yaw of the plant anchor since the catch, radians (web
    /// `setPlantTipWorld`'s `courseTurn`): the plant is fixed in course
    /// space at the catch, so in rig-local its lateral/forward offset
    /// rotates by this about the rig origin. ~0.013 rad per cycle at 8 m
    /// strokes on the 1 km loop (the fixture's 0.240 → 0.235 `z` drift).
    pub plant_course_turn: f64,
    /// Forward coordinate of the retired course-anchored plant model
    /// (pre-Phase-7.5). Superseded by [`Self::plant_course_turn`] for the
    /// rendered plant; kept only because the Studio contract tests read it.
    pub plant_basket_z: f64,
    /// Preferred hand height: the web's shoulder-arc hand path evaluated in
    /// the hinging torso frame (the V4 clip's hand keys follow it).
    pub preferred_hand_y: f64,
    /// Preferred hand forward offset (see `preferred_hand_y`).
    pub preferred_hand_z: f64,
    /// Shoulder height in the same hinging torso frame.
    pub shoulder_y: f64,
    /// Shoulder forward offset.
    pub shoulder_z: f64,
}

impl SkiErgRigPose {
    /// The plant anchor's forward coordinate in rig-local, accounting for
    /// the course-turn rotation since plant (web `setPlantTipWorld`:
    /// `z = localX·sin(−courseTurn) + localZ·cos(−courseTurn)` with
    /// `localX = side·polePlantLateral`, `localZ = polePlantForwardOffset`).
    #[must_use]
    pub fn plant_course_turn_z(&self, side: f64) -> f64 {
        side * ski_proportions::SKI_PLANT_LATERAL * self.plant_course_turn.sin()
            + ski_proportions::POLE_PLANT_FORWARD_OFFSET * self.plant_course_turn.cos()
    }

    /// The plant anchor's lateral coordinate under the same rotation
    /// (`x = localX·cos(−courseTurn) − localZ·sin(−courseTurn)`).
    #[must_use]
    pub fn plant_course_turn_x(&self, side: f64) -> f64 {
        side * ski_proportions::SKI_PLANT_LATERAL * self.plant_course_turn.cos()
            - ski_proportions::POLE_PLANT_FORWARD_OFFSET * self.plant_course_turn.sin()
    }
}

/// A pedal position relative to the bottom bracket (Studio
/// `ReplayPedalPosition`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PedalPosition {
    /// Vertical offset.
    pub y: f64,
    /// Forward offset.
    pub z: f64,
}

/// BikeErg rig pose (Studio `ReplayBikeErgRigPose`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BikeErgRigPose {
    /// Common athlete joint angles.
    pub joints: AthleteJointPose,
    /// Crank angle, continuous, drives the pedal positions.
    pub crank_angle: f64,
    /// Wheel rotation, continuous.
    pub wheel_angle: f64,
    /// Left pedal relative to the bottom bracket.
    pub pedal_pos_l: PedalPosition,
    /// Right pedal relative to the bottom bracket.
    pub pedal_pos_r: PedalPosition,
    /// Rider lateral sway angle.
    pub rider_sway: f64,
}

/// The sport-specific rig pose (Studio `ReplaySportRigPose`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SportRigPose {
    /// RowErg.
    Rower(RowerRigPose),
    /// SkiErg.
    SkiErg(SkiErgRigPose),
    /// BikeErg.
    Bike(BikeErgRigPose),
}

/// Studio `ReplaySkiGripContract.AthleteProportions` — the reach the ski
/// hand path is fitted inside, and where the pole plants ahead of the feet.
pub mod ski_proportions {
    /// Upper arm length, metres.
    pub const UPPER_ARM_LENGTH: f64 = 0.49;
    /// Forearm length, metres.
    pub const FOREARM_LENGTH: f64 = 0.47;
    /// Forward offset of the planted basket from the rig root, metres.
    pub const POLE_PLANT_FORWARD_OFFSET: f64 = 0.24;
    /// Lateral offset of the planted basket, just outside the ski (web
    /// `SKI_ATHLETE_PROPORTIONS.polePlantLateralOffset`, the fixture's
    /// `poleTipLeft.x ≈ −0.46`).
    pub const SKI_PLANT_LATERAL: f64 = 0.46;
}

/// Studio `ReplayBikeGripContract` frame geometry (real road geometry).
pub mod bike_geometry {
    /// Wheel rotation radius, metres (web `bikeRig.js` `WHEEL_RADIUS`): the
    /// wheel rolls `distance / WHEEL_RADIUS`, as the web avatar rolls its
    /// wheels.
    pub const WHEEL_RADIUS: f64 = 0.31;
    /// Tyre tube radius, metres.
    pub const TYRE_TUBE: f64 = 0.025;
    /// Outer tyre radius; also the axle height, so the tread rests on `y = 0`
    /// (web `bikeWheelAxleY`). This is the *placement* contract — wheel
    /// rotation divides by [`WHEEL_RADIUS`], not by this.
    pub const AXLE_Y: f64 = WHEEL_RADIUS + TYRE_TUBE;
    /// Crank arm length for a rider this size, metres.
    pub const CRANK_RADIUS: f64 = 0.1725;
}

/// Solve the rig pose for `sport` at `pose` (Studio `ReplayRigPoseSolver.solve`).
///
/// `distance` is the cumulative distance in metres (the bike wheel rolls on
/// it); `reduce_motion` returns the sport's stable neutral pose instead.
#[must_use]
pub fn solve_rig_pose(
    sport: Sport,
    pose: &StrokePose,
    distance: f64,
    reduce_motion: bool,
) -> SportRigPose {
    if reduce_motion {
        return reduced_pose(sport);
    }
    match sport {
        Sport::Rower => SportRigPose::Rower(solve_rower(pose)),
        Sport::Skierg => SportRigPose::SkiErg(solve_skierg(pose)),
        Sport::Bike => SportRigPose::Bike(solve_bike(pose, distance)),
    }
}

fn solve_rower(pose: &StrokePose) -> RowerRigPose {
    let graph = sample_rower_motion_graph(pose);
    let legs = unit(graph.body.leg_extension.value);
    let torso = unit(graph.body.spine_hinge.value);
    let arms = unit(graph.body.arm_draw.value);
    let feather = unit(graph.contacts.blade_feather.value);

    // The web avatar's phase calibration (renderer3dRowAvatar.ts `animate`),
    // pinned sample-by-sample by the replay-row-phase-parity fixture: the
    // seat rides `pelvisTravel` from the catch z +0.26 — CLOSEST to the
    // fixed feet — through −0.44 of travel; the **oar** sweep and the roll's
    // handle-rise term ride `armDraw`, the arm-authority profile, from the
    // catch yaw +0.68 (grips ahead of the shoulders toward the stretcher) to
    // the draw's −0.80. The web passes
    // `equipmentHandleTravel = graph.body.armDraw.value` to `placeOars` and
    // warns that the aggregate handle channel "would include its leg
    // contribution and pull the grip through the knees and torso too early" —
    // during the leg drive the athlete slides away from a nearly stationary
    // catch handle, so the oar must stay at the catch yaw while `handleTravel`
    // is already rising. Studio's ranges here (seat −0.20 + legs·0.40, sweep
    // −0.58 + handle·1.16, roll −0.06 + feather·0.34) are phase-inverted
    // against the web — its seat starts farthest from the feet and its catch
    // grips sit behind the torso; see the divergence row in
    // docs/source-map.md.
    let pelvis_travel = unit(graph.body.pelvis_travel.value);
    // `handleTravel` is retained only for Studio's unconsumed `handle_*`
    // joint outputs below; nothing the scene renders uses it.
    let handle = unit(graph.body.handle_travel.value);
    let draw = unit(graph.body.arm_draw.value);
    let blade_water = unit(graph.contacts.blade_water.value);
    let seat_z = 0.26 + pelvis_travel * -0.44;
    let oar_sweep = 0.68 + draw * (-0.8 - 0.68);
    let oar_feather = blade_water * 0.28 + draw * 0.04;

    // The remaining channels are Studio's joint-pose outputs; the Qt port
    // poses the athlete from the V4 clip plus the contact targets above, so
    // nothing consumes them (kept for the Studio surface — see source-map).
    let handle_y = 0.62 - handle * 0.05 + feather * 0.03;
    let handle_z = 0.66 - handle * 0.22;
    let handle_rot_x = feather * 0.20;
    let torso_lean = -0.28 + torso * 0.46;
    let shoulder_flex = -0.22 + arms * 0.34;
    let elbow_flex = arms * 0.42;
    let hip_flex = (1.0 - legs) * 0.48;
    let knee_flex = (1.0 - legs) * 0.82;
    let ankle_dorsi = (1.0 - legs) * -0.15;

    let joints = AthleteJointPose {
        torso_lean: finite(torso_lean, 0.0),
        torso_tilt: 0.0,
        head_pitch: finite(-torso_lean * 0.3, 0.0),
        shoulder_flex_l: finite(shoulder_flex, 0.0),
        shoulder_flex_r: finite(shoulder_flex, 0.0),
        elbow_flex_l: finite(elbow_flex, 0.0),
        elbow_flex_r: finite(elbow_flex, 0.0),
        hip_flex_l: finite(hip_flex, 0.0),
        hip_flex_r: finite(hip_flex, 0.0),
        knee_flex_l: finite(knee_flex, 0.0),
        knee_flex_r: finite(knee_flex, 0.0),
        ankle_dorsi_l: finite(ankle_dorsi, 0.0),
        ankle_dorsi_r: finite(ankle_dorsi, 0.0),
    };

    RowerRigPose {
        joints,
        seat_z: finite(seat_z, -0.1),
        handle_y: finite(handle_y, 0.72),
        handle_z: finite(handle_z, 0.58),
        handle_rot_x: finite(handle_rot_x, 0.0),
        oar_sweep: finite(oar_sweep, 0.0),
        oar_feather: finite(oar_feather, -0.06),
        blade_feather: feather,
        arm_draw: draw,
    }
}

fn solve_skierg(pose: &StrokePose) -> SkiErgRigPose {
    let graph = sample_skier_motion_graph(pose);
    let press = unit(graph.body.arm_press.value);
    let hinge = unit(graph.body.pelvis_hinge.value);
    let knees = unit(graph.body.knee_flex.value);
    let elbow = unit(graph.body.elbow_load.value);
    let pole_sweep = unit(graph.body.pole_sweep.value);
    let arm_extension = unit(graph.body.arm_extension.value);
    let pole_contact = unit(graph.contacts.pole_plant.value);
    let hip_compression = unit(graph.body.torso_compression.value);
    // Web `renderer3dSkiAvatar.animate` (pinned by
    // `replay-rig-phase-parity.json` for skierg): the torso pitch is
    // `SKI_NEUTRAL_TORSO_PITCH 0.055 + hipHinge · SKI_TORSO_HINGE_RANGE 0.56`,
    // and the head counter-tilts locally by `-hipHinge · 0.38` so the gaze
    // stays down-course while the torso hinges. Studio's `0.18 + 0.55·hinge`
    // torso base and `torso_lean · 0.2` head derivation are op-for-op with
    // Studio's `ReplayRigPose` and phase-inverted / off-scale against the web
    // avatar the SkiErg scene actually mirrors — the audit's rig-phase
    // fixture caught them (docs/parity-coverage.md ranking 2).
    let torso_lean = 0.055 + hinge * 0.56;
    let head_local_pitch = -hinge * 0.38;
    let hip_counter_tilt = -hinge * 0.14;
    // `pose::skierg_targets` composes the pole carry as
    // `carried = normalize([side·0.12, -cos(θ).max(0.18), sin(θ)])` with
    // axes X=lateral, Y=up, Z=forward. `θ = 0` puts the pole straight down
    // (`carried.y = -1`); `θ = -π/2` puts it straight back
    // (`carried.z = -1`). The port's `pole_rotation` is therefore the
    // pole's angle-from-vertical (with a negative sign so `sin(θ)`'s
    // negativity carries the "back" direction directly).
    //
    // Web `renderer3dSkiAvatar.placePoleArms` (line 971) computes
    // `poleAngle = degToRad(80 - poleSweep · 57)` — the angle from
    // horizontal — with `desiredVertical = -sin(poleAngle) · POLE_LENGTH`
    // and `horizontal = |cos(poleAngle)| · POLE_LENGTH`. Normalising the
    // resulting `(courseRight·lateral·horizontal + courseForward·forward·horizontal, vertical)`
    // gives `(0.04, -0.985, -0.17)` at the reach (mostly down) and
    // `(0.20, -0.39, -0.90)` at the finish (mostly back).
    //
    // Studio's `-0.20 - poleSweep · 0.92` produces `(0.12, -0.98, -0.20)`
    // at the reach and `(0.12, -0.44, -0.90)` at the finish under the
    // port's composition — same direction, same phase as the web, within
    // 1–3° of the exact `poleAngle - π/2` reparameterisation (the earlier
    // "phase-inverted" reading was mine; Studio approximated the web here
    // in a different frame). Kept until the rig-phase fixture ships real
    // scene-graph oracle values for the pole shafts — the current
    // `poleShaftLeftRotation` samples are all zero because
    // `placePoleArms` early-returns on `!group.parent` and the generator
    // constructs the avatar detached (docs/parity-coverage.md).
    let pole_rotation = -0.20 - pole_sweep * 0.92;
    // Web `renderer3dSkiAvatar.placePoleArms` authors the carried pole's
    // attitude directly from the technique phase (its line ~971):
    // `poleAngle = degToRad(80 - poleSweep · 57)` measured from
    // horizontal — steep ~80° at the plant, shallowest ~23° at pole-off,
    // both on-snow measured. Supersedes the Studio approximation kept
    // "until the fixture ships real oracle values for the pole shafts" —
    // it now has them (`poleTipLeft/Right` in the rig-phase fixture).
    let pole_attitude = (80.0 - pole_sweep * 57.0).to_radians();
    let pole_lift = unit(graph.body.pole_lift.value);
    let pole_flight = unit(graph.body.pole_flight.value);
    // Web `releaseFade`: 1 − smoothstep(cyc, off, off+0.05) inside the
    // early-release window only; 1 elsewhere.
    let release_fade = if pose.cycle_frac > SKI_POLE_OFF_CYCLE
        && pose.cycle_frac < SKI_POLE_APPROACH_START_CYCLE
    {
        let t = (pose.cycle_frac - SKI_POLE_OFF_CYCLE) / 0.05;
        1.0 - smoothstep01(t)
    } else {
        1.0
    };
    // Web `setPlantTipWorld`: the plant is fixed at the catch in course
    // space; in rig-local it rotates with the course turn since plant —
    // `(distanceSincePlant / COURSE_LOOP_METERS) · τ`, distance measured
    // from the plant cycle (current, or next when the approach window
    // has begun). This replaces the retreating
    // `POLE_PLANT_FORWARD_OFFSET − distanceSincePlant` model whose basket
    // slid ~2 m rearward through every pull (the Phase 7.5 defect).
    let index = pose.index as f64;
    let plant_cycle = index
        + if pose.cycle_frac >= SKI_POLE_APPROACH_START_CYCLE {
            1.0
        } else {
            0.0
        };
    let current_cycle = index + pose.cycle_frac;
    let distance_since_plant = (current_cycle - plant_cycle) * pose.stroke_meters.max(0.0);
    let plant_course_turn = distance_since_plant / 1000.0 * std::f64::consts::TAU;
    let plant_basket_z = ski_proportions::POLE_PLANT_FORWARD_OFFSET - distance_since_plant;
    let shoulder_flex = 0.26 - press * 0.64;
    let elbow_flex = elbow * 0.55 + (1.0 - arm_extension) * 0.16;
    let leg_flex = knees * 0.24;

    // Web-parity preferred hand path (`renderer3dSkiAvatar.skiPreferredHand`):
    // a polar arc around the shoulder during pole contact and a Bezier return
    // during recovery, authored in the hinging torso frame.
    let rebound = unit(graph.accents.rebound.value);
    let max_arm_reach = ski_proportions::UPPER_ARM_LENGTH + ski_proportions::FOREARM_LENGTH - 0.02;
    let arm_reach = (0.72 - elbow * 0.28 + arm_extension * 0.08).min(max_arm_reach * 0.96);
    let arm_angle = 0.56 - pole_sweep * 2.56;
    let mut hand_local_y = 0.54 + arm_angle.sin() * arm_reach;
    let mut hand_local_z = 0.05 + arm_angle.cos() * arm_reach;
    if pose.cycle_frac > SKI_POLE_OFF_CYCLE {
        // Recovery return: hands come forward close to the body then lift to
        // the high reach. Endpoints reuse the polar formula at its boundary
        // sweeps so the hand-off and the cycle seam stay continuous.
        let t = 1.0 - unit(pole_sweep);
        let off_y = 0.54 + (0.56_f64 - 2.56).sin() * arm_reach;
        let off_z = 0.05 + (0.56_f64 - 2.56).cos() * arm_reach;
        let reach_y = 0.54 + 0.56_f64.sin() * arm_reach;
        let reach_z = 0.05 + 0.56_f64.cos() * arm_reach;
        hand_local_y = cubic_bezier(off_y, 0.1, 0.48, reach_y, t);
        hand_local_z = cubic_bezier(off_z, 0.2, 0.3, reach_z, t);
    }
    // Hinging torso frame: pelvis carry plus forward pitch; the shoulder
    // rides the same frame so the rigid pole contact stays inside the arm's
    // reach annulus the way the web does. Pelvis carry and torso pitch are
    // the same authored values the web `animate` writes to `upper.position`
    // and `upper.rotation.x`; `torso_lean` above is that pitch.
    let pelvis_carry_y = 0.735 - knees * 0.11 + rebound * 0.045;
    let pelvis_carry_z = hinge * 0.055;
    let in_root_frame = |y: f64, z: f64| -> (f64, f64) {
        (
            pelvis_carry_y + y * torso_lean.cos() - z * torso_lean.sin(),
            pelvis_carry_z + y * torso_lean.sin() + z * torso_lean.cos(),
        )
    };
    let preferred_hand = in_root_frame(hand_local_y, hand_local_z);
    let shoulder = in_root_frame(0.54, 0.05);

    let joints = AthleteJointPose {
        torso_lean: finite(torso_lean, 0.055),
        torso_tilt: 0.0,
        head_pitch: finite(head_local_pitch, 0.0),
        shoulder_flex_l: finite(shoulder_flex, 0.0),
        shoulder_flex_r: finite(shoulder_flex, 0.0),
        elbow_flex_l: finite(elbow_flex, 0.0),
        elbow_flex_r: finite(elbow_flex, 0.0),
        hip_flex_l: finite(leg_flex, 0.0),
        hip_flex_r: finite(leg_flex, 0.0),
        knee_flex_l: finite(-leg_flex * 0.75, 0.0),
        knee_flex_r: finite(-leg_flex * 0.75, 0.0),
        ankle_dorsi_l: 0.0,
        ankle_dorsi_r: 0.0,
    };

    SkiErgRigPose {
        joints,
        hip_compression: finite(hip_compression, 0.0),
        pelvis_y: finite(pelvis_carry_y, 0.735),
        pelvis_z: finite(pelvis_carry_z, 0.0),
        hip_counter_tilt: finite(hip_counter_tilt, 0.0),
        pole_rotation: finite(pole_rotation, -0.1),
        pole_attitude: finite(pole_attitude, 1.2),
        pole_contact: finite(pole_contact, 0.0),
        pole_lift: finite(pole_lift, 0.0),
        pole_flight: finite(pole_flight, 0.0),
        pole_sweep: finite(pole_sweep, 0.0),
        elbow_load: finite(elbow, 0.0),
        cycle_frac: finite(unit(pose.cycle_frac), 0.0),
        release_fade: finite(release_fade, 1.0),
        plant_course_turn: finite(plant_course_turn, 0.0),
        plant_basket_z: finite(plant_basket_z, 0.24),
        preferred_hand_y: finite(preferred_hand.0, 0.66),
        preferred_hand_z: finite(preferred_hand.1, 0.09),
        shoulder_y: finite(shoulder.0, 1.27),
        shoulder_z: finite(shoulder.1, 0.09),
    }
}

fn solve_bike(pose: &StrokePose, distance: f64) -> BikeErgRigPose {
    let graph = sample_bike_motion_graph(pose);
    let crank_angle = graph.crank.angle;
    // Web avatar calibration (renderer3dBikeAvatar.ts `placePedalLegs` /
    // wheel roll, pinned by the rig-phase parity fixture): the pedals ride
    // `-(r·cos, r·sin)` around the bottom bracket, and the wheel rolls on
    // the rim radius `WHEEL_RADIUS` (0.31), not the outer-tyre axle height
    // (0.335) — the previous divisor under-rotated the wheels ≈7.5% against
    // the distance travelled, and the un-negated pedals sat π around the
    // crank circle from the web's.
    let wheel_angle = finite(distance, 0.0) / bike_geometry::WHEEL_RADIUS;
    let crank_radius = bike_geometry::CRANK_RADIUS;
    let cos_crank = graph.left_pedal.rotation.cos;
    let sin_crank = graph.left_pedal.rotation.sin;
    let pedal_y_l = -crank_radius * cos_crank;
    let pedal_z_l = -crank_radius * sin_crank;
    let pedal_y_r = -pedal_y_l;
    let pedal_z_r = -pedal_z_l;
    let rider_sway = graph.body.torso_sway.value * 0.15;
    let thigh_angle_l = (pedal_z_l + 0.35).atan2(0.8 - pedal_y_l);
    let thigh_angle_r = (pedal_z_r + 0.35).atan2(0.8 - pedal_y_r);
    let left_knee = unit(graph.left_pedal.knee_lift.value);
    let right_knee = unit(graph.right_pedal.knee_lift.value);

    let joints = AthleteJointPose {
        torso_lean: finite(0.74 + graph.body.spine_lean.value, 0.74),
        torso_tilt: finite(rider_sway, 0.0),
        head_pitch: finite(0.1 + graph.body.head_stabilization.value, 0.1),
        shoulder_flex_l: finite(-0.3 + graph.body.shoulder_counter_rotation.value, -0.3),
        shoulder_flex_r: finite(-0.3 - graph.body.shoulder_counter_rotation.value, -0.3),
        elbow_flex_l: 0.4,
        elbow_flex_r: 0.4,
        hip_flex_l: finite(thigh_angle_l, 0.0),
        hip_flex_r: finite(thigh_angle_r, 0.0),
        knee_flex_l: finite(left_knee * 0.8, 0.0),
        knee_flex_r: finite(right_knee * 0.8, 0.0),
        ankle_dorsi_l: finite(graph.left_pedal.ankle_flex.value * 0.3, 0.0),
        ankle_dorsi_r: finite(graph.right_pedal.ankle_flex.value * 0.3, 0.0),
    };

    BikeErgRigPose {
        joints,
        crank_angle: finite(crank_angle, 0.0),
        wheel_angle: finite(wheel_angle, 0.0),
        pedal_pos_l: PedalPosition {
            y: finite(pedal_y_l, 0.0),
            z: finite(pedal_z_l, 0.0),
        },
        pedal_pos_r: PedalPosition {
            y: finite(pedal_y_r, 0.0),
            z: finite(pedal_z_r, 0.0),
        },
        rider_sway: finite(rider_sway, 0.0),
    }
}

/// The stable neutral pose per sport (Studio `reducedPose`).
#[must_use]
pub fn reduced_pose(sport: Sport) -> SportRigPose {
    match sport {
        Sport::Rower => SportRigPose::Rower(RowerRigPose {
            joints: AthleteJointPose::NEUTRAL,
            seat_z: -0.1,
            handle_y: 0.72,
            handle_z: 0.58,
            handle_rot_x: 0.0,
            oar_sweep: 0.0,
            oar_feather: -0.06,
            blade_feather: 0.0,
            arm_draw: 0.0,
        }),
        Sport::Skierg => SportRigPose::SkiErg(SkiErgRigPose {
            joints: AthleteJointPose {
                torso_lean: 0.055,
                hip_flex_l: 0.08,
                hip_flex_r: 0.08,
                knee_flex_l: -0.05,
                knee_flex_r: -0.05,
                ..AthleteJointPose::NEUTRAL
            },
            hip_compression: 0.0,
            pelvis_y: 0.735,
            pelvis_z: 0.0,
            hip_counter_tilt: 0.0,
            pole_rotation: -0.2,
            pole_attitude: (80.0_f64).to_radians(),
            pole_contact: 0.0,
            pole_lift: 0.0,
            pole_flight: 0.0,
            pole_sweep: 0.0,
            elbow_load: 0.0,
            cycle_frac: 0.0,
            release_fade: 1.0,
            plant_course_turn: 0.0,
            plant_basket_z: ski_proportions::POLE_PLANT_FORWARD_OFFSET,
            preferred_hand_y: 0.663,
            preferred_hand_z: 0.094,
            shoulder_y: 1.271,
            shoulder_z: 0.080,
        }),
        Sport::Bike => SportRigPose::Bike(BikeErgRigPose {
            joints: AthleteJointPose {
                torso_lean: 0.74,
                shoulder_flex_l: -0.3,
                shoulder_flex_r: -0.3,
                elbow_flex_l: 0.4,
                elbow_flex_r: 0.4,
                ..AthleteJointPose::NEUTRAL
            },
            crank_angle: 0.0,
            wheel_angle: 0.0,
            pedal_pos_l: PedalPosition {
                y: -bike_geometry::CRANK_RADIUS,
                z: 0.0,
            },
            pedal_pos_r: PedalPosition {
                y: bike_geometry::CRANK_RADIUS,
                z: 0.0,
            },
            rider_sway: 0.0,
        }),
    }
}

/// Clamp a channel value into `[0, 1]`, treating non-finite as 0.
fn unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// `v` when finite, otherwise `fallback`.
fn finite(v: f64, fallback: f64) -> f64 {
    if v.is_finite() { v } else { fallback }
}

/// Hermite smoothstep on `[0, 1]` (three.js `MathUtils.smoothstep(x, 0, 1)`),
/// clamped outside.
fn smoothstep01(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Scalar cubic Bezier through `p0` / `p3` with interior controls `p1` /
/// `p2`, evaluated at `t` in `[0, 1]` (the web ski-recovery curve, whose
/// value is bounded by its control points).
fn cubic_bezier(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let clamped = t.clamp(0.0, 1.0);
    let inverse = 1.0 - clamped;
    inverse * inverse * inverse * p0
        + 3.0 * inverse * inverse * clamped * p1
        + 3.0 * inverse * clamped * clamped * p2
        + clamped * clamped * clamped * p3
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::stroke_model::fallback_stroke_pose;

    fn poses(sport: Sport) -> Vec<StrokePose> {
        (0..=400)
            .map(|i| {
                fallback_stroke_pose(sport, f64::from(i) / 400.0 * std::f64::consts::TAU, 28.0)
            })
            .collect()
    }

    fn assert_joints_bounded(joints: &AthleteJointPose) {
        for value in [
            joints.torso_lean,
            joints.torso_tilt,
            joints.head_pitch,
            joints.shoulder_flex_l,
            joints.shoulder_flex_r,
            joints.elbow_flex_l,
            joints.elbow_flex_r,
            joints.hip_flex_l,
            joints.hip_flex_r,
            joints.knee_flex_l,
            joints.knee_flex_r,
            joints.ankle_dorsi_l,
            joints.ankle_dorsi_r,
        ] {
            assert!(
                value.is_finite() && value.abs() <= std::f64::consts::PI,
                "{value}"
            );
        }
    }

    #[test]
    fn rower_pose_stays_inside_its_calibration_ranges() {
        for pose in poses(Sport::Rower) {
            let SportRigPose::Rower(rig) = solve_rig_pose(Sport::Rower, &pose, 0.0, false) else {
                panic!("sport mismatch");
            };
            assert_joints_bounded(&rig.joints);
            // Web ranges (renderer3dRowAvatar.ts; the row-phase-parity
            // fixture pins them sample-by-sample): the catch seat sits
            // CLOSEST to the feet at +0.26, the catch sweep ahead of the
            // shoulders at +0.68.
            assert!((-0.18..=0.26).contains(&rig.seat_z), "seat {}", rig.seat_z);
            assert!(
                (0.57..=0.65).contains(&rig.handle_y),
                "handle y {}",
                rig.handle_y
            );
            assert!(
                (0.44..=0.66).contains(&rig.handle_z),
                "handle z {}",
                rig.handle_z
            );
            assert!((0.0..=0.20).contains(&rig.handle_rot_x));
            assert!(
                (-0.8..=0.68).contains(&rig.oar_sweep),
                "sweep {}",
                rig.oar_sweep
            );
            assert!((0.0..=0.32).contains(&rig.oar_feather));
            assert!((0.0..=1.0).contains(&rig.blade_feather));
            let graph = sample_rower_motion_graph(&pose);
            // The roll's handle-rise term rides armDraw, not handleTravel
            // (web `placeOars(equipmentHandleTravel = armDraw)`) — see the
            // channel note in `solve_rower`.
            let expected_feather = unit(graph.contacts.blade_water.value) * 0.28
                + unit(graph.body.arm_draw.value) * 0.04;
            assert!((rig.oar_feather - expected_feather).abs() < 1e-12);
            assert!((-0.28..=0.18).contains(&rig.joints.torso_lean));
            // Knee and hip flexion share the leg-extension cue: 0.82 : 0.48.
            if rig.joints.hip_flex_l > 1e-9 {
                let ratio = rig.joints.knee_flex_l / rig.joints.hip_flex_l;
                assert!((ratio - 0.82 / 0.48).abs() < 1e-9, "{ratio}");
            }
            assert_eq!(rig.joints.hip_flex_l, rig.joints.hip_flex_r);
            assert!((rig.joints.head_pitch + rig.joints.torso_lean * 0.3).abs() < 1e-12);
        }
    }

    #[test]
    fn rower_seat_leads_the_cycle_from_the_catch() {
        // The inversion guard: at the catch the seat is at its travel maximum
        // (closest to the feet) and the sweep at its catch yaw; at the finish
        // both have crossed to the far end.
        let catch = fallback_stroke_pose(Sport::Rower, 0.0, 28.0);
        let SportRigPose::Rower(rig) = solve_rig_pose(Sport::Rower, &catch, 0.0, false) else {
            panic!("sport mismatch");
        };
        assert!((rig.seat_z - 0.26).abs() < 1e-9, "seat {}", rig.seat_z);
        assert!(
            (rig.oar_sweep - 0.68).abs() < 1e-9,
            "sweep {}",
            rig.oar_sweep
        );
    }

    #[test]
    fn rower_seat_and_handle_travel_the_full_range_over_a_cycle() {
        let rigs: Vec<RowerRigPose> = poses(Sport::Rower)
            .iter()
            .map(
                |pose| match solve_rig_pose(Sport::Rower, pose, 0.0, false) {
                    SportRigPose::Rower(rig) => rig,
                    _ => panic!("sport mismatch"),
                },
            )
            .collect();
        let seat_min = rigs.iter().map(|r| r.seat_z).fold(f64::INFINITY, f64::min);
        let seat_max = rigs
            .iter()
            .map(|r| r.seat_z)
            .fold(f64::NEG_INFINITY, f64::max);
        let sweep_min = rigs
            .iter()
            .map(|r| r.oar_sweep)
            .fold(f64::INFINITY, f64::min);
        let sweep_max = rigs
            .iter()
            .map(|r| r.oar_sweep)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            seat_max - seat_min > 0.30,
            "seat travel {seat_min}..{seat_max}"
        );
        assert!(
            sweep_max - sweep_min > 0.9,
            "sweep {sweep_min}..{sweep_max}"
        );
    }

    #[test]
    fn skierg_pose_is_bounded_and_the_hand_stays_in_reach() {
        let max_reach = ski_proportions::UPPER_ARM_LENGTH + ski_proportions::FOREARM_LENGTH;
        for pose in poses(Sport::Skierg) {
            let SportRigPose::SkiErg(rig) = solve_rig_pose(Sport::Skierg, &pose, 0.0, false) else {
                panic!("sport mismatch");
            };
            assert_joints_bounded(&rig.joints);
            assert!((0.0..=1.0).contains(&rig.hip_compression));
            assert!((0.0..=1.0).contains(&rig.pole_contact));
            assert!(
                (-1.12..=-0.20).contains(&rig.pole_rotation),
                "{}",
                rig.pole_rotation
            );
            // The basket sits at most the plant offset ahead of the rig, or
            // one cycle's travel further once the next plant is being approached.
            let ahead = if pose.cycle_frac >= SKI_POLE_APPROACH_START_CYCLE {
                (1.0 - pose.cycle_frac) * pose.stroke_meters.max(0.0)
            } else {
                0.0
            };
            assert!(
                rig.plant_basket_z <= ski_proportions::POLE_PLANT_FORWARD_OFFSET + ahead + 1e-9,
                "basket {} ahead {ahead}",
                rig.plant_basket_z
            );
            let dy = rig.preferred_hand_y - rig.shoulder_y;
            let dz = rig.preferred_hand_z - rig.shoulder_z;
            let reach = (dy * dy + dz * dz).sqrt();
            assert!(reach <= max_reach, "hand {reach} m from the shoulder");
            assert!(
                rig.shoulder_y > 1.0 && rig.shoulder_y < 1.45,
                "{}",
                rig.shoulder_y
            );
        }
    }

    #[test]
    fn skierg_plant_tracks_the_catch_not_the_frame() {
        // Within one cycle the basket stays put in course space: its rig-local
        // z retreats exactly by the travel since the plant.
        let mut pose = fallback_stroke_pose(Sport::Skierg, 0.1, 30.0);
        pose.index = 3;
        pose.cycle_frac = 0.10;
        pose.stroke_meters = 8.0;
        let SportRigPose::SkiErg(early) = solve_rig_pose(Sport::Skierg, &pose, 0.0, false) else {
            panic!("sport mismatch");
        };
        pose.cycle_frac = 0.25;
        let SportRigPose::SkiErg(later) = solve_rig_pose(Sport::Skierg, &pose, 0.0, false) else {
            panic!("sport mismatch");
        };
        assert!((early.plant_basket_z - (0.24 - 0.8)).abs() < 1e-9);
        assert!((later.plant_basket_z - (0.24 - 2.0)).abs() < 1e-9);
        // Past the approach start the plant belongs to the next cycle.
        pose.cycle_frac = 0.95;
        let SportRigPose::SkiErg(next) = solve_rig_pose(Sport::Skierg, &pose, 0.0, false) else {
            panic!("sport mismatch");
        };
        assert!((next.plant_basket_z - (0.24 + 0.05 * 8.0)).abs() < 1e-9);
    }

    #[test]
    fn bike_pedals_ride_the_crank_circle_and_the_wheel_rolls_on_distance() {
        for (i, pose) in poses(Sport::Bike).iter().enumerate() {
            let distance = f64::from(i as u32) * 0.5;
            let SportRigPose::Bike(rig) = solve_rig_pose(Sport::Bike, pose, distance, false) else {
                panic!("sport mismatch");
            };
            assert_joints_bounded(&rig.joints);
            let r = bike_geometry::CRANK_RADIUS;
            let l = rig.pedal_pos_l;
            assert!(
                (l.y * l.y + l.z * l.z - r * r).abs() < 1e-9,
                "left pedal off the circle"
            );
            assert!(
                (rig.pedal_pos_r.y + l.y).abs() < 1e-12 && (rig.pedal_pos_r.z + l.z).abs() < 1e-12
            );
            assert!((rig.wheel_angle - distance / bike_geometry::WHEEL_RADIUS).abs() < 1e-12);
            assert!((rig.joints.elbow_flex_l - 0.4).abs() < 1e-12);
            assert!(rig.joints.torso_lean > 0.3, "{}", rig.joints.torso_lean);
        }
        // Two radii, as on the web (`bikeRig.js`): rotation on the rim,
        // placement on the outer tyre.
        assert!((bike_geometry::WHEEL_RADIUS - 0.31).abs() < 1e-12);
        assert!((bike_geometry::AXLE_Y - 0.335).abs() < 1e-12);
    }

    #[test]
    fn reduced_motion_returns_the_studio_neutral_poses() {
        let pose = fallback_stroke_pose(Sport::Rower, 1.0, 30.0);
        assert_eq!(
            solve_rig_pose(Sport::Rower, &pose, 5.0, true),
            reduced_pose(Sport::Rower)
        );
        match reduced_pose(Sport::Rower) {
            SportRigPose::Rower(rig) => {
                assert_eq!(rig.joints, AthleteJointPose::NEUTRAL);
                assert_eq!((rig.seat_z, rig.handle_y, rig.handle_z), (-0.1, 0.72, 0.58));
                assert_eq!(rig.oar_feather, -0.06);
            }
            _ => panic!("sport mismatch"),
        }
        match reduced_pose(Sport::Skierg) {
            SportRigPose::SkiErg(rig) => {
                assert_eq!(rig.joints.torso_lean, 0.055);
                assert_eq!(rig.joints.knee_flex_l, -0.05);
                assert_eq!((rig.pelvis_y, rig.pelvis_z), (0.735, 0.0));
                assert_eq!(rig.hip_counter_tilt, 0.0);
                assert_eq!(rig.plant_basket_z, 0.24);
                assert_eq!((rig.preferred_hand_y, rig.preferred_hand_z), (0.663, 0.094));
                assert_eq!((rig.shoulder_y, rig.shoulder_z), (1.271, 0.080));
            }
            _ => panic!("sport mismatch"),
        }
        match reduced_pose(Sport::Bike) {
            SportRigPose::Bike(rig) => {
                assert_eq!(rig.pedal_pos_l.y, -bike_geometry::CRANK_RADIUS);
                assert_eq!(rig.pedal_pos_r.y, bike_geometry::CRANK_RADIUS);
                assert_eq!(rig.joints.torso_lean, 0.74);
            }
            _ => panic!("sport mismatch"),
        }
    }

    #[test]
    fn non_finite_inputs_fall_back_instead_of_propagating() {
        let mut pose = fallback_stroke_pose(Sport::Bike, 0.3, 80.0);
        pose.stroke_meters = f64::NAN;
        let SportRigPose::Bike(rig) = solve_rig_pose(Sport::Bike, &pose, f64::INFINITY, false)
        else {
            panic!("sport mismatch");
        };
        assert_eq!(rig.wheel_angle, 0.0);
        assert_eq!(unit(f64::NAN), 0.0);
        assert_eq!(finite(f64::NEG_INFINITY, 7.0), 7.0);
        assert!((cubic_bezier(0.0, 1.0, 1.0, 1.0, 2.0) - 1.0).abs() < 1e-12);
    }
}
