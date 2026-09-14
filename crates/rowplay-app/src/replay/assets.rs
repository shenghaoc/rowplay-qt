// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay asset metadata (Phase 5a spec R2).
//!
//! The scene instantiates `balsam`-generated QML components from the
//! generated `RowPlay.ReplayAssets` module (bundled by `build.rs`), because a
//! `RuntimeLoader` scene is not addressable from QML — no `objectName` on the
//! imported nodes and no traversable `children` list — so neither the
//! material-role walk nor Phase 5b joint posing could reach it
//! (docs/qt-bridges-notes.md). `balsam`'s components name every node and
//! every `Joint` of the athlete's skin with the contract's bone names.
//!
//! `build.rs` validates the exact pack bytes it converts; development
//! (`ROWPLAY_REPLAY_ASSETS`, or the repository assets in debug builds)
//! re-validates the on-disk pack at startup so an edited asset fails loudly
//! with its slot name instead of rendering wrong.

use rowplay_viewmodel::replay::glb::{self, V3Library};

/// Metadata embedded by `build.rs` (validated V3 manifest + name → role map).
pub struct Meta {
    raw: serde_json::Value,
}

impl Meta {
    /// The build-time metadata of the bundled pack.
    #[must_use]
    pub fn embedded() -> Meta {
        let raw: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/replay_assets_meta.json"
        )))
        .expect("replay_assets_meta.json");
        Meta { raw }
    }
    /// `balsam`: the generated-component path (the only shipped mode).
    #[must_use]
    pub fn asset_mode(&self) -> &str {
        self.raw["assetMode"].as_str().expect("assetMode")
    }

    /// The name → role map as a JSON object for QML.
    #[must_use]
    pub fn mesh_roles(&self) -> serde_json::Value {
        self.raw["meshRoles"].clone()
    }

    /// A leaf slot's mesh bounds (`min`, `max`), for the runtime shell fits.
    #[must_use]
    pub fn leaf_bounds(&self, slot: &str) -> Option<([f64; 3], [f64; 3])> {
        let leaf = self.raw["leaves"]
            .as_array()?
            .iter()
            .find(|leaf| leaf["slot"].as_str() == Some(slot))?;
        let read = |index: usize| -> Option<[f64; 3]> {
            let values = leaf["bounds"].get(index)?.as_array()?;
            Some([
                values.first()?.as_f64()?,
                values.get(1)?.as_f64()?,
                values.get(2)?.as_f64()?,
            ])
        };
        Some((read(0)?, read(1)?))
    }

    /// Every node name the loaded scene must contain (roots + leaves).
    #[must_use]
    pub fn scene_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.raw["templates"]
            .as_array()
            .expect("templates")
            .iter()
            .filter_map(|entry| entry["template"].as_str().map(ToOwned::to_owned))
            .collect();
        names.extend(
            self.raw["meshRoles"]
                .as_object()
                .expect("meshRoles")
                .keys()
                .cloned(),
        );
        names.sort();
        names.dedup();
        names
    }
}

/// The V4 athlete embedded by `build.rs` (skin joints, rest hierarchy, the
/// three clips), already cross-checked against the vendored contract.
#[must_use]
pub fn embedded_athlete() -> rowplay_viewmodel::replay::athlete::V4Athlete {
    serde_json::from_str(include_str!(concat!(
        env!("OUT_DIR"),
        "/replay_athlete_v4.json"
    )))
    .expect("replay_athlete_v4.json")
}

/// The development asset directory, when an on-disk pack should be
/// re-validated at startup (the scene itself always loads the balsam bundle).
#[must_use]
pub fn dev_assets_dir() -> Option<std::path::PathBuf> {
    let configured = std::env::var("ROWPLAY_REPLAY_ASSETS").ok();
    let fallback = cfg!(debug_assertions).then(|| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("assets")
            .join("replay")
            .to_string_lossy()
            .into_owned()
    });
    configured
        .or(fallback)
        .map(std::path::PathBuf::from)
        .filter(|dir| dir.join("rowplay-rigs-v3.glb").is_file())
}

/// Re-validates an on-disk pack (development path, spec R2.4).
///
/// # Errors
/// The named contract violation from [`glb::validate_v3`].
pub fn validate_on_disk(
    dev_dir: &std::path::Path,
    file: &str,
) -> Result<V3Library, glb::AssetError> {
    let bytes = std::fs::read(dev_dir.join(file))
        .map_err(|error| glb::AssetError::Container(error.to_string()))?;
    glb::validate_v3(&bytes)
}

/// The embedded venue runtime plans (Phase 6b), written by `build.rs` after
/// the 6a drift gate: per sport and tier, the balsam component URL, the
/// material specs with resolved rcc texture sources, the bucketed instance
/// groups and the structural inventory the gate asserts.
pub struct VenueMeta {
    raw: serde_json::Value,
}

impl VenueMeta {
    /// The build-time plans of all twelve variants.
    #[must_use]
    pub fn embedded() -> VenueMeta {
        let raw: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/replay_venues_meta.json"
        )))
        .expect("replay_venues_meta.json");
        VenueMeta { raw }
    }

    /// The plan for one sport and tier, with the loadable component URL.
    #[must_use]
    pub fn plan(&self, sport: &str, tier: &str) -> Option<serde_json::Value> {
        let mut plan = self
            .raw
            .get(format!("rowplay-venue-{sport}-{tier}"))?
            .clone();
        let component = plan["component"].as_str()?.to_owned();
        plan["url"] =
            format!("qrc:/qt/qml/RowPlay/ReplayAssets/venues/{sport}-{tier}/{component}.qml")
                .into();
        Some(plan)
    }
}
