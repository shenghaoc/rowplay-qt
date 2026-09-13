// SPDX-License-Identifier: GPL-3.0-or-later
//! Phase 6a venue contract tests: the vendored pairs validate, and every
//! class of defect fails with a message naming the file, node or material.
//!
//! The vendored test is gated on the repository layout (`CARGO_MANIFEST_DIR`)
//! like `glb.rs`'s `the_vendored_rig_pack_validates`; the app crate's
//! `build.rs` runs the same validator over the same bytes at build time.

use rowplay_viewmodel::replay::venue::{VenueError, validate_venue};

use std::path::PathBuf;

fn venue_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("replay")
        .join("venues")
}

use std::path::Path;

const SPORTS: [&str; 3] = ["rower", "skierg", "bike"];
const TIERS: [&str; 4] = ["low", "medium", "high", "ultra"];

#[test]
fn every_vendored_venue_validates_against_its_contract() {
    let dir = venue_dir();
    for sport in SPORTS {
        for tier in TIERS {
            let stem = format!("rowplay-venue-{sport}-{tier}");
            let glb = std::fs::read(dir.join(format!("{stem}.glb"))).unwrap_or_else(|error| {
                panic!(
                    "vendored venue missing: {stem}.glb ({error}); run tools/bake-venues/bake.sh"
                )
            });
            let contract = std::fs::read_to_string(dir.join(format!("{stem}.json")))
                .unwrap_or_else(|error| panic!("vendored contract missing: {stem}.json ({error})"));
            let package =
                validate_venue(&glb, &contract).unwrap_or_else(|error| panic!("{stem}: {error}"));
            assert_eq!(package.sport, sport, "{stem}: sport");
            assert_eq!(package.quality, tier, "{stem}: tier");
            assert_eq!(package.seed, 20_260_913, "{stem}: bake seed");
            assert!(!package.nodes.is_empty(), "{stem}: venue has no nodes");
            assert!(
                !package.instance_groups.is_empty(),
                "{stem}: venue has no instance groups (trees/berms should instance)"
            );
            // The root plus at least one named mesh.
            assert!(
                package
                    .nodes
                    .iter()
                    .any(|name| name == &format!("venue-{sport}-{tier}")),
                "{stem}: root node missing"
            );
            // Bounds are within the plausible extent (checked in the validator)
            // and the venue actually has vertical relief.
            let height = package.bounds.1[1] - package.bounds.0[1];
            assert!(
                height > 1.0,
                "{stem}: venue has no vertical relief ({height} m)"
            );
        }
    }
}

/// The hidden infield underlay is recorded in the contract, never in the GLB.
#[test]
fn hidden_meshes_are_recorded_not_shipped() {
    let dir = venue_dir();
    for sport in ["rower", "skierg"] {
        let stem = format!("rowplay-venue-{sport}-low");
        let glb = std::fs::read(dir.join(format!("{stem}.glb"))).unwrap();
        let contract = std::fs::read_to_string(dir.join(format!("{stem}.json"))).unwrap();
        let package = validate_venue(&glb, &contract).unwrap();
        assert!(
            package.hidden.iter().any(|name| name.ends_with(":infield")),
            "{stem}: the hidden generic infield should be recorded"
        );
        assert!(
            !package.nodes.iter().any(|name| name.ends_with(":infield")),
            "{stem}: the hidden infield must not be in the GLB"
        );
    }
}

#[test]
fn tiers_grow_monotonically() {
    let dir = venue_dir();
    for sport in SPORTS {
        let mut previous = 0;
        for tier in TIERS {
            let stem = format!("rowplay-venue-{sport}-{tier}");
            let glb = std::fs::read(dir.join(format!("{stem}.glb"))).unwrap();
            let contract = std::fs::read_to_string(dir.join(format!("{stem}.json"))).unwrap();
            let package = validate_venue(&glb, &contract).unwrap();
            let count = package.mesh_count;
            assert!(
                count >= previous,
                "{sport}: {tier} has fewer meshes ({count}) than the tier below ({previous})"
            );
            previous = count;
        }
    }
}

// --- Synthesised defects -------------------------------------------------

fn minimal_contract(sport: &str, quality: &str) -> String {
    format!(
        r#"{{"format":1,"seed":20260913,"sport":"{sport}","quality":"{quality}","innerR":22.0,"outerR":34.0,"materials":{{}},"instancing":{{}},"hidden":[]}}"#
    )
}

