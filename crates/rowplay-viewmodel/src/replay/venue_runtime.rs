// SPDX-License-Identifier: GPL-3.0-or-later
//! Runtime plans for the baked venues (Phase 6b, spec R2).
//!
//! `build.rs` feeds each vendored contract through [`venue_plan`] and embeds
//! the result; the `Replay` singleton exposes it to the scene, which builds
//! materials and bucketed `InstanceList`s from it. Qt-free and unit-tested.
//!
//! Per-instance colour tints are delivered without custom shaders: each
//! instance group's tints are quantised onto a fixed 2×2 grid — lightness
//! below/above 1.0 × warm/cool skew — producing at most
//! [`INSTANCE_BUCKETS`] buckets per group, each drawn with its own averaged
//! tint. This mirrors the web `scatterTint` spread/warmth semantics (its
//! whole purpose is breaking up repeated-stamp tree lines) while keeping
//! plain `PrincipledMaterial`s and one `InstanceList` per bucket.

use serde_json::{Map, Value, json};

/// Maximum buckets per instance group (2 lightness × 2 warmth; groups whose
/// tints land on one side collapse to fewer).
pub const INSTANCE_BUCKETS: usize = 4;

/// A flavour of plan-build failure. Build-time embedding panics with the
/// named file; the tests assert the messages.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlanError {
    /// The contract lacks a required field or carries an unexpected value.
    #[error("venue contract {file}: {reason}")]
    Contract {
        /// Which vendored file failed.
        file: String,
        /// What failed.
        reason: String,
    },
}

use thiserror::Error;

/// One shade bucket: an averaged tint and the transforms of every instance
/// quantised into it.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceBucket {
    /// Averaged linear tint (the contract tints are linear multipliers).
    pub tint: [f64; 3],
    /// Per instance: `[px, py, pz, qx, qy, qz, qw, sx, sy, sz]`.
    pub transforms: Vec<[f64; 10]>,
}

/// One instanced archetype and its shade buckets.
#[derive(Debug, Clone, PartialEq)]
pub struct InstanceGroupPlan {
    /// The archetype node's `objectName`.
    pub node: String,
    /// Buckets, ordered light-to-dark then cool-to-warm (stable for tests).
    pub buckets: Vec<InstanceBucket>,
}

/// The complete runtime plan for one venue variant.
#[derive(Debug, Clone, PartialEq)]
pub struct VenuePlan {
    /// Sport the venue is authored for.
    pub sport: String,
    /// Quality tier the venue was baked at.
    pub quality: String,
    /// The balsam component exporting this variant.
    pub component: String,
    /// Materials as the scene should build them (JSON, contract-shaped).
    pub materials: Value,
    /// Instance groups with their buckets.
    pub instance_groups: Vec<InstanceGroupPlan>,
}

