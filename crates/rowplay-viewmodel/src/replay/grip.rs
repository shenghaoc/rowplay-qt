// SPDX-License-Identifier: GPL-3.0-or-later
//! Hand-grip runtime application (Phase 7, slices 3–4): the per-sport grip
//! frames the `PoseSolver` orients hands with each frame, the digit-chain
//! collector feeding the install-time closure, and the per-sport pose table
//! the scene applies to the finger helper joints.
//!
//! The frames mirror the web sport avatar layers (`renderer3dRowAvatar.ts`,
//! `renderer3dSkiAvatar.ts`, `renderer3dBikeAvatar.ts`): each names the
//! equipment shaft signed thumb-ward, the roll reference, and the base
//! orientation the channel alignment starts from, all in rig-root space.
//! The closure surfaces mirror `gripContractFor` (`renderer3d.ts`).

use rowplay_core::models::Sport;
use rowplay_core::replay::bike_equipment as bike;
use rowplay_core::replay::hand_grip::{
    ClosureOptions, DigitJoint, GripClosure, GripSurface, HandDigitChain, solve_hand_grip_closure,
};
use rowplay_core::replay::rig_pose::SportRigPose;
use rowplay_core::replay::row_equipment as row;
use rowplay_core::replay::ski_equipment as ski;

use super::athlete::V4Athlete;
use super::equipment::{oar_rotations, pole_rotation, quat_mul, rotate_vec};
use super::pose::PolePlacement;

/// Rower palm-tilt relief cap (rad; web `ROWER_PALM_TILT`).
pub const ROWER_PALM_TILT: f64 = 0.75;
/// Rower tilt comfort band (rad; web `ROWER_PALM_TILT_COMFORT`).
pub const ROWER_PALM_TILT_COMFORT: f64 = 0.3;
/// SkiErg spin relief cap (rad, 48°; web `SKI_FLAT_MAX_SPIN`).
pub const SKI_FLAT_MAX_SPIN: f64 = 48.0 * std::f64::consts::PI / 180.0;
/// SkiErg tilt relief cap (rad; web `SKI_PALM_TILT`).
pub const SKI_PALM_TILT: f64 = 0.65;
/// SkiErg tilt comfort band (rad; web `SKI_PALM_TILT_COMFORT`).
pub const SKI_PALM_TILT_COMFORT: f64 = 1.15;

/// One hand's grip frame, all in rig-root space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GripFrame {
    /// Equipment channel axis, signed pinky → thumb.
    pub shaft_thumbward: [f64; 3],
    /// Desired direction of (channel centre − wrist).
    pub roll_reference: [f64; 3],
    /// Starting orientation for the channel alignment.
    pub base: [f64; 4],
    /// Resolve the remaining spin against the palm's true facing (SkiErg)
    /// rather than the default channel-centre ray.
    pub palm_roll: bool,
    /// Equipment surface radius the closure solved against (m).
    pub radius: f64,
    /// Rower flat-wrist window weight (1 for the other sports; the forearm
    /// projection weight is applied by the solver).
    pub flat_window: f64,
}

/// THREE `MathUtils.smoothstep(x, min, max)`: 0 at or below `min`, 1 at or
/// above `max`, a C1 blend between. Note the argument order — `x` first.
#[must_use]
pub fn smoothstep(x: f64, min: f64, max: f64) -> f64 {
    if x <= min {
        return 0.0;
    }
    if x >= max {
        return 1.0;
    }
    let t = (x - min) / (max - min);
    t * t * (3.0 - 2.0 * t)
}

/// Rower flat-wrist window from the stroke's warped phase (web
/// `pendingFlatWristWindow`): open through the drive, shut through the
/// feather window where the forearm crosses the handle line, dipped at the
/// deepest catch reach where a flat hand costs 4 mm more reach than the arm
/// has. `warped_cycle` is the warped phase wrapped to 0..1.
#[must_use]
pub fn row_flat_wrist_window(warped_cycle: f64) -> f64 {
    let feather_shut =
        smoothstep(warped_cycle, 0.475, 0.53) - smoothstep(warped_cycle, 0.555, 0.61);
    let catch_dip = smoothstep(warped_cycle, 0.8, 0.88) - smoothstep(warped_cycle, 0.98, 1.0);
    (1.0 - feather_shut) * (1.0 - 0.35 * catch_dip)
}

