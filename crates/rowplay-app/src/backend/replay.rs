// SPDX-License-Identifier: GPL-3.0-or-later
//! The `Replay` QML singleton (Phase 5a assets and scene, Phase 5b
//! playback): asset metadata, load state, the resolved venue palette and
//! key light, and the playback transport whose every tick produces one flat
//! frame bundle for the scene.
//!
//! A thin adapter over `rowplay_viewmodel::replay` and
//! `rowplay_core::replay`: the contract validation, the material-role table,
//! the palettes, the rig-pose solver, the athlete posing and the chase
//! camera all live Qt-free; this object shapes them into bridge types. QML
//! drives `tick(dt)` from one `FrameAnimation` and reads `poseFrame` in one
//! `onFrameChanged` handler — exactly one bridge crossing per frame (spec
//! R1.3), counted at the single emission site.

use qtbridge::qobject;
use qtbridge::qtbridge_runtime::{QObjectHolder, QmlRegister};
use rowplay_core::demo::DEFAULT_WORKOUT_ID;
use rowplay_core::models::Sport;
use rowplay_core::replay::engine::{ReplaySpeed, ReplayState};
use rowplay_core::replay::motion::PerfGovernor;
use rowplay_core::replay::motion_graph::{ReplayMotionGraph, sample_motion_graph};
use rowplay_core::replay::quality::RenderQuality;
use rowplay_core::replay::rig_pose::{SportRigPose, solve_rig_pose};
use rowplay_core::replay::stroke_model::{
    StrokeTimeline, build_stroke_timeline, fallback_stroke_pose, reduced_motion, stroke_pose_at,
};
use rowplay_viewmodel::replay::athlete::V4Athlete;
use rowplay_viewmodel::replay::camera::{CameraInput, CameraState, chase};
use rowplay_viewmodel::replay::course::{
    AccentCues, LIVE_LOOP_RADIUS, Placement, accents, advance_anim_phase, place,
};
use rowplay_viewmodel::replay::equipment::{
    PoleLeafFit, blade_position, blade_roll_degrees, crank_rotation,
    layout_json as equipment_layout_json, oar_rotations, oar_rotations_from_yaws, pole_leaf_fits,
    pole_leaf_position, pole_rotation, roll_rotation, wheel_rotation, yaw_rotation,
};
use rowplay_viewmodel::replay::frame;
use rowplay_viewmodel::replay::grip::{
    closure_options, collect_hand_chains, grip_frames, solve_grip_table, warped_cycle,
};
use rowplay_viewmodel::replay::hud::{hud_bundle, hud_numbers, hud_strings};
use rowplay_viewmodel::replay::pose::{PoseSolver, clip_fraction, rig_targets};
use rowplay_viewmodel::replay::tier::tier_settings_json;
use rowplay_viewmodel::replay::{anchors, glb, materials, palette};

use crate::backend::AppState;
use crate::replay::assets;

/// The three sports in the sidebar filter's order.
const SPORTS: [Sport; 3] = [Sport::Rower, Sport::Skierg, Sport::Bike];

/// The playback speed presets (core `ReplaySpeed`, sidebar order).
const SPEEDS: [ReplaySpeed; 5] = [
    ReplaySpeed::Half,
    ReplaySpeed::One,
    ReplaySpeed::OneAndHalf,
    ReplaySpeed::Two,
    ReplaySpeed::Four,
];

/// The web clamps a frame delta to 100 ms (`clampDt`).
const MAX_DT_SECONDS: f64 = 0.1;

/// The loaded workout's playback state.
struct Playback {
    state: ReplayState,
    timeline: StrokeTimeline,
    sport: Sport,
}

/// Backend for the 3D replay route.
pub struct ReplayBackend {
    // Constants resolved at construction.
    asset_mode: String,
    validation_mode: String,
    material_specs: serde_json::Value,
    mesh_roles: serde_json::Value,
    anchors: serde_json::Value,
    mirror_anchors: serde_json::Value,
    scene_names: Vec<String>,
    frame_layout: serde_json::Value,
    equipment_layout: serde_json::Value,
    speed_labels: Vec<String>,
    // Tier settings resolved from (quality, sport) — the scene reads this
    // to apply shadows, MSAA, textures.
    tier_settings: serde_json::Value,
    // The active venue's runtime plan (component URL, materials, bucketed
    // instance groups, inventory) for the current (sport, effective tier).
    venue_plan: serde_json::Value,
    quality_index: i64,
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
    // Playback (Phase 5b).
    athlete: V4Athlete,
    solver: Option<PoseSolver>,
    // Capture-walk close-up camera (T8): frames the athlete's torso and
    // hands instead of the chase view while the phase-shot walk takes a
    // "-closeup" twin. Chase state is untouched so it flips back cleanly.
    closeup_camera: bool,
    /// Capture-strip override: when ≥ 0, `advance` poses with the guard's
    /// `fallback_stroke_pose(sport, step/2000·τ, 30)` and `metres = step·3`
    /// so a frame named `stepN.png` is the same sample as guard step N.
    /// −1 (default) uses the loaded workout's timeline.
    guard_cycle_step: i64,
    pole_fits: [Option<PoleLeafFit>; 3],
    playback: Option<Playback>,
    camera: CameraState,
    anim_phase: f64,
    last_distance: f64,
    dirty: bool,
    reduce_motion: bool,
    aspect: f64,
    frame: Vec<f32>,
    frame_seq: i64,
    hud_text: String,
    playing: bool,
    progress: f64,
    duration_seconds: f64,
    speed_index: i64,
    has_workout: bool,
    workout_id: i64,
    emit_count: u64,
    // Per-sport finger grip table (Phase 7): helper objectName → final
    // local rotation, solved once per sport switch and applied QML-side.
    grip_poses: String,
    /// Contact count of the current table ("n/m digit contacts").
    grip_contacts: String,
    // Ghost (Phase 5c).
    ghost_playback: Option<Playback>,
    ghost_frame: Vec<f32>,
    ghost_camera: CameraState,
    ghost_anim_phase: f64,
    ghost_last_distance: f64,
    has_ghost: bool,
    gap_text: String,
    verdict_text: String,
    // Governor (Phase 5c).
    governor: PerfGovernor,
    /// True = Automatic (governor applies degradation); false = pinned tier.
    governor_auto: bool,
    /// The effective quality after governor degradation (may differ from
    /// `quality_index` when the governor has stepped down).
    effective_quality: i64,
    /// Diagnostics text for the developer strip (updated at ~4 Hz).
    diagnostics_text: String,
    diag_counter: u32,
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

        // The README anchor table: primary clone (index 0) and mirror clone
        // (index 1) positions and yaws. QML places one `Rigs` at the primary
        // anchors and a second at the mirror anchors for multi-instance
        // templates (oars, skis, wheels).
        let build_anchors = |clone_index: u8| -> Vec<serde_json::Value> {
            anchors::ANCHORS
                .iter()
                .map(|row| {
                    let position = anchors::clone_position(row.template, clone_index);
                    let yaw = anchors::clone_yaw(row.template, clone_index);
                    serde_json::json!({
                        "template": row.template,
                        "position": [position[0], position[1], position[2]],
                        "yaw": yaw,
                        "instances": row.instances,
                        "space": row.space,
                    })
                })
                .collect()
        };
        let anchors = build_anchors(0);
        let mirror_anchors = build_anchors(1);