impl VenuePlan {
    /// JSON for the `Replay` singleton's QML consumers.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "sport": self.sport,
            "quality": self.quality,
            "component": self.component,
            "materials": self.materials,
            "instanceGroups": self.instance_groups.iter().map(|group| json!({
                "node": group.node,
                "buckets": group.buckets.iter().map(|bucket| json!({
                    "tint": bucket.tint,
                    "transforms": bucket.transforms,
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }
}

/// Balsam component name for a variant (mirrors the `PACKS` rows in the
/// app's `build.rs`; balsam derives the output name from the GLB stem).
#[must_use]
pub fn component_name(sport: &str, quality: &str) -> String {
    format!("Rowplay_venue_{sport}_{quality}")
}

/// Quantise a group's instances into at most [`INSTANCE_BUCKETS`] shade
/// buckets. Untinted instances (no `c` in the contract) land in one neutral
/// bucket. Buckets sort light→dark then cool→warm so the output is stable.
///
/// # Errors
/// When an instance record is missing its transform fields.
pub fn bucket_instances(group: &str, instances: &[Value]) -> Result<InstanceGroupPlan, PlanError> {
    let mut tinted: Vec<([f64; 3], [f64; 10])> = Vec::new();
    let mut neutral: Vec<[f64; 10]> = Vec::new();
    for instance in instances {
        let read3 = |key: &str| -> Result<[f64; 3], PlanError> {
            instance
                .get(key)
                .and_then(Value::as_array)
                .and_then(|list| {
                    let out: Option<Vec<f64>> =
                        list.iter().map(Value::as_f64).collect::<Option<_>>();
                    out
                })
                .filter(|values| values.len() == 3)
                .map(|values| [values[0], values[1], values[2]])
                .ok_or_else(|| PlanError::Contract {
                    file: group.to_owned(),
                    reason: format!("instance {key} is not [x, y, z]"),
                })
        };
        let read4 = |key: &str| -> Result<[f64; 4], PlanError> {
            instance
                .get(key)
                .and_then(Value::as_array)
                .and_then(|list| {
                    let out: Option<Vec<f64>> =
                        list.iter().map(Value::as_f64).collect::<Option<_>>();
                    out
                })
                .filter(|values| values.len() == 4)
                .map(|values| [values[0], values[1], values[2], values[3]])
                .ok_or_else(|| PlanError::Contract {
                    file: group.to_owned(),
                    reason: format!("instance {key} is not a quaternion"),
                })
        };
        let p = read3("p")?;
        let q = read4("q")?;
        let s = read3("s")?;
        let mut transform = [0.0; 10];
        transform[..3].copy_from_slice(&p);
        transform[3..7].copy_from_slice(&q);
        transform[7..10].copy_from_slice(&s);
        match instance.get("c").and_then(|tint| {
            tint.as_array().and_then(|list| {
                let out: Option<Vec<f64>> = list.iter().map(Value::as_f64).collect::<Option<_>>();
                out
            })
        }) {
            Some(tint) if tint.len() == 3 => tinted.push(([tint[0], tint[1], tint[2]], transform)),
            _ => neutral.push(transform),
        }
    }

    // 2 × 2 quantisation: lightness (mean channel) below/above 1.0, warmth
    // (red − blue skew) below/above 0. Exactly the scatterTint axes.
    struct Cell {
        key: (bool, bool),
        tint_sum: [f64; 3],
        transforms: Vec<[f64; 10]>,
    }
    let mut cells: Vec<Cell> = Vec::new();
    for (tint, transform) in tinted {
        let lightness = (tint[0] + tint[1] + tint[2]) / 3.0;
        let warmth = tint[0] - tint[2];
        let key = (lightness >= 1.0, warmth >= 0.0);
        if let Some(cell) = cells.iter_mut().find(|cell| cell.key == key) {
            cell.tint_sum[0] += tint[0];
            cell.tint_sum[1] += tint[1];
            cell.tint_sum[2] += tint[2];
            cell.transforms.push(transform);
        } else {
            cells.push(Cell {
                key,
                tint_sum: tint,
                transforms: vec![transform],
            });
        }
    }
    let mut buckets: Vec<InstanceBucket> = cells
        .into_iter()
        .map(|cell| {
            let count = cell.transforms.len() as f64;
            InstanceBucket {
                tint: [
                    cell.tint_sum[0] / count,
                    cell.tint_sum[1] / count,
                    cell.tint_sum[2] / count,
                ],
                transforms: cell.transforms,
            }
        })
        .collect();
    // Light → dark, then cool → warm, for a stable order.
    buckets.sort_by(|left, right| {
        let left_light = (left.tint[0] + left.tint[1] + left.tint[2]) / 3.0;
        let right_light = (right.tint[0] + right.tint[1] + right.tint[2]) / 3.0;
        right_light
            .partial_cmp(&left_light)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                (left.tint[0] - left.tint[2])
                    .partial_cmp(&(right.tint[0] - right.tint[2]))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });
    if !neutral.is_empty() {
        buckets.push(InstanceBucket {
            tint: [1.0, 1.0, 1.0],
            transforms: neutral,
        });
    }
    Ok(InstanceGroupPlan {
        node: group.to_owned(),
        buckets,
    })
}

/// Build the runtime plan from one vendored contract JSON.
///
/// # Errors
/// When the contract is missing required fields (a bake or vendor bug; the
/// 6a validator should catch those first).
pub fn venue_plan(sport: &str, quality: &str, contract: &Value) -> Result<VenuePlan, PlanError> {
    let fail = |reason: String| PlanError::Contract {
        file: format!("rowplay-venue-{sport}-{quality}.json"),
        reason,
    };
    let instancing = contract
        .get("instancing")
        .and_then(Value::as_object)
        .ok_or_else(|| fail("missing instancing".to_owned()))?;
    let mut materials = contract
        .get("materials")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| fail("missing materials".to_owned()))?;

    // Rewrite each material into the shape the scene builds from: resolved
    // rcc texture paths and flat colour fields.
    let mut rewritten = Map::new();
    for (name, material) in &materials {
        let mut entry = material.clone();
        let color = entry
            .get("color")
            .ok_or_else(|| fail(format!("material {name}: missing color")))?;
        let color_light = color
            .get("light")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(format!("material {name}: missing color.light")))?
            .to_owned();
        let color_dark = color
            .get("dark")
            .and_then(Value::as_str)
            .ok_or_else(|| fail(format!("material {name}: missing color.dark")))?
            .to_owned();
        entry["colorLight"] = color_light.into();
        entry["colorDark"] = color_dark.into();
        entry
            .as_object_mut()
            .expect("material is an object")
            .remove("color");
        if let Some(textures) = entry.get_mut("textures").and_then(Value::as_object_mut) {
            for (_slot, texture) in textures.iter_mut() {
                let kind = texture
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let qrc = match kind {
                    "set" => {
                        // The rcc mirrors assets/replay's layout under the
                        // Environments prefix (environments/<family>/<file>).
                        let path = texture
                            .get("path")
                            .and_then(Value::as_str)
                            .and_then(|path| path.strip_prefix("/replay-assets/"))
                            .ok_or_else(|| {
                                fail(format!("material {name}: set texture without a set path"))
                            })?;
                        format!("qrc:/qt/qml/RowPlay/Environments/{path}")
                    }
                    "procedural" => {
                        let png = texture
                            .get("png")
                            .and_then(Value::as_str)
                            .and_then(|png| png.strip_prefix("procedural/"))
                            .ok_or_else(|| {
                                fail(format!("material {name}: procedural texture without png"))
                            })?;
                        format!("qrc:/qt/qml/RowPlay/Environments/procedural/{png}")
                    }
                    other => {
                        return Err(fail(format!(
                            "material {name}: unknown texture kind {other}"
                        )));
                    }
                };
                texture["source"] = qrc.into();
            }
        }
        rewritten.insert(name.clone(), entry);
    }
    materials = rewritten;

    let mut instance_groups = Vec::with_capacity(instancing.len());
    for (group, list) in instancing {
        let instances = list
            .as_array()
            .ok_or_else(|| fail(format!("instance group {group} is not a list")))?;
        instance_groups.push(bucket_instances(group, instances)?);
    }
    instance_groups.sort_by(|left, right| left.node.cmp(&right.node));

    Ok(VenuePlan {
        sport: sport.to_owned(),
        quality: quality.to_owned(),
        component: component_name(sport, quality),
        materials: Value::Object(materials),
        instance_groups,
    })
}

/// [`venue_shadow_flags`]: the mesh casts the key light's shadow.
pub const SHADOW_CASTS: u8 = 1;
/// [`venue_shadow_flags`]: the mesh receives the key light's shadow.
pub const SHADOW_RECEIVES: u8 = 2;

/// The venue meshes the web lets cast the key light's shadow, by name, `#`
/// standing for an index. `renderer3dEnvironment.ts`: `addOverheadSpan`
/// flags the legs and the deck, `addTrackEdgePosts` the posts (not their
/// boards), `addRowerFinishTower` every part of the tower.
const SHADOW_CASTERS: &[&str] = &[
    "environment:bike:finish-gantry-deck",
    "environment:bike:finish-gantry-leg-#",
    "environment:bike:rail-posts",
    "environment:rower:course-bridge-deck",
    "environment:rower:course-bridge-leg-#",
    "environment:rower:distance-posts",
    "environment:rower:finish-tower-cabin",
    "environment:rower:finish-tower-glazing",
    "environment:rower:finish-tower-mast",
    "environment:rower:finish-tower-shaft",
    "environment:rower:finish-tower-wing",
    "environment:rower:wetland-boardwalk-posts",
    "environment:skierg:piste-poles",
    "environment:skierg:timing-arch-deck",
    "environment:skierg:timing-arch-leg-#",
];

/// The venue meshes the web lets receive the key light's shadow, by name,
/// `#` standing for an index: every arc the builder makes
/// (`makeHorizontalArc` in `renderer3dEnvironment.ts`, `makeVerticalArc` in
/// `renderer3d.ts`), the renderer's infield and apron, and the few surfaces
/// the builder flags itself (the island, the ski stadium field, start pad
/// and snowbank, the bike's infield floor).
const SHADOW_RECEIVERS: &[&str] = &[
    "environment:bike:ad-band",
    "environment:bike:apron",
    "environment:bike:arena-wall-bay-#",
    "environment:bike:infield",
    "environment:bike:infield-floor",
    "environment:bike:stands-tier-#-sector-#",
    "environment:bike:track-boards-inner",
    "environment:bike:track-boards-outer",
    "environment:rower:apron",
    "environment:rower:bank-terrace-#",
    "environment:rower:campus-path",
    "environment:rower:earth-bank-#",
    "environment:rower:grass-bank-#",
    "environment:rower:island-core",
    "environment:rower:island-lawn",
    "environment:rower:mist-band-#",
    "environment:rower:reflection-band-#",
    "environment:rower:vista-shingle",
    "environment:rower:vista-wet-edge",
    "environment:rower:water-ripple-#",
    "environment:rower:waterline-#",
    "environment:rower:wet-edge-#",
    "environment:rower:wetland-boardwalk-deck",
    "environment:skierg:apron",
    "environment:skierg:snowbank",
    "environment:skierg:stadium-field",
    "environment:skierg:start-chute",
    "environment:skierg:start-pad",
    "environment:skierg:terrace-step-#",
    "environment:skierg:valley-shadow-#",
    "environment:skierg:valley-sun-#",
    "environment:skierg:wind-lip-#",
    // The generic infields of the RowErg and SkiErg are hidden, so only the
    // bike's reaches a GLB; the rule still names what the renderer flags.
    "environment:rower:infield",
    "environment:skierg:infield",
];

/// The name with every index replaced by `#`: a run of digits after a `-`
/// that ends the name or another `-`.
fn venue_mesh_family(name: &str) -> String {
    let mut family = String::with_capacity(name.len());
    for (i, part) in name.split('-').enumerate() {
        if i > 0 {
            family.push('-');
        }
        if i > 0 && !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()) {
            family.push('#');
        } else {
            family.push_str(part);
        }
    }
    family
}

/// Whether a baked venue mesh casts and receives the key light's shadow, as
/// a mask of [`SHADOW_CASTS`] and [`SHADOW_RECEIVES`]: the web's flags,
/// which the GLB bake does not carry (`tests/fixtures/
/// replay-venue-shadow-parity.json` records them). three.js meshes cast and
/// receive nothing unless flagged, a Qt Quick 3D `Model` both by default:
/// unflagged, the BikeErg roof shell shadowed the whole track and the snow
/// field shadowed itself (#96). Only High and Ultra cast a shadow at all.
#[must_use]
pub fn venue_shadow_flags(name: &str) -> u8 {
    let family = venue_mesh_family(name);
    let mut flags = 0;
    if SHADOW_CASTERS.contains(&family.as_str()) {
        flags |= SHADOW_CASTS;
    }
    if SHADOW_RECEIVERS.contains(&family.as_str()) {
        flags |= SHADOW_RECEIVES;
    }
    flags
}
