// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Replay` QML singleton (Phase 5a): asset sources, load state and the
//! resolved replay palette.
//!
//! A thin adapter over `rowplay_viewmodel::replay`: the contract validation,
//! the material-role table, the venue palettes and the key-light data live
//! Qt-free; this object only shapes them into bridge types and tracks the
//! scene's per-pack status reports (the balsam-generated rig and athlete
//! components, `build.rs`). Playback (transport, pose, camera, HUD) arrives
//! in Phase 5b on the same singleton.

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::QmlRegister;
use rowplay_core::models::Sport;
use rowplay_viewmodel::replay::{anchors, glb, materials, palette};

use crate::replay::assets;

/// The three sports in the sidebar filter's order.
const SPORTS: [Sport; 3] = [Sport::Rower, Sport::Skierg, Sport::Bike];

/// Backend for the 3D replay route.
pub struct ReplayBackend {
    // Constants resolved at construction.
    asset_mode: String,
    validation_mode: String,
    material_specs: serde_json::Value,
    mesh_roles: serde_json::Value,
    anchors: serde_json::Value,
    scene_names: Vec<String>,
    // Load state: the startup validation result, then the scene's report.
    load_state: String,
    error_text: String,
    // Palette inputs.
    sport_index: i64,
    scheme_dark: bool,
    sky_zenith: String,
    sky_horizon: String,
    sky_ground: String,
    sky_sun: String,
    ground_color: String,
    live_paint: String,
    ghost_paint: String,
    // Key light (web SUN_OFFSETS / SHADOW_TARGET_HEIGHT), metres and degrees.
    sun_offset: Vec<f64>,
    sun_elevation: f64,
    sun_azimuth: f64,
    shadow_target_height: f64,
}

impl Default for ReplayBackend {
    fn default() -> Self {
        let meta = assets::Meta::embedded();
        let dev_dir = assets::dev_assets_dir();

        // Development re-validates the on-disk pack at startup so an edited
        // asset fails loudly with its slot name; release relies on the
        // build-time validation of exactly the bundled bytes.
        let (validation_mode, mesh_roles, load_state, error_text) = match &dev_dir {
            Some(dir) => match assets::validate_on_disk(dir, "rowplay-rigs-v3.glb") {
                Ok(library) => (
                    "startup".to_owned(),
                    mesh_roles_json(&library),
                    "loading".to_owned(),
                    String::new(),
                ),
                Err(error) => (
                    "startup".to_owned(),
                    serde_json::Value::Object(serde_json::Map::new()),
                    "error".to_owned(),
                    error.to_string(),
                ),
            },
            None => (
                "build".to_owned(),
                meta.mesh_roles(),
                "loading".to_owned(),
                String::new(),
            ),
        };
        if load_state == "error" {
            eprintln!("replay assets failed validation: {error_text}");
        }

        let mut specs = Vec::new();
        for role in materials::ALL_ROLES {
            let spec = role.spec();
            specs.push(serde_json::json!({
                "id": role.as_id(),
                "themeKey": spec.theme_key,
                "venue": spec.venue,
                "metalness": spec.metalness,
                "roughness": spec.roughness,
                "ghostOpacity": spec.ghost_opacity(role),
            }));
        }

        // The README anchor table: each template's primary clone position and
        // yaw (5a places one instance statically; 5b clones and animates).
        let anchors: Vec<serde_json::Value> = anchors::ANCHORS
            .iter()
            .map(|row| {
                let position = anchors::clone_position(row.template, 0);
                let yaw = anchors::clone_yaw(row.template, 0);
                serde_json::json!({
                    "template": row.template,
                    "position": [position[0], position[1], position[2]],
                    "yaw": yaw,
                    "instances": row.instances,
                    "space": row.space,
                })
            })
            .collect();

        let mut backend = ReplayBackend {
            asset_mode: meta.asset_mode().to_owned(),
            validation_mode,
            material_specs: serde_json::Value::Array(specs),
            mesh_roles,
            anchors: serde_json::Value::Array(anchors),
            scene_names: meta.scene_names(),
            load_state,
            error_text,
            sport_index: 0,
            scheme_dark: false,
            sky_zenith: String::new(),
            sky_horizon: String::new(),
            sky_ground: String::new(),
            sky_sun: String::new(),
            ground_color: String::new(),
            live_paint: String::new(),
            ghost_paint: String::new(),
            sun_offset: Vec::new(),
            sun_elevation: 0.0,
            sun_azimuth: 0.0,
            shadow_target_height: f64::from(palette::SHADOW_TARGET_HEIGHT),
        };
        backend.refresh_palette();
        backend
    }
}

fn mesh_roles_json(library: &glb::V3Library) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for entry in &library.mesh_roles {
        map.insert(
            entry.name.clone(),
            serde_json::json!({
                "role": entry.role,
                "template": entry.template,
                "slot": entry.slot,
            }),
        );
    }
    serde_json::Value::Object(map)
}

