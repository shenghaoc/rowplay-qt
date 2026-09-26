// SPDX-License-Identifier: GPL-3.0-or-later
//! The authored rowing course dressing (Blender Phase 4).
//!
//! `assets/replay/authored/rowing-dressing.glb` carries the course
//! structures (the finish tower, the pontoons, the bridge, the campus
//! buildings, the boardwalk and hide, the distance boards, the island) and
//! the furniture variants, one mesh each, exported from the reviewed
//! `rowing-dressing.blend` by `tools/blender/export_dressing.py`. Each
//! mesh's primitives are split by material class, in the order the
//! exporter records in `authored/dressing.json`, and the scene passes one
//! material per primitive. The app's build script converts the GLB with
//! balsam for the mesh files alone and generates the scene that draws
//! them, so a node transform or a material in the GLB would be dropped
//! without a sound. [`validate_dressing`] is the build-time gate: exactly
//! the meshes the placements name, each drawn by one top-level node with no
//! transform, one triangle-list primitive per recorded class with
//! positions, normals and vertex colours, no materials, and each mesh's
//! triangles recounted from its accessors against the budgets.

use serde_json::Value;

use super::glb::{AssetError, chunks, is_identity, parse_nodes, validate_accessors};
use super::rowing_shell::mesh_triangles;

/// The prefix of every dressing mesh name.
pub const DRESSING_PREFIX: &str = "dressing:row:";

/// The material classes a primitive may carry, as the scene names them.
pub const DRESSING_CLASSES: [&str; 6] = ["paint", "timber", "metal", "glass", "float", "ground"];

/// Triangles one structure may draw (`tools/blender/export_dressing.py`).
pub const STRUCTURE_BUDGET: u64 = 5_000;
/// Triangles all the structures may draw together.
pub const STRUCTURES_BUDGET: u64 = 30_000;
/// Triangles one furniture variant may draw.
pub const VARIANT_BUDGET: u64 = 600;

/// The attributes the scene's materials read: vertex colours carry the albedo.
const ATTRIBUTES: [&str; 3] = ["POSITION", "NORMAL", "COLOR_0"];

/// One mesh of the pack, as `dressing.json` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DressingMesh {
    /// The name without the prefix.
    pub name: String,
    /// The material class of each primitive, in primitive order.
    pub classes: Vec<String>,
    /// Whether it casts the key light's shadow (structures only).
    pub casts: bool,
    /// Whether it receives the key light's shadow (structures only).
    pub receives: bool,
    /// Its triangles, recounted from the GLB.
    pub triangles: u64,
}

/// The validated dressing pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dressing {
    /// Byte length of the container.
    pub byte_length: usize,
    /// The structures, in the placements' order.
    pub structures: Vec<DressingMesh>,
    /// The furniture variants, in the placements' order.
    pub variants: Vec<DressingMesh>,
}

fn described(entry: &Value, kind: &str) -> Result<DressingMesh, AssetError> {
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| AssetError::Hierarchy(format!("a {kind} in dressing.json has no name")))?;
    let classes: Vec<String> = entry
        .get("classes")
        .and_then(Value::as_array)
        .ok_or_else(|| AssetError::Hierarchy(format!("{kind} {name} lists no classes")))?
        .iter()
        .map(|class| {
            class
                .as_str()
                .filter(|class| DRESSING_CLASSES.contains(class))
                .map(str::to_owned)
                .ok_or_else(|| {
                    AssetError::Hierarchy(format!(
                        "{kind} {name} names a class the scene has no material for"
                    ))
                })
        })
        .collect::<Result<_, _>>()?;
    if classes.is_empty() {
        return Err(AssetError::Hierarchy(format!(
            "{kind} {name} lists no classes"
        )));
    }
    Ok(DressingMesh {
        name: name.to_owned(),
        classes,
        casts: entry.get("casts").and_then(Value::as_bool).unwrap_or(false),
        receives: entry
            .get("receives")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        triangles: 0,
    })
}