        let athlete = assets::embedded_athlete();
        let (solver, load_state, error_text) = match PoseSolver::new(&athlete) {
            Ok(solver) => (Some(solver), load_state, error_text),
            Err(error) if load_state != "error" => {
                eprintln!("replay athlete failed its contact plan: {error}");
                (None, "error".to_owned(), error.to_string())
            }
            Err(_) => (None, load_state, error_text),
        };
        let equipment_layout = equipment_layout_json(&|slot| meta.leaf_bounds(slot));
        let pole_fits = pole_leaf_fits(&|slot| meta.leaf_bounds(slot));

        let mut backend = ReplayBackend {
            asset_mode: meta.asset_mode().to_owned(),
            validation_mode,
            material_specs: serde_json::Value::Array(specs),
            mesh_roles,
            anchors: serde_json::Value::Array(anchors),
            mirror_anchors: serde_json::Value::Array(mirror_anchors),
            scene_names: meta.scene_names(),
            frame_layout: frame::layout_json(),
            equipment_layout,
            speed_labels: SPEEDS
                .iter()
                .map(|speed| speed.label().to_owned())
                .collect(),
            tier_settings: tier_settings_json(
                quality_from_index(AppState::get().prefs().replay_quality),
                Sport::Rower,
            ),
            venue_plan: serde_json::Value::Null,
            quality_index: i64::from(AppState::get().prefs().replay_quality.unwrap_or(1)),
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
            athlete,
            solver,
            pole_fits,
            playback: None,
            camera: CameraState::new(Sport::Rower),
            closeup_camera: false,
            guard_cycle_step: -1,
            anim_phase: 0.0,
            last_distance: 0.0,
            dirty: false,
            reduce_motion: false,
            aspect: 1.5,
            frame: frame::empty_frame(),
            frame_seq: 0,
            hud_text: String::new(),
            playing: false,
            progress: 0.0,
            duration_seconds: 0.0,
            speed_index: 1,
            has_workout: false,
            workout_id: -1,
            emit_count: 0,
            grip_poses: String::from("{}"),
            grip_contacts: String::from("0/0"),
            ghost_playback: None,
            ghost_frame: Vec::new(),
            ghost_camera: CameraState::new(Sport::Rower),
            ghost_anim_phase: 0.0,
            ghost_last_distance: 0.0,
            has_ghost: false,
            gap_text: String::new(),
            verdict_text: String::new(),
            // Governor: 22 ms budget, 30-frame window, 60-frame grace,
            // max 3 levels, 120-frame calibration (web defaults).
            governor: PerfGovernor::new(22.0, 30, 60, 3, 120),
            governor_auto: true,
            effective_quality: i64::from(AppState::get().prefs().replay_quality.unwrap_or(1)),
            diagnostics_text: String::new(),
            diag_counter: 0,
        };
        backend.refresh_palette();
        backend.refresh_grip();
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

fn quality_from_index(index: Option<u8>) -> RenderQuality {
    match index.unwrap_or(1) {
        0 => RenderQuality::Low,
        2 => RenderQuality::High,
        3 => RenderQuality::Ultra,
        _ => RenderQuality::Medium,
    }
}

fn sport_name(sport: Sport) -> &'static str {
    match sport {
        Sport::Rower => "rower",
        Sport::Skierg => "skierg",
        Sport::Bike => "bike",
    }
}

/// Athlete close-up framing for the capture walk (T8): a front three-quarter
/// view of the torso and the hands' stroke path, in rig-local space.
///
/// `placement` places the rig on the loop and gives its rig→world yaw.
/// `aim_local` is the point to frame, in rig-local coordinates — the midpoint
/// of the athlete's torso and hands for the current frame. The camera offset
/// is *not* a fixed rig offset: the seat slides 0.44 m and the torso lays
/// back, so a fixed offset cropped the near body at the catch and the finish
/// (the two phases the lens exists to judge) while mid-drive framed well.
/// Following the contact midpoint keeps the athlete in frame through the
/// stroke. `distance` scales the offset: small for a tight wrist read, larger
/// to keep the whole draw path inside the frustum at its extremes.
/// Camera stand-off for the close-up walk: far enough that the reach and the
/// layback stay inside the frustum at both stroke extremes, close enough to
/// read the wrist and the draw onset.
const CLOSEUP_DISTANCE: f64 = 2.4;
/// Chest lift above the SkiErg pelvis so the close-up frames torso and arms
/// rather than the hips.
const SKI_CLOSEUP_CHEST: f64 = 0.45;

fn yaw_to_world(yaw: f64, x: f64, y: f64, z: f64) -> [f64; 3] {
    let (sin, cos) = yaw.sin_cos();
    [x * cos + z * sin, y, -x * sin + z * cos]
}

fn closeup_offset(placement: &Placement, distance: f64) -> [f64; 3] {
    // Front three-quarter: inboard of the athlete and above, ahead of the aim
    // along rig +z (the direction the athlete faces).
    yaw_to_world(
        placement.yaw,
        0.62 * distance,
        0.20 * distance,
        0.55 * distance,
    )
}

fn closeup_aim_world(placement: &Placement, aim_local: [f64; 3]) -> [f64; 3] {
    let aim = yaw_to_world(placement.yaw, aim_local[0], aim_local[1], aim_local[2]);
    [placement.x + aim[0], aim_local[1], placement.z + aim[2]]
}

fn closeup_camera_view(
    placement: &Placement,
    aim_local: [f64; 3],
    distance: f64,
) -> ([f64; 3], [f64; 3], f64) {
    let offset = closeup_offset(placement, distance);
    let aim_world = closeup_aim_world(placement, aim_local);
    // Placement-relative stand-off: the whole RowErg draw path stays in the
    // frustum at the catch and the finish.
    let position = [
        placement.x + offset[0],
        aim_local[1] + offset[1],
        placement.z + offset[2],
    ];
    (position, aim_world, 45.0)
}

