// SPDX-License-Identifier: GPL-3.0-or-later
//! The authored single scull and sculls (Blender Phase 2).
//!
//! `assets/replay/authored/rowing-shell.glb` replaces the vendored V3 pack's
//! rowing geometry in the scene, inside the V3 contract: the same three
//! rowing template roots and the blade leaf, under the same node names, the
//! same material roles and the same composite rules. [`validate_rowing_shell`]
//! is the build-time gate: every mesh node's name, role, template and slot
//! must equal the vendored pack's rowing entries, so the material walker, the
//! anchors and the frame bundle apply unchanged, and the pack must stay within
//! its triangle budget as drawn. A drift fails the build naming what moved.

use serde_json::Value;

use super::glb::{
    AssetError, MeshNodeRole, V3Library, chunks, declared_roles, extra_str, is_identity,
    parse_nodes, snapshot, template_error, template_of, validate_accessors,
};

/// The rowing template roots the authored pack carries.
pub const ROWING_TEMPLATES: [&str; 3] = [
    "equipment:row:boat-assembly",
    "equipment:row:oar-rig",
    "equipment:row:seat-carriage",
];

/// The rowing blade leaf slot.
pub const ROWING_BLADE_SLOT: &str = "equipment:row:blade";

/// Triangle budget for the shell and both oars as drawn: the boat and seat
/// once, the oar rig and blade once per side.
pub const ROWING_TRIANGLE_BUDGET: u64 = 60_000;

/// The validated authored rowing pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowingShell {
    /// Byte length of the container.
    pub byte_length: usize,
    /// Triangles per template root and for the blade leaf, sorted by slot.
    pub triangles: Vec<(String, u64)>,
    /// Triangles as drawn: boat and seat once, oar rig and blade twice.
    pub rendered_triangles: u64,
}

fn is_rowing(role: &MeshNodeRole) -> bool {
    role.template
        .as_deref()
        .is_some_and(|template| ROWING_TEMPLATES.contains(&template))
        || role.slot.as_deref() == Some(ROWING_BLADE_SLOT)
}

/// Triangles in one mesh (every primitive must be a triangle list).
pub(crate) fn mesh_triangles(json: &Value, mesh: usize, owner: &str) -> Result<u64, AssetError> {
    let geometry = |reason: String| AssetError::Geometry {
        node: owner.to_owned(),
        reason,
    };
    let primitives = json
        .get("meshes")
        .and_then(Value::as_array)
        .and_then(|meshes| meshes.get(mesh))
        .and_then(|mesh| mesh.get("primitives"))
        .and_then(Value::as_array)
        .ok_or_else(|| geometry("mesh has no primitives".to_owned()))?;
    let mut total = 0;
    for primitive in primitives {
        if primitive.get("mode").and_then(Value::as_u64).unwrap_or(4) != 4 {
            return Err(geometry("primitive is not a triangle list".to_owned()));
        }
        let accessor = primitive
            .get("indices")
            .or_else(|| primitive.get("attributes").and_then(|a| a.get("POSITION")))
            .and_then(Value::as_u64)
            .ok_or_else(|| geometry("primitive has no indices or positions".to_owned()))?;
        let count = json
            .get("accessors")
            .and_then(Value::as_array)
            .and_then(|accessors| accessors.get(usize::try_from(accessor).ok()?))
            .and_then(|accessor| accessor.get("count"))
            .and_then(Value::as_u64)
            .ok_or_else(|| geometry(format!("accessor {accessor} has no count")))?;
        if count % 3 != 0 {
            return Err(geometry(format!("{count} indices is not whole triangles")));
        }
        total += count / 3;
    }
    Ok(total)
}