/// Validates the authored dressing pack against its placements and
/// recounts its triangles.
///
/// # Errors
/// Any contract violation, naming the mesh or node involved.
pub fn validate_dressing(bytes: &[u8], placements: &Value) -> Result<Dressing, AssetError> {
    let (json, _) = chunks(bytes)?;
    let nodes = parse_nodes(&json)?;
    validate_accessors(&json, &nodes)?;
    if json
        .get("materials")
        .and_then(Value::as_array)
        .is_some_and(|materials| !materials.is_empty())
    {
        return Err(AssetError::Hierarchy(
            "the dressing carries materials; the scene declares them".to_owned(),
        ));
    }
    let empty = Vec::new();
    let mut structures: Vec<DressingMesh> = placements
        .get("structures")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
        .iter()
        .map(|entry| described(entry, "structure"))
        .collect::<Result<_, _>>()?;
    let mut variants: Vec<DressingMesh> = placements
        .get("variants")
        .and_then(Value::as_array)
        .unwrap_or(&empty)
        .iter()
        .map(|entry| described(entry, "variant"))
        .collect::<Result<_, _>>()?;
    if structures.is_empty() || variants.is_empty() {
        return Err(AssetError::Hierarchy(
            "dressing.json names no structures or no variants".to_owned(),
        ));
    }

    let meshes = json
        .get("meshes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let names: Vec<&str> = meshes
        .iter()
        .map(|mesh| mesh.get("name").and_then(Value::as_str).unwrap_or_default())
        .collect();
    let expected: Vec<String> = structures
        .iter()
        .chain(&variants)
        .map(|mesh| format!("{DRESSING_PREFIX}{}", mesh.name))
        .collect();
    let mut found = names.clone();
    found.sort_unstable();
    let mut wanted: Vec<&str> = expected.iter().map(String::as_str).collect();
    wanted.sort_unstable();
    wanted.dedup();
    if found != wanted || wanted.len() != expected.len() {
        return Err(AssetError::Hierarchy(format!(
            "the dressing holds the meshes {found:?}, expected {wanted:?}"
        )));
    }

    let raw_nodes = json
        .get("nodes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let mut drawn = vec![0_usize; meshes.len()];
    for (node, raw) in nodes.iter().zip(raw_nodes) {
        let Some(mesh) = node.mesh.filter(|&mesh| mesh < meshes.len()) else {
            return Err(AssetError::Hierarchy(format!(
                "{} draws no dressing mesh",
                node.description()
            )));
        };
        if node.parent.is_some() || !node.children.is_empty() {
            return Err(AssetError::Hierarchy(format!(
                "{} is nested; the dressing's nodes are flat",
                node.description()
            )));
        }
        if !is_identity(node) || raw.get("matrix").is_some() {
            return Err(AssetError::Geometry {
                node: node.description(),
                reason: "carries a transform, which balsam's mesh files drop; \
                         model it where it stands in the source"
                    .to_owned(),
            });
        }
        drawn[mesh] += 1;
    }

    let mut total = 0;
    for (entry, budget) in structures
        .iter_mut()
        .map(|entry| (entry, STRUCTURE_BUDGET))
        .chain(variants.iter_mut().map(|entry| (entry, VARIANT_BUDGET)))
    {
        let name = format!("{DRESSING_PREFIX}{}", entry.name);
        let mesh = names
            .iter()
            .position(|found| *found == name)
            .expect("the mesh names were checked");
        if drawn[mesh] != 1 {
            return Err(AssetError::Hierarchy(format!(
                "{name} is drawn by {} nodes, not one",
                drawn[mesh]
            )));
        }
        let primitives = meshes[mesh]
            .get("primitives")
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        if primitives.len() != entry.classes.len() {
            return Err(AssetError::Geometry {
                node: name,
                reason: format!(
                    "{} primitives for the {} classes dressing.json records",
                    primitives.len(),
                    entry.classes.len()
                ),
            });
        }
        for primitive in primitives {
            if let Some(missing) = ATTRIBUTES
                .iter()
                .find(|attribute| primitive["attributes"].get(**attribute).is_none())
            {
                return Err(AssetError::Geometry {
                    node: name,
                    reason: format!("a primitive has no {missing}"),
                });
            }
        }
        entry.triangles = mesh_triangles(&json, mesh, &name)?;
        if entry.triangles > budget {
            return Err(AssetError::Geometry {
                node: name,
                reason: format!("{} triangles over its budget of {budget}", entry.triangles),
            });
        }
        if budget == STRUCTURE_BUDGET {
            total += entry.triangles;
        }
    }
    if total > STRUCTURES_BUDGET {
        return Err(AssetError::Hierarchy(format!(
            "the structures draw {total} triangles, over {STRUCTURES_BUDGET}"
        )));
    }
    Ok(Dressing {
        byte_length: bytes.len(),
        structures,
        variants,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn glb(json: &Value) -> Vec<u8> {
        let mut body = serde_json::to_vec(json).expect("json");
        body.resize(body.len().div_ceil(4) * 4, b' ');
        let mut out = Vec::new();
        out.extend_from_slice(b"glTF");
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&u32::try_from(20 + body.len()).expect("len").to_le_bytes());
        out.extend_from_slice(&u32::try_from(body.len()).expect("len").to_le_bytes());
        out.extend_from_slice(b"JSON");
        out.extend_from_slice(&body);
        out
    }

    /// Two structures and one variant; mesh `i` has `i + 1` triangles per
    /// primitive, the tower two primitives.
    fn synthetic() -> (Value, Value) {
        let placements = json!({
            "structures": [
                {"name": "finish-tower", "classes": ["paint", "metal"], "casts": true, "receives": true},
                {"name": "island", "classes": ["ground"], "casts": false, "receives": true},
            ],
            "variants": [{"name": "bollard", "classes": ["metal"]}],
            "instances": [],
        });
        let bounds = json!({"count": 3, "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0]});
        let mut accessors = vec![bounds.clone(), bounds.clone(), bounds];
        let mut meshes = Vec::new();
        let mut nodes = Vec::new();
        for (index, (name, primitives)) in [("finish-tower", 2), ("island", 1), ("bollard", 1)]
            .into_iter()
            .enumerate()
        {
            let mut list = Vec::new();
            for _ in 0..primitives {
                accessors.push(json!({"count": 3 * (index + 1)}));
                list.push(json!({
                    "attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2},
                    "indices": accessors.len() - 1,
                }));
            }
            let full = format!("{DRESSING_PREFIX}{name}");
            meshes.push(json!({"name": full, "primitives": list}));
            nodes.push(json!({"name": full, "mesh": index}));
        }
        (
            json!({"nodes": nodes, "meshes": meshes, "accessors": accessors}),
            placements,
        )
    }

    #[test]
    fn a_flat_pack_matching_its_placements_validates_and_is_recounted() {
        let (json, placements) = synthetic();
        let dressing = validate_dressing(&glb(&json), &placements).expect("valid");
        assert_eq!(dressing.structures.len(), 2);
        assert_eq!(dressing.variants.len(), 1);
        assert_eq!(dressing.structures[0].name, "finish-tower");
        assert_eq!(dressing.structures[0].classes, ["paint", "metal"]);
        assert_eq!(dressing.structures[0].triangles, 2);
        assert!(dressing.structures[0].casts && dressing.structures[0].receives);
        assert_eq!(dressing.structures[1].triangles, 2);
        assert!(!dressing.structures[1].casts);
        assert_eq!(dressing.variants[0].triangles, 3);
    }

    #[test]
    fn drift_from_the_contract_fails_naming_it() {
        let moved = |json: &mut Value, _: &mut Value| {
            json["nodes"][1]["translation"] = json!([0.0, 0.5, 0.0]);
        };
        let matrix = |json: &mut Value, _: &mut Value| {
            json["nodes"][0]["matrix"] = json!([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0.2, 0, 1]);
        };
        let renamed = |json: &mut Value, _: &mut Value| {
            json["meshes"][2]["name"] = json!("dressing:row:post");
        };
        let missing = |json: &mut Value, _: &mut Value| {
            json["nodes"].as_array_mut().expect("nodes").pop();
        };
        let nested = |json: &mut Value, _: &mut Value| json["nodes"][0]["children"] = json!([1]);
        let no_colour = |json: &mut Value, _: &mut Value| {
            json["meshes"][1]["primitives"][0]["attributes"]
                .as_object_mut()
                .expect("attributes")
                .remove("COLOR_0");
        };
        let material = |json: &mut Value, _: &mut Value| {
            json["materials"] = json!([{"name": "paint"}]);
        };
        let fewer_classes = |_: &mut Value, placements: &mut Value| {
            placements["structures"][0]["classes"] = json!(["paint"]);
        };
        let unknown_class = |_: &mut Value, placements: &mut Value| {
            placements["structures"][1]["classes"] = json!(["chrome"]);
        };
        let extra_structure = |_: &mut Value, placements: &mut Value| {
            placements["structures"]
                .as_array_mut()
                .expect("structures")
                .push(json!({"name": "hut", "classes": ["timber"]}));
        };
        let lines = |json: &mut Value, _: &mut Value| {
            json["meshes"][2]["primitives"][0]["mode"] = json!(1);
        };
        for (label, mutate, needle) in [
            (
                "moved node",
                &moved as &dyn Fn(&mut Value, &mut Value),
                "carries a transform",
            ),
            ("matrix node", &matrix, "carries a transform"),
            ("renamed mesh", &renamed, "expected"),
            ("undrawn mesh", &missing, "drawn by 0 nodes"),
            ("nested node", &nested, "nested"),
            ("no vertex colour", &no_colour, "no COLOR_0"),
            ("material", &material, "carries materials"),
            (
                "fewer classes",
                &fewer_classes,
                "primitives for the 1 classes",
            ),
            ("unknown class", &unknown_class, "no material for"),
            ("extra structure", &extra_structure, "expected"),
            ("line primitive", &lines, "not a triangle list"),
        ] {
            let (mut json, mut placements) = synthetic();
            mutate(&mut json, &mut placements);
            let error = validate_dressing(&glb(&json), &placements).expect_err(label);
            assert!(error.to_string().contains(needle), "{label}: {error}");
        }
        let (json, placements) = synthetic();
        let mut truncated = glb(&json);
        truncated.truncate(30);
        let error = validate_dressing(&truncated, &placements).expect_err("truncated");
        assert!(matches!(error, AssetError::Container(_)), "{error}");
    }

    #[test]
    fn the_authored_dressing_honours_the_contract() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/replay/authored");
        let placements: Value =
            serde_json::from_slice(&std::fs::read(dir.join("dressing.json")).expect("placements"))
                .expect("placements JSON");
        let bytes = std::fs::read(dir.join("rowing-dressing.glb")).expect("dressing GLB");
        let dressing = validate_dressing(&bytes, &placements).expect("valid");
        assert!(dressing.structures.iter().any(|s| s.name == "finish-tower"));
        assert!(dressing.variants.iter().any(|v| v.name == "finish-buoy"));
        assert!(
            dressing
                .structures
                .iter()
                .chain(&dressing.variants)
                .all(|mesh| mesh.triangles > 0)
        );
    }
}
