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
use rowplay_core::replay::motion_graph::{ReplayMotionGraph, sample_motion_graph};
use rowplay_core::replay::rig_pose::{SportRigPose, solve_rig_pose};
use rowplay_core::replay::stroke_model::{
    StrokeTimeline, build_stroke_timeline, reduced_motion, stroke_pose_at,
};
use rowplay_viewmodel::replay::athlete::V4Athlete;
use rowplay_viewmodel::replay::camera::{CameraInput, CameraState, chase};
use rowplay_viewmodel::replay::course::{
    AccentCues, LIVE_LOOP_RADIUS, accents, advance_anim_phase, place,
};
use rowplay_viewmodel::replay::equipment::{
    PoleLeafFit, blade_position, blade_roll_degrees, crank_rotation,
    layout_json as equipment_layout_json, oar_rotations, pole_leaf_fits, pole_leaf_position,
    pole_rotation, roll_rotation, wheel_rotation, yaw_rotation,
};
use rowplay_viewmodel::replay::frame;
use rowplay_viewmodel::replay::hud::{hud_bundle, hud_numbers, hud_strings};
use rowplay_viewmodel::replay::pose::{PoseSolver, clip_fraction, rig_targets};
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

fn sport_name(sport: Sport) -> &'static str {
    match sport {
        Sport::Rower => "rower",
        Sport::Skierg => "skierg",
        Sport::Bike => "bike",
    }
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
        self.notify_replay();
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
        let mut stroke = stroke_pose_at(&playback.timeline, time);
        if self.reduce_motion {
            stroke = reduced_motion(&stroke);
        }
        let distance = sampled.d;

        // Rig pose → contact targets → the posed athlete.
        let rig = solve_rig_pose(sport, &stroke, distance, self.reduce_motion);
        let targets = rig_targets(&rig);
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
            );
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
        for i in 0..3 {
            f[frame::CAMERA_POSITION + i] = self.camera.position[i] as f32;
            f[frame::CAMERA_AIM + i] = self.camera.aim[i] as f32;
        }
        f[frame::CAMERA_FOV] = self.camera.fov as f32;
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
                let [left, right] = oar_rotations(rower.oar_sweep, rower.oar_feather);
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
}
