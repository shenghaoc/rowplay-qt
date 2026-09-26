// SPDX-License-Identifier: GPL-3.0-or-later
//! The authored rowing environment (Blender Phase 3).
//!
//! `assets/replay/authored/rowing-environment.glb` carries the rowing land
//! and the vegetation variants, one mesh each, exported from the reviewed
//! `rowing-environment.blend` by `tools/blender/export_environment.py`. The
//! app's build script converts it with balsam for the mesh files alone and
//! generates the scene that draws them: the placements come from
//! `authored/vegetation.json` and the materials from QML. A node transform
//! or a material in the GLB would therefore be dropped without a sound.
//! [`validate_environment`] is the build-time gate: exactly the expected
//! meshes, each drawn by one top-level node with no transform, every
//! primitive a triangle list with positions, normals and vertex colours, no
//! materials, and each mesh's triangles recounted from its accessors, so the
//! manifest's counts are checked rather than trusted.

use serde_json::Value;

use super::glb::{AssetError, chunks, is_identity, parse_nodes, validate_accessors};
use super::rowing_shell::mesh_triangles;

/// The prefix of every environment mesh name.
pub const ENVIRONMENT_PREFIX: &str = "environment:row:";

/// The land parts: the terrain, the far-bank silhouette and the woodland
/// canopy behind the woodland edges.
pub const ENVIRONMENT_LAND: [&str; 3] = ["terrain", "far-bank", "woodland"];

/// The vegetation variants, each drawn instanced.
pub const ENVIRONMENT_VARIANTS: [&str; 6] = [
    "tree-broadleaf-a",
    "tree-broadleaf-b",
    "tree-conifer",
    "tree-poplar",
    "shrub",
    "reeds",
];

/// The attributes the scene's materials read: vertex colours carry the albedo.
const ATTRIBUTES: [&str; 3] = ["POSITION", "NORMAL", "COLOR_0"];

/// The validated environment pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Environment {
    /// Byte length of the container.
    pub byte_length: usize,
    /// Triangles per land part and variant, named without the prefix, in the
    /// order of [`ENVIRONMENT_LAND`] and then [`ENVIRONMENT_VARIANTS`].
    pub triangles: Vec<(String, u64)>,
}

