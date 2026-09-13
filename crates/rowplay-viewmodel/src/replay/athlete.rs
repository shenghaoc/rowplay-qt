// SPDX-License-Identifier: GPL-3.0-or-later
//! The V4 athlete pack: skin joints, rest hierarchy and the three authored
//! cycle clips read out of `rowplay-athlete-v4.glb` and cross-checked against
//! `rowplay-athlete-v4.contract.json` (Phase 5b spec R2.2, task T3).
//!
//! Rust evaluates the clips itself (`balsam --removeComponentAnimations`
//! strips them from the QML component): a clip is sampled to per-joint local
//! transforms in the skin's joint order, LINEAR (slerp for rotations), STEP
//! and CUBICSPLINE exactly as the glTF 2.0 specification defines them. Only
//! `float32` accessors are decoded — the pack uses nothing else — and every
//! shape mismatch is a named error, never a silent default.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use super::glb::AssetError;

use super::glb::{Node, chunks, parse_nodes};

/// Component type code of `float32` in glTF.
const FLOAT32: u64 = 5126;

/// A joint's rest transform and its place in the skin hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Joint {
    /// The glTF node name (the balsam `Node.objectName`).
    pub name: String,
    /// Parent joint index in skin order, `None` for the root.
    pub parent: Option<usize>,
    /// Rest translation, metres.
    pub translation: [f64; 3],
    /// Rest rotation quaternion `x, y, z, w`.
    pub rotation: [f64; 4],
    /// Rest scale.
    pub scale: [f64; 3],
}

/// Which property a channel animates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Path {
    /// `translation`.
    Translation,
    /// `rotation`.
    Rotation,
    /// `scale`.
    Scale,
}

/// glTF sampler interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    /// Linear (slerp for rotations).
    Linear,
    /// Hold the previous key.
    Step,
    /// Cubic Hermite with in/out tangents per key.
    CubicSpline,
}

/// One animation channel: keyframe times and values for one joint property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    /// Target joint, skin order.
    pub joint: usize,
    /// Target property.
    pub path: Path,
    /// Sampler interpolation.
    pub interpolation: Interpolation,
    /// Keyframe times, seconds, ascending.
    pub times: Vec<f32>,
    /// Keyframe values, `stride` per key (`3 × stride` for cubic splines:
    /// in-tangent, value, out-tangent).
    pub values: Vec<f32>,
    /// Components per value (3 for translation/scale, 4 for rotation).
    pub stride: usize,
}

/// One authored cycle clip.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    /// The glTF animation name (`rowplay-v4-<sport>-cycle`).
    pub name: String,
    /// The contract's sport for this clip (`rower` / `skierg` / `bike`).
    pub sport: String,
    /// Clip duration, seconds (the last keyframe time).
    pub duration: f32,
    /// The authored drive end as a fraction of the cycle (contract).
    pub drive_end: f64,
    /// The channels.
    pub channels: Vec<Channel>,
}

/// The loaded and cross-checked V4 athlete.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V4Athlete {
    /// Every skin joint in skin order (the order the balsam `Skin` lists).
    pub joints: Vec<Joint>,
    /// Joint indices of the contract's semantic bones, contract order.
    pub semantic: Vec<usize>,
    /// The clips, GLB order.
    pub clips: Vec<Clip>,
    /// The contract's contact offsets: `(bone name, role, local offset)`.
    pub contacts: Vec<(String, String, [f64; 3])>,
}

/// A sampled local transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTransform {
    /// Translation, metres.
    pub translation: [f64; 3],
    /// Rotation quaternion `x, y, z, w`, unit length.
    pub rotation: [f64; 4],
    /// Scale.
    pub scale: [f64; 3],
}

impl V4Athlete {
    /// The clip authored for `sport` (`rower`, `skierg`, `bike`).
    #[must_use]
    pub fn clip_for(&self, sport: &str) -> Option<&Clip> {
        self.clips.iter().find(|clip| clip.sport == sport)
    }

