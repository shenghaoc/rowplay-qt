// SPDX-License-Identifier: GPL-3.0-or-later
//! The V3 replay-asset contract reader (web `renderer3dAssets.ts`).
//!
//! Parses the glTF 2.0 JSON chunk of a `.glb` byte slice — no buffers, no
//! images, no GL — and validates exactly what
//! `collectReplayAssetTemplateLibrary` validates before the web renderer may
//! hide a fallback: the seven composite template roots with version 3,
//! identity transforms, an exact part count and a role list that matches its
//! meshes; the eighteen V3 leaf slots with known material roles; no nested
//! roots; no skinned meshes inside a template; finite accessor bounds. Every
//! failure names the slot, template or node path that failed (Phase 5a spec
//! R2.3–R2.5): validation is a hard gate, never a warning.

use serde_json::{Map, Value};
use thiserror::Error;

/// The eleven `replayMaterialRole` values of the V3 pack, web order.
pub const V3_MATERIAL_ROLES: [&str; 11] = [
    "athlete-skin",
    "athlete-fabric",
    "athlete-hair",
    "athlete-footwear",
    "equipment-painted",
    "equipment-dark",
    "equipment-light",
    "equipment-metal",
    "equipment-rubber",
    "equipment-grip",
    "equipment-trim",
];

/// The eighteen V3 leaf slots (web `REQUIRED_REPLAY_ASSET_V3_LEAF_SLOTS`).
pub const V3_LEAF_SLOTS: [&str; 18] = [
    "athlete:torso",
    "athlete:pelvis",
    "athlete:head",
    "athlete:hair",
    "athlete:upper-arm",
    "athlete:forearm",
    "athlete:thigh",
    "athlete:shin",
    "athlete:hand",
    "athlete:elbow",
    "athlete:shoe",
    "athlete:neck",
    "athlete:shoulder",
    "athlete:helmet",
    "equipment:row:blade",
    "equipment:ski:pole-shaft",
    "equipment:ski:pole-grip",
    "equipment:ski:pole-basket",
];

/// The seven composite template roots (web manifest-derived, build-verified).
pub const V3_TEMPLATE_ROOTS: [&str; 7] = [
    "equipment:row:boat-assembly",
    "equipment:row:oar-rig",
    "equipment:row:seat-carriage",
    "equipment:ski:ski-assembly",
    "equipment:bike:wheel-assembly",
    "equipment:bike:frame-assembly",
    "equipment:bike:drivetrain-assembly",
];

/// glTF binary cap. The athlete pack is 4.6 MiB; the bound keeps a hostile or
/// truncated file from allocating ahead of parsing (bounded-scan style).
const MAX_GLB_BYTES: usize = 64 * 1024 * 1024;

/// Transform comparison epsilon (web `isIdentityTemplateRoot`).
const EPSILON: f64 = 1e-6;

/// V3 contract violations, each naming the offending slot or node path.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum AssetError {
    /// The file is not a glTF 2.0 binary, or is truncated / oversized.
    #[error("not a glTF 2.0 binary container: {0}")]
    Container(String),
    /// The JSON chunk is absent or not valid JSON.
    #[error("glTF JSON chunk unreadable: {0}")]
    Json(String),
    /// A composite root is missing, duplicated or nested.
    #[error("replay asset V3 template hierarchy invalid: {0}")]
    Hierarchy(String),
    /// A root's declared metadata contradicts its subtree.
    #[error("replay asset V3 template {template} invalid: {reason}")]
    Template {
        /// The composite template slot that failed.
        template: String,
        /// What failed (version, part count, role mismatch, …).
        reason: String,
    },
    /// A leaf slot is missing, duplicated or carries an unknown role.
    #[error("replay asset V3 leaf slot {slot} invalid: {reason}")]
    Leaf {
        /// The leaf slot id that failed.
        slot: String,
        /// What failed.
        reason: String,
    },
    /// The V4 athlete pack contradicts its contract JSON or cannot be decoded.
    #[error("replay athlete V4 pack invalid: {0}")]
    Athlete(String),
    /// A mesh attribute accessor declares non-finite bounds.
    #[error("replay asset geometry {node} invalid: {reason}")]
    Geometry {
        /// The mesh node whose geometry failed.
        node: String,
        /// What failed.
        reason: String,
    },
}

