// SPDX-License-Identifier: GPL-3.0-or-later
//! Venue pack reader and contract validator (Phase 6a, spec R4.2).
//!
//! The Phase 6a bake (`tools/bake-venues/`) writes one `.glb` and one contract
//! JSON per sport per quality tier. This module reads the pair back and
//! asserts the structure the runtime depends on: every node and material
//! named, names carrying the `environment:<sport>:` prefix, no embedded
//! images (textures are rebound from the vendored sets at runtime), finite
//! POSITION accessor bounds, a plausible world extent, and a name → contract
//! inventory match. `build.rs` runs it over every vendored pair and panics on
//! drift, the same gate `validate_v3` provides for the rig pack.
//!
//! Same lesson as the Phase 5a equipment inventory: a file that loads is not
//! a file that contains what it should. The reader is Qt-free and bounded.

use std::collections::BTreeSet;

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

use super::glb;

/// World extent cap, metres. The web venue's furthest authoring is the
/// far horizon ring at radius 116 plus a horizon height; the bike roof reaches
/// ~72. 200 m allows generous headroom while catching a unit or axis mistake
/// (the earlier spike found the SkiErg massif at radius ~158, so this is not a
/// tight fit but still rejects a runaway transform).
const MAX_VENUE_RADIUS: f64 = 200.0;