/// Wraps a glTF JSON string in a minimal, correctly-sized GLB container.
fn glb_from_json(json: &str) -> Vec<u8> {
    let json_bytes = json.as_bytes();
    let padded = (json_bytes.len() + 3) & !3;
    let total = 12 + 8 + padded;
    let mut bytes = Vec::with_capacity(total);
    bytes.extend_from_slice(b"glTF");
    bytes.extend_from_slice(&2u32.to_le_bytes());
    bytes.extend_from_slice(&(total as u32).to_le_bytes());
    bytes.extend_from_slice(&(padded as u32).to_le_bytes());
    bytes.extend_from_slice(b"JSON");
    bytes.extend_from_slice(json_bytes);
    bytes.resize(total, b' ');
    bytes
}

fn minimal_glb(node_name: &str, material_name: Option<&str>) -> Vec<u8> {
    let material = material_name.map_or(String::new(), |name| {
        format!(r#","materials":[{{"name":"{name}"}}]"#)
    });
    let json = format!(
        r#"{{"asset":{{"version":"2.0"}},"nodes":[{{"name":"{node_name}"}}],"meshes":[{{"primitives":[{{"attributes":{{"POSITION":0}}}}]}}],"accessors":[{{"type":"VEC3","min":[-1.0,-1.0,-1.0],"max":[1.0,1.0,1.0]}}]{material}}}"#
    );
    glb_from_json(&json)
}

#[test]
fn unnamed_node_fails() {
    let json = r#"{"asset":{"version":"2.0"},"nodes":[{"name":""}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"type":"VEC3","min":[-1.0,-1.0,-1.0],"max":[1.0,1.0,1.0]}]}"#;
    let error =
        validate_venue(&glb_from_json(json), &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Node { .. }),
        "expected a node error, got {error}"
    );
}

#[test]
fn wrong_prefix_fails() {
    let glb = minimal_glb("environment:skierg:thing", None);
    let error = validate_venue(&glb, &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Node { ref node, .. } if node == "environment:skierg:thing"),
        "expected a prefix error naming the node, got {error}"
    );
}

#[test]
fn unnamed_material_fails() {
    let glb = minimal_glb("environment:rower:thing", Some(""));
    let error = validate_venue(&glb, &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Material { .. }),
        "expected a material error, got {error}"
    );
}

#[test]
fn material_missing_from_contract_fails() {
    let glb = minimal_glb(
        "environment:rower:thing",
        Some("environment:rower:grass-material"),
    );
    let error = validate_venue(&glb, &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Material { ref material, .. } if material == "environment:rower:grass-material"),
        "expected a material error naming it, got {error}"
    );
}

#[test]
fn contract_material_missing_from_glb_fails() {
    let glb = minimal_glb("environment:rower:thing", None);
    let contract = minimal_contract("rower", "low").replace(
        r#""materials":{}"#,
        r#""materials":{"environment:rower:ghost-material":{}}"#,
    );
    let error = validate_venue(&glb, &contract).unwrap_err();
    assert!(
        matches!(error, VenueError::Material { ref material, .. } if material == "environment:rower:ghost-material"),
        "expected an orphan-contract-material error, got {error}"
    );
}

#[test]
fn embedded_images_fail() {
    let base = r#"{"asset":{"version":"2.0"},"nodes":[{"name":"environment:rower:thing"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"type":"VEC3","min":[-1.0,-1.0,-1.0],"max":[1.0,1.0,1.0]}]}"#;
    let with_image = base.replace(
        r#""nodes""#,
        r#""images":[{"uri":"data:image/png;base64,AA=="}],"nodes""#,
    );
    let error = validate_venue(
        &glb_from_json(&with_image),
        &minimal_contract("rower", "low"),
    )
    .unwrap_err();
    assert!(
        matches!(error, VenueError::EmbeddedImage(1)),
        "expected an embedded-image error, got {error}"
    );
}

#[test]
fn out_of_range_bounds_fail() {
    let base = r#"{"asset":{"version":"2.0"},"nodes":[{"name":"environment:rower:thing"}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"accessors":[{"type":"VEC3","min":[-1.0,-1.0,-1.0],"max":[1.0,1.0,999.0]}]}"#;
    let error =
        validate_venue(&glb_from_json(base), &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Bounds(_)),
        "expected a bounds error, got {error}"
    );
}

#[test]
fn inventory_mismatch_fails() {
    let glb = minimal_glb("environment:rower:thing", None);
    let contract = minimal_contract("rower", "low").replace(
        r#""hidden":[]"#,
        r#""hidden":[],"inventory":{"nodes":99,"meshes":1,"instancedMeshes":0,"instances":0,"materials":0}"#,
    );
    let error = validate_venue(&glb, &contract).unwrap_err();
    assert!(
        matches!(error, VenueError::Inventory(_)),
        "expected an inventory error, got {error}"
    );
}

#[test]
fn unreadable_container_fails() {
    let error = validate_venue(b"not a glb", &minimal_contract("rower", "low")).unwrap_err();
    assert!(
        matches!(error, VenueError::Container(_)),
        "expected a container error, got {error}"
    );
}