/// One validated composite template (web `ReplayAssetTemplateManifestEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateManifestEntry {
    /// The template slot, e.g. `equipment:row:boat-assembly`.
    pub template: String,
    /// Mesh nodes in the subtree; equals the declared part count.
    pub part_count: u32,
    /// Sorted, duplicate-free material roles of those meshes.
    pub material_roles: Vec<String>,
}

/// The validated V3 manifest (web `ReplayAssetTemplateManifest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateManifest {
    /// Always 3 for this reader.
    pub version: u32,
    /// One entry per composite root, sorted by slot name.
    pub templates: Vec<TemplateManifestEntry>,
}

/// A validated leaf slot and its material role.
#[derive(Debug, Clone, PartialEq)]
pub struct LeafSlot {
    /// The slot id, e.g. `athlete:torso`.
    pub slot: String,
    /// Its `replayMaterialRole`.
    pub material_role: String,
    /// The mesh's POSITION accessor bounds (`min`, `max`), metres, in the
    /// leaf's own space — leaf shells are fitted into a target box at
    /// runtime (Studio `attachFittedVisual`), so the scene needs them.
    pub bounds: ([f64; 3], [f64; 3]),
}

/// One mesh node and its resolved role, for the runtime material walker.
///
/// Node names are unique across the pack (the validator's error paths rely on
/// them), so a loaded scene graph can be re-materialled by `objectName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeshNodeRole {
    /// The glTF node name (unique in the pack).
    pub name: String,
    /// Its `replayMaterialRole`.
    pub role: String,
    /// The composite template the mesh belongs to, if any.
    pub template: Option<String>,
    /// The leaf slot, if this mesh is a leaf.
    pub slot: Option<String>,
}

/// The full validated V3 library (web `ReplayAssetTemplateLibrary`).
#[derive(Debug, Clone, PartialEq)]
pub struct V3Library {
    /// Byte length of the container the manifest was read from.
    pub byte_length: usize,
    /// The composite manifest.
    pub manifest: TemplateManifest,
    /// The eighteen leaf slots with their roles, in web order.
    pub leaves: Vec<LeafSlot>,
    /// Every mesh node with its role, template and slot (pack order).
    pub mesh_roles: Vec<MeshNodeRole>,
}

/// The glTF node fields the contracts read.
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) children: Vec<usize>,
    pub(crate) parent: Option<usize>,
    pub(crate) extras: Map<String, Value>,
    pub(crate) mesh: Option<usize>,
    pub(crate) skin: Option<usize>,
    pub(crate) translation: Option<[f64; 3]>,
    pub(crate) rotation: Option<[f64; 4]>,
    pub(crate) scale: Option<[f64; 3]>,
}

impl Node {
    pub(crate) fn description(&self) -> String {
        if self.name.is_empty() {
            format!("node {}", self.index)
        } else {
            self.name.clone()
        }
    }
}

fn extra_str<'a>(node: &'a Node, key: &str) -> Option<&'a str> {
    node.extras.get(key).and_then(Value::as_str)
}

fn template_of(node: &Node) -> Option<&str> {
    extra_str(node, "replayAssetTemplateSlot")
}