/// A venue contract violation, naming the offending file, node or material.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VenueError {
    /// The GLB container or JSON chunk is unreadable.
    #[error("venue container invalid: {0}")]
    Container(String),
    /// The contract JSON is missing, malformed or an unexpected format.
    #[error("venue contract invalid: {0}")]
    Contract(String),
    /// A node is unnamed, wrongly prefixed or otherwise malformed.
    #[error("venue node {node} invalid: {reason}")]
    Node {
        /// The node (or its index, when unnamed).
        node: String,
        /// What failed.
        reason: String,
    },
    /// A material is unnamed, duplicated or absent from the contract.
    #[error("venue material {material} invalid: {reason}")]
    Material {
        /// The material name (or its index, when unnamed).
        material: String,
        /// What failed.
        reason: String,
    },
    /// A geometry accessor lacks finite bounds.
    #[error("venue geometry {node} invalid: {reason}")]
    Geometry {
        /// The mesh node.
        node: String,
        /// What failed.
        reason: String,
    },
    /// The declared inventory does not match the GLB.
    #[error("venue inventory mismatch: {0}")]
    Inventory(String),
    /// A computed bound is outside the plausible venue extent.
    #[error("venue bounds implausible: {0}")]
    Bounds(String),
    /// The GLB embeds an image (textures must live in the vendored sets).
    #[error("venue embeds {0} image(s); textures must be rebound at runtime")]
    EmbeddedImage(usize),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContractFile {
    format: u32,
    seed: u64,
    sport: String,
    quality: String,
    inner_r: f64,
    outer_r: f64,
    materials: serde_json::Map<String, Value>,
    #[serde(default)]
    instancing: serde_json::Map<String, Value>,
    #[serde(default)]
    hidden: Vec<String>,
    #[serde(default)]
    inventory: Option<ContractInventory>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ContractInventory {
    nodes: u64,
    meshes: u64,
    instanced_meshes: u64,
    instances: u64,
    materials: u64,
}

/// The validated facts about one venue GLB.
#[derive(Debug, Clone, PartialEq)]
pub struct VenuePackage {
    /// Sport the venue is authored for (`rower` | `skierg` | `bike`).
    pub sport: String,
    /// Quality tier (`low` | `medium` | `high` | `ultra`).
    pub quality: String,
    /// The bake seed (recorded for provenance; must match `SEED`).
    pub seed: u64,
    /// glTF node names, in document order.
    pub nodes: Vec<String>,
    /// Material names, in document order.
    pub materials: Vec<String>,
    /// Instance-group node names (contract `instancing` keys).
    pub instance_groups: Vec<String>,
    /// Total instance transforms across all groups.
    pub instances: u64,
    /// Meshes the web builds but hides (not present in the GLB).
    pub hidden: Vec<String>,
    /// World-space `(min, max)` over all POSITION accessors.
    pub bounds: ([f64; 3], [f64; 3]),
    /// Meshes in the document.
    pub mesh_count: u64,
}

impl VenuePackage {
    /// A stable, JSON-friendly inventory for the app's metadata sidecar.
    #[must_use]
    pub fn inventory_json(&self) -> Value {
        serde_json::json!({
            "sport": self.sport,
            "quality": self.quality,
            "seed": self.seed,
            "nodes": self.nodes.len(),
            "meshes": self.mesh_count,
            "instancedMeshes": self.instance_groups.len(),
            "instances": self.instances,
            "materials": self.materials.len(),
            "instanceGroups": self.instance_groups,
            "hidden": self.hidden,
        })
    }
}

/// Reads and validates one venue `.glb` against its contract JSON.
///
/// # Errors
/// Any structural violation, naming the file, node or material involved.
pub fn validate_venue(glb_bytes: &[u8], contract_json: &str) -> Result<VenuePackage, VenueError> {
    let contract: ContractFile = serde_json::from_str(contract_json)
        .map_err(|error| VenueError::Contract(error.to_string()))?;
    if contract.format != 1 {
        return Err(VenueError::Contract(format!(
            "unsupported format {}",
            contract.format
        )));
    }

    let (json, _) =
        glb::chunks(glb_bytes).map_err(|error| VenueError::Container(error.to_string()))?;

    if let Some(images) = json.get("images").and_then(Value::as_array) {
        if !images.is_empty() {
            return Err(VenueError::EmbeddedImage(images.len()));
        }
    }

    let prefix = format!("environment:{}:", contract.sport);
    let root_name = format!("venue-{}-{}", contract.sport, contract.quality);

    let raw_nodes = json
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut nodes = Vec::with_capacity(raw_nodes.len());
    for (index, node) in raw_nodes.iter().enumerate() {
        let name = node
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| VenueError::Node {
                node: format!("node {index}"),
                reason: "unnamed".to_owned(),
            })?;
        if name != root_name && !name.starts_with(&prefix) {
            return Err(VenueError::Node {
                node: name.to_owned(),
                reason: format!("does not start with `{prefix}` and is not the root `{root_name}`"),
            });
        }
        nodes.push(name.to_owned());
    }

    let raw_materials = json
        .get("materials")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut materials = Vec::with_capacity(raw_materials.len());
    for (index, material) in raw_materials.iter().enumerate() {
        let name = material
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| VenueError::Material {
                material: format!("material {index}"),
                reason: "unnamed".to_owned(),
            })?;
        if !contract.materials.contains_key(name) {
            return Err(VenueError::Material {
                material: name.to_owned(),
                reason: "not present in the contract".to_owned(),
            });
        }
        if name.ends_with(".001") {
            return Err(VenueError::Material {
                material: name.to_owned(),
                reason: "Blender duplicate suffix survived the bake".to_owned(),
            });
        }
        materials.push(name.to_owned());
    }
    let unique: BTreeSet<&String> = materials.iter().collect();
    if unique.len() != materials.len() {
        return Err(VenueError::Material {
            material: "<duplicate>".to_owned(),
            reason: "two materials share a name".to_owned(),
        });
    }

    // Every named node must be a mesh or a group; and every material the
    // contract names must actually appear (no orphan contract entries that
    // would silently never bind).
    for name in contract.materials.keys() {
        if !materials.contains(name) {
            return Err(VenueError::Material {
                material: name.clone(),
                reason: "in the contract but not in the GLB".to_owned(),
            });
        }
    }

    // Instance groups must name real archetype nodes.
    let node_set: BTreeSet<&String> = nodes.iter().collect();
    let mut instance_groups = Vec::with_capacity(contract.instancing.len());
    for (group, list) in &contract.instancing {
        if !node_set.contains(group) {
            return Err(VenueError::Node {
                node: group.clone(),
                reason: "instance group has no matching archetype node".to_owned(),
            });
        }
        if list.as_array().is_none_or(Vec::is_empty) {
            return Err(VenueError::Node {
                node: group.clone(),
                reason: "instance group is empty".to_owned(),
            });
        }
        instance_groups.push(group.clone());
    }

    let bounds = position_bounds(&json, &nodes)?;
    for axis in 0..3 {
        if bounds.0[axis].abs() > MAX_VENUE_RADIUS || bounds.1[axis].abs() > MAX_VENUE_RADIUS {
            return Err(VenueError::Bounds(format!(
                "axis {axis} spans [{}, {}], outside ±{MAX_VENUE_RADIUS} m",
                bounds.0[axis], bounds.1[axis]
            )));
        }
    }
    if !bounds.0[0].is_finite() || !bounds.1[0].is_finite() {
        return Err(VenueError::Bounds("non-finite extent".to_owned()));
    }
    if contract.outer_r <= contract.inner_r {
        return Err(VenueError::Contract(format!(
            "outer radius {} must exceed inner radius {}",
            contract.outer_r, contract.inner_r
        )));
    }

    let mesh_count = json
        .get("meshes")
        .and_then(Value::as_array)
        .map_or(0, |meshes| meshes.len() as u64);

    if let Some(inventory) = &contract.inventory {
        let actual = ContractInventory {
            nodes: nodes.len() as u64,
            meshes: mesh_count,
            instanced_meshes: instance_groups.len() as u64,
            instances: contract
                .instancing
                .values()
                .filter_map(Value::as_array)
                .map(|list| list.len() as u64)
                .sum(),
            materials: materials.len() as u64,
        };
        if actual != *inventory {
            return Err(VenueError::Inventory(format!(
                "GLB has nodes={} meshes={} instancedMeshes={} instances={} materials={}, \
                 contract declares nodes={} meshes={} instancedMeshes={} instances={} materials={}",
                actual.nodes,
                actual.meshes,
                actual.instanced_meshes,
                actual.instances,
                actual.materials,
                inventory.nodes,
                inventory.meshes,
                inventory.instanced_meshes,
                inventory.instances,
                inventory.materials,
            )));
        }
    }

    Ok(VenuePackage {
        sport: contract.sport,
        quality: contract.quality,
        seed: contract.seed,
        nodes,
        materials,
        instance_groups,
        instances: contract
            .instancing
            .values()
            .filter_map(Value::as_array)
            .map(|list| list.len() as u64)
            .sum(),
        hidden: contract.hidden,
        bounds,
        mesh_count,
    })
}