    /// Joint index by bone name.
    #[must_use]
    pub fn joint_index(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|joint| joint.name == name)
    }

    /// The rest pose: every joint's rest transform, skin order.
    #[must_use]
    pub fn rest_pose(&self) -> Vec<LocalTransform> {
        self.joints
            .iter()
            .map(|joint| LocalTransform {
                translation: joint.translation,
                rotation: joint.rotation,
                scale: joint.scale,
            })
            .collect()
    }

    /// Sample `clip` at `t` seconds (wrapped into the clip) over the rest
    /// pose; channels the clip does not carry keep their rest values.
    #[must_use]
    pub fn sample(&self, clip: &Clip, t: f64) -> Vec<LocalTransform> {
        let mut pose = self.rest_pose();
        let duration = f64::from(clip.duration);
        let wrapped = if duration > 0.0 && t.is_finite() {
            t.rem_euclid(duration)
        } else {
            0.0
        };
        for channel in &clip.channels {
            let Some(target) = pose.get_mut(channel.joint) else {
                continue;
            };
            let value = sample_channel(channel, wrapped);
            match channel.path {
                Path::Translation => {
                    target.translation = [value[0], value[1], value[2]];
                }
                Path::Scale => {
                    target.scale = [value[0], value[1], value[2]];
                }
                Path::Rotation => {
                    target.rotation = normalise([value[0], value[1], value[2], value[3]]);
                }
            }
        }
        pose
    }
}

/// Read the V4 athlete pack and cross-check it against the contract JSON.
///
/// # Errors
/// Any decode failure or contract disagreement, naming the joint, clip or
/// accessor involved.
pub fn read_v4(bytes: &[u8], contract_json: &str) -> Result<V4Athlete, AssetError> {
    let contract: Value = serde_json::from_str(contract_json)
        .map_err(|error| AssetError::Athlete(format!("contract JSON unreadable: {error}")))?;
    let (json, bin) = chunks(bytes)?;
    let nodes = parse_nodes(&json)?;
    let (joints, node_to_joint) = read_skin(&json, &nodes)?;
    let semantic = cross_check_bones(&contract, &joints)?;
    let clips = read_clips(&json, bin, &contract, &node_to_joint, &nodes)?;
    let contacts = read_contacts(&contract, &joints)?;
    Ok(V4Athlete {
        joints,
        semantic,
        clips,
        contacts,
    })
}

fn athlete_error(message: impl Into<String>) -> AssetError {
    AssetError::Athlete(message.into())
}

/// Skin 0's joints in order, with parents remapped into joint indices.
fn read_skin(json: &Value, nodes: &[Node]) -> Result<(Vec<Joint>, Vec<Option<usize>>), AssetError> {
    let skin = json
        .get("skins")
        .and_then(Value::as_array)
        .and_then(|skins| skins.first())
        .ok_or_else(|| athlete_error("no skin"))?;
    let joint_nodes: Vec<usize> = skin
        .get("joints")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_u64)
                .map(|v| v as usize)
                .collect()
        })
        .unwrap_or_default();
    if joint_nodes.is_empty() {
        return Err(athlete_error("skin has no joints"));
    }
    let mut node_to_joint = vec![None; nodes.len()];
    for (joint_index, &node_index) in joint_nodes.iter().enumerate() {
        if node_index >= nodes.len() {
            return Err(athlete_error(format!(
                "skin joint {node_index} out of range"
            )));
        }
        if node_to_joint[node_index].is_some() {
            return Err(athlete_error(format!(
                "skin lists {} twice",
                nodes[node_index].description()
            )));
        }
        node_to_joint[node_index] = Some(joint_index);
    }
    let mut joints = Vec::with_capacity(joint_nodes.len());
    for &node_index in &joint_nodes {
        let node = &nodes[node_index];
        // A parent outside the skin (the athlete root Model) means a root joint.
        let parent = node.parent.and_then(|parent| node_to_joint[parent]);
        joints.push(Joint {
            name: node.name.clone(),
            parent,
            translation: node.translation.unwrap_or([0.0; 3]),
            rotation: node.rotation.unwrap_or([0.0, 0.0, 0.0, 1.0]),
            scale: node.scale.unwrap_or([1.0; 3]),
        });
    }
    let roots = joints.iter().filter(|joint| joint.parent.is_none()).count();
    if roots != 1 {
        return Err(athlete_error(format!(
            "skin has {roots} root joints, expected 1"
        )));
    }
    Ok((joints, node_to_joint))
}