/// Reads and validates the V3 contract from `.glb` bytes.
///
/// # Errors
/// Any contract violation, naming the slot, template or node path involved.
pub fn validate_v3(bytes: &[u8]) -> Result<V3Library, AssetError> {
    let (json, _) = chunks(bytes)?;
    let nodes = parse_nodes(&json)?;
    validate_accessors(&json, &nodes)?;
    let roots = composite_roots(&nodes)?;
    let (templates, mut mesh_roles) = validate_templates(&nodes, &roots)?;
    let (leaves, leaf_roles) = validate_leaves(&json, &nodes)?;
    mesh_roles.extend(leaf_roles);
    Ok(V3Library {
        byte_length: bytes.len(),
        manifest: TemplateManifest {
            version: 3,
            templates,
        },
        leaves,
        mesh_roles,
    })
}

/// Extracts the JSON chunk and the (optional) binary chunk of a glTF 2.0
/// binary container. The binary chunk is borrowed, never copied.
pub(crate) fn chunks(bytes: &[u8]) -> Result<(Value, &[u8]), AssetError> {
    if bytes.len() > MAX_GLB_BYTES {
        return Err(AssetError::Container(format!(
            "{} bytes exceeds the {MAX_GLB_BYTES} byte cap",
            bytes.len()
        )));
    }
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return Err(AssetError::Container(
            "missing glTF magic or truncated header".to_owned(),
        ));
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().expect("length checked"));
    if version != 2 {
        return Err(AssetError::Container(format!(
            "unsupported glTF version {version}"
        )));
    }
    let total = u32::from_le_bytes(bytes[8..12].try_into().expect("length checked")) as usize;
    if total != bytes.len() {
        return Err(AssetError::Container(format!(
            "header length {total} != file length {}",
            bytes.len()
        )));
    }
    let mut at = 12;
    let mut json: Option<Value> = None;
    let mut bin: &[u8] = &[];
    while at + 8 <= bytes.len() {
        let chunk_len =
            u32::from_le_bytes(bytes[at..at + 4].try_into().expect("range checked")) as usize;
        let chunk_type = &bytes[at + 4..at + 8];
        let start = at + 8;
        let end = start
            .checked_add(chunk_len)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| AssetError::Container("chunk overruns container".to_owned()))?;
        if chunk_type == b"JSON" && json.is_none() {
            json = Some(
                serde_json::from_slice(&bytes[start..end])
                    .map_err(|error| AssetError::Json(error.to_string()))?,
            );
        } else if chunk_type == b"BIN\0" && bin.is_empty() {
            bin = &bytes[start..end];
        }
        at = end;
    }
    json.map(|json| (json, bin))
        .ok_or_else(|| AssetError::Container("no JSON chunk".to_owned()))
}

fn vec3(value: Option<&Value>) -> Option<[f64; 3]> {
    let list = value?.as_array()?;
    if list.len() != 3 {
        return None;
    }
    Some([list[0].as_f64()?, list[1].as_f64()?, list[2].as_f64()?])
}

fn vec4(value: Option<&Value>) -> Option<[f64; 4]> {
    let list = value?.as_array()?;
    if list.len() != 4 {
        return None;
    }
    Some([
        list[0].as_f64()?,
        list[1].as_f64()?,
        list[2].as_f64()?,
        list[3].as_f64()?,
    ])
}

