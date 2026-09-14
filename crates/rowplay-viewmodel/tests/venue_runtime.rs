// SPDX-License-Identifier: GPL-3.0-or-later
//! Phase 6b runtime-plan tests: bucketing is deterministic and complete
//! (every contract instance lands in exactly one bucket), and every vendored
//! contract produces a plan the scene can build — texture sources resolve to
//! vendored files, component names match the build's `PACKS` rows.

use rowplay_viewmodel::replay::venue_runtime::{INSTANCE_BUCKETS, bucket_instances, venue_plan};
use serde_json::{Value, json};

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

fn contract(sport: &str, tier: &str) -> Value {
    let path = venue_dir().join(format!("rowplay-venue-{sport}-{tier}.json"));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).expect("contract JSON")
}

#[test]
fn every_vendored_contract_yields_a_plan() {
    for sport in SPORTS {
        for tier in TIERS {
            let plan = venue_plan(sport, tier, &contract(sport, tier))
                .unwrap_or_else(|error| panic!("{sport}/{tier}: {error}"));
            assert_eq!(plan.sport, sport);
            assert_eq!(plan.quality, tier);
            assert_eq!(
                plan.component,
                format!("Rowplay_venue_{sport}_{tier}"),
                "{sport}/{tier}: component name"
            );
            assert!(!plan.materials.as_object().expect("materials").is_empty());

            // Completeness: every instance of the contract lands in exactly
            // one bucket, transforms preserved.
            let contract = contract(sport, tier);
            let instancing = contract
                .get("instancing")
                .and_then(Value::as_object)
                .expect("instancing");
            for (group, list) in instancing {
                let expected = list.as_array().expect("list").len();
                let plan_group = plan
                    .instance_groups
                    .iter()
                    .find(|plan_group| plan_group.node == *group)
                    .unwrap_or_else(|| panic!("{sport}/{tier}: group {group} missing from plan"));
                let total: usize = plan_group
                    .buckets
                    .iter()
                    .map(|bucket| bucket.transforms.len())
                    .sum();
                assert_eq!(total, expected, "{sport}/{tier}: {group} instance count");
                assert!(
                    plan_group.buckets.len() <= INSTANCE_BUCKETS,
                    "{sport}/{tier}: {group} exceeds the bucket cap"
                );
            }
        }
    }
}

#[test]
fn texture_sources_resolve_to_vendored_files() {
    let dir = venue_dir();
    let assets = dir.parent().expect("assets/replay");
    for sport in SPORTS {
        for tier in ["high", "ultra"] {
            let plan = venue_plan(sport, tier, &contract(sport, tier)).unwrap();
            for (name, material) in plan.materials.as_object().expect("materials") {
                let textures = material
                    .get("textures")
                    .and_then(Value::as_object)
                    .expect("textures object");
                for (slot, texture) in textures {
                    let source = texture
                        .get("source")
                        .and_then(Value::as_str)
                        .unwrap_or_else(|| panic!("{sport}/{tier}: {name}.{slot} has no source"));
                    let rel = source
                        .strip_prefix("qrc:/qt/qml/RowPlay/Environments/")
                        .unwrap_or_else(|| panic!("{source}: unexpected prefix"));
                    // Set textures map to environments/<family>/<file>;
                    // procedural ones to venues/procedural/<file>.
                    let disk = if rel.starts_with("procedural/") {
                        assets.join("venues").join(rel)
                    } else {
                        assets.join(rel)
                    };
                    assert!(
                        disk.is_file(),
                        "{sport}/{tier}: {name}.{slot} → {} does not exist",
                        disk.display()
                    );
                }
            }
        }
    }
}

#[test]
fn low_and_medium_plans_bind_no_texture_sets() {
    for sport in SPORTS {
        for tier in ["low", "medium"] {
            let plan = venue_plan(sport, tier, &contract(sport, tier)).unwrap();
            for (name, material) in plan.materials.as_object().expect("materials") {
                if let Some(textures) = material.get("textures").and_then(Value::as_object) {
                    for (slot, texture) in textures {
                        assert_ne!(
                            texture.get("kind").and_then(Value::as_str),
                            Some("set"),
                            "{sport}/{tier}: {name}.{slot} binds a set against the \
                             environments README (Low/Medium load no sets)"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn bucketing_splits_tints_across_the_two_axes() {
    let instances = vec![
        json!({"p": [0.0, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0],
               "c": [1.2, 1.2, 1.2]}), // light, neutral (warmth 0 → warm cell)
        json!({"p": [1.0, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0],
               "c": [0.8, 0.8, 0.8]}), // dark, neutral (same boundary)
        json!({"p": [2.0, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0],
               "c": [1.1, 1.0, 0.9]}), // light, warm — joins the first cell
        json!({"p": [3.0, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0],
               "c": [0.9, 1.0, 1.1]}), // dark, cool
        json!({"p": [4.0, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0]}), // untinted
    ];
    let plan = bucket_instances("environment:rower:test", &instances).unwrap();
    assert_eq!(plan.node, "environment:rower:test");
    assert_eq!(
        plan.buckets.len(),
        4,
        "three tint cells (the two neutral-tint instances share the warm cell) \
         plus the trailing neutral bucket"
    );
    assert_eq!(
        plan.buckets[3].transforms.len(),
        1,
        "the untinted instance lands in the trailing neutral bucket"
    );
    assert_eq!(plan.buckets[3].tint, [1.0, 1.0, 1.0]);
    // Light buckets sort before dark ones.
    let lightness = |bucket: &rowplay_viewmodel::replay::venue_runtime::InstanceBucket| {
        (bucket.tint[0] + bucket.tint[1] + bucket.tint[2]) / 3.0
    };
    assert!(lightness(&plan.buckets[0]) >= lightness(&plan.buckets[2]));
}

#[test]
fn bucketing_is_deterministic() {
    let instances: Vec<Value> = (0..40)
        .map(|index| {
            let t = f64::from(index) / 40.0;
            json!({"p": [t, 0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0],
                   "c": [0.8 + t * 0.4, 1.0, 1.0]})
        })
        .collect();
    let first = bucket_instances("g", &instances).unwrap();
    let second = bucket_instances("g", &instances).unwrap();
    assert_eq!(first, second, "bucketing must not depend on map order");
}

#[test]
fn malformed_instances_name_the_group() {
    let instances = vec![json!({"p": [0.0, 0.0], "q": [0.0, 0.0, 0.0, 1.0], "s": [1.0, 1.0, 1.0]})];
    let error = bucket_instances("environment:rower:broken", &instances).unwrap_err();
    assert!(
        error.to_string().contains("environment:rower:broken"),
        "error must name the group: {error}"
    );
}