#[qobject(NoQmlElement, ConvertToCamelCase)]
impl ReplayBackend {
    // "balsam": the generated RowPlay.ReplayAssets components (build.rs).
    qproperty!("assetMode", Member = asset_mode, Constant);
    // "startup" re-validated the on-disk pack at launch (development);
    // "build" relied on the compile-time validation of the bundled bytes.
    qproperty!("validationMode", Member = validation_mode, Constant);
    // [{id, themeKey, venue, metalness, roughness, ghostOpacity}] per role.
    qproperty!("materialSpecs", Member = material_specs, Constant);
    // { node name: {role, template, slot} } for the QML material walker.
    qproperty!("meshRoles", Member = mesh_roles, Constant);
    // [{template, position, yaw, instances, space}] — the asset README's
    // anchor table (primary clone), applied to the loaded template roots.
    qproperty!("anchors", Member = anchors, Constant);
    // Every node name the loaded scene must contain (gate assertion).
    qproperty!("sceneNames", Member = scene_names, Constant);
    // "loading" | "ready" | "error".
    qproperty!("loadState", Member = load_state, Notify = replay_changed);
    qproperty!("errorText", Member = error_text, Notify = replay_changed);
    // 0 = RowErg, 1 = SkiErg, 2 = BikeErg.
    qproperty!("sportIndex", Member = sport_index, Notify = replay_changed);
    qproperty!("schemeDark", Member = scheme_dark, Notify = replay_changed);
    // Procedural-sky inputs from the sport's venue palette (ADR 0004).
    qproperty!("skyZenith", Member = sky_zenith, Notify = replay_changed);
    qproperty!("skyHorizon", Member = sky_horizon, Notify = replay_changed);
    qproperty!("skyGround", Member = sky_ground, Notify = replay_changed);
    qproperty!("skySun", Member = sky_sun, Notify = replay_changed);
    qproperty!(
        "groundColor",
        Member = ground_color,
        Notify = replay_changed
    );
    // Lane paint for the live and ghost participants.
    qproperty!("livePaint", Member = live_paint, Notify = replay_changed);
    qproperty!("ghostPaint", Member = ghost_paint, Notify = replay_changed);
    // The key light's position relative to its target, [x, y, z] metres
    // (web SUN_OFFSETS), and the sun's elevation / azimuth in degrees for the
    // procedural sky's sunLatitude / sunLongitude.
    qproperty!("sunOffset", Member = sun_offset, Notify = replay_changed);
    qproperty!(
        "sunElevation",
        Member = sun_elevation,
        Notify = replay_changed
    );
    qproperty!("sunAzimuth", Member = sun_azimuth, Notify = replay_changed);
    // Height of the key light's target above the ground, metres.
    qproperty!(
        "shadowTargetHeight",
        Member = shadow_target_height,
        Constant
    );

    /// Emitted when load state, sport or scheme changed.
    #[qsignal]
    fn replay_changed(&mut self);

    /// Selects the sport whose equipment and palette the scene shows.
    #[qslot]
    fn set_sport(&mut self, index: i64) {
        let clamped = index.clamp(0, (SPORTS.len() as i64) - 1);
        if clamped == self.sport_index {
            return;
        }
        self.sport_index = clamped;
        self.refresh_palette();
        self.replay_changed();
    }

    /// Mirrors `Theme.dark` so the Rust palette follows the scheme.
    #[qslot]
    fn set_scheme_dark(&mut self, dark: bool) {
        if dark == self.scheme_dark {
            return;
        }
        self.scheme_dark = dark;
        self.refresh_palette();
        self.replay_changed();
    }

    /// The scene reports that both components instantiated and the scene
    /// rules were applied. A startup validation failure stays an error.
    #[qslot]
    fn report_ready(&mut self) {
        if self.load_state != "loading" {
            return;
        }
        "ready".clone_into(&mut self.load_state);
        self.replay_changed();
    }

    /// The scene reports a failure while applying its rules; the overlay
    /// shows the message.
    #[qslot]
    fn report_error(&mut self, error: String) {
        self.error_text = error;
        "error".clone_into(&mut self.load_state);
        self.replay_changed();
    }
}

impl ReplayBackend {
    fn sport(&self) -> Sport {
        SPORTS[self.sport_index as usize]
    }

    fn refresh_palette(&mut self) {
        let sky = palette::sky_palette(self.sport(), self.scheme_dark);
        sky.zenith.clone_into(&mut self.sky_zenith);
        sky.horizon.clone_into(&mut self.sky_horizon);
        sky.ground.clone_into(&mut self.sky_ground);
        sky.sun.clone_into(&mut self.sky_sun);
        palette::ground_color(self.sport(), self.scheme_dark).clone_into(&mut self.ground_color);
        palette::live_paint(self.scheme_dark).clone_into(&mut self.live_paint);
        palette::ghost_paint(self.scheme_dark).clone_into(&mut self.ghost_paint);
        self.sun_offset = palette::sun_offset(self.sport())
            .iter()
            .map(|value| f64::from(*value))
            .collect();
        self.sun_elevation = f64::from(palette::sun_elevation_degrees(self.sport()));
        self.sun_azimuth = f64::from(palette::sun_azimuth_degrees(self.sport()));
    }
}

// qtbridge derives the module URI from the Cargo package name; the manual
// impl keeps the QML-facing name `RowPlay` (qt-bridges-notes #1).
impl QmlRegister for ReplayBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Replay";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