/// The contract's bone tables: total count, the semantic bones in order.
fn cross_check_bones(contract: &Value, joints: &[Joint]) -> Result<Vec<usize>, AssetError> {
    let bones = contract
        .get("bones")
        .ok_or_else(|| athlete_error("contract has no bones section"))?;
    let total = bones
        .get("totalCount")
        .and_then(Value::as_u64)
        .ok_or_else(|| athlete_error("contract bones.totalCount missing"))?;
    if total as usize != joints.len() {
        return Err(athlete_error(format!(
            "skin has {} joints, contract says {total}",
            joints.len()
        )));
    }
    let semantic_names: Vec<&str> = bones
        .get("semanticOrderedNames")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if semantic_names.is_empty() {
        return Err(athlete_error("contract bones.semanticOrderedNames missing"));
    }
    let mut semantic = Vec::with_capacity(semantic_names.len());
    for name in semantic_names {
        let index = joints
            .iter()
            .position(|joint| joint.name == name)
            .ok_or_else(|| athlete_error(format!("semantic bone {name} missing from the skin")))?;
        semantic.push(index);
    }
    // Every joint name must be unique or the balsam objectName lookup is ambiguous.
    let mut names: Vec<&str> = joints.iter().map(|joint| joint.name.as_str()).collect();
    names.sort_unstable();
    if names.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(athlete_error("duplicate joint names in the skin"));
    }
    Ok(semantic)
}

fn read_contacts(
    contract: &Value,
    joints: &[Joint],
) -> Result<Vec<(String, String, [f64; 3])>, AssetError> {
    let empty = Vec::new();
    let list = contract
        .get("contacts")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let mut contacts = Vec::with_capacity(list.len());
    for entry in list {
        let bone = entry
            .get("bone")
            .and_then(Value::as_str)
            .ok_or_else(|| athlete_error("contact without a bone"))?;
        let role = entry
            .get("role")
            .and_then(Value::as_str)
            .ok_or_else(|| athlete_error(format!("contact on {bone} without a role")))?;
        let offset = entry
            .get("localOffset")
            .and_then(Value::as_array)
            .filter(|values| values.len() == 3)
            .map(|values| {
                let mut out = [0.0; 3];
                for (slot, value) in out.iter_mut().zip(values) {
                    *slot = value.as_f64().unwrap_or(f64::NAN);
                }
                out
            })
            .ok_or_else(|| {
                athlete_error(format!("contact {role} without a 3-vector localOffset"))
            })?;
        if offset.iter().any(|v| !v.is_finite()) {
            return Err(athlete_error(format!(
                "contact {role} has a non-finite offset"
            )));
        }
        if !joints.iter().any(|joint| joint.name == bone) {
            return Err(athlete_error(format!(
                "contact {role} names unknown bone {bone}"
            )));
        }
        contacts.push((bone.to_owned(), role.to_owned(), offset));
    }
    if contacts.len() != 4 {
        return Err(athlete_error(format!(
            "{} contacts, expected 4",
            contacts.len()
        )));
    }
    Ok(contacts)
}

