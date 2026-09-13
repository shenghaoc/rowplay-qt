// SPDX-License-Identifier: GPL-3.0-or-later
//! The chase camera (Phase 5b spec R4; web `renderer3d.ts` `CAMERA_RIGS`,
//! `BASE_CAMERA_FOV`, `SPEED_CAMERA_FOV_GAIN` and the per-frame camera block
//! of `render`), ported for the desktop production path: no ghost lane
//! (Phase 5c) and none of the query-gated QA cameras.
//!
//! All maths runs in Rust inside `tick`; the result crosses the bridge as
//! part of the flat frame (position, aim, field of view) and QML only
//! assigns them. Damping uses [`rowplay_core::replay::motion::damp_factor`]
//! so it is frame-rate independent, exactly as the web.

use rowplay_core::models::Sport;
use rowplay_core::replay::motion::damp_factor;

/// Per-sport chase framing (web `CAMERA_RIGS`), metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChaseRig {
    /// Distance behind the focus along the tangent.
    pub back: f64,
    /// Camera height.
    pub height: f64,
    /// Aim point ahead of the focus along the tangent.
    pub ahead: f64,
    /// Lateral offset along the loop's radial direction.
    pub lateral: f64,
    /// Aim height.
    pub aim_y: f64,
}

/// The web's `CAMERA_RIGS` per sport.
#[must_use]
pub fn rig(sport: Sport) -> ChaseRig {
    match sport {
        Sport::Rower => ChaseRig {
            back: 4.05,
            height: 1.78,
            ahead: 0.88,
            lateral: 2.16,
            aim_y: 0.84,
        },
        Sport::Skierg => ChaseRig {
            back: 3.15,
            height: 2.3,
            ahead: 0.9,
            lateral: 1.86,
            aim_y: 1.14,
        },
        Sport::Bike => ChaseRig {
            back: 3.12,
            height: 1.96,
            ahead: 0.58,
            lateral: 1.92,
            aim_y: 0.92,
        },
    }
}

/// The web's `BASE_CAMERA_FOV` per sport, degrees (vertical).
#[must_use]
pub fn base_fov(sport: Sport) -> f64 {
    match sport {
        Sport::Rower => 40.0,
        Sport::Skierg | Sport::Bike => 42.0,
    }
}

/// Extra field of view at full speed, degrees (web `SPEED_CAMERA_FOV_GAIN`).
pub const SPEED_CAMERA_FOV_GAIN: f64 = 2.0;
/// The desktop RowErg pull-back: the 7.8 m shell needs 5.4 m, not the rig's
/// 4.05 (web: `this.sport === "rower" ? 5.4 + ghostPullback : sportRig.back`).
pub const ROWER_DESKTOP_BACK: f64 = 5.4;
/// Viewports narrower than this aspect are "narrow" (web `aspect < 1.25`).
pub const NARROW_ASPECT: f64 = 1.25;

/// What the camera needs from one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraInput {
    /// The live focus (the rig root on the loop), metres.
    pub focus_x: f64,
    /// See `focus_x`.
    pub focus_z: f64,
    /// Unit course tangent at the focus.
    pub tangent_x: f64,
    /// See `tangent_x`.
    pub tangent_z: f64,
    /// Metres the live athlete advanced since the previous frame (negative
    /// or huge on a seek; the web's `dLive`).
    pub advanced: f64,
    /// Wall-clock seconds since the previous frame.
    pub dt: f64,
    /// Whether playback is running.
    pub playing: bool,
    /// Viewport aspect ratio (width / height).
    pub aspect: f64,
    /// The reduce-motion preference.
    pub reduce_motion: bool,
}

/// The camera's persistent state; feed the previous frame's back in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    /// Camera position, metres.
    pub position: [f64; 3],
    /// Aim point, metres (QML calls `lookAt` with it).
    pub aim: [f64; 3],
    /// Vertical field of view, degrees.
    pub fov: f64,
    /// Smoothed live speed, m/s (breathes the lens).
    pub smoothed_speed: f64,
    /// Whether a first frame has been placed (the web `cameraInit`).
    pub initialised: bool,
    /// The last layout mode (narrow / reduce-motion bits); a change snaps.
    pub layout_mode: u8,
}