/// Validates the authored environment pack and recounts its triangles.
///
/// # Errors
/// Any contract violation, naming the mesh or node involved.
pub fn validate_environment(bytes: &[u8]) -> Result<Environment, AssetError> {
    let (json, _) = chunks(bytes)?;
    let nodes = parse_nodes(&json)?;
    validate_accessors(&json, &nodes)?;
    if json
        .get("materials")
        .and_then(Value::as_array)
        .is_some_and(|materials| !materials.is_empty())
    {
        return Err(AssetError::Hierarchy(
            "the environment carries materials; the scene declares them".to_owned(),
        ));
    }

    let empty = Vec::new();
    let meshes = json
        .get("meshes")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let names: Vec<&str> = meshes
        .iter()
        .map(|mesh| mesh.get("name").and_then(Value::as_str).unwrap_or_default())
        .collect();
    let expected: Vec<String> = ENVIRONMENT_LAND
        .iter()
        .chain(&ENVIRONMENT_VARIANTS)
        .map(|name| format!("{ENVIRONMENT_PREFIX}{name}"))
        .collect();
    let mut found = names.clone();
    found.sort_unstable();
    let mut wanted: Vec<&str> = expected.iter().map(String::as_str).collect();
    wanted.sort_unstable();
    if found != wanted {
        return Err(AssetError::Hierarchy(format!(
            "the environment holds the meshes {found:?}, expected {wanted:?}"
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
                "{} draws no environment mesh",
                node.description()
            )));
        };
        if node.parent.is_some() || !node.children.is_empty() {
            return Err(AssetError::Hierarchy(format!(
                "{} is nested; the environment's nodes are flat",
                node.description()
            )));
        }
        if !is_identity(node) || raw.get("matrix").is_some() {
            return Err(AssetError::Geometry {
                node: node.description(),
                reason: "carries a transform, which balsam's mesh files drop; \
                         apply it in the source"
                    .to_owned(),
            });
        }
        drawn[mesh] += 1;
    }

    let mut triangles = Vec::with_capacity(expected.len());
    for name in &expected {
        let mesh = names
            .iter()
            .position(|found| found == name)
            .expect("the mesh names were checked");
        if drawn[mesh] != 1 {
            return Err(AssetError::Hierarchy(format!(
                "{name} is drawn by {} nodes, not one",
                drawn[mesh]
            )));
        }
        for primitive in meshes[mesh]
            .get("primitives")
            .and_then(Value::as_array)
            .unwrap_or(&empty)
        {
            if let Some(missing) = ATTRIBUTES
                .iter()
                .find(|attribute| primitive["attributes"].get(**attribute).is_none())
            {
                return Err(AssetError::Geometry {
                    node: name.clone(),
                    reason: format!("a primitive has no {missing}"),
                });
            }
        }
        let count = mesh_triangles(&json, mesh, name)?;
        triangles.push((name[ENVIRONMENT_PREFIX.len()..].to_owned(), count));
    }
    Ok(Environment {
        byte_length: bytes.len(),
        triangles,
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

    /// Nine flat nodes, one mesh each; mesh `i` has `i + 1` triangles.
    fn synthetic() -> Value {
        let names: Vec<String> = ENVIRONMENT_LAND
            .iter()
            .chain(&ENVIRONMENT_VARIANTS)
            .map(|name| format!("{ENVIRONMENT_PREFIX}{name}"))
            .collect();
        let bounds = json!({"count": 3, "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0]});
        let mut accessors = vec![bounds.clone(), bounds.clone(), bounds];
        let mut meshes = Vec::new();
        let mut nodes = Vec::new();
        for (index, name) in names.iter().enumerate() {
            accessors.push(json!({"count": 3 * (index + 1)}));
            meshes.push(json!({"name": name, "primitives": [{
                "attributes": {"POSITION": 0, "NORMAL": 1, "COLOR_0": 2},
                "indices": 3 + index,
            }]}));
            nodes.push(json!({"name": name, "mesh": index}));
        }
        json!({"nodes": nodes, "meshes": meshes, "accessors": accessors})
    }

    #[test]
    fn a_flat_pack_of_the_expected_meshes_validates_and_is_recounted() {
        let environment = validate_environment(&glb(&synthetic())).expect("valid");
        assert_eq!(environment.triangles.len(), 9);
        assert_eq!(environment.triangles[0], ("terrain".to_owned(), 1));
        assert_eq!(environment.triangles[8], ("reeds".to_owned(), 9));
    }

    #[test]
    fn drift_from_the_contract_fails_naming_it() {
        let moved = |json: &mut Value| json["nodes"][1]["translation"] = json!([0.0, 0.5, 0.0]);
        let turned =
            |json: &mut Value| json["nodes"][2]["rotation"] = json!([0.0, 0.1, 0.0, 0.995]);
        let matrix = |json: &mut Value| {
            json["nodes"][0]["matrix"] = json!([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0.2, 0, 1]);
        };
        let renamed = |json: &mut Value| json["meshes"][3]["name"] = json!("environment:row:tree");
        let missing = |json: &mut Value| {
            json["nodes"].as_array_mut().expect("nodes").pop();
        };
        let shared = |json: &mut Value| json["nodes"][8]["mesh"] = json!(7);
        let nested = |json: &mut Value| json["nodes"][0]["children"] = json!([1]);
        let no_colour = |json: &mut Value| {
            json["meshes"][5]["primitives"][0]["attributes"]
                .as_object_mut()
                .expect("attributes")
                .remove("COLOR_0");
        };
        let lines = |json: &mut Value| json["meshes"][4]["primitives"][0]["mode"] = json!(1);
        let material = |json: &mut Value| json["materials"] = json!([{"name": "grass"}]);
        for (label, mutate, needle) in [
            (
                "moved node",
                &moved as &dyn Fn(&mut Value),
                "carries a transform",
            ),
            ("turned node", &turned, "carries a transform"),
            ("matrix node", &matrix, "carries a transform"),
            ("renamed mesh", &renamed, "expected"),
            ("undrawn mesh", &missing, "drawn by 0 nodes"),
            ("shared mesh", &shared, "drawn by 2 nodes"),
            ("nested node", &nested, "nested"),
            ("no vertex colour", &no_colour, "no COLOR_0"),
            ("line primitive", &lines, "not a triangle list"),
            ("material", &material, "carries materials"),
        ] {
            let mut json = synthetic();
            mutate(&mut json);
            let error = validate_environment(&glb(&json)).expect_err(label);
            assert!(error.to_string().contains(needle), "{label}: {error}");
        }
        let mut truncated = glb(&synthetic());
        truncated.truncate(30);
        let error = validate_environment(&truncated).expect_err("truncated");
        assert!(matches!(error, AssetError::Container(_)), "{error}");
    }

    #[test]
    fn the_authored_environment_honours_the_contract() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/replay/authored/rowing-environment.glb");
        let environment =
            validate_environment(&std::fs::read(path).expect("environment GLB")).expect("valid");
        let names: Vec<&str> = environment
            .triangles
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let expected: Vec<&str> = ENVIRONMENT_LAND
            .iter()
            .chain(&ENVIRONMENT_VARIANTS)
            .copied()
            .collect();
        assert_eq!(names, expected);
        assert!(environment.triangles.iter().all(|(_, count)| *count > 0));
    }
}