/// The animations, decoded from the binary chunk and matched to the
/// contract's clip table by name.
fn read_clips(
    json: &Value,
    bin: &[u8],
    contract: &Value,
    node_to_joint: &[Option<usize>],
    nodes: &[Node],
) -> Result<Vec<Clip>, AssetError> {
    let empty = Vec::new();
    let animations = json
        .get("animations")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let contract_clips = contract
        .get("animation")
        .and_then(|animation| animation.get("clips"))
        .and_then(Value::as_array)
        .ok_or_else(|| athlete_error("contract has no animation.clips"))?;
    if animations.len() != contract_clips.len() {
        return Err(athlete_error(format!(
            "{} animations in the pack, {} clips in the contract",
            animations.len(),
            contract_clips.len()
        )));
    }
    let mut clips = Vec::with_capacity(animations.len());
    for animation in animations {
        let name = animation
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let spec = contract_clips
            .iter()
            .find(|clip| clip.get("name").and_then(Value::as_str) == Some(name.as_str()))
            .ok_or_else(|| athlete_error(format!("clip {name} is not in the contract")))?;
        let sport = spec
            .get("sport")
            .and_then(Value::as_str)
            .ok_or_else(|| athlete_error(format!("clip {name} has no sport")))?
            .to_owned();
        let drive_end = spec
            .get("driveEnd")
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && (0.0..=1.0).contains(v))
            .ok_or_else(|| athlete_error(format!("clip {name} has no valid driveEnd")))?;
        let expected_duration = spec
            .get("durationSeconds")
            .and_then(Value::as_f64)
            .ok_or_else(|| athlete_error(format!("clip {name} has no durationSeconds")))?;
        let samplers = animation
            .get("samplers")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        let mut channels = Vec::new();
        let mut duration: f32 = 0.0;
        for (channel_index, channel) in animation
            .get("channels")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
            .iter()
            .enumerate()
        {
            let context = format!("clip {name} channel {channel_index}");
            let target = channel
                .get("target")
                .ok_or_else(|| athlete_error(format!("{context} has no target")))?;
            let node_index = target
                .get("node")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .ok_or_else(|| athlete_error(format!("{context} targets no node")))?;
            let joint = node_to_joint
                .get(node_index)
                .copied()
                .flatten()
                .ok_or_else(|| {
                    athlete_error(format!(
                        "{context} targets {}, which is not a skin joint",
                        nodes
                            .get(node_index)
                            .map_or_else(|| format!("node {node_index}"), Node::description)
                    ))
                })?;
            let path = match target.get("path").and_then(Value::as_str) {
                Some("translation") => Path::Translation,
                Some("rotation") => Path::Rotation,
                Some("scale") => Path::Scale,
                other => {
                    return Err(athlete_error(format!(
                        "{context} animates unsupported path {other:?}"
                    )));
                }
            };
            let sampler_index = channel
                .get("sampler")
                .and_then(Value::as_u64)
                .map(|v| v as usize)
                .ok_or_else(|| athlete_error(format!("{context} has no sampler")))?;
            let sampler = samplers.get(sampler_index).ok_or_else(|| {
                athlete_error(format!("{context} sampler {sampler_index} missing"))
            })?;
            let interpolation = match sampler.get("interpolation").and_then(Value::as_str) {
                None | Some("LINEAR") => Interpolation::Linear,
                Some("STEP") => Interpolation::Step,
                Some("CUBICSPLINE") => Interpolation::CubicSpline,
                Some(other) => {
                    return Err(athlete_error(format!(
                        "{context} uses unsupported interpolation {other}"
                    )));
                }
            };
            let input = sampler
                .get("input")
                .and_then(Value::as_u64)
                .ok_or_else(|| athlete_error(format!("{context} sampler has no input")))?;
            let output = sampler
                .get("output")
                .and_then(Value::as_u64)
                .ok_or_else(|| athlete_error(format!("{context} sampler has no output")))?;
            let (times, time_stride) = read_accessor(json, bin, input as usize, &context)?;
            if time_stride != 1 {
                return Err(athlete_error(format!("{context} input is not SCALAR")));
            }
            let (values, stride) = read_accessor(json, bin, output as usize, &context)?;
            let expected_stride = match path {
                Path::Rotation => 4,
                Path::Translation | Path::Scale => 3,
            };
            if stride != expected_stride {
                return Err(athlete_error(format!(
                    "{context} output has {stride} components, expected {expected_stride}"
                )));
            }
            let per_key = if interpolation == Interpolation::CubicSpline {
                3 * stride
            } else {
                stride
            };
            if times.is_empty() || values.len() != times.len() * per_key {
                return Err(athlete_error(format!(
                    "{context} has {} keys but {} values",
                    times.len(),
                    values.len()
                )));
            }
            if times.windows(2).any(|pair| pair[1] <= pair[0])
                || times.iter().any(|t| !t.is_finite())
            {
                return Err(athlete_error(format!("{context} times are not ascending")));
            }
            if values.iter().any(|v| !v.is_finite()) {
                return Err(athlete_error(format!("{context} has non-finite values")));
            }
            duration = duration.max(*times.last().expect("non-empty"));
            channels.push(Channel {
                joint,
                path,
                interpolation,
                times,
                values,
                stride,
            });
        }
        if channels.is_empty() {
            return Err(athlete_error(format!("clip {name} has no channels")));
        }
        if (f64::from(duration) - expected_duration).abs() > 1e-3 {
            return Err(athlete_error(format!(
                "clip {name} lasts {duration} s, contract says {expected_duration} s"
            )));
        }
        clips.push(Clip {
            name,
            sport,
            duration,
            drive_end,
            channels,
        });
    }
    Ok(clips)
}