pub(crate) fn parse_nodes(json: &Value) -> Result<Vec<Node>, AssetError> {
    let empty = Vec::new();
    let raw = json
        .get("nodes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let mut nodes = Vec::with_capacity(raw.len());
    for (index, value) in raw.iter().enumerate() {
        let children = value
            .get("children")
            .and_then(Value::as_array)
            .map(|list| {
                list.iter()
                    .filter_map(Value::as_u64)
                    .map(|child| child as usize)
                    .collect()
            })
            .unwrap_or_default();
        let extras = value
            .get("extras")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        nodes.push(Node {
            index,
            name: value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned(),
            children,
            parent: None,
            extras,
            mesh: value
                .get("mesh")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
            skin: value
                .get("skin")
                .and_then(Value::as_u64)
                .map(|v| v as usize),
            translation: vec3(value.get("translation")),
            rotation: vec4(value.get("rotation")),
            scale: vec3(value.get("scale")),
        });
    }
    for parent in 0..nodes.len() {
        for &child in &nodes[parent].children.clone() {
            if child >= nodes.len() {
                return Err(AssetError::Hierarchy(format!(
                    "{} lists child {child} out of range",
                    nodes[parent].description()
                )));
            }
            nodes[child].parent = Some(parent);
        }
    }
    Ok(nodes)
}

/// Finite-bounds check over every mesh attribute accessor (web
/// `cloneFiniteGeometry`); the exporter writes min/max for each attribute.
fn validate_accessors(json: &Value, nodes: &[Node]) -> Result<(), AssetError> {
    let empty = Vec::new();
    let accessors = json
        .get("accessors")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let meshes = json
        .get("meshes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let owner = nodes
            .iter()
            .find(|node| node.mesh == Some(mesh_index))
            .map_or_else(|| format!("mesh {mesh_index}"), Node::description);
        for primitive in mesh
            .get("primitives")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            let Some(attributes) = primitive.get("attributes").and_then(Value::as_object) else {
                continue;
            };
            for (attribute, accessor) in attributes {
                let Some(index) = accessor.as_u64() else {
                    continue;
                };
                let Some(entry) = accessors.get(index as usize) else {
                    continue;
                };
                let finite = ["min", "max"].iter().all(|key| {
                    entry
                        .get(*key)
                        .and_then(Value::as_array)
                        .is_some_and(|values| {
                            values
                                .iter()
                                .all(|value| value.as_f64().is_some_and(f64::is_finite))
                        })
                });
                if !finite {
                    return Err(AssetError::Geometry {
                        node: owner.clone(),
                        reason: format!("attribute {attribute} has non-finite bounds"),
                    });
                }
            }
        }
    }
    Ok(())
}

/// The POSITION bounds of a node's mesh (first primitive), already checked
/// finite by [`validate_accessors`].
fn mesh_bounds(json: &Value, node: &Node) -> Result<([f64; 3], [f64; 3]), AssetError> {
    let read = |key: &str| -> Option<[f64; 3]> {
        let mesh = json.get("meshes")?.as_array()?.get(node.mesh?)?;
        let primitive = mesh.get("primitives")?.as_array()?.first()?;
        let accessor = primitive.get("attributes")?.get("POSITION")?.as_u64()?;
        let values = json
            .get("accessors")?
            .as_array()?
            .get(usize::try_from(accessor).ok()?)?
            .get(key)?;
        vec3(Some(values))
    };
    match (read("min"), read("max")) {
        (Some(min), Some(max)) => Ok((min, max)),
        _ => Err(AssetError::Geometry {
            node: node.description(),
            reason: "leaf mesh has no POSITION bounds".to_owned(),
        }),
    }
}

/// The composite roots: exactly the seven known slots, once each, un-nested.
fn composite_roots(nodes: &[Node]) -> Result<Vec<usize>, AssetError> {
    let mut roots: Vec<usize> = Vec::new();
    for node in nodes {
        if extra_str(node, "replayAssetKind") != Some("composite") {
            continue;
        }
        let Some(template) = template_of(node) else {
            return Err(AssetError::Hierarchy(format!(
                "composite {} has no replayAssetTemplateSlot",
                node.description()
            )));
        };
        if !V3_TEMPLATE_ROOTS.contains(&template) {
            return Err(AssetError::Hierarchy(format!(
                "unknown composite template slot {template} on {}",
                node.description()
            )));
        }
        if roots
            .iter()
            .any(|&existing| template_of(&nodes[existing]) == Some(template))
        {
            return Err(AssetError::Hierarchy(format!(
                "duplicate replay asset V3 template: {template}"
            )));
        }
        roots.push(node.index);
    }
    for &root in &roots {
        let mut parent = nodes[root].parent;
        while let Some(index) = parent {
            if roots.contains(&index) {
                return Err(AssetError::Hierarchy(format!(
                    "replay asset V3 templates cannot be nested: {}",
                    template_of(&nodes[root]).unwrap_or_default()
                )));
            }
            parent = nodes[index].parent;
        }
    }
    for expected in V3_TEMPLATE_ROOTS {
        if !roots
            .iter()
            .any(|&root| template_of(&nodes[root]) == Some(expected))
        {
            return Err(AssetError::Hierarchy(format!(
                "replay asset V3 pack is missing composite template {expected}"
            )));
        }
    }
    Ok(roots)
}