impl CameraState {
    /// The state before the first frame (the web's `cameraInit = false`).
    #[must_use]
    pub fn new(sport: Sport) -> Self {
        CameraState {
            position: [0.0; 3],
            aim: [0.0; 3],
            fov: base_fov(sport),
            smoothed_speed: 0.0,
            initialised: false,
            layout_mode: 0,
        }
    }
}

/// Advance the chase camera by one frame (the web camera block, production
/// path). Returns the new state; the caller stores it for the next tick.
#[must_use]
pub fn chase(sport: Sport, input: CameraInput, prev: CameraState) -> CameraState {
    let base = base_fov(sport);
    let dt = if input.dt.is_finite() { input.dt } else { 0.0 };
    let advanced = if input.advanced.is_finite() {
        input.advanced
    } else {
        0.0
    };

    // Speed-aware FOV: the lens breathes out as the athlete runs faster.
    // Seek-sized jumps are excluded from the estimate; reduced motion pins
    // the lens flat.
    let mut smoothed = prev.smoothed_speed;
    let mut fov = prev.fov;
    if input.reduce_motion {
        smoothed = 0.0;
        fov = base;
    } else {
        if dt > 0.0 && advanced >= 0.0 && advanced < dt * 120.0 {
            let inst = if advanced > 0.0 {
                (advanced / dt).min(40.0)
            } else {
                0.0
            };
            smoothed += (inst - smoothed) * damp_factor(3.0, dt);
        }
        let fov_target = base + ((smoothed - 3.0) / 6.0).clamp(0.0, 1.0) * SPEED_CAMERA_FOV_GAIN;
        let blend = if prev.initialised {
            damp_factor(2.5, dt)
        } else {
            1.0
        };
        fov += (fov_target - fov) * blend;
    }

    // Sport-aware chase framing (desktop; narrow stages pull back).
    let rig = rig(sport);
    let narrow = input.aspect < NARROW_ASPECT;
    let rower = sport == Sport::Rower;
    let back = if input.reduce_motion {
        if rower { 6.2 } else { rig.back + 0.8 }
    } else if narrow {
        rig.back * if rower { 2.1 } else { 1.2 }
    } else if rower {
        ROWER_DESKTOP_BACK
    } else {
        rig.back
    };
    let height = if input.reduce_motion {
        rig.height + 0.7
    } else {
        rig.height + if narrow { 0.3 } else { 0.0 }
    };
    let lateral = rig.lateral
        * if narrow && sport == Sport::Skierg {
            0.68
        } else {
            1.0
        };
    let ahead = rig.ahead;

    let (fx, fz) = (input.focus_x, input.focus_z);
    let tangent_len = input.tangent_x.hypot(input.tangent_z).max(1e-6);
    let (tx, tz) = (input.tangent_x / tangent_len, input.tangent_z / tangent_len);
    let focus_radius = fx.hypot(fz).max(1e-6);
    let (rx, rz) = (fx / focus_radius, fz / focus_radius);
    let chase = [
        fx - tx * back + rx * lateral,
        height,
        fz - tz * back + rz * lateral,
    ];
    let look_at = [fx + tx * ahead, rig.aim_y, fz + tz * ahead];

    let layout_mode = u8::from(narrow) | (u8::from(input.reduce_motion) << 1);
    let layout_changed = layout_mode != prev.layout_mode;

    let mut next = CameraState {
        position: prev.position,
        aim: prev.aim,
        fov,
        smoothed_speed: smoothed,
        initialised: true,
        layout_mode,
    };
    if !prev.initialised || (input.playing && input.reduce_motion) {
        // First frame, or reduced motion while playing: an exact relative
        // rig, no lag or spring.
        next.position = chase;
        next.aim = look_at;
    } else if input.playing {
        // Exponential damping is frame-rate independent; aim is softer than
        // translation so course curvature cannot snap the horizon.
        let speed_follow = (smoothed * 0.55).min(18.0);
        let position_rate = 8.0 + speed_follow;
        let aim_rate = 6.0 + speed_follow * 0.65;
        let fp = damp_factor(position_rate, dt);
        let fa = damp_factor(aim_rate, dt);
        for i in 0..3 {
            next.position[i] = prev.position[i] + (chase[i] - prev.position[i]) * fp;
            next.aim[i] = prev.aim[i] + (look_at[i] - prev.aim[i]) * fa;
        }
    } else if advanced != 0.0 || layout_changed || distance_squared(prev.position, chase) > 9.0 {
        // Paused renders are on demand, so nothing would drive a gradual
        // convergence: snap only when the target actually jumped (seek,
        // workout change). The sub-metre trailing lag left at the pause
        // boundary is kept, avoiding a visible pop.
        next.position = chase;
        next.aim = look_at;
    } else if distance_squared(prev.aim, look_at) > 1.0 {
        next.aim = look_at;
    }
    for value in next.position.iter_mut().chain(next.aim.iter_mut()) {
        if !value.is_finite() {
            *value = 0.0;
        }
    }
    if !next.fov.is_finite() {
        next.fov = base;
    }
    next
}