/// SkiErg close-up: stand off the surged torso. The 1.45 m surge lives on
/// the rig group, not in the contact targets, and a placement-relative lens
/// of 1.32 m sits *inside* the figure.
fn closeup_camera_view_from_aim(
    placement: &Placement,
    aim_local: [f64; 3],
    distance: f64,
) -> ([f64; 3], [f64; 3], f64) {
    let offset = closeup_offset(placement, distance);
    let aim_world = closeup_aim_world(placement, aim_local);
    let position = [
        aim_world[0] + offset[0],
        aim_world[1] + offset[1],
        aim_world[2] + offset[2],
    ];
    (position, aim_world, 45.0)
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
    // Same table for clone index 1 (the mirror side: left oarlock, left
    // ski, second wheel); the scene places a second Rigs component here.
    qproperty!("mirrorAnchors", Member = mirror_anchors, Constant);
    // Every node name the loaded scene must contain (gate assertion).
    qproperty!("sceneNames", Member = scene_names, Constant);
    // The frame bundle's named offsets (rowplay_viewmodel::replay::frame).
    qproperty!("frameLayout", Member = frame_layout, Constant);
    // Static instance placements and leaf fits (replay::equipment).
    qproperty!("equipmentLayout", Member = equipment_layout, Constant);
    // The playback speed preset labels, in speedIndex order.
    qproperty!("speedLabels", Member = speed_labels, Constant);
    // "loading" | "ready" | "error".
    qproperty!("loadState", Member = load_state, Notify = replay_changed);
    qproperty!("errorText", Member = error_text, Notify = replay_changed);
    // The resolved tier settings JSON for the current quality + sport.
    qproperty!(
        "tierSettings",
        Member = tier_settings,
        Notify = replay_changed
    );
    // The active venue's plan (component URL, materials, bucketed instance
    // groups, inventory) for the current sport + effective tier; refreshed
    // whenever either changes.
    qproperty!("venuePlan", Member = venue_plan, Notify = replay_changed);
    qproperty!(
        "qualityIndex",
        Member = quality_index,
        Notify = replay_changed
    );
    // The effective quality after governor degradation (may differ from
    // qualityIndex when the governor has stepped down).
    qproperty!(
        "effectiveQuality",
        Member = effective_quality,
        Notify = replay_changed
    );
    qproperty!(
        "governorAuto",
        Member = governor_auto,
        Notify = replay_changed
    );
    qproperty!(
        "diagnosticsText",
        Member = diagnostics_text,
        Notify = replay_changed
    );
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
    // Playback: the frame bundle (read in one onFrameChanged handler) and
    // the HUD strings, updated together by tick.
    qproperty!("poseFrame", Member = frame, Notify = frame_changed);
    qproperty!("frameSeq", Member = frame_seq, Notify = frame_changed);
    qproperty!("hudText", Member = hud_text, Notify = frame_changed);
    qproperty!("progress", Member = progress, Notify = frame_changed);
    // Ghost frame (same layout as poseFrame, empty when no ghost).
    qproperty!("ghostFrame", Member = ghost_frame, Notify = frame_changed);
    qproperty!("hasGhost", Member = has_ghost, Notify = playback_changed);
    qproperty!("gapText", Member = gap_text, Notify = frame_changed);
    qproperty!("verdictText", Member = verdict_text, Notify = frame_changed);
    // Finger grip table for the current sport (helper → final local
    // rotation, JSON) and its contact count, solved once per sport switch.
    qproperty!("gripPoses", Member = grip_poses, Notify = replay_changed);
    qproperty!(
        "gripContacts",
        Member = grip_contacts,
        Notify = replay_changed
    );
    // Transport state.
    qproperty!("playing", Member = playing, Notify = playback_changed);
    qproperty!(
        "durationSeconds",
        Member = duration_seconds,
        Notify = playback_changed
    );
    qproperty!(
        "speedIndex",
        Member = speed_index,
        Notify = playback_changed
    );
    qproperty!(
        "hasWorkout",
        Member = has_workout,
        Notify = playback_changed
    );
    qproperty!("workoutId", Member = workout_id, Notify = playback_changed);
    qproperty!(
        "reduceMotion",
        Member = reduce_motion,
        Notify = playback_changed
    );

    /// Emitted when load state, sport or scheme changed.
    #[qsignal]
    fn replay_changed(&mut self);

    /// Emitted once per tick that produced a frame (and once per seek/load).
    #[qsignal(qml_name = "frameChanged")]
    fn frame_changed(&mut self);

    /// Emitted when the transport state changed.
    #[qsignal(qml_name = "playbackChanged")]
    fn playback_changed(&mut self);

    /// Selects the sport whose equipment and palette the scene shows.
    #[qslot]
    fn set_sport(&mut self, index: i64) {
        let clamped = index.clamp(0, (SPORTS.len() as i64) - 1);
        if clamped == self.sport_index {
            return;
        }
        self.sport_index = clamped;
        self.refresh_palette();
        self.refresh_tier();
        self.refresh_grip();
        self.notify_replay();
    }

    /// Sets the quality tier (0 Low, 1 Medium, 2 High, 3 Ultra).
    #[qslot]
    fn set_quality_index(&mut self, index: i64) {
        let clamped = index.clamp(0, 3);
        if clamped == self.quality_index {
            return;
        }
        self.quality_index = clamped;
        self.effective_quality = clamped; // user override resets degradation
        self.governor.reset();
        self.refresh_tier();
        self.notify_replay();
    }

    /// Toggle automatic quality degradation (governor on/off).
    #[qslot]
    fn set_governor_auto(&mut self, auto: bool) {
        if auto == self.governor_auto {
            return;
        }
        self.governor_auto = auto;
        if auto {
            self.governor.reset();
        }
        self.notify_replay();
    }

    /// Called from QML with `renderStats.frameTime` after each rendered
    /// frame. Feeds the governor and, in Automatic mode, applies any
    /// degradation step to the effective tier.
    ///
    /// Guard: zero, negative or absurdly large samples (>250 ms) are
    /// dropped — `PerfGovernor::sample` rejects them — so a lost stats
    /// source makes the governor sit still rather than slide to Low.
    #[qslot]
    fn sample_render_time(&mut self, ms: f64) {
        if !self.governor_auto || !ms.is_finite() || ms <= 0.0 {
            return;
        }
        if let Some(new_level) = self.governor.sample(ms) {
            let base = quality_from_index(Some(self.quality_index as u8));
            let degraded = base.degraded(new_level as u8);
            let new_effective = degraded as i64;
            if new_effective != self.effective_quality {
                self.effective_quality = new_effective;
                self.refresh_tier();
                self.notify_replay();
            }
        }
        // Diagnostics: update at ~4 Hz (every 15th sample).
        self.diag_counter += 1;
        if self.diag_counter.is_multiple_of(15) {
            let tier_names = ["Low", "Medium", "High", "Ultra"];
            let base_idx = self.quality_index.clamp(0, 3) as usize;
            let eff_idx = self.effective_quality.clamp(0, 3) as usize;
            self.diagnostics_text = format!(
                "{} → {} | gov L{} | {:.1} ms",
                tier_names[base_idx],
                tier_names[eff_idx],
                self.governor.level(),
                self.governor.active_budget_ms()
            );
            self.notify_replay();
        }
    }

    /// Mirrors `Theme.dark` so the Rust palette follows the scheme.
    #[qslot]
    fn set_scheme_dark(&mut self, dark: bool) {
        if dark == self.scheme_dark {
            return;
        }
        self.scheme_dark = dark;
        self.refresh_palette();
        self.notify_replay();
    }

    /// The scene reports that both components instantiated and the scene
    /// rules were applied. A startup validation failure stays an error.
    #[qslot]
    fn report_ready(&mut self) {
        if self.load_state != "loading" {
            return;
        }
        "ready".clone_into(&mut self.load_state);
        self.notify_replay();
    }

    /// The scene reports a failure while applying its rules; the overlay
    /// shows the message.
    #[qslot]
    fn report_error(&mut self, error: String) {
        self.error_text = error;
        "error".clone_into(&mut self.load_state);
        self.notify_replay();
    }

    /// Loads workout `id` from the library (the demo default when `id` is
    /// −1 or unknown), switches the scene to its sport and renders the first
    /// frame paused.
    #[qslot]
    fn load_workout(&mut self, id: i64) {
        if !self.load_playback(id) {
            self.playback = None;
            self.has_workout = false;
            self.workout_id = -1;
            self.notify_playback();
            return;
        }
        self.dirty = true;
        self.advance(0.0);
        self.notify_playback();
        self.notify_frame();
    }

    /// Loads a rival workout as the ghost. The ghost uses the same sport's
    /// clips and poses, driven by its own `ReplayState` over the rival's
    /// strokes. Pass −1 to dismiss the ghost.
    #[qslot]
    fn load_ghost(&mut self, id: i64) {
        if id < 0 {
            self.ghost_playback = None;
            self.ghost_frame = Vec::new();
            self.has_ghost = false;
            self.gap_text.clear();
            self.verdict_text.clear();
            self.notify_playback();
            return;
        }
        let state = AppState::get();
        let details = state.details();
        let Some(detail) = details.iter().find(|d| d.id() == id) else {
            return;
        };
        let sport = detail.workout.sport;
        let timeline =
            build_stroke_timeline(&detail.strokes, sport, detail.workout.has_stroke_data);
        let mut replay = ReplayState::new(detail.strokes.clone());
        replay.set_speed(SPEEDS[self.speed_index as usize].factor());
        self.ghost_playback = Some(Playback {
            state: replay,
            timeline,
            sport,
        });
        self.ghost_frame = frame::empty_frame();
        self.ghost_camera = CameraState::new(sport);
        self.ghost_anim_phase = 0.0;
        self.ghost_last_distance = 0.0;
        self.has_ghost = true;
        self.dirty = true;
        self.notify_playback();
    }

    /// Advance playback by `dt` seconds of wall time (one call per rendered
    /// frame from the scene's `FrameAnimation`); emits `frameChanged` at most
    /// once.
    #[qslot]
    fn tick(&mut self, dt: f64) {
        if self.advance(dt) {
            self.notify_frame();
        }
    }

    /// Start playback.
    #[qslot]
    fn play(&mut self) {
        if let Some(playback) = self.playback.as_mut() {
            playback.state.play();
            self.playing = playback.state.playing();
            self.notify_playback();
        }
    }

    /// Pause playback.
    #[qslot]
    fn pause(&mut self) {
        if let Some(playback) = self.playback.as_mut() {
            playback.state.pause();
            self.playing = false;
            self.notify_playback();
        }
    }

    /// Toggle playback (Space).
    #[qslot]
    fn toggle(&mut self) {
        if let Some(playback) = self.playback.as_mut() {
            playback.state.toggle();
            self.playing = playback.state.playing();
            self.notify_playback();
        }
    }

    /// Seek to a fraction 0..1 of the workout (the transport slider).
    #[qslot]
    fn seek(&mut self, fraction: f64) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        let fraction = if fraction.is_finite() {
            fraction.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let time = fraction * playback.state.duration();
        playback.state.seek(time);
        self.dirty = true;
        self.advance(0.0);
        self.notify_frame();
    }

    /// Seek by `delta` seconds (the web's ±10 s arrows, ±30 s with Shift).
    #[qslot]
    fn seek_by(&mut self, delta: f64) {
        let Some(playback) = self.playback.as_mut() else {
            return;
        };
        if !delta.is_finite() {
            return;
        }
        let time = playback.state.time() + delta;
        playback.state.seek(time);
        self.dirty = true;
        self.advance(0.0);
        self.notify_frame();
    }

    /// Select a speed preset by index into `speedLabels`.
    #[qslot]
    fn set_speed_index(&mut self, index: i64) {
        let clamped = index.clamp(0, (SPEEDS.len() as i64) - 1);
        self.speed_index = clamped;
        if let Some(playback) = self.playback.as_mut() {
            playback.state.set_speed(SPEEDS[clamped as usize].factor());
        }
        self.notify_playback();
    }

    /// Capture-walk hook: pose as the dense continuity guard does at `step`
    /// (`fallback_stroke_pose` at 30 spm, metres = step·3). Pass −1 to return
    /// to the loaded workout. A paused replay emits no frames, so the flip
    /// must push one the way a seek does.
    #[qslot]
    fn set_guard_cycle_step(&mut self, step: i64) {
        if self.guard_cycle_step == step {
            return;
        }
        self.guard_cycle_step = step;
        self.dirty = true;
        self.advance(0.0);
        self.notify_frame();
    }

    /// Capture-walk hook: frame the athlete's torso and hands instead of the
    /// chase view (the phase-shot close-up twins judge wrist and posture).
    /// A paused replay emits no frames, so the flip must push one the way a
    /// seek does or the lens never swaps.
    #[qslot]
    fn set_closeup_camera(&mut self, on: bool) {
        if self.closeup_camera == on {
            return;
        }
        self.closeup_camera = on;
        self.dirty = true;
        self.advance(0.0);
        self.notify_frame();
    }

    /// Step the speed preset up or down (the web's `[` and `]`).
    #[qslot]
    fn step_speed(&mut self, direction: i64) {
        let next = self.speed_index + direction.signum();
        if (0..SPEEDS.len() as i64).contains(&next) {
            self.set_speed_index(next);
        }
    }

    /// Mirrors the reduce-motion preference into the replay.
    #[qslot]
    fn set_reduce_motion(&mut self, enabled: bool) {
        if enabled == self.reduce_motion {
            return;
        }
        self.reduce_motion = enabled;
        self.dirty = true;
        self.notify_playback();
        if self.playback.is_some() {
            self.advance(0.0);
            self.notify_frame();
        }
    }

    /// The scene's viewport size, for the chase camera's aspect rules.
    #[qslot]
    fn set_viewport(&mut self, width: f64, height: f64) {
        if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
            self.aspect = width / height;
        }
    }
}