/// Validates the authored rowing pack against the vendored V3 library's
/// rowing entries and the triangle budget.
///
/// # Errors
/// Any contract violation, naming the template, slot or node involved.
pub fn validate_rowing_shell(bytes: &[u8], base: &V3Library) -> Result<RowingShell, AssetError> {
    let (json, _) = chunks(bytes)?;
    let nodes = parse_nodes(&json)?;
    validate_accessors(&json, &nodes)?;

    let mut roots = Vec::new();
    for node in &nodes {
        if extra_str(node, "replayAssetKind") != Some("composite") {
            continue;
        }
        let template = template_of(node).unwrap_or_default();
        if !ROWING_TEMPLATES.contains(&template) {
            return Err(AssetError::Hierarchy(format!(
                "the rowing shell carries a non-rowing composite {} ({template})",
                node.description()
            )));
        }
        if roots
            .iter()
            .any(|&root: &usize| template_of(&nodes[root]) == Some(template))
        {
            return Err(AssetError::Hierarchy(format!(
                "duplicate rowing template {template}"
            )));
        }
        let mut parent = node.parent;
        while let Some(index) = parent {
            if extra_str(&nodes[index], "replayAssetKind") == Some("composite") {
                return Err(AssetError::Hierarchy(format!(
                    "rowing templates cannot be nested: {template}"
                )));
            }
            parent = nodes[index].parent;
        }
        roots.push(node.index);
    }

    let mut mesh_roles = Vec::new();
    let mut triangles = Vec::new();
    for template in ROWING_TEMPLATES {
        let Some(&root) = roots
            .iter()
            .find(|&&root| template_of(&nodes[root]) == Some(template))
        else {
            return Err(AssetError::Hierarchy(format!(
                "the rowing shell is missing {template}"
            )));
        };
        let node = &nodes[root];
        if node
            .extras
            .get("replayAssetVersion")
            .and_then(Value::as_u64)
            != Some(3)
        {
            return Err(template_error(template, "has an invalid version"));
        }
        if !is_identity(node) {
            return Err(template_error(
                template,
                "root must have identity transforms",
            ));
        }
        let declared = declared_roles(node, template)?;
        let part_count = node
            .extras
            .get("replayAssetPartCount")
            .and_then(Value::as_u64)
            .ok_or_else(|| template_error(template, "has an invalid part count"))?;
        let mut roles = Vec::new();
        let mut path = String::new();
        let before = mesh_roles.len();
        let meshes = u64::from(snapshot(
            &nodes,
            root,
            template,
            &mut roles,
            &mut mesh_roles,
            &mut path,
        )?);
        if meshes != part_count {
            return Err(template_error(
                template,
                format!("part count {part_count} does not match meshes ({meshes})"),
            ));
        }
        roles.sort();
        roles.dedup();
        if roles != declared {
            return Err(template_error(
                template,
                "material roles do not match meshes",
            ));
        }
        let mut total = 0;
        for role in &mesh_roles[before..] {
            let mesh_node = nodes
                .iter()
                .find(|node| node.name == role.name)
                .expect("snapshot names come from the node list");
            total += mesh_triangles(&json, mesh_node.mesh.expect("a mesh part"), &role.name)?;
        }
        triangles.push((template.to_owned(), total));
    }

    let leaves: Vec<_> = nodes
        .iter()
        .filter(|node| node.mesh.is_some() && extra_str(node, "replayAssetSlot").is_some())
        .collect();
    let blade = match leaves.as_slice() {
        [leaf] if extra_str(leaf, "replayAssetSlot") == Some(ROWING_BLADE_SLOT) => *leaf,
        _ => {
            return Err(AssetError::Leaf {
                slot: ROWING_BLADE_SLOT.to_owned(),
                reason: format!(
                    "the rowing shell must carry exactly this leaf, found {}",
                    leaves.len()
                ),
            });
        }
    };
    let blade_role = extra_str(blade, "replayMaterialRole").unwrap_or_default();
    mesh_roles.push(MeshNodeRole {
        name: blade.name.clone(),
        role: blade_role.to_owned(),
        template: None,
        slot: Some(ROWING_BLADE_SLOT.to_owned()),
    });
    let blade_triangles = mesh_triangles(&json, blade.mesh.expect("leaf mesh"), &blade.name)?;
    triangles.push((ROWING_BLADE_SLOT.to_owned(), blade_triangles));

    let stray = nodes
        .iter()
        .find(|node| node.mesh.is_some() && !mesh_roles.iter().any(|role| role.name == node.name));
    if let Some(node) = stray {
        return Err(AssetError::Hierarchy(format!(
            "mesh {} belongs to no rowing template or slot",
            node.description()
        )));
    }

    // The contract the scene maps materials and anchors onto: exactly the
    // vendored pack's rowing names, roles, templates and slots.
    let mut expected: Vec<_> = base
        .mesh_roles
        .iter()
        .filter(|role| is_rowing(role))
        .collect();
    let mut actual: Vec<_> = mesh_roles.iter().collect();
    expected.sort_by(|a, b| a.name.cmp(&b.name));
    actual.sort_by(|a, b| a.name.cmp(&b.name));
    if expected != actual {
        let missing: Vec<_> = expected
            .iter()
            .filter(|role| !actual.contains(role))
            .map(|role| format!("{} ({})", role.name, role.role))
            .collect();
        let extra: Vec<_> = actual
            .iter()
            .filter(|role| !expected.contains(role))
            .map(|role| format!("{} ({})", role.name, role.role))
            .collect();
        return Err(AssetError::Hierarchy(format!(
            "the rowing shell's names and roles drift from the V3 pack: missing [{}], unexpected [{}]",
            missing.join(", "),
            extra.join(", ")
        )));
    }

    triangles.sort();
    let count = |slot: &str| {
        triangles
            .iter()
            .find(|(name, _)| name == slot)
            .map_or(0, |(_, count)| *count)
    };
    let rendered = count("equipment:row:boat-assembly")
        + count("equipment:row:seat-carriage")
        + 2 * (count("equipment:row:oar-rig") + count(ROWING_BLADE_SLOT));
    if rendered > ROWING_TRIANGLE_BUDGET {
        return Err(AssetError::Geometry {
            node: "rowing shell".to_owned(),
            reason: format!(
                "{rendered} triangles as drawn exceeds the {ROWING_TRIANGLE_BUDGET} budget"
            ),
        });
    }
    Ok(RowingShell {
        byte_length: bytes.len(),
        triangles,
        rendered_triangles: rendered,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::anchors::OARLOCK_PIVOT;
    use crate::replay::glb::validate_v3;
    use crate::replay::pose::geometry::{ROW_GRIP_DROP, ROW_INBOARD_CONTACT};
    use rowplay_core::replay::row_equipment::{
        SCULL_GRIP_ANCHOR_FROM_END, SCULL_GRIP_LENGTH, SCULL_GRIP_RADIUS,
    };
    use serde_json::json;
    use std::path::PathBuf;

    fn assets() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay")
    }

    fn base() -> V3Library {
        validate_v3(&std::fs::read(assets().join("rowplay-rigs-v3.glb")).expect("V3 pack"))
            .expect("V3 pack validates")
    }

    fn shell_bytes() -> Vec<u8> {
        std::fs::read(assets().join("authored").join("rowing-shell.glb")).expect("rowing shell")
    }

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

    /// A synthetic pack with the V3 pack's own rowing names and roles.
    fn synthetic(base: &V3Library, triangles: u64) -> Value {
        let mut nodes = Vec::new();
        for template in ROWING_TEMPLATES {
            let parts: Vec<_> = base
                .mesh_roles
                .iter()
                .filter(|role| role.template.as_deref() == Some(template))
                .collect();
            let first = nodes.len() + 1;
            let mut roles: Vec<_> = parts.iter().map(|p| p.role.clone()).collect();
            roles.sort();
            roles.dedup();
            nodes.push(json!({
                "name": template,
                "children": (first..first + parts.len()).collect::<Vec<_>>(),
                "extras": {"replayAssetKind": "composite", "replayAssetTemplateSlot": template,
                           "replayAssetVersion": 3, "replayAssetPartCount": parts.len(),
                           "replayMaterialRoles": roles},
            }));
            for part in parts {
                nodes.push(json!({"name": part.name, "mesh": 0, "extras": {
                    "replayAssetTemplateSlot": template, "replayMaterialRole": part.role}}));
            }
        }
        nodes.push(json!({"name": ROWING_BLADE_SLOT, "mesh": 0, "extras": {
            "replayAssetSlot": ROWING_BLADE_SLOT, "replayAssetKind": "leaf",
            "replayMaterialRole": "equipment-painted"}}));
        json!({
            "nodes": nodes,
            "meshes": [{"primitives": [{"attributes": {"POSITION": 0}, "indices": 1}]}],
            "accessors": [{"count": 3, "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 1.0]},
                          {"count": triangles * 3}],
        })
    }

    #[test]
    fn a_pack_with_the_v3_rowing_names_and_roles_validates() {
        let base = base();
        let shell = validate_rowing_shell(&glb(&synthetic(&base, 1)), &base).expect("valid");
        // 18 boat and seat parts once, 5 oar parts and the blade twice.
        assert_eq!(shell.rendered_triangles, 18 + 2 * 6);
    }

    #[test]
    fn drift_from_the_v3_contract_fails_naming_it() {
        let base = base();
        let rename = |json: &mut Value| {
            json["nodes"][1]["name"] = json!("equipment:row:boat-assembly:canvas");
        };
        let recolour = |json: &mut Value| {
            json["nodes"][1]["extras"]["replayMaterialRole"] = json!("equipment-light");
        };
        let no_blade = |json: &mut Value| {
            json["nodes"].as_array_mut().expect("nodes").pop();
        };
        let ski = |json: &mut Value| {
            json["nodes"][0]["extras"]["replayAssetTemplateSlot"] =
                json!("equipment:ski:ski-assembly");
        };
        let moved = |json: &mut Value| {
            json["nodes"][0]["translation"] = json!([0.0, 0.1, 0.0]);
        };
        for (label, mutate, needle) in [
            (
                "renamed part",
                &rename as &dyn Fn(&mut Value),
                "drift from the V3 pack",
            ),
            ("changed role", &recolour, "drift from the V3 pack"),
            ("missing blade", &no_blade, "equipment:row:blade"),
            ("foreign template", &ski, "non-rowing composite"),
            ("moved root", &moved, "identity transforms"),
        ] {
            let mut json = synthetic(&base, 1);
            mutate(&mut json);
            let error = validate_rowing_shell(&glb(&json), &base).expect_err(label);
            assert!(error.to_string().contains(needle), "{label}: {error}");
        }
        let error =
            validate_rowing_shell(&glb(&synthetic(&base, 2001)), &base).expect_err("over budget");
        assert!(
            error.to_string().contains("exceeds the 60000 budget"),
            "{error}"
        );
    }

    #[test]
    fn the_authored_shell_honours_the_v3_rowing_contract_and_budget() {
        let shell = validate_rowing_shell(&shell_bytes(), &base()).expect("authored shell");
        assert!(shell.rendered_triangles <= ROWING_TRIANGLE_BUDGET);
        assert_eq!(shell.triangles.len(), 4);
    }

    /// POSITION values of one named mesh node (the pipeline's canonical
    /// layout: one embedded buffer, tightly packed float views).
    fn positions(name: &str) -> Vec<[f64; 3]> {
        let bytes = shell_bytes();
        let (json, bin) = chunks(&bytes).expect("container");
        let node = json["nodes"]
            .as_array()
            .expect("nodes")
            .iter()
            .find(|node| node["name"] == name)
            .unwrap_or_else(|| panic!("{name}"));
        let mesh =
            &json["meshes"][usize::try_from(node["mesh"].as_u64().expect("mesh")).expect("index")];
        let mut out = Vec::new();
        for primitive in mesh["primitives"].as_array().expect("primitives") {
            let accessor = &json["accessors"][usize::try_from(
                primitive["attributes"]["POSITION"].as_u64().expect("pos"),
            )
            .expect("index")];
            let view = &json["bufferViews"]
                [usize::try_from(accessor["bufferView"].as_u64().expect("view")).expect("index")];
            let start = usize::try_from(
                view["byteOffset"].as_u64().unwrap_or(0)
                    + accessor["byteOffset"].as_u64().unwrap_or(0),
            )
            .expect("offset");
            for i in 0..usize::try_from(accessor["count"].as_u64().expect("count")).expect("count")
            {
                let at = start + i * 12;
                let read = |k: usize| {
                    f64::from(f32::from_le_bytes(
                        bin[at + 4 * k..at + 4 * k + 4].try_into().expect("f32"),
                    ))
                };
                out.push([read(0), read(1), read(2)]);
            }
        }
        out
    }

    #[test]
    fn the_oarlock_pins_stand_on_the_animated_pivots() {
        let pins = positions("equipment:row:boat-assembly:oarlocks");
        for side in [1.0, -1.0] {
            let mine: Vec<_> = pins.iter().filter(|p| p[0] * side > 0.0).collect();
            let span = |k: usize| {
                let lo = mine.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min);
                let hi = mine.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max);
                (lo + hi) / 2.0
            };
            assert!(
                (span(0) - side * f64::from(OARLOCK_PIVOT[0])).abs() < 1e-4,
                "x {}",
                span(0)
            );
            assert!(
                (span(2) - f64::from(OARLOCK_PIVOT[2])).abs() < 1e-4,
                "z {}",
                span(2)
            );
        }
    }

    #[test]
    fn the_grip_is_where_the_hands_close() {
        // Hand contact 0.78 inboard, 0.04 from the handle end, on a 0.32 m grip.
        let start = -(ROW_INBOARD_CONTACT + SCULL_GRIP_ANCHOR_FROM_END);
        let end = start + SCULL_GRIP_LENGTH;
        let grip = positions("equipment:row:oar-rig:grip");
        let lo = grip.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
        let hi = grip.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (lo - start).abs() < 1e-4 && (hi - end).abs() < 1e-4,
            "grip {lo}..{hi}"
        );
        let radius = grip
            .iter()
            .map(|p| (p[1] - ROW_GRIP_DROP).hypot(p[2]))
            .fold(0.0, f64::max);
        assert!(
            (SCULL_GRIP_RADIUS..=SCULL_GRIP_RADIUS + 0.0006).contains(&radius),
            "grip radius {radius}"
        );
    }

    #[test]
    fn the_hull_is_the_7_8_m_shell() {
        let hull = positions("equipment:row:boat-assembly:hull");
        let lo = hull.iter().map(|p| p[2]).fold(f64::INFINITY, f64::min);
        let hi = hull.iter().map(|p| p[2]).fold(f64::NEG_INFINITY, f64::max);
        assert!((hi - lo - 7.8).abs() < 0.002, "hull length {}", hi - lo);
    }
}