/// The stroke's warped cycle fraction in 0..1 (web `euclideanModulo` of the
/// warped phase): the flat-wrist window's clock.
#[must_use]
pub fn warped_cycle(warped_phase: f64) -> f64 {
    if !warped_phase.is_finite() {
        return 0.0;
    }
    let cycles = warped_phase / std::f64::consts::TAU;
    cycles - cycles.floor()
}

/// Both hands' grip frames for this frame's rig state. Ski pole directions
/// come from the solved placements; the rower window from the stroke's
/// warped cycle fraction.
#[must_use]
pub fn grip_frames(
    sport: Sport,
    rig: &SportRigPose,
    poles: Option<[PolePlacement; 2]>,
    warped_cycle: f64,
) -> [GripFrame; 2] {
    match sport {
        Sport::Rower => {
            let (sweep, feather) = match rig {
                SportRigPose::Rower(rower) => (rower.oar_sweep, rower.oar_feather),
                _ => (0.0, 0.0),
            };
            let rotations = oar_rotations(sweep, feather);
            let window = row_flat_wrist_window(warped_cycle);
            [rotations[0], rotations[1]].map(|base| {
                // Thumb faces the flat handle end at the inboard tip. Both
                // oar instances share the right-authored template, so the
                // handle end is local −X on each — the left instance's π
                // mirror rides inside `base` and must not be applied twice
                // by signing the vector here.
                GripFrame {
                    shaft_thumbward: rotate_vec(base, [-1.0, 0.0, 0.0]),
                    roll_reference: [0.0, -1.0, 0.0],
                    base,
                    palm_roll: false,
                    radius: row::SCULL_GRIP_RADIUS,
                    flat_window: window,
                }
            })
        }
        Sport::Skierg => {
            let directions = poles.map(|pair| [pair[0].direction, pair[1].direction]);
            [(-1.0, 0), (1.0, 1)].map(|(side, index)| {
                let direction = directions.map_or([0.0, -1.0, 0.0], |pair| pair[index]);
                GripFrame {
                    shaft_thumbward: [-direction[0], -direction[1], -direction[2]],
                    roll_reference: if side < 0.0 {
                        [1.0, 0.0, 0.0]
                    } else {
                        [-1.0, 0.0, 0.0]
                    },
                    base: pole_rotation(direction),
                    palm_roll: true,
                    radius: ski::POLE_GRIP_RADIUS,
                    flat_window: 1.0,
                }
            })
        }
        Sport::Bike => {
            let rotation = bike::HOOD_ROTATION_X;
            let base = super::equipment::axis_angle([1.0, 0.0, 0.0], rotation);
            [(-1.0, base), (1.0, base)].map(|(_, frame_base)| GripFrame {
                shaft_thumbward: [0.0, -rotation.sin(), rotation.cos()],
                roll_reference: [0.0, -rotation.cos(), -rotation.sin()],
                base: frame_base,
                palm_roll: false,
                radius: bike::HOOD_RADIUS,
                flat_window: 1.0,
            })
        }
    }
}

/// The install-time closure options per sport (web `gripContractFor`).
#[must_use]
pub fn closure_options(sport: Sport, side: f64) -> ClosureOptions {
    let (surface, thumb_oppose, wrap_finger_stages) = match sport {
        Sport::Rower => (
            GripSurface {
                radius: row::SCULL_GRIP_RADIUS,
                thumb_end_axial: Some(row::SCULL_GRIP_ANCHOR_FROM_END),
            },
            row::SCULL_THUMB_OPPOSE,
            false,
        ),
        Sport::Skierg => (
            GripSurface {
                radius: ski::POLE_GRIP_RADIUS,
                thumb_end_axial: None,
            },
            ski::POLE_THUMB_OPPOSE,
            false,
        ),
        Sport::Bike => (
            GripSurface {
                radius: bike::HOOD_RADIUS,
                thumb_end_axial: None,
            },
            bike::HOOD_THUMB_OPPOSE,
            true,
        ),
    };
    ClosureOptions {
        side,
        surface,
        thumb_oppose,
        finger_flesh: None,
        thumb_flesh: None,
        wrap_finger_stages,
    }
}

const DIGITS: [(&str, [&str; 3]); 5] = [
    ("index", ["Proximal", "Intermediate", "Distal"]),
    ("middle", ["Proximal", "Intermediate", "Distal"]),
    ("ring", ["Proximal", "Intermediate", "Distal"]),
    ("pinky", ["Proximal", "Intermediate", "Distal"]),
    ("thumb", ["", "Intermediate", "Distal"]),
];