/// The web's `isIdentityTemplateRoot`: absent or unit translation, rotation
/// and scale on every composite root.
fn is_identity(node: &Node) -> bool {
    let translation_ok = node
        .translation
        .is_none_or(|t| t.iter().all(|c| c.abs() < EPSILON));
    let rotation_ok = node
        .rotation
        .is_none_or(|r| r[0..3].iter().all(|c| c.abs() < EPSILON) && (r[3] - 1.0).abs() < EPSILON);
    let scale_ok = node
        .scale
        .is_none_or(|s| s.iter().all(|c| (c - 1.0).abs() < EPSILON));
    translation_ok && rotation_ok && scale_ok
}

fn template_error(template: &str, reason: impl Into<String>) -> AssetError {
    AssetError::Template {
        template: template.to_owned(),
        reason: reason.into(),
    }
}

/// Declared role list of a root (web `readDeclaredRoles`).
fn declared_roles(node: &Node, template: &str) -> Result<Vec<String>, AssetError> {
    let Some(list) = node
        .extras
        .get("replayMaterialRoles")
        .and_then(Value::as_array)
    else {
        return Err(template_error(
            template,
            "has invalid declared material roles",
        ));
    };
    let roles: Vec<String> = list
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    if roles.len() != list.len() || roles.is_empty() {
        return Err(template_error(
            template,
            "has invalid declared material roles",
        ));
    }
    for role in &roles {
        if !V3_MATERIAL_ROLES.contains(&role.as_str()) {
            return Err(template_error(
                template,
                format!("declares unknown material role {role}"),
            ));
        }
    }
    let mut sorted = roles.clone();
    sorted.sort();
    if sorted.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(template_error(
            template,
            "has duplicate declared material roles",
        ));
    }
    Ok(sorted)
}

/// Walks one composite subtree (web `snapshotTemplateNode`), collecting mesh
/// roles and enforcing the per-node contract.
fn snapshot(
    nodes: &[Node],
    index: usize,
    template: &str,
    roles: &mut Vec<String>,
    mesh_roles: &mut Vec<MeshNodeRole>,
    path: &mut String,
) -> Result<u32, AssetError> {
    let node = &nodes[index];
    let description = format!("{template}/{}", node.description());
    if let Some(other) = template_of(node) {
        if other != template {
            return Err(template_error(
                template,
                format!("has a child assigned to {other} ({})", node.description()),
            ));
        }
    }
    let mut mesh_count = 0;
    if node.mesh.is_some() {
        if node.skin.is_some() {
            return Err(template_error(
                template,
                format!("uses an unsupported skinned mesh: {description}"),
            ));
        }
        let Some(role) = extra_str(node, "replayMaterialRole") else {
            return Err(template_error(
                template,
                format!("has an invalid material role: {description}"),
            ));
        };
        if !V3_MATERIAL_ROLES.contains(&role) {
            return Err(template_error(
                template,
                format!("has an invalid material role {role}: {description}"),
            ));
        }
        roles.push(role.to_owned());
        mesh_roles.push(MeshNodeRole {
            name: node.name.clone(),
            role: role.to_owned(),
            template: Some(template.to_owned()),
            slot: None,
        });
        mesh_count += 1;
    }
    let saved = path.len();
    for &child in &node.children {
        path.push('/');
        path.push_str(&nodes[child].description());
        mesh_count += snapshot(nodes, child, template, roles, mesh_roles, path)?;
        path.truncate(saved);
    }
    Ok(mesh_count)
}