/// Decode a `float32` accessor into `(values, components per element)`.
fn read_accessor(
    json: &Value,
    bin: &[u8],
    index: usize,
    context: &str,
) -> Result<(Vec<f32>, usize), AssetError> {
    let accessor = json
        .get("accessors")
        .and_then(Value::as_array)
        .and_then(|list| list.get(index))
        .ok_or_else(|| athlete_error(format!("{context}: accessor {index} missing")))?;
    let component_type = accessor
        .get("componentType")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if component_type != FLOAT32 {
        return Err(athlete_error(format!(
            "{context}: accessor {index} has componentType {component_type}, only float32 is decoded"
        )));
    }
    let components = match accessor.get("type").and_then(Value::as_str) {
        Some("SCALAR") => 1,
        Some("VEC3") => 3,
        Some("VEC4") => 4,
        other => {
            return Err(athlete_error(format!(
                "{context}: accessor {index} has unsupported type {other:?}"
            )));
        }
    };
    let count = accessor.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
    let accessor_offset = accessor
        .get("byteOffset")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let view_index = accessor
        .get("bufferView")
        .and_then(Value::as_u64)
        .ok_or_else(|| athlete_error(format!("{context}: accessor {index} has no bufferView")))?
        as usize;
    let view = json
        .get("bufferViews")
        .and_then(Value::as_array)
        .and_then(|list| list.get(view_index))
        .ok_or_else(|| athlete_error(format!("{context}: bufferView {view_index} missing")))?;
    if view.get("buffer").and_then(Value::as_u64).unwrap_or(0) != 0 {
        return Err(athlete_error(format!(
            "{context}: bufferView {view_index} is not in the binary chunk"
        )));
    }
    let view_offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let view_length = view.get("byteLength").and_then(Value::as_u64).unwrap_or(0) as usize;
    let element_bytes = components * 4;
    let stride = view
        .get("byteStride")
        .and_then(Value::as_u64)
        .map_or(element_bytes, |v| v as usize);
    if stride < element_bytes {
        return Err(athlete_error(format!(
            "{context}: bufferView {view_index} stride too small"
        )));
    }
    let view_end = view_offset
        .checked_add(view_length)
        .filter(|end| *end <= bin.len())
        .ok_or_else(|| {
            athlete_error(format!(
                "{context}: bufferView {view_index} overruns the binary chunk"
            ))
        })?;
    let mut values = Vec::with_capacity(count * components);
    for element in 0..count {
        let start = view_offset + accessor_offset + element * stride;
        let end = start + element_bytes;
        if end > view_end {
            return Err(athlete_error(format!(
                "{context}: accessor {index} overruns its bufferView"
            )));
        }
        for component in 0..components {
            let at = start + component * 4;
            values.push(f32::from_le_bytes(
                bin[at..at + 4].try_into().expect("range checked"),
            ));
        }
    }
    Ok((values, components))
}

/// Evaluate one channel at `t` (already wrapped into the clip).
fn sample_channel(channel: &Channel, time: f64) -> [f64; 4] {
    let stride = channel.stride;
    let times = &channel.times;
    let key_count = times.len();
    let value_at = |key: usize, slot: usize| -> f64 {
        let base = if channel.interpolation == Interpolation::CubicSpline {
            key * 3 * stride + stride + slot // the value lies between the tangents
        } else {
            key * stride + slot
        };
        f64::from(channel.values[base])
    };
    let mut out = [0.0; 4];
    if time <= f64::from(times[0]) || key_count == 1 {
        for (slot, value) in out.iter_mut().enumerate().take(stride) {
            *value = value_at(0, slot);
        }
        return out;
    }
    if time >= f64::from(times[key_count - 1]) {
        for (slot, value) in out.iter_mut().enumerate().take(stride) {
            *value = value_at(key_count - 1, slot);
        }
        return out;
    }
    // The last key at or before time.
    let key = times.partition_point(|key_time| f64::from(*key_time) <= time) - 1;
    let start = f64::from(times[key]);
    let end = f64::from(times[key + 1]);
    let span = end - start;
    let fraction = ((time - start) / span).clamp(0.0, 1.0);
    match channel.interpolation {
        Interpolation::Step => {
            for (slot, value) in out.iter_mut().enumerate().take(stride) {
                *value = value_at(key, slot);
            }
        }
        Interpolation::Linear => {
            if channel.path == Path::Rotation {
                let from = [
                    value_at(key, 0),
                    value_at(key, 1),
                    value_at(key, 2),
                    value_at(key, 3),
                ];
                let to = [
                    value_at(key + 1, 0),
                    value_at(key + 1, 1),
                    value_at(key + 1, 2),
                    value_at(key + 1, 3),
                ];
                out = slerp(from, to, fraction);
            } else {
                for (slot, value) in out.iter_mut().enumerate().take(stride) {
                    *value = value_at(key, slot)
                        + (value_at(key + 1, slot) - value_at(key, slot)) * fraction;
                }
            }
        }
        Interpolation::CubicSpline => {
            // glTF: p(fraction) = (2u³ − 3u² + 1)·v_k + span(u³ − 2u² + fraction)·b_k
            //            + (−2u³ + 3u²)·v_{k+1} + span(u³ − u²)·a_{k+1}
            let squared = fraction * fraction;
            let cubed = squared * fraction;
            let h00 = 2.0 * cubed - 3.0 * squared + 1.0;
            let h10 = cubed - 2.0 * squared + fraction;
            let h01 = -2.0 * cubed + 3.0 * squared;
            let h11 = cubed - squared;
            for (slot, value) in out.iter_mut().enumerate().take(stride) {
                let base_k = key * 3 * stride;
                let base_n = (key + 1) * 3 * stride;
                let v_k = f64::from(channel.values[base_k + stride + slot]);
                let b_k = f64::from(channel.values[base_k + 2 * stride + slot]);
                let a_n = f64::from(channel.values[base_n + slot]);
                let v_n = f64::from(channel.values[base_n + stride + slot]);
                *value = h00 * v_k + span * h10 * b_k + h01 * v_n + span * h11 * a_n;
            }
        }
    }
    out
}