fn digit_helper_names(prefix: &str, digit: &str, stages: &[&str; 3]) -> [String; 3] {
    if digit == "thumb" {
        [
            format!("{prefix}Thumb"),
            format!("{prefix}ThumbIntermediate"),
            format!("{prefix}ThumbDistal"),
        ]
    } else {
        let capital = match digit {
            "index" => "Index",
            "middle" => "Middle",
            "ring" => "Ring",
            _ => "Pinky",
        };
        [
            format!("{prefix}{capital}{}", stages[0]),
            format!("{prefix}{capital}{}", stages[1]),
            format!("{prefix}{capital}{}", stages[2]),
        ]
    }
}

/// Collect one hand's digit chains from the athlete's rest hierarchy (web
/// `collectHandDigitChains`): each helper's rest transform composed relative
/// to the hand bone, the `v4*Fingers` cup node captured on the way, the tip
/// length derived from the distal segment. Returns `None` for a hand whose
/// helpers are incomplete.
#[must_use]
pub fn collect_hand_chains(athlete: &V4Athlete, side: f64) -> Option<Vec<HandDigitChain>> {
    let prefix = if side < 0.0 { "v4Left" } else { "v4Right" };
    let hand_name = format!("{prefix}Hand");
    let hand = athlete.joint_index(&hand_name)?;
    let joint = |name: &str| -> Option<usize> { athlete.joint_index(name) };
    let mut chains = Vec::with_capacity(5);
    for (digit, stages) in DIGITS {
        let names = digit_helper_names(prefix, digit, &stages);
        let mut joints = Vec::with_capacity(3);
        let mut cup_node = None;
        for name in &names {
            let mut bone = joint(name)?;
            // Compose rest transforms from the hand's child down to the bone.
            let mut stack = vec![bone];
            while bone != hand {
                let parent = athlete.joints.get(bone)?.parent?;
                if parent == hand {
                    break;
                }
                bone = parent;
                stack.push(bone);
            }
            if athlete.joints.get(bone)?.parent != Some(hand) && bone != hand {
                return None;
            }
            let mut position = [0.0; 3];
            let mut quaternion = [0.0, 0.0, 0.0, 1.0];
            for &node in stack.iter().rev() {
                let rest = &athlete.joints[node];
                position = add_vec(position, rotate_vec(quaternion, rest.translation));
                quaternion = quat_mul(quaternion, rest.rotation);
                if cup_node.is_none() && rest.name == format!("{prefix}Fingers") {
                    cup_node = Some(DigitJoint {
                        helper: rest.name.clone(),
                        position,
                        quaternion,
                    });
                }
            }
            joints.push(DigitJoint {
                helper: name.clone(),
                position,
                quaternion,
            });
        }
        if joints.len() < 3 {
            return None;
        }
        let segment = sub_vec(joints[2].position, joints[1].position);
        let tip_length = (length_vec(segment) * 0.92).max(0.012);
        chains.push(HandDigitChain {
            digit,
            joints,
            tip_length,
            cup_node,
        });
    }
    Some(chains)
}