fn validate_templates(
    nodes: &[Node],
    roots: &[usize],
) -> Result<(Vec<TemplateManifestEntry>, Vec<MeshNodeRole>), AssetError> {
    let mut entries = Vec::new();
    let mut mesh_roles = Vec::new();
    for &root in roots {
        let template = template_of(&nodes[root]).expect("checked by composite_roots");
        if nodes[root]
            .extras
            .get("replayAssetVersion")
            .and_then(Value::as_u64)
            != Some(3)
        {
            return Err(template_error(template, "has an invalid version"));
        }
        if !is_identity(&nodes[root]) {
            return Err(template_error(
                template,
                "root must have identity transforms",
            ));
        }
        let declared_part_count = nodes[root]
            .extras
            .get("replayAssetPartCount")
            .and_then(Value::as_u64)
            .filter(|count| *count > 0)
            .ok_or_else(|| template_error(template, "has an invalid part count"))?;
        let declared = declared_roles(&nodes[root], template)?;
        let mut actual = Vec::new();
        let mut path = String::new();
        let mesh_count = u64::from(snapshot(
            nodes,
            root,
            template,
            &mut actual,
            &mut mesh_roles,
            &mut path,
        )?);
        if mesh_count != declared_part_count {
            return Err(template_error(
                template,
                format!("part count {declared_part_count} does not match meshes ({mesh_count})"),
            ));
        }
        actual.sort();
        actual.dedup();
        if actual != declared {
            return Err(template_error(
                template,
                "material roles do not match meshes",
            ));
        }
        entries.push(TemplateManifestEntry {
            template: template.to_owned(),
            part_count: u32::try_from(mesh_count).expect("bounded by part count"),
            material_roles: declared,
        });
    }
    entries.sort_by(|left, right| left.template.cmp(&right.template));
    Ok((entries, mesh_roles))
}