fn distance_squared(a: [f64; 3], b: [f64; 3]) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(playing: bool, dt: f64, advanced: f64) -> CameraInput {
        CameraInput {
            focus_x: 0.0,
            focus_z: 30.0,
            tangent_x: 1.0,
            tangent_z: 0.0,
            advanced,
            dt,
            playing,
            aspect: 1.5,
            reduce_motion: false,
        }
    }

    #[test]
    fn the_first_frame_snaps_to_the_rig_and_the_base_lens() {
        let state = chase(
            Sport::Skierg,
            input(true, 1.0 / 60.0, 0.05),
            CameraState::new(Sport::Skierg),
        );
        let rig = rig(Sport::Skierg);
        // focus (0, 30), tangent +x, radial +z: back along -x, lateral along +z.
        assert!((state.position[0] + rig.back).abs() < 1e-9);
        assert!((state.position[1] - rig.height).abs() < 1e-9);
        assert!((state.position[2] - (30.0 + rig.lateral)).abs() < 1e-9);
        assert!((state.aim[0] - rig.ahead).abs() < 1e-9);
        assert!((state.aim[1] - rig.aim_y).abs() < 1e-9);
        assert!((state.aim[2] - 30.0).abs() < 1e-9);
        assert!(state.initialised);
        assert!(
            (state.fov - 42.0).abs() < 1e-9,
            "no speed yet: {}",
            state.fov
        );
    }

    #[test]
    fn the_desktop_rower_pulls_back_to_five_point_four_metres() {
        let state = chase(
            Sport::Rower,
            input(true, 0.016, 0.0),
            CameraState::new(Sport::Rower),
        );
        assert!((state.position[0] + ROWER_DESKTOP_BACK).abs() < 1e-9);
        let mut narrow = input(true, 0.016, 0.0);
        narrow.aspect = 1.0;
        let state = chase(Sport::Rower, narrow, CameraState::new(Sport::Rower));
        assert!((state.position[0] + 4.05 * 2.1).abs() < 1e-9);
        assert!((state.position[1] - (1.78 + 0.3)).abs() < 1e-9);
    }

    #[test]
    fn playing_damps_toward_the_target_with_the_web_rates() {
        let mut state = chase(
            Sport::Bike,
            input(true, 0.016, 0.0),
            CameraState::new(Sport::Bike),
        );
        // Move the focus 2 m ahead; the camera follows exponentially. The rig
        // sits back along the tangent plus the lateral offset along the
        // radial direction of the (now off-centre) focus.
        let mut moved = input(true, 0.1, 0.5);
        moved.focus_x = 2.0;
        let target_x = 2.0 - 3.12 + 2.0 / 2.0_f64.hypot(30.0) * 1.92;
        let before = state.position[0];
        state = chase(Sport::Bike, moved, state);
        let smoothed = 5.0 * damp_factor(3.0, 0.1);
        let expected =
            before + (target_x - before) * damp_factor(8.0 + (smoothed * 0.55).min(18.0), 0.1);
        assert!(
            (state.position[0] - expected).abs() < 1e-9,
            "{} vs {expected}",
            state.position[0]
        );
        assert!(state.position[0] > before && state.position[0] < target_x);
        // Aim is damped more softly than position.
        let aim_before = 0.58;
        let aim_target = 2.58;
        assert!(state.aim[0] > aim_before && state.aim[0] < aim_target);
        let position_progress = (state.position[0] - before) / (target_x - before);
        let aim_progress = (state.aim[0] - aim_before) / (aim_target - aim_before);
        assert!(aim_progress < position_progress);
    }

    #[test]
    fn the_lens_breathes_with_speed_and_stays_flat_under_reduced_motion() {
        let mut state = chase(
            Sport::Rower,
            input(true, 0.016, 0.0),
            CameraState::new(Sport::Rower),
        );
        for _ in 0..600 {
            state = chase(Sport::Rower, input(true, 0.016, 0.016 * 6.0), state);
        }
        assert!(state.smoothed_speed > 5.9 && state.smoothed_speed <= 6.0);
        assert!(state.fov > 40.9 && state.fov <= 42.0, "{}", state.fov);
        let mut reduced = input(true, 0.016, 0.1);
        reduced.reduce_motion = true;
        let state = chase(Sport::Rower, reduced, state);
        assert_eq!(state.fov, 40.0);
        assert_eq!(state.smoothed_speed, 0.0);
        assert!((state.position[0] + 6.2).abs() < 1e-9, "reduced rower back");
        assert!((state.position[1] - (1.78 + 0.7)).abs() < 1e-9);
    }

    #[test]
    fn paused_frames_keep_the_trailing_lag_but_snap_on_a_seek() {
        let mut state = chase(
            Sport::Skierg,
            input(true, 0.016, 0.0),
            CameraState::new(Sport::Skierg),
        );
        // Play a frame with the focus slightly ahead so a lag exists.
        let rig_x = |focus_x: f64| focus_x - 3.15 + focus_x / focus_x.hypot(30.0) * 1.86;
        let mut ahead = input(true, 0.016, 0.1);
        ahead.focus_x = 0.5;
        state = chase(Sport::Skierg, ahead, state);
        let lagging = state.position;
        assert!((lagging[0] - rig_x(0.5)).abs() > 1e-6, "a lag must exist");
        // Pause with no movement: nothing snaps.
        let mut paused = ahead;
        paused.playing = false;
        paused.advanced = 0.0;
        let held = chase(Sport::Skierg, paused, state);
        assert_eq!(held.position, lagging);
        // Pause after a seek (advanced != 0): snap to the rig.
        paused.advanced = 250.0;
        paused.focus_x = 12.0;
        let snapped = chase(Sport::Skierg, paused, held);
        assert!((snapped.position[0] - rig_x(12.0)).abs() < 1e-9);
        // A 3 m+ discrepancy also snaps even without a reported seek.
        let mut far = paused;
        far.advanced = 0.0;
        far.focus_x = 40.0;
        let snapped_far = chase(Sport::Skierg, far, snapped);
        assert!((snapped_far.position[0] - rig_x(40.0)).abs() < 1e-9);
    }

    /// Quote check against the pinned web checkout when it is present: the
    /// constants above must appear verbatim in `renderer3d.ts`.
    #[test]
    fn constants_match_the_web_source_when_the_reference_checkout_exists() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference/rowplay/src/lib/replay/renderer3d.ts");
        let Ok(source) = std::fs::read_to_string(&path) else {
            eprintln!("skipping: {} not checked out", path.display());
            return;
        };
        for needle in [
            "rower: { back: 4.05, height: 1.78, ahead: 0.88, lateral: 2.16, aimY: 0.84 }",
            "skierg: { back: 3.15, height: 2.3, ahead: 0.9, lateral: 1.86, aimY: 1.14 }",
            "bike: { back: 3.12, height: 1.96, ahead: 0.58, lateral: 1.92, aimY: 0.92 }",
            "const speedFollow = Math.min(18, this.smoothedSpeed * 0.55);",
            "const positionRate = 8 + speedFollow;",
            "const aimRate = 6 + speedFollow * 0.65;",
            "this.smoothedSpeed += (inst - this.smoothedSpeed) * dampFactor(3, dt);",
            "this.camera.position.distanceToSquared(this.chase) > 9",
            "this.cameraAim.distanceToSquared(this.lookAt) > 1",
            "const narrow = this.camera.aspect < 1.25;",
            "? 5.4 + ghostPullback",
        ] {
            assert!(
                source.contains(needle),
                "renderer3d.ts no longer contains {needle:?}"
            );
        }
        let fov_block = source
            .find("const BASE_CAMERA_FOV")
            .map(|at| &source[at..at + 400])
            .expect("BASE_CAMERA_FOV");
        assert!(
            fov_block.contains("rower: 40")
                && fov_block.contains("skierg: 42")
                && fov_block.contains("bike: 42"),
            "{fov_block}"
        );
        assert!(source.contains("const SPEED_CAMERA_FOV_GAIN = 2"));
    }
}