fn add_vec(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub_vec(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn length_vec(vector: [f64; 3]) -> f64 {
    (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
}

/// One sport's solved grip table: every helper's final local rotation plus
/// the contact count the gate asserts.
#[derive(Debug, Clone, PartialEq)]
pub struct GripTable {
    /// `(helper objectName, final local rotation xyzw)`, closure order.
    pub poses: Vec<(String, [f64; 4])>,
    /// Digits reporting contact.
    pub contacts: usize,
    /// Digits in the closure.
    pub digits: usize,
}

/// Solve the install-time closure for one sport and bake the helpers' final
/// local rotations (web `applyGripHelpers` composition over the solved
/// poses): the cup helper holds the carrying posture, every other helper
/// composes rest × oppose × flex. Solved once per sport switch, not per
/// frame.
#[must_use]
pub fn solve_grip_table(
    chains: &[HandDigitChain],
    options: &ClosureOptions,
    rest_rotation_of: &dyn Fn(&str) -> Option<[f64; 4]>,
) -> GripTable {
    let solved: GripClosure = solve_hand_grip_closure(chains, options);
    let by_helper: std::collections::HashMap<
        &str,
        &rowplay_core::replay::hand_grip::DigitStagePose,
    > = solved
        .poses
        .iter()
        .map(|pose| (pose.helper.as_str(), pose))
        .collect();
    let mut poses = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for chain in chains {
        let mut helpers: Vec<&str> = Vec::with_capacity(4);
        if let Some(cup) = &chain.cup_node {
            helpers.push(cup.helper.as_str());
        }
        helpers.extend(chain.joints.iter().map(|joint| joint.helper.as_str()));
        for helper in helpers {
            if !seen.insert(helper) {
                continue;
            }
            let Some(rest) = rest_rotation_of(helper) else {
                continue;
            };
            let is_cup = helper.ends_with("Fingers");
            let (flex, oppose) = by_helper
                .get(helper)
                .map_or((0.0, 0.0), |pose| (pose.flex, pose.oppose));
            poses.push((
                helper.to_owned(),
                rowplay_core::replay::hand_grip::pose_digit_stage(
                    rest,
                    flex,
                    oppose,
                    options.side,
                    is_cup,
                ),
            ));
        }
    }
    GripTable {
        poses,
        contacts: solved
            .contacts
            .iter()
            .filter(|contact| contact.contact)
            .count(),
        digits: solved.contacts.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::pose::rig_targets;
    use super::*;
    use rowplay_core::replay::rig_pose::solve_rig_pose;
    use rowplay_core::replay::stroke_model::fallback_stroke_pose;

    #[test]
    fn smoothstep_matches_the_three_blend() {
        assert_eq!(smoothstep(0.1, 0.2, 0.5), 0.0);
        assert_eq!(smoothstep(0.6, 0.2, 0.5), 1.0);
        assert!((smoothstep(0.35, 0.2, 0.5) - 0.5).abs() < 1e-15);
    }

    #[test]
    fn warped_cycle_wraps_like_euclidean_modulo() {
        use std::f64::consts::TAU;
        assert_eq!(warped_cycle(0.0), 0.0);
        assert!((warped_cycle(TAU) - 0.0).abs() < 1e-15);
        assert!((warped_cycle(-std::f64::consts::PI) - 0.5).abs() < 1e-15);
        assert_eq!(warped_cycle(f64::NAN), 0.0);
    }

    #[test]
    fn flat_window_opens_through_the_drive_only() {
        // Mid-drive the wrist stays flat; the feather window shuts it; the
        // deepest catch reach dips it without closing it.
        assert!((row_flat_wrist_window(0.2) - 1.0).abs() < 1e-12);
        assert_eq!(row_flat_wrist_window(0.54), 0.0);
        assert!((row_flat_wrist_window(0.9) - 0.65).abs() < 1e-12);
        assert!((row_flat_wrist_window(0.0) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn closure_surfaces_match_the_fixture_options() {
        // The grips fixture's per-sport `options` (web `gripContractFor`).
        let rower = closure_options(Sport::Rower, -1.0);
        assert!((rower.surface.radius - 0.023).abs() < 1e-15);
        assert_eq!(rower.surface.thumb_end_axial, Some(0.04));
        assert!((rower.thumb_oppose - 0.3).abs() < 1e-15);
        assert!(!rower.wrap_finger_stages);
        let ski = closure_options(Sport::Skierg, 1.0);
        assert!((ski.surface.radius - 0.016).abs() < 1e-15);
        assert_eq!(ski.surface.thumb_end_axial, None);
        assert!((ski.thumb_oppose - 1.75).abs() < 1e-15);
        assert!(!ski.wrap_finger_stages);
        let bike = closure_options(Sport::Bike, -1.0);
        assert!((bike.surface.radius - 0.018).abs() < 1e-15);
        assert_eq!(bike.surface.thumb_end_axial, None);
        assert!((bike.thumb_oppose - 1.56).abs() < 1e-15);
        assert!(bike.wrap_finger_stages);
    }

    #[test]
    fn shafts_point_where_the_anatomy_needs_them() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            let stroke = fallback_stroke_pose(sport, 1.0, 30.0);
            let rig = solve_rig_pose(sport, &stroke, 3.0, false);
            let targets = rig_targets(&rig);
            let frames = grip_frames(sport, &rig, targets.poles, 0.25);
            for (side, frame) in [(-1.0, frames[0]), (1.0, frames[1])] {
                match sport {
                    // Thumb toward the inboard handle end: the shaft runs
                    // toward the midline on both sides.
                    Sport::Rower => assert!(
                        side * frame.shaft_thumbward[0] < -0.5,
                        "{side}: {:?}",
                        frame.shaft_thumbward
                    ),
                    // Thumb toward the grip top: the shaft runs from the
                    // basket back to the hand.
                    Sport::Skierg => {
                        let poles = targets.poles.expect("poles");
                        let pole = if side < 0.0 { poles[0] } else { poles[1] };
                        let back = [
                            pole.root[0] - pole.basket[0],
                            pole.root[1] - pole.basket[1],
                            pole.root[2] - pole.basket[2],
                        ];
                        let len =
                            (back[0] * back[0] + back[1] * back[1] + back[2] * back[2]).sqrt();
                        let dot = frame.shaft_thumbward[0] * back[0] / len
                            + frame.shaft_thumbward[1] * back[1] / len
                            + frame.shaft_thumbward[2] * back[2] / len;
                        assert!(dot > 0.999, "{side}: {dot}");
                        // Palm toward the midline.
                        assert_eq!(
                            frame.roll_reference,
                            if side < 0.0 {
                                [1.0, 0.0, 0.0]
                            } else {
                                [-1.0, 0.0, 0.0]
                            }
                        );
                        assert!(frame.palm_roll);
                    }
                    // The fixed hood frame, identical both sides.
                    Sport::Bike => {
                        assert!((frame.shaft_thumbward[1] - 0.237_702_626_427_134_6).abs() < 1e-12);
                        assert!((frame.shaft_thumbward[2] - 0.971_337_974_852_029_7).abs() < 1e-12);
                        assert!(!frame.palm_roll);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod collector_tests {
    use super::super::athlete::read_v4;
    use super::*;

    fn vendored() -> V4Athlete {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay");
        let bytes = std::fs::read(dir.join("rowplay-athlete-v4.glb")).expect("athlete pack");
        let contract = std::fs::read_to_string(dir.join("rowplay-athlete-v4.contract.json"))
            .expect("contract");
        read_v4(&bytes, &contract).expect("V4 pack")
    }

    #[test]
    fn collected_chains_match_the_fixture_hands() {
        // The T2 cross-check: the runtime collector reads the same rest
        // hierarchy the grips fixture's `hands` section was generated from.
        let athlete = vendored();
        let fixture: serde_json::Value =
            rowplay_fixtures::load_json("replay-current-main-grips.json").expect("fixture");
        for (side, key) in [(-1.0, "left"), (1.0, "right")] {
            let chains = collect_hand_chains(&athlete, side).expect("complete hands");
            assert_eq!(chains.len(), 5, "{key}");
            let expected_hand = &fixture["hands"][key];
            let expected_chains = expected_hand["chains"].as_array().expect("chains");
            assert_eq!(expected_chains.len(), 5, "{key}");
            for chain in &chains {
                let expected = expected_chains
                    .iter()
                    .find(|entry| entry["digit"] == chain.digit)
                    .unwrap_or_else(|| panic!("{key}: digit {}", chain.digit));
                let expected_joints = expected["joints"].as_array().expect("joints");
                assert_eq!(
                    chain.joints.len(),
                    expected_joints.len(),
                    "{key}/{}",
                    chain.digit
                );
                for (joint, expected_joint) in chain.joints.iter().zip(expected_joints) {
                    assert_eq!(
                        joint.helper,
                        expected_joint["helper"].as_str().expect("helper")
                    );
                    for axis in 0..3 {
                        assert!(
                            (joint.position[axis]
                                - expected_joint["position"][axis].as_f64().expect("pos"))
                            .abs()
                                < 1e-9,
                            "{key}/{} position axis {axis}",
                            chain.digit
                        );
                        assert!(
                            (joint.quaternion[axis]
                                - expected_joint["quaternion"][axis].as_f64().expect("quat"))
                            .abs()
                                < 1e-9,
                            "{key}/{} quaternion axis {axis}",
                            chain.digit
                        );
                    }
                    assert!(
                        (joint.quaternion[3]
                            - expected_joint["quaternion"][3].as_f64().expect("quat"))
                        .abs()
                            < 1e-9,
                        "{key}/{} quaternion w",
                        chain.digit
                    );
                }
                assert!(
                    (chain.tip_length - expected["tipLength"].as_f64().expect("tip")).abs() < 1e-9,
                    "{key}/{} tip",
                    chain.digit
                );
                match (&chain.cup_node, &expected["cupNode"]) {
                    (None, serde_json::Value::Null) => {}
                    (Some(cup), expected_cup) => {
                        assert_eq!(cup.helper, expected_cup["helper"].as_str().expect("cup"));
                        for axis in 0..3 {
                            assert!(
                                (cup.position[axis]
                                    - expected_cup["position"][axis].as_f64().expect("pos"))
                                .abs()
                                    < 1e-9,
                                "{key}/{} cup position {axis}",
                                chain.digit
                            );
                        }
                    }
                    (None, unexpected) => panic!(
                        "{key}/{}: missing cup, fixture has {unexpected}",
                        chain.digit
                    ),
                }
            }
        }
    }
}