/// Union of every mesh primitive's POSITION accessor bounds.
fn position_bounds(json: &Value, nodes: &[String]) -> Result<([f64; 3], [f64; 3]), VenueError> {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    let mut found = false;
    let accessors = json.get("accessors").and_then(Value::as_array);
    let meshes = json.get("meshes").and_then(Value::as_array);
    let (Some(accessors), Some(meshes)) = (accessors, meshes) else {
        return Err(VenueError::Geometry {
            node: "<scene>".to_owned(),
            reason: "missing accessors or meshes".to_owned(),
        });
    };
    for (mesh_index, mesh) in meshes.iter().enumerate() {
        let primitives = mesh
            .get("primitives")
            .and_then(Value::as_array)
            .ok_or_else(|| VenueError::Geometry {
                node: format!("mesh {mesh_index}"),
                reason: "no primitives".to_owned(),
            })?;
        for primitive in primitives {
            let Some(accessor_index) = primitive
                .get("attributes")
                .and_then(|attributes| attributes.get("POSITION"))
                .and_then(Value::as_u64)
            else {
                return Err(VenueError::Geometry {
                    node: format!("mesh {mesh_index}"),
                    reason: "primitive has no POSITION".to_owned(),
                });
            };
            let Some(accessor) = accessors.get(accessor_index as usize) else {
                return Err(VenueError::Geometry {
                    node: format!("mesh {mesh_index}"),
                    reason: format!("POSITION accessor {accessor_index} out of range"),
                });
            };
            if accessor.get("type").and_then(Value::as_str) != Some("VEC3") {
                return Err(VenueError::Geometry {
                    node: format!("mesh {mesh_index}"),
                    reason: "POSITION accessor is not VEC3".to_owned(),
                });
            }
            let read = |key: &str| -> Result<[f64; 3], VenueError> {
                let list = accessor
                    .get(key)
                    .and_then(Value::as_array)
                    .filter(|list| list.len() == 3)
                    .ok_or_else(|| VenueError::Geometry {
                        node: format!("mesh {mesh_index}"),
                        reason: format!("POSITION accessor has no finite {key}"),
                    })?;
                let mut out = [0.0; 3];
                for (index, value) in list.iter().enumerate() {
                    let value = value
                        .as_f64()
                        .filter(|value| value.is_finite())
                        .ok_or_else(|| VenueError::Geometry {
                            node: format!("mesh {mesh_index}"),
                            reason: format!("POSITION {key}[{index}] is not finite"),
                        })?;
                    out[index] = value;
                }
                Ok(out)
            };
            let mesh_min = read("min")?;
            let mesh_max = read("max")?;
            for axis in 0..3 {
                min[axis] = min[axis].min(mesh_min[axis]);
                max[axis] = max[axis].max(mesh_max[axis]);
            }
            found = true;
        }
    }
    if !found {
        return Err(VenueError::Geometry {
            node: nodes.first().cloned().unwrap_or_default(),
            reason: "no mesh bounds".to_owned(),
        });
    }
    Ok((min, max))
}