impl ReplayBackend {
    fn sport(&self) -> Sport {
        SPORTS[self.sport_index as usize]
    }

    /// Build a `Workout` for `race_result` from the loaded library.
    fn workout_for_result(&self) -> Option<rowplay_core::models::Workout> {
        let state = AppState::get();
        let details = state.details();
        details
            .iter()
            .find(|d| d.id() == self.workout_id)
            .map(|d| d.workout.clone())
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

    fn refresh_tier(&mut self) {
        // Use effective_quality (which reflects governor degradation) rather
        // than the user's selected quality_index.
        let quality = quality_from_index(Some(self.effective_quality as u8));
        self.tier_settings = tier_settings_json(quality, self.sport());
        self.refresh_venue_plan();
    }

    /// Solve the finger grip table for the current sport: collect both
    /// hands' digit chains from the athlete's rest hierarchy, close each
    /// around the sport's equipment surface, and bake the helpers' final
    /// local rotations for the QML sport walk. A hand whose helpers are
    /// incomplete keeps the previous table rather than blanking the scene.
    fn refresh_grip(&mut self) {
        let sport = SPORTS[self.sport_index as usize];
        let mut poses = serde_json::Map::new();
        let mut contacts = 0;
        let mut digits = 0;
        for side in [-1.0, 1.0] {
            let Some(chains) = collect_hand_chains(&self.athlete, side) else {
                continue;
            };
            let options = closure_options(sport, side);
            let table = solve_grip_table(&chains, &options, &|helper| {
                self.athlete
                    .joint_index(helper)
                    .map(|index| self.athlete.joints[index].rotation)
            });
            for (helper, rotation) in &table.poses {
                poses.insert(
                    helper.clone(),
                    serde_json::json!([rotation[0], rotation[1], rotation[2], rotation[3]]),
                );
            }
            contacts += table.contacts;
            digits += table.digits;
        }
        if !poses.is_empty() {
            self.grip_poses = serde_json::Value::Object(poses).to_string();
            self.grip_contacts = format!("{contacts}/{digits}");
        }
    }

    /// The venue plan for the current (sport, effective tier); the scene
    /// (re)loads its venue component whenever this changes.
    fn refresh_venue_plan(&mut self) {
        const TIERS: [&str; 4] = ["low", "medium", "high", "ultra"];
        let tier = TIERS[self.effective_quality.clamp(0, 3) as usize];
        self.venue_plan = crate::replay::assets::VenueMeta::embedded()
            .plan(sport_name(self.sport()), tier)
            .unwrap_or(serde_json::Value::Null);
    }

    /// The single emission site of the per-frame notify (spec R1.3): counted
    /// always, emitted only when a QML engine holds this object.
    fn notify_frame(&mut self) {
        self.emit_count += 1;
        if self.attached() {
            self.frame_changed();
        }
    }

    /// Resolve `id` in the loaded library (the demo default for −1 / unknown)
    /// and build its playback state.
    fn load_playback(&mut self, id: i64) -> bool {
        let state = AppState::get();
        let details = state.details();
        let detail = details
            .iter()
            .find(|detail| detail.id() == id)
            .or_else(|| {
                details
                    .iter()
                    .find(|detail| detail.id() == DEFAULT_WORKOUT_ID)
            })
            .or_else(|| details.first());
        let Some(detail) = detail else {
            return false;
        };
        let sport = detail.workout.sport;
        let timeline =
            build_stroke_timeline(&detail.strokes, sport, detail.workout.has_stroke_data);
        let mut replay = ReplayState::new(detail.strokes.clone());
        replay.set_speed(SPEEDS[self.speed_index as usize].factor());
        self.duration_seconds = replay.duration();
        self.playback = Some(Playback {
            state: replay,
            timeline,
            sport,
        });
        self.workout_id = detail.id();
        self.has_workout = true;
        self.playing = false;
        self.progress = 0.0;
        self.camera = CameraState::new(sport);
        self.anim_phase = 0.0;
        self.last_distance = 0.0;
        let index = SPORTS.iter().position(|s| *s == sport).unwrap_or(0) as i64;
        if index != self.sport_index {
            self.sport_index = index;
            self.refresh_palette();
            self.refresh_tier();
            self.notify_replay();
        }
        true
    }

    /// The Qt signals panic on an object no QML engine holds (qtbridge's
    /// generated `expect("No proxy")`), so every emission goes through a
    /// guard; the unit tests drive the object unattached.
    fn attached(&self) -> bool {
        self.try_get_rust_proxy_ptr().is_some()
    }

    fn notify_replay(&mut self) {
        if self.attached() {
            self.replay_changed();
        }
    }

    fn notify_playback(&mut self) {
        if self.attached() {
            self.playback_changed();
        }
    }

    /// Run the per-frame pipeline. Returns whether a new frame was produced
    /// (playing, or a pending seek / load / preference change while paused).
    fn advance(&mut self, dt: f64) -> bool {
        let Some(playback) = self.playback.as_mut() else {
            return false;
        };
        let dt = if dt.is_finite() {
            dt.clamp(0.0, MAX_DT_SECONDS)
        } else {
            0.0
        };
        let moved = playback.state.tick(dt);
        if !moved && !self.dirty {
            return false;
        }
        self.dirty = false;

        let sport = playback.sport;
        let sampled = playback.state.current_frame();
        let time = playback.state.time();
        let duration = playback.state.duration();
        let playing = playback.state.playing();
        // The dense guard samples `fallback_stroke_pose(..., 30 spm)` and
        // `metres = step·3`, not the loaded workout. A capture named
        // `stepN.png` must use that same sample or the rendered jump and
        // the guard's failing step disagree (live 1003: drive_frac 0.46
        // vs 0.34, jump at 522 vs 529).
        let (mut stroke, distance) = if self.guard_cycle_step >= 0 {
            let step_f = self.guard_cycle_step as f64;
            let phase = step_f / 2000.0 * std::f64::consts::TAU;
            (fallback_stroke_pose(sport, phase, 30.0), step_f * 3.0)
        } else {
            (stroke_pose_at(&playback.timeline, time), sampled.d)
        };
        if self.reduce_motion {
            stroke = reduced_motion(&stroke);
        }

        // Rig pose → contact targets → the posed athlete. The posed result is
        // kept for the equipment pack below: the RowErg oar rotations are the
        // arm-authority yaws solved in the pose pass, not `oar_sweep`.
        let rig = solve_rig_pose(sport, &stroke, distance, self.reduce_motion);
        let targets = rig_targets(&rig);
        let mut live_oar_yaw: Option<[f64; 2]> = None;
        if let (Some(solver), Some(clip)) = (&self.solver, self.athlete.clip_for(sport_name(sport)))
        {
            let fraction = clip_fraction(
                stroke.cycle_frac,
                stroke.phase,
                stroke.drive_frac,
                clip.drive_end,
            );
            let posed = solver.pose(
                &self.athlete,
                sport,
                clip,
                fraction * f64::from(clip.duration),
                &targets.contacts,
                &grip_frames(
                    sport,
                    &rig,
                    targets.poles,
                    warped_cycle(stroke.warped_phase),
                ),
                targets.oar,
            );
            live_oar_yaw = posed.oar_yaw;
            solver.pack(&posed, &mut self.frame);
        }

        // Course placement and the profile accents.
        let placement = place(sport, distance, LIVE_LOOP_RADIUS);
        let cues = match sample_motion_graph(sport, &stroke) {
            ReplayMotionGraph::Rower(graph) => AccentCues {
                vertical: graph.accents.vertical.value,
                surge: graph.accents.surge.value,
            },
            ReplayMotionGraph::Skierg(graph) => AccentCues {
                vertical: graph.accents.rebound.value,
                surge: graph.accents.surge.value,
            },
            ReplayMotionGraph::Bike(_) => AccentCues::default(),
        };
        self.anim_phase = advance_anim_phase(
            self.anim_phase,
            sampled.spm,
            dt,
            playing,
            self.reduce_motion,
        );
        let accent = accents(
            sport,
            cues,
            self.anim_phase,
            sampled.spm,
            self.reduce_motion,
        );

        // Chase camera.
        let advanced = distance - self.last_distance;
        self.last_distance = distance;
        // Ghost placement for the chase camera's midpoint framing +
        // comparison pullback: use the ghost's current course position (its
        // packed `COURSE_X/Z`) if the ghost pipeline has run at least once
        // this session (i.e. the frame carries non-default values). The
        // ghost pipeline runs after this camera call, so on the first
        // ghost-enabled frame the pack is still at its default zero; that
        // still trips the `is_finite && != 0` filter below cleanly on the
        // next tick, and the frame-to-frame position lag is invisible in
        // the damped chase. The web samples `this.ghostPlacement` set by
        // its own ghost render pass; both apps have the same one-tick
        // lag on the first activation.
        let ghost_placement = if self.ghost_playback.is_some() {
            let gx = f64::from(self.ghost_frame[frame::COURSE_X]);
            let gz = f64::from(self.ghost_frame[frame::COURSE_Z]);
            if gx.is_finite() && gz.is_finite() && (gx != 0.0 || gz != 0.0) {
                Some((gx, gz))
            } else {
                None
            }
        } else {
            None
        };
        self.camera = chase(
            sport,
            CameraInput {
                focus_x: placement.x,
                focus_z: placement.z,
                tangent_x: placement.tx,
                tangent_z: placement.tz,
                advanced,
                dt,
                playing,
                aspect: self.aspect,
                reduce_motion: self.reduce_motion,
                ghost_placement,
            },
            self.camera,
        );

        // HUD.
        let unit = AppState::get().prefs().preferred_distance_unit;
        self.hud_text = hud_bundle(&hud_strings(&sampled, duration, unit));

        // Pack the scalar blocks (the joints were packed above).
        self.frame_seq += 1;
        let f = &mut self.frame;
        f[frame::SEQUENCE] = f32::from_bits(self.frame_seq as u32);
        f[frame::HUD_DISTANCE..=frame::HUD_PROGRESS]
            .copy_from_slice(&hud_numbers(&sampled, duration));
        // Camera for this frame: the chase view, or the capture walk's
        // athlete close-up (chase state untouched, so flipping back to it
        // mid-session resumes smoothly).
        // The close-up frames the midpoint of the pelvis and the hands: those
        // follow the seat slide and the torso layback, so the athlete stays in
        // frame at the catch and the finish where a fixed rig offset cropped
        // the near body. Rig-local, matching `contacts`.
        // SkiErg: the 1.45 m surge lives on the rig group, not in the contact
        // targets. Aim at the surged chest and stand off from that point or
        // the figure sits inside the lens (a flesh sliver on the viewport
        // edge — the 2026-09 step-529 recapture).
        let contacts = targets.contacts;
        let closeup_aim = if sport == Sport::Skierg {
            [
                contacts.pelvis[0],
                contacts.pelvis[1] + accent.bob + SKI_CLOSEUP_CHEST,
                contacts.pelvis[2] + accent.surge,
            ]
        } else {
            [
                (contacts.pelvis[0] + contacts.left_hand[0] + contacts.right_hand[0]) / 3.0,
                (contacts.pelvis[1] + contacts.left_hand[1] + contacts.right_hand[1]) / 3.0,
                (contacts.pelvis[2] + contacts.left_hand[2] + contacts.right_hand[2]) / 3.0,
            ]
        };
        let (camera_position, camera_aim, camera_fov) = if self.closeup_camera {
            if sport == Sport::Skierg {
                closeup_camera_view_from_aim(&placement, closeup_aim, CLOSEUP_DISTANCE)
            } else {
                closeup_camera_view(&placement, closeup_aim, CLOSEUP_DISTANCE)
            }
        } else {
            (self.camera.position, self.camera.aim, self.camera.fov)
        };
        for i in 0..3 {
            f[frame::CAMERA_POSITION + i] = camera_position[i] as f32;
            f[frame::CAMERA_AIM + i] = camera_aim[i] as f32;
        }
        f[frame::CAMERA_FOV] = camera_fov as f32;
        f[frame::COURSE_X] = placement.x as f32;
        f[frame::COURSE_Z] = placement.z as f32;
        write_quat(f, frame::COURSE_YAW, yaw_rotation(placement.yaw));
        f[frame::ACCENT_BOB] = accent.bob as f32;
        f[frame::ACCENT_SURGE] = accent.surge as f32;
        write_quat(f, frame::ACCENT_ROLL, roll_rotation(accent.roll));
        for value in &mut f[frame::EQUIPMENT..frame::EQUIPMENT + frame::EQUIPMENT_COUNT] {
            *value = 0.0;
        }
        match rig {
            SportRigPose::Rower(rower) => {
                f[frame::EQ_SEAT_Z] = rower.seat_z as f32;
                // The composed arm-authority yaw when the pose pass ran (the
                // web renders this); `oar_sweep` is only its branch fallback,
                // used when there is no solver/clip (e.g. reduced motion).
                let [left, right] = match live_oar_yaw {
                    Some([l, r]) => oar_rotations_from_yaws([l, r], rower.oar_feather),
                    None => oar_rotations(rower.oar_sweep, rower.oar_feather),
                };
                write_quat(f, frame::EQ_OAR_LEFT, left);
                write_quat(f, frame::EQ_OAR_RIGHT, right);
                f[frame::EQ_BLADE_ROLL_DEG] = blade_roll_degrees(rower.blade_feather) as f32;
                // Blade leaf transforms: oarlock + oarQuat × BLADE_OFFSET, rotation = oarQuat.
                let left_oarlock = [
                    -f64::from(anchors::OARLOCK_PIVOT[0]),
                    f64::from(anchors::OARLOCK_PIVOT[1]),
                    f64::from(anchors::OARLOCK_PIVOT[2]),
                ];
                let right_oarlock = [
                    f64::from(anchors::OARLOCK_PIVOT[0]),
                    f64::from(anchors::OARLOCK_PIVOT[1]),
                    f64::from(anchors::OARLOCK_PIVOT[2]),
                ];
                let lbp = blade_position(left_oarlock, left);
                write_vec3(f, frame::EQ_BLADE_LEFT, lbp);
                write_quat(f, frame::EQ_BLADE_LEFT + 3, left);
                let rbp = blade_position(right_oarlock, right);
                write_vec3(f, frame::EQ_BLADE_RIGHT, rbp);
                write_quat(f, frame::EQ_BLADE_RIGHT + 3, right);
            }
            SportRigPose::SkiErg(_) => {
                if let Some([left, right]) = targets.poles {
                    for (at, leaves_at, pole) in [
                        (frame::EQ_POLE_LEFT, frame::EQ_POLE_LEAVES_LEFT, left),
                        (frame::EQ_POLE_RIGHT, frame::EQ_POLE_LEAVES_RIGHT, right),
                    ] {
                        let pole_pos = pole.root;
                        let pole_rot = pole_rotation(pole.direction);
                        for i in 0..3 {
                            f[at + i] = pole_pos[i] as f32;
                        }
                        write_quat(f, at + 3, pole_rot);
                        // Pack the three pole leaf positions.
                        for (li, leaf_fit) in self.pole_fits.iter().enumerate() {
                            let fit_pos = match leaf_fit {
                                Some(fit) => fit.position,
                                None => [0.0; 3],
                            };
                            let leaf_pos = pole_leaf_position(pole_pos, pole_rot, fit_pos);
                            write_vec3(f, leaves_at + li * 3, leaf_pos);
                        }
                    }
                }
            }
            SportRigPose::Bike(bike) => {
                write_quat(f, frame::EQ_CRANK, crank_rotation(bike.crank_angle));
                write_quat(f, frame::EQ_WHEEL, wheel_rotation(bike.wheel_angle));
            }
        }
        self.playing = playing;
        self.progress = if duration > 0.0 {
            (time / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Ghost pipeline: same pose/course/equipment but into ghost_frame.
        if let Some(ghost) = self.ghost_playback.as_mut() {
            ghost.state.tick(dt);
            let g_time = ghost.state.time();
            let g_sampled = ghost.state.current_frame();
            let mut g_stroke = stroke_pose_at(&ghost.timeline, g_time);
            if self.reduce_motion {
                g_stroke = reduced_motion(&g_stroke);
            }
            let g_distance = g_sampled.d;
            let g_sport = ghost.sport;

            let g_rig = solve_rig_pose(g_sport, &g_stroke, g_distance, self.reduce_motion);
            let g_targets = rig_targets(&g_rig);
            if let (Some(solver), Some(clip)) =
                (&self.solver, self.athlete.clip_for(sport_name(g_sport)))
            {
                let fraction = clip_fraction(
                    g_stroke.cycle_frac,
                    g_stroke.phase,
                    g_stroke.drive_frac,
                    clip.drive_end,
                );
                let posed = solver.pose(
                    &self.athlete,
                    g_sport,
                    clip,
                    fraction * f64::from(clip.duration),
                    &g_targets.contacts,
                    &grip_frames(
                        g_sport,
                        &g_rig,
                        g_targets.poles,
                        warped_cycle(g_stroke.warped_phase),
                    ),
                    g_targets.oar,
                );
                solver.pack(&posed, &mut self.ghost_frame);
            }

            // Ghost course placement.
            use rowplay_viewmodel::replay::course::GHOST_LOOP_RADIUS;
            let g_placement = place(g_sport, g_distance, GHOST_LOOP_RADIUS);
            let g_cues = match sample_motion_graph(g_sport, &g_stroke) {
                ReplayMotionGraph::Rower(g) => AccentCues {
                    vertical: g.accents.vertical.value,
                    surge: g.accents.surge.value,
                },
                ReplayMotionGraph::Skierg(g) => AccentCues {
                    vertical: g.accents.rebound.value,
                    surge: g.accents.surge.value,
                },
                ReplayMotionGraph::Bike(_) => AccentCues::default(),
            };
            self.ghost_anim_phase = advance_anim_phase(
                self.ghost_anim_phase,
                g_sampled.spm,
                dt,
                playing,
                self.reduce_motion,
            );
            let g_accent = accents(
                g_sport,
                g_cues,
                self.ghost_anim_phase,
                g_sampled.spm,
                self.reduce_motion,
            );

            // Pack ghost course/accents into ghost_frame.
            let gf = &mut self.ghost_frame;
            gf[frame::COURSE_X] = g_placement.x as f32;
            gf[frame::COURSE_Z] = g_placement.z as f32;
            write_quat(gf, frame::COURSE_YAW, yaw_rotation(g_placement.yaw));
            gf[frame::ACCENT_BOB] = g_accent.bob as f32;
            gf[frame::ACCENT_SURGE] = g_accent.surge as f32;
            write_quat(gf, frame::ACCENT_ROLL, roll_rotation(g_accent.roll));

            // Ghost equipment (same structure as player).
            for value in &mut gf[frame::EQUIPMENT..frame::EQUIPMENT + frame::EQUIPMENT_COUNT] {
                *value = 0.0;
            }
            match g_rig {
                SportRigPose::Rower(rower) => {
                    gf[frame::EQ_SEAT_Z] = rower.seat_z as f32;
                    let [left, right] = oar_rotations(rower.oar_sweep, rower.oar_feather);
                    write_quat(gf, frame::EQ_OAR_LEFT, left);
                    write_quat(gf, frame::EQ_OAR_RIGHT, right);
                    gf[frame::EQ_BLADE_ROLL_DEG] = blade_roll_degrees(rower.blade_feather) as f32;
                    // Blade positions.
                    let anchors_data = &anchors::ANCHORS;
                    let oar_anchor = anchors_data
                        .iter()
                        .find(|a| a.template == "equipment:row:oar-rig");
                    if let Some(anchor) = oar_anchor {
                        for (at, oar_q, idx) in [
                            (frame::EQ_BLADE_LEFT, left, 1u8),
                            (frame::EQ_BLADE_RIGHT, right, 0),
                        ] {
                            let pos = anchors::clone_position(anchor.template, idx);
                            let oarlock = [f64::from(pos[0]), f64::from(pos[1]), f64::from(pos[2])];
                            let bp = blade_position(oarlock, oar_q);
                            for i in 0..3 {
                                gf[at + i] = bp[i] as f32;
                            }
                            write_quat(gf, at + 3, oar_q);
                        }
                    }
                }
                SportRigPose::SkiErg(_) => {
                    if let Some([left_pole, right_pole]) = g_targets.poles {
                        for (pole_at, leaves_at, pole) in [
                            (frame::EQ_POLE_LEFT, frame::EQ_POLE_LEAVES_LEFT, left_pole),
                            (
                                frame::EQ_POLE_RIGHT,
                                frame::EQ_POLE_LEAVES_RIGHT,
                                right_pole,
                            ),
                        ] {
                            for i in 0..3 {
                                gf[pole_at + i] = pole.root[i] as f32;
                            }
                            let pr = pole_rotation(pole.direction);
                            write_quat(gf, pole_at + 3, pr);
                            for (leaf_idx, fit) in self.pole_fits.iter().enumerate() {
                                if let Some(fit) = fit {
                                    let lp = pole_leaf_position(pole.root, pr, fit.position);
                                    for i in 0..3 {
                                        gf[leaves_at + leaf_idx * 3 + i] = lp[i] as f32;
                                    }
                                }
                            }
                        }
                    }
                }
                SportRigPose::Bike(bike) => {
                    write_quat(gf, frame::EQ_CRANK, crank_rotation(bike.crank_angle));
                    write_quat(gf, frame::EQ_WHEEL, wheel_rotation(bike.wheel_angle));
                }
            }

            // Race gap: positive = player ahead, negative = behind.
            // The text uses absolute values with a sign label so QML shows
            // e.g. "+20 m (0:04 ahead)" or "15 m (0:03 behind)".
            use rowplay_core::formatting::fmt_time;
            use rowplay_core::replay::race_gap::{race_gap_metres, race_gap_seconds};
            let gap_m = race_gap_metres(sampled.d, g_sampled.d);
            let gap_s = race_gap_seconds(gap_m, sampled.pace);
            if gap_m.abs() < 0.5 {
                "—".clone_into(&mut self.gap_text);
            } else {
                let abs_m = gap_m.abs();
                let abs_s = gap_s.abs();
                let label = if gap_m > 0.0 { "ahead" } else { "behind" };
                self.gap_text = format!("{:.0} m ({} {label})", abs_m, fmt_time(abs_s, false));
            }

            // Race result at finish: computed by race_result from the
            // player and rival strokes, shown as a verdict string. The
            // overlay reads the result rather than deciding the winner.
            // Race result at finish: pass structured data so QML can
            // format with the web's locale ids (replay.raceVerdictWin/
            // LoseSession). Format: "win|<seconds>|<metres>" or
            // "lose|<seconds>|<metres>" or "tie".
            if !playing && self.verdict_text.is_empty() {
                use rowplay_core::replay::race_result::{RaceOutcome, race_result};
                let ghost_strokes = ghost.state.strokes().to_vec();
                if let (Some(playback), Some(workout)) =
                    (&self.playback, &self.workout_for_result())
                {
                    let result = race_result(playback.state.strokes(), &ghost_strokes, workout);
                    if let Some(result) = result {
                        let seconds = result.time_margin.unwrap_or(0.0);
                        let metres = result.distance_margin.unwrap_or(0.0);
                        self.verdict_text = match result.outcome {
                            RaceOutcome::PlayerWon => {
                                format!("win|{seconds:.1}|{metres:.0}")
                            }
                            RaceOutcome::RivalWon => {
                                format!("lose|{seconds:.1}|{metres:.0}")
                            }
                            RaceOutcome::Tie => "tie".to_owned(),
                        };
                    }
                }
            }
        }

        true
    }
}

fn write_quat(frame: &mut [f32], at: usize, q: [f64; 4]) {
    for i in 0..4 {
        frame[at + i] = q[i] as f32;
    }
}

fn write_vec3(frame: &mut [f32], at: usize, v: [f64; 3]) {
    for i in 0..3 {
        frame[at + i] = v[i] as f32;
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The library is loaded by the `Library` singleton in the app; the
    /// tests publish the demo library directly.
    fn seed_demo_library() {
        AppState::get().set_details(rowplay_core::demo::demo_details());
    }

    /// Spec R6.2: 600 driven ticks produce exactly one notify bundle each,
    /// and every bundle has the layout's length. The singleton is built
    /// without a QML engine, so the notify is counted at its single
    /// emission site and the Qt signal itself is skipped (unattached).
    #[test]
    fn six_hundred_ticks_cross_the_bridge_once_each() {
        seed_demo_library();
        let mut replay = ReplayBackend::default();
        assert_eq!(replay.load_state, "loading", "{}", replay.error_text);
        replay.load_workout(-1);
        assert!(replay.has_workout, "the demo default loads without a token");
        assert_eq!(replay.emit_count, 1, "loading renders one paused frame");
        let seq_after_load = replay.frame_seq;
        replay.play();
        for _ in 0..600 {
            replay.tick(1.0 / 60.0);
        }
        assert_eq!(replay.emit_count, 601);
        assert_eq!(replay.frame_seq, seq_after_load + 600);
        assert_eq!(replay.frame.len(), frame::LENGTH);
        assert!(
            replay
                .frame
                .iter()
                .skip(1)
                .all(|v| v.is_finite() || v.is_nan())
        );
        assert!(replay.frame[frame::HUD_ELAPSED] > 9.9, "ten seconds played");
        assert_eq!(replay.hud_text.split('|').count(), 7);
        // Paused ticks produce no frame; a seek produces exactly one.
        replay.pause();
        replay.tick(1.0 / 60.0);
        assert_eq!(replay.emit_count, 601);
        replay.seek(0.5);
        assert_eq!(replay.emit_count, 602);
        assert!((replay.progress - 0.5).abs() < 1e-9);
    }

    /// The V4 athlete contract declares 51 skin joints (19 semantic + 32
    /// helpers). The 19 semantic bones are the ones the pose solver packs
    /// into the frame bundle; the remaining 32 are passive (finger, toe,
    /// ring bones that stay at their rest pose). A renamed or missing
    /// semantic bone would silently pose nothing, so assert the count
    /// and confirm every semantic name resolves.
    #[test]
    fn semantic_joint_count_matches_the_contract() {
        let athlete = assets::embedded_athlete();
        assert_eq!(athlete.joints.len(), 51, "V4 contract: 51 skin joints");
        assert_eq!(athlete.semantic.len(), 19, "V4 contract: 19 semantic bones");
        assert_eq!(athlete.semantic.len(), frame::JOINT_COUNT);
        // Every semantic index resolves to a real joint.
        for (slot, &joint_index) in athlete.semantic.iter().enumerate() {
            assert!(
                joint_index < athlete.joints.len(),
                "semantic slot {slot} points at joint index {joint_index} \
                 but the skeleton has only {} joints",
                athlete.joints.len()
            );
        }
        // The solver can be built (the contact plan resolves).
        assert!(
            PoseSolver::new(&athlete).is_ok(),
            "PoseSolver::new must succeed on the embedded athlete"
        );
    }

    #[test]
    fn loading_a_workout_switches_the_scene_to_its_sport() {
        seed_demo_library();
        let mut replay = ReplayBackend::default();
        replay.load_workout(1003);
        assert_eq!(replay.workout_id, 1003);
        assert_eq!(replay.sport_index, 1, "1003 is the demo SkiErg workout");
        assert_eq!(replay.sky_zenith, "#357db3");
        replay.load_workout(1004);
        assert_eq!(replay.sport_index, 2);
        replay.load_workout(424_242);
        assert_eq!(
            replay.workout_id, DEFAULT_WORKOUT_ID,
            "unknown ids fall back to the demo default"
        );
    }

    /// The capture-walk close-up camera must frame the athlete, not the venue
    /// centre: it is the lens used to judge wrist and posture detail, and it
    /// previously sat ~one loop radius away (framing venue geometry on all
    /// three sports). Assert the camera lands within a couple of metres of the
    /// athlete's placement, on the far side of the loop too.
    ///
    /// It must also *track* the stroke: a fixed rig offset cropped the near
    /// body at the catch and the finish (where the seat slide and layback are
    /// extreme) — the two phases this lens exists to judge. The aim follows
    /// the contact midpoint, so it must move between catch and finish.
    #[test]
    fn the_closeup_camera_tracks_the_athlete_through_the_stroke() {
        seed_demo_library();
        let replay_state = |fraction: f64| {
            let mut replay = ReplayBackend::default();
            replay.load_workout(1001);
            replay.seek(fraction);
            replay.set_closeup_camera(true);
            let frame = replay.frame.clone();
            (
                [
                    frame[frame::CAMERA_POSITION],
                    frame[frame::CAMERA_POSITION + 1],
                    frame[frame::CAMERA_POSITION + 2],
                ],
                [
                    frame[frame::CAMERA_AIM],
                    frame[frame::CAMERA_AIM + 1],
                    frame[frame::CAMERA_AIM + 2],
                ],
            )
        };
        // Catch and finish are the two extremes of the stroke.
        let (_, catch_aim) = replay_state(0.49995);
        let (_, finish_aim) = replay_state(0.50187);
        let moved = ((catch_aim[0] - finish_aim[0]).powi(2)
            + (catch_aim[2] - finish_aim[2]).powi(2))
        .sqrt();
        assert!(
            moved > 0.05,
            "the close-up aim barely moves between catch {catch_aim:?} and finish {finish_aim:?} \
             ({moved:.3} m) — it must follow the athlete, not a fixed rig offset"
        );
    }

    #[test]
    fn the_closeup_camera_frames_the_athlete_not_the_venue_centre() {
        seed_demo_library();
        let mut replay = ReplayBackend::default();
        for (id, expect_sport) in [(1001i64, 0i64), (1003, 1i64), (1004, 2i64)] {
            replay.load_workout(id);
            replay.seek(0.5);
            assert_eq!(replay.sport_index, expect_sport, "workout {id}");
            replay.set_closeup_camera(true);
            let f = &replay.frame;
            let camera = [
                f[frame::CAMERA_POSITION],
                f[frame::CAMERA_POSITION + 1],
                f[frame::CAMERA_POSITION + 2],
            ];
            let aim = [
                f[frame::CAMERA_AIM],
                f[frame::CAMERA_AIM + 1],
                f[frame::CAMERA_AIM + 2],
            ];
            let athlete = [f[frame::COURSE_X], 0.0f32, f[frame::COURSE_Z]];
            let camera_distance =
                ((camera[0] - athlete[0]).powi(2) + (camera[2] - athlete[2]).powi(2)).sqrt();
            assert!(
                (1.0..5.0).contains(&camera_distance),
                "workout {id}: close-up camera is {camera_distance:.2} m from the athlete \
                 (camera {camera:?}, athlete {athlete:?}) — it must frame the athlete, \
                 not the venue centre"
            );
            let aim_distance =
                ((aim[0] - athlete[0]).powi(2) + (aim[2] - athlete[2]).powi(2)).sqrt();
            // SkiErg aim includes the 1.45 m surge, so it sits farther from
            // the unsurged course root than the row/bike midpoint.
            let aim_budget = if id == 1003 { 2.5 } else { 1.2 };
            assert!(
                aim_distance < aim_budget,
                "workout {id}: close-up aim is {aim_distance:.2} m off the athlete"
            );
            let cam_to_aim = ((camera[0] - aim[0]).powi(2)
                + (camera[1] - aim[1]).powi(2)
                + (camera[2] - aim[2]).powi(2))
            .sqrt();
            assert!(
                cam_to_aim > 1.5,
                "workout {id}: close-up stand-off is {cam_to_aim:.2} m — the lens is \
                 inside the figure"
            );
            replay.set_closeup_camera(false);
        }
    }

    /// Ghost rendering: loading a rival workout produces a ghost frame
    /// that advances in parallel with the player.
    #[test]
    fn ghost_advances_on_the_ghost_loop() {
        seed_demo_library();
        let mut replay = ReplayBackend::default();
        replay.load_workout(1001); // rower
        replay.load_ghost(1002); // another rower as rival
        assert!(replay.has_ghost);
        assert_eq!(replay.ghost_frame.len(), frame::LENGTH);
        replay.play();
        for _ in 0..60 {
            replay.tick(1.0 / 60.0);
        }
        // The ghost frame should have moved on the course.
        assert!(
            replay.ghost_frame[frame::COURSE_X] != 0.0
                || replay.ghost_frame[frame::COURSE_Z] != 0.0,
            "ghost should be placed on the course loop"
        );
        // Gap text should be populated.
        assert!(!replay.gap_text.is_empty(), "gap text should be set");
        // Dismissing the ghost clears the frame.
        replay.load_ghost(-1);
        assert!(!replay.has_ghost);
        assert!(replay.ghost_frame.is_empty());
    }
}