/// Spherical linear interpolation with the shortest-path sign flip.
fn slerp(from: [f64; 4], mut to: [f64; 4], fraction: f64) -> [f64; 4] {
    let mut dot = from.iter().zip(&to).map(|(x, y)| x * y).sum::<f64>();
    if dot < 0.0 {
        for value in &mut to {
            *value = -*value;
        }
        dot = -dot;
    }
    let (weight_from, weight_to) = if dot > 0.9995 {
        (1.0 - fraction, fraction)
    } else {
        let theta = dot.clamp(-1.0, 1.0).acos();
        let sin = theta.sin();
        (
            ((1.0 - fraction) * theta).sin() / sin,
            (fraction * theta).sin() / sin,
        )
    };
    normalise([
        weight_from * from[0] + weight_to * to[0],
        weight_from * from[1] + weight_to * to[1],
        weight_from * from[2] + weight_to * to[2],
        weight_from * from[3] + weight_to * to[3],
    ])
}

fn normalise(q: [f64; 4]) -> [f64; 4] {
    let length = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    if !length.is_finite() || length < 1e-9 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [q[0] / length, q[1] / length, q[2] / length, q[3] / length]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vendored() -> (Vec<u8>, String) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay");
        (
            std::fs::read(dir.join("rowplay-athlete-v4.glb")).expect("athlete pack"),
            std::fs::read_to_string(dir.join("rowplay-athlete-v4.contract.json"))
                .expect("contract"),
        )
    }

    /// A minimal skinned GLB with one root joint and one child, plus one
    /// clip whose channels use the given interpolation.
    fn synthetic(
        interpolation: &str,
        rotation_keys: &[(f32, [f32; 4])],
        cubic: bool,
    ) -> (Vec<u8>, String) {
        let mut bin: Vec<u8> = Vec::new();
        let times: Vec<f32> = rotation_keys.iter().map(|k| k.0).collect();
        for t in &times {
            bin.extend_from_slice(&t.to_le_bytes());
        }
        let times_len = bin.len();
        for (_, q) in rotation_keys {
            if cubic {
                for _ in 0..4 {
                    bin.extend_from_slice(&0.0_f32.to_le_bytes());
                }
            }
            for v in q {
                bin.extend_from_slice(&v.to_le_bytes());
            }
            if cubic {
                for _ in 0..4 {
                    bin.extend_from_slice(&0.0_f32.to_le_bytes());
                }
            }
        }
        let values_len = bin.len() - times_len;
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let count = rotation_keys.len();
        let value_count = if cubic { count * 3 } else { count };
        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "nodes": [
                {"name": "root", "children": [1]},
                {"name": "v4Hips", "children": [2], "translation": [0.0, 1.0, 0.0]},
                {"name": "v4Spine", "translation": [0.0, 0.2, 0.0]}
            ],
            "skins": [{"joints": [1, 2]}],
            "buffers": [{"byteLength": bin.len()}],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": times_len},
                {"buffer": 0, "byteOffset": times_len, "byteLength": values_len}
            ],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": count, "type": "SCALAR"},
                {"bufferView": 1, "componentType": 5126, "count": value_count, "type": "VEC4"}
            ],
            "animations": [{
                "name": "rowplay-v4-row-cycle",
                "samplers": [{"input": 0, "output": 1, "interpolation": interpolation}],
                "channels": [{"sampler": 0, "target": {"node": 2, "path": "rotation"}}]
            }]
        });
        let json_bytes = serde_json::to_vec(&json).expect("json");
        let mut json_chunk = json_bytes.clone();
        while !json_chunk.len().is_multiple_of(4) {
            json_chunk.push(b' ');
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        let total = 12 + 8 + json_chunk.len() + 8 + bin.len();
        out.extend_from_slice(&(total as u32).to_le_bytes());
        out.extend_from_slice(&(json_chunk.len() as u32).to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&json_chunk);
        out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        out.extend_from_slice(b"BIN\0");
        out.extend_from_slice(&bin);
        let last = times.last().copied().unwrap_or(0.0);
        let contract = serde_json::json!({
            "bones": {"totalCount": 2, "semanticOrderedNames": ["v4Hips", "v4Spine"]},
            "animation": {"clips": [{"name": "rowplay-v4-row-cycle", "sport": "rower", "durationSeconds": last, "driveEnd": 0.38}]},
            "contacts": [
                {"bone": "v4Spine", "role": "left-hand", "localOffset": [0.0, 0.0, 0.0]},
                {"bone": "v4Spine", "role": "right-hand", "localOffset": [0.0, 0.0, 0.0]},
                {"bone": "v4Spine", "role": "left-foot", "localOffset": [0.0, 0.0, 0.0]},
                {"bone": "v4Spine", "role": "right-foot", "localOffset": [0.0, 0.0, 0.0]}
            ]
        });
        (out, contract.to_string())
    }

    #[test]
    fn the_vendored_athlete_matches_its_contract() {
        let (bytes, contract) = vendored();
        let athlete = read_v4(&bytes, &contract).expect("V4 pack");
        assert_eq!(athlete.joints.len(), 51);
        assert_eq!(athlete.semantic.len(), 19);
        assert_eq!(athlete.joints[athlete.semantic[0]].name, "v4Hips");
        assert_eq!(athlete.joints[athlete.semantic[18]].name, "v4RightFoot");
        assert_eq!(
            athlete.joints.iter().filter(|j| j.parent.is_none()).count(),
            1
        );
        assert_eq!(athlete.clips.len(), 3);
        let expected = [
            ("rowplay-v4-row-cycle", "rower", 14, 0.38),
            ("rowplay-v4-ski-cycle", "skierg", 14, 0.34),
            ("rowplay-v4-bike-cycle", "bike", 9, 0.5),
        ];
        for (clip, (name, sport, keys, drive_end)) in athlete.clips.iter().zip(expected) {
            assert_eq!(clip.name, name);
            assert_eq!(clip.sport, sport);
            assert_eq!(clip.channels.len(), 20, "{name}");
            assert!(
                (clip.duration - 1.0).abs() < 1e-6,
                "{name}: {}",
                clip.duration
            );
            assert_eq!(clip.drive_end, drive_end);
            for channel in &clip.channels {
                assert_eq!(channel.times.len(), keys, "{name}");
                assert_eq!(channel.interpolation, Interpolation::Linear);
                assert!(
                    athlete.semantic.contains(&channel.joint),
                    "{name} animates a helper bone"
                );
            }
            let translations = clip
                .channels
                .iter()
                .filter(|c| c.path == Path::Translation)
                .count();
            assert_eq!(translations, 1, "{name}: only the hips translate");
            assert_eq!(athlete.clip_for(sport).map(|c| c.name.as_str()), Some(name));
        }
        assert_eq!(athlete.contacts.len(), 4);
        assert_eq!(
            athlete.contacts[0],
            (
                "v4LeftHand".to_owned(),
                "left-hand".to_owned(),
                [-0.08, -0.01, 0.035]
            )
        );
        // The GLB's hips rest pose is what balsam wrote into the component.
        let hips = &athlete.joints[athlete.semantic[0]];
        assert!(
            (hips.translation[1] - 0.96).abs() < 1e-6 && (hips.translation[2] + 0.27).abs() < 1e-6
        );
    }

    #[test]
    fn sampling_wraps_and_stays_unit_length() {
        let (bytes, contract) = vendored();
        let athlete = read_v4(&bytes, &contract).expect("V4 pack");
        let clip = athlete.clip_for("rower").expect("row clip");
        let at_zero = athlete.sample(clip, 0.0);
        let at_lap = athlete.sample(clip, 1.0);
        let at_two = athlete.sample(clip, 2.0);
        assert_eq!(at_zero.len(), 51);
        for (a, b) in at_zero.iter().zip(&at_lap) {
            for (x, y) in a.rotation.iter().zip(&b.rotation) {
                assert!((x - y).abs() < 1e-9, "the clip loops");
            }
        }
        assert_eq!(at_lap, at_two);
        for t in [0.0, 0.1234, 0.5, 0.999] {
            for joint in athlete.sample(clip, t) {
                let q = joint.rotation;
                let len = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
                assert!((len - 1.0).abs() < 1e-9, "t {t}: {q:?}");
            }
        }
        // Mid-cycle the pose differs from the rest pose on some joint.
        let mid = athlete.sample(clip, 0.5);
        assert!(
            mid.iter()
                .zip(athlete.rest_pose())
                .any(|(a, b)| a.rotation != b.rotation)
        );
    }

    #[test]
    fn linear_rotation_slerps_and_step_holds() {
        let half_root_two = std::f32::consts::FRAC_1_SQRT_2;
        let keys = [
            (0.0, [0.0, 0.0, 0.0, 1.0]),
            (1.0, [0.0, half_root_two, 0.0, half_root_two]),
        ];
        let (bytes, contract) = synthetic("LINEAR", &keys, false);
        let athlete = read_v4(&bytes, &contract).expect("synthetic");
        let clip = &athlete.clips[0];
        let half = athlete.sample(clip, 0.5)[1].rotation;
        // Halfway between identity and 90° about Y is 45° about Y.
        let s = (std::f64::consts::FRAC_PI_8).sin();
        let c = (std::f64::consts::FRAC_PI_8).cos();
        assert!(
            (half[1] - s).abs() < 1e-6 && (half[3] - c).abs() < 1e-6,
            "{half:?}"
        );
        let (bytes, contract) = synthetic("STEP", &keys, false);
        let athlete = read_v4(&bytes, &contract).expect("synthetic step");
        assert_eq!(
            athlete.sample(&athlete.clips[0], 0.5)[1].rotation,
            [0.0, 0.0, 0.0, 1.0]
        );
    }

    #[test]
    fn cubic_spline_with_zero_tangents_is_a_smooth_hermite_blend() {
        let keys = [(0.0, [0.0, 0.0, 0.0, 1.0]), (1.0, [0.0, 1.0, 0.0, 0.0])];
        let (bytes, contract) = synthetic("CUBICSPLINE", &keys, true);
        let athlete = read_v4(&bytes, &contract).expect("synthetic cubic");
        let clip = &athlete.clips[0];
        assert_eq!(clip.channels[0].values.len(), 2 * 3 * 4);
        // Zero tangents: p(u) = h00·v0 + h01·v1; at u = 0.5 both weights are 0.5.
        let half = athlete.sample(clip, 0.5)[1].rotation;
        let expected = normalise([0.0, 0.5, 0.0, 0.5]);
        for (a, b) in half.iter().zip(&expected) {
            assert!((a - b).abs() < 1e-9, "{half:?}");
        }
        assert_eq!(athlete.sample(clip, 0.0)[1].rotation, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn contract_disagreements_are_named_errors() {
        let (bytes, contract) = vendored();
        let wrong_count = contract.replace("\"totalCount\": 51", "\"totalCount\": 50");
        let error = read_v4(&bytes, &wrong_count).expect_err("count mismatch");
        assert!(
            error.to_string().contains("51 joints, contract says 50"),
            "{error}"
        );
        let renamed = contract.replace("\"v4RightFoot\"", "\"v4RightPaw\"");
        let error = read_v4(&bytes, &renamed).expect_err("missing bone");
        assert!(error.to_string().contains("v4RightPaw"), "{error}");
        let error = read_v4(b"nope", &contract).expect_err("container");
        assert!(matches!(error, AssetError::Container(_)));
        let (bytes, contract) = synthetic(
            "LINEAR",
            &[(0.0, [0.0, 0.0, 0.0, 1.0]), (0.5, [0.0, 0.0, 0.0, 1.0])],
            false,
        );
        let slow = contract.replace("\"durationSeconds\":0.5", "\"durationSeconds\":1.0");
        assert_ne!(slow, contract);
        let error = read_v4(&bytes, &slow).expect_err("duration mismatch");
        assert!(error.to_string().contains("lasts 0.5 s"), "{error}");
    }
}