/// The eighteen leaf slots, once each, each with a known role (web
/// `collectLegacyGeometries` over `REQUIRED_REPLAY_ASSET_V3_LEAF_SLOTS`).
fn validate_leaves(
    json: &Value,
    nodes: &[Node],
) -> Result<(Vec<LeafSlot>, Vec<MeshNodeRole>), AssetError> {
    let mut found: Vec<LeafSlot> = Vec::new();
    let mut mesh_roles: Vec<MeshNodeRole> = Vec::new();
    for node in nodes {
        let Some(slot) = extra_str(node, "replayAssetSlot") else {
            continue;
        };
        if !V3_LEAF_SLOTS.contains(&slot) {
            continue;
        }
        if node.mesh.is_none() {
            continue;
        }
        if found.iter().any(|leaf| leaf.slot == slot) {
            return Err(AssetError::Leaf {
                slot: slot.to_owned(),
                reason: format!("appears twice ({})", node.description()),
            });
        }
        let Some(role) = extra_str(node, "replayMaterialRole") else {
            return Err(AssetError::Leaf {
                slot: slot.to_owned(),
                reason: format!("mesh {} has no material role", node.description()),
            });
        };
        if !V3_MATERIAL_ROLES.contains(&role) {
            return Err(AssetError::Leaf {
                slot: slot.to_owned(),
                reason: format!("mesh {} has unknown role {role}", node.description()),
            });
        }
        found.push(LeafSlot {
            slot: slot.to_owned(),
            material_role: role.to_owned(),
            bounds: mesh_bounds(json, node)?,
        });
        mesh_roles.push(MeshNodeRole {
            name: node.name.clone(),
            role: role.to_owned(),
            template: None,
            slot: Some(slot.to_owned()),
        });
    }
    for expected in V3_LEAF_SLOTS {
        if !found.iter().any(|leaf| leaf.slot == expected) {
            return Err(AssetError::Leaf {
                slot: expected.to_owned(),
                reason: "leaf slot is missing from the pack".to_owned(),
            });
        }
    }
    found.sort_by(|left, right| {
        V3_LEAF_SLOTS
            .iter()
            .position(|slot| *slot == left.slot.as_str())
            .cmp(
                &V3_LEAF_SLOTS
                    .iter()
                    .position(|slot| *slot == right.slot.as_str()),
            )
    });
    mesh_roles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok((found, mesh_roles))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal glTF 2.0 binary around one JSON object.
    fn glb(json: &Value) -> Vec<u8> {
        let json_bytes = serde_json::to_vec(json).expect("json");
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        chunk.extend_from_slice(b"JSON");
        chunk.extend_from_slice(&json_bytes);
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&((12 + chunk.len()) as u32).to_le_bytes());
        out.extend_from_slice(&chunk);
        out
    }

    fn mesh_node(name: &str, slot: Option<&str>, role: Option<&str>) -> Value {
        let mut extras = Map::new();
        if let Some(slot) = slot {
            extras.insert("replayAssetSlot".into(), Value::String(slot.to_owned()));
            extras.insert("replayAssetKind".into(), Value::String("leaf".into()));
        }
        if let Some(role) = role {
            extras.insert("replayMaterialRole".into(), Value::String(role.to_owned()));
        }
        serde_json::json!({
            "name": name,
            "mesh": 0,
            "extras": extras,
        })
    }

    fn composite(template: &str, part_count: u64, roles: &[&str], children: &[usize]) -> Value {
        serde_json::json!({
            "name": template,
            "children": children,
            "extras": {
                "replayAssetKind": "composite",
                "replayAssetTemplateSlot": template,
                "replayAssetVersion": 3,
                "replayAssetPartCount": part_count,
                "replayMaterialRoles": roles,
            },
        })
    }

    /// A pack with all seven roots (one mesh part each) and all leaves.
    fn valid_pack() -> Value {
        let mut nodes = Vec::new();
        for template in V3_TEMPLATE_ROOTS {
            let part = nodes.len() + 7;
            nodes.push(composite(template, 1, &["equipment-metal"], &[part]));
        }
        for template in V3_TEMPLATE_ROOTS {
            nodes.push(mesh_node(
                &format!("{template}-part"),
                None,
                Some("equipment-metal"),
            ));
        }
        for (slot, role) in V3_LEAF_SLOTS.iter().zip(leaf_roles()) {
            nodes.push(mesh_node(&format!("leaf-{slot}"), Some(slot), Some(role)));
        }
        serde_json::json!({
            "nodes": nodes,
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}}]}],
            "accessors": [{"min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0]}],
        })
    }

    fn leaf_roles() -> [&'static str; 18] {
        [
            "athlete-fabric",
            "athlete-fabric",
            "athlete-skin",
            "athlete-hair",
            "athlete-skin",
            "athlete-skin",
            "athlete-fabric",
            "athlete-fabric",
            "athlete-skin",
            "athlete-skin",
            "athlete-footwear",
            "athlete-skin",
            "athlete-fabric",
            "equipment-painted",
            "equipment-painted",
            "equipment-metal",
            "equipment-rubber",
            "equipment-painted",
        ]
    }

    #[test]
    fn a_valid_pack_validates() {
        let library = validate_v3(&glb(&valid_pack())).expect("valid pack");
        assert_eq!(library.manifest.version, 3);
        assert_eq!(library.manifest.templates.len(), 7);
        assert_eq!(library.leaves.len(), 18);
        assert_eq!(
            library.manifest.templates[0].template,
            "equipment:bike:drivetrain-assembly"
        );
    }

    #[test]
    fn the_vendored_rig_pack_validates() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay")
            .join("rowplay-rigs-v3.glb");
        let bytes = std::fs::read(&path).expect("vendored V3 pack");
        let library = validate_v3(&bytes).expect("vendored pack honours the V3 contract");
        assert_eq!(library.byte_length, 733_864);
        assert_eq!(library.manifest.templates.len(), 7);
        let boat = library
            .manifest
            .templates
            .iter()
            .find(|entry| entry.template == "equipment:row:boat-assembly")
            .expect("boat assembly");
        assert!(boat.part_count > 0);
        assert_eq!(library.leaves.len(), 18);
    }

    fn break_pack(mutate: impl FnOnce(&mut Vec<Value>)) -> Vec<u8> {
        let mut json = valid_pack();
        let nodes = json["nodes"].as_array_mut().expect("nodes");
        mutate(nodes);
        glb(&json)
    }

    #[test]
    fn failures_name_the_offending_slot() {
        // Missing root: drop the ski assembly composite.
        let bytes = break_pack(|nodes| {
            let at = nodes
                .iter()
                .position(|node| node["name"] == "equipment:ski:ski-assembly")
                .expect("ski root");
            nodes.remove(at);
        });
        let error = validate_v3(&bytes).expect_err("missing root");
        assert!(
            error.to_string().contains("equipment:ski:ski-assembly"),
            "{error}"
        );

        // Part count mismatch names the template.
        let bytes = break_pack(|nodes| {
            nodes[0]["extras"]["replayAssetPartCount"] = serde_json::json!(9);
        });
        let error = validate_v3(&bytes).expect_err("part count");
        assert!(
            error.to_string().contains("equipment:row:boat-assembly"),
            "{error}"
        );
        assert!(error.to_string().contains("part count"), "{error}");

        // Role mismatch names the template.
        let bytes = break_pack(|nodes| {
            nodes[0]["extras"]["replayMaterialRoles"] = serde_json::json!(["equipment-rubber"]);
        });
        let error = validate_v3(&bytes).expect_err("role mismatch");
        assert!(
            error.to_string().contains("material roles do not match"),
            "{error}"
        );

        // Non-identity root names the template.
        let bytes = break_pack(|nodes| {
            nodes[1]["translation"] = serde_json::json!([0.0, 0.5, 0.0]);
        });
        let error = validate_v3(&bytes).expect_err("identity");
        assert!(error.to_string().contains("identity transforms"), "{error}");

        // Bad version names the template.
        let bytes = break_pack(|nodes| {
            nodes[2]["extras"]["replayAssetVersion"] = serde_json::json!(2);
        });
        let error = validate_v3(&bytes).expect_err("version");
        assert!(error.to_string().contains("invalid version"), "{error}");

        // A part assigned to another template names both.
        let bytes = break_pack(|nodes| {
            nodes[8]["extras"]["replayAssetTemplateSlot"] =
                serde_json::json!("equipment:ski:ski-assembly");
        });
        let error = validate_v3(&bytes).expect_err("cross template child");
        assert!(error.to_string().contains("child assigned to"), "{error}");

        // Missing leaf names the slot.
        let bytes = break_pack(|nodes| {
            let at = nodes
                .iter()
                .position(|node| node["name"] == "leaf-athlete:helmet")
                .expect("helmet leaf");
            nodes.remove(at);
        });
        let error = validate_v3(&bytes).expect_err("missing leaf");
        assert!(error.to_string().contains("athlete:helmet"), "{error}");

        // Non-finite accessor bounds name the mesh node.
        let mut json = valid_pack();
        json["accessors"][0]["max"] = serde_json::json!([1.0, f64::NAN, 1.0]);
        let error = validate_v3(&glb(&json)).expect_err("non-finite");
        assert!(error.to_string().contains("non-finite bounds"), "{error}");
    }

    #[test]
    fn containers_are_bounded_and_typed() {
        assert!(matches!(
            validate_v3(b"not gltf at all"),
            Err(AssetError::Container(_))
        ));
        let mut json = valid_pack();
        json["nodes"] = serde_json::json!([]);
        let error = validate_v3(&glb(&json)).expect_err("no composites");
        assert!(matches!(error, AssetError::Hierarchy(_)), "{error}");
    }
}
