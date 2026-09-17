// SPDX-License-Identifier: GPL-3.0-or-later
//! The chase camera (Phase 5b spec R4; web `renderer3d.ts` `CAMERA_RIGS`,
//! `BASE_CAMERA_FOV`, `SPEED_CAMERA_FOV_GAIN` and the per-frame camera block
//! of `render`), ported for the desktop production path. Includes the
//! ghost-comparison framing (`ghostPullback`, midpoint focus,
//! `comparisonPullback` from the horizontal FOV, with-ghost narrow scales
//! `2.12`/`1.38`, height addend from `comparisonSpan`) — the audit called
//! this the last uncovered composition; without it Phase 5c's ghost could
//! sit outside the framed lane. None of the query-gated QA cameras
//! (`athlete-grip`, `athlete-front`, `athlete-rear`, `athlete-top`) are
//! ported: those exist only for capture harnesses on the web and the port
//! uses `closeup_camera_view` for its own close-up.
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
/// Extra back distance when a ghost is present (web `ghostPullback`, added
/// to `back` on top of `sportRig.back` / `ROWER_DESKTOP_BACK`).
pub const GHOST_PULLBACK: f64 = 1.05;
/// Extra narrow-stage back multiplier when the rower has a ghost — a wider
/// oar-plus-ghost envelope (web `state.ghost ? 2.12 : 2.1`).
pub const ROWER_NARROW_GHOST_SCALE: f64 = 2.12;
/// Non-rower narrow-stage back multiplier when a ghost is present (web
/// `state.ghost ? 1.38 : 1.2`).
pub const NARROW_GHOST_SCALE: f64 = 1.38;
/// Screen-space air the pair should keep inside the horizontal FOV, metres
/// (web `comparisonMargin = this.sport === "rower" ? 1.6 : 1.1`, RowErg
/// value — SkiErg and BikeErg use [`COMPARISON_MARGIN_SPRINT`]).
pub const COMPARISON_MARGIN_ROWER: f64 = 1.6;
/// See [`COMPARISON_MARGIN_ROWER`] — non-rower value.
pub const COMPARISON_MARGIN_SPRINT: f64 = 1.1;
/// Fraction of the horizontal half-tangent the pair may occupy before the
/// camera pulls back (web `Math.tan(horizontalHalfFov) * 0.9`).
pub const COMPARISON_FILL: f64 = 0.9;
/// Height addend per metre of comparison span, capped (web
/// `Math.min(2.5, comparisonSpan * 0.16)`).
pub const COMPARISON_HEIGHT_RATE: f64 = 0.16;
/// Cap on the height addend from [`COMPARISON_HEIGHT_RATE`].
pub const COMPARISON_HEIGHT_CAP: f64 = 2.5;

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
    /// Ghost placement `(x, z)` in the same course-world frame as the focus
    /// when a ghost is racing (Phase 5c); `None` if there is no ghost. The
    /// chase then frames the midpoint of `(focus, ghost)`, adds
    /// [`GHOST_PULLBACK`] to the base back, and, when the pair is far
    /// apart, adds a comparison pullback derived from the horizontal FOV
    /// so both fit in frame (web `state.ghost ? ... : ...` branches).
    pub ghost_placement: Option<(f64, f64)>,
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
    // When a ghost is present (web `state.ghost`) the base back gains
    // `GHOST_PULLBACK`, the narrow scales widen, the focus is the midpoint
    // of live and ghost, the height rises with the ghost span, and a
    // horizontal-FOV `comparisonPullback` adds enough back to fit both.
    let rig = rig(sport);
    let narrow = input.aspect < NARROW_ASPECT;
    let rower = sport == Sport::Rower;
    let ghost = input.ghost_placement;
    let ghost_pullback = if ghost.is_some() { GHOST_PULLBACK } else { 0.0 };
    let narrow_scale = if rower {
        if ghost.is_some() {
            ROWER_NARROW_GHOST_SCALE
        } else {
            2.1
        }
    } else if ghost.is_some() {
        NARROW_GHOST_SCALE
    } else {
        1.2
    };
    let base_back = if input.reduce_motion {
        (if rower { 6.2 } else { rig.back + 0.8 }) + ghost_pullback
    } else if narrow {
        (rig.back + ghost_pullback) * narrow_scale
    } else if rower {
        ROWER_DESKTOP_BACK + ghost_pullback
    } else {
        rig.back + ghost_pullback
    };
    let base_height = if input.reduce_motion {
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
    let (focus_x, focus_z, comparison_span) = match ghost {
        Some((gx, gz)) => ((fx + gx) * 0.5, (fz + gz) * 0.5, (fx - gx).hypot(fz - gz)),
        None => (fx, fz, 0.0),
    };
    // Horizontal FOV from the vertical `fov` and the current aspect. When a
    // ghost is present, add enough back so the pair (with a per-sport
    // margin) fits inside `COMPARISON_FILL` of the horizontal half-tangent
    // — the web derives `comparisonPullback` this way rather than assuming
    // a small lane-only offset, so it stays valid to the largest possible
    // 500 m chord on the 1 km loop.
    let vertical_half_fov = fov.to_radians() * 0.5;
    let horizontal_half_fov = (vertical_half_fov.tan() * input.aspect.max(0.01)).atan();
    let comparison_margin = if rower {
        COMPARISON_MARGIN_ROWER
    } else {
        COMPARISON_MARGIN_SPRINT
    };
    let required_back = if ghost.is_some() {
        (comparison_span * 0.5 + comparison_margin)
            / (horizontal_half_fov.tan() * COMPARISON_FILL).max(0.05)
    } else {
        base_back
    };
    let comparison_pullback = (required_back - base_back).max(0.0);
    let back = base_back + comparison_pullback;
    let height =
        base_height + (comparison_span * COMPARISON_HEIGHT_RATE).min(COMPARISON_HEIGHT_CAP);

    let tangent_len = input.tangent_x.hypot(input.tangent_z).max(1e-6);
    let (tx, tz) = (input.tangent_x / tangent_len, input.tangent_z / tangent_len);
    let focus_radius = focus_x.hypot(focus_z).max(1e-6);
    let (rx, rz) = (focus_x / focus_radius, focus_z / focus_radius);
    let chase = [
        focus_x - tx * back + rx * lateral,
        height,
        focus_z - tz * back + rz * lateral,
    ];
    let look_at = [focus_x + tx * ahead, rig.aim_y, focus_z + tz * ahead];

    let layout_mode =
        u8::from(narrow) | (u8::from(input.reduce_motion) << 1) | (u8::from(ghost.is_some()) << 2);
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
            ghost_placement: None,
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
            // Ghost-framing constants: `GHOST_PULLBACK`,
            // `ROWER_NARROW_GHOST_SCALE` (`2.12`) vs `2.1` without,
            // `NARROW_GHOST_SCALE` (`1.38`) vs `1.2` without,
            // `COMPARISON_MARGIN_ROWER/SPRINT` (`1.6`/`1.1`),
            // `COMPARISON_FILL` (`0.9`), `COMPARISON_HEIGHT_RATE` (`0.16`)
            // and `COMPARISON_HEIGHT_CAP` (`2.5`).
            "const ghostPullback = state.ghost ? 1.05 : 0;",
            "this.sport === \"rower\" ? (state.ghost ? 2.12 : 2.1) : state.ghost ? 1.38 : 1.2;",
            "const comparisonMargin = this.sport === \"rower\" ? 1.6 : 1.1;",
            "Math.tan(horizontalHalfFov) * 0.9",
            "Math.min(2.5, comparisonSpan * 0.16)",
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

    #[test]
    fn ghost_pullback_and_midpoint_focus_engage_when_ghost_is_placed() {
        // BikeErg desktop, aspect 1.5 (not narrow), ghost 6 m ahead of the
        // live athlete along +x. Base back is `rig.back + ghostPullback`;
        // focus is the midpoint of live and ghost; height gains
        // `comparisonSpan * 0.16` capped at 2.5.
        let mut inp = input(true, 0.016, 0.0);
        inp.ghost_placement = Some((6.0, 30.0));
        let state = chase(Sport::Bike, inp, CameraState::new(Sport::Bike));
        let rig_bike = rig(Sport::Bike);
        // Focus midpoint x: (0 + 6) / 2 = 3.0. Tangent is +x, so `chase.x
        // = focus_x - back + lateral_x_contrib`. With focus at (3, 30),
        // radial = (3, 30)/hypot ≈ (0.0995, 0.9950), so lateral adds
        // `0.0995 * 1.92 ≈ 0.191` on x and `0.995 * 1.92 ≈ 1.910` on z.
        // Verify midpoint (not the live focus) drives placement.
        let mid_x: f64 = 3.0;
        let mid_z: f64 = 30.0;
        let radial_len = mid_x.hypot(mid_z);
        let expected_lateral_x = mid_x / radial_len * rig_bike.lateral;
        let expected_lateral_z = mid_z / radial_len * rig_bike.lateral;
        // Base back before comparisonPullback: rig.back + GHOST_PULLBACK.
        let base_back = rig_bike.back + GHOST_PULLBACK;
        // comparisonSpan = 6, margin 1.1, fov 42° → horizontal fov derives
        // enough back to fit; assert the composition is honoured (back must
        // be at least base_back).
        let back = mid_x - state.position[0] + expected_lateral_x;
        assert!(back > base_back - 1e-9, "back {back} < base {base_back}");
        // Height gains comparisonSpan * 0.16 = 0.96, uncapped at 2.5.
        let expected_height = rig_bike.height + 6.0 * COMPARISON_HEIGHT_RATE;
        assert!(
            (state.position[1] - expected_height).abs() < 1e-9,
            "{}",
            state.position[1]
        );
        // Lateral z contribution places the camera off the +z side of the
        // midpoint (radial direction is +z-heavy from the midpoint).
        assert!(
            (state.position[2] - (mid_z + expected_lateral_z - back * 0.0)).abs() < 1e-3,
            "{} vs mid+lateral_z={}",
            state.position[2],
            mid_z + expected_lateral_z
        );
        // Aim tracks the midpoint too (tangent +x, aim ahead).
        assert!((state.aim[0] - (mid_x + rig_bike.ahead)).abs() < 1e-9);
        assert!((state.aim[2] - mid_z).abs() < 1e-9);
    }

    #[test]
    fn narrow_ghost_widens_the_pullback_scale_per_sport() {
        // Narrow aspect 1.0 (< NARROW_ASPECT 1.25). RowErg with a ghost
        // scales by `ROWER_NARROW_GHOST_SCALE 2.12` (vs 2.1 without).
        // Non-rower with a ghost scales by `NARROW_GHOST_SCALE 1.38`
        // (vs 1.2 without).
        let mut inp = input(true, 0.016, 0.0);
        inp.aspect = 1.0;
        inp.ghost_placement = Some((0.0, 30.0)); // Same lane as live (0 span).
        let row = chase(Sport::Rower, inp, CameraState::new(Sport::Rower));
        // Zero-span ghost still adds GHOST_PULLBACK to back but no
        // comparisonPullback (pair fits trivially). Row narrow back:
        // (rig.back + GHOST_PULLBACK) * 2.12.
        let rig_row = rig(Sport::Rower);
        let expected_row_back = (rig_row.back + GHOST_PULLBACK) * ROWER_NARROW_GHOST_SCALE;
        assert!(
            (row.position[0] + expected_row_back).abs() < 1e-9,
            "{} vs {expected_row_back}",
            row.position[0]
        );
        let ski = chase(Sport::Skierg, inp, CameraState::new(Sport::Skierg));
        let rig_ski = rig(Sport::Skierg);
        let expected_ski_back = (rig_ski.back + GHOST_PULLBACK) * NARROW_GHOST_SCALE;
        // SkiErg narrow: lateral scales by 0.68 too; use radial-decomposed
        // check like the primary composition test.
        let lateral = rig_ski.lateral * 0.68;
        let mid_z: f64 = 30.0;
        let mid_x: f64 = 0.0;
        let radial_len = mid_x.hypot(mid_z);
        let expected_ski_x = mid_x - expected_ski_back + mid_x / radial_len * lateral;
        assert!(
            (ski.position[0] - expected_ski_x).abs() < 1e-9,
            "{} vs {expected_ski_x}",
            ski.position[0]
        );
    }

    #[test]
    fn a_wide_pair_forces_comparison_pullback_from_the_horizontal_fov() {
        // A 200 m gap between live and ghost must not crop out either — the
        // web derives `comparisonPullback` from the horizontal FOV so the
        // pair (plus per-sport margin) fits in `COMPARISON_FILL 0.9` of the
        // horizontal half-tangent.
        let mut inp = input(true, 0.016, 0.0);
        inp.focus_x = -100.0;
        inp.aspect = 1.5;
        inp.ghost_placement = Some((100.0, 30.0));
        let state = chase(Sport::Bike, inp, CameraState::new(Sport::Bike));
        let rig_bike = rig(Sport::Bike);
        // Expected: back derived from `(comparisonSpan/2 + margin) /
        // (tan(horizontal_half_fov) * 0.9)`. `comparisonSpan =
        // hypot(200, 0)` (both on z=30). At fov 42° and aspect 1.5, the
        // horizontal half-tangent ≈ tan(atan(tan(21°)·1.5)) ≈ 0.575.
        // Required back ≈ (100 + 1.1) / (0.575·0.9) ≈ 195.4 m.
        let vertical_half = (42.0f64).to_radians() * 0.5;
        let horiz_half = (vertical_half.tan() * 1.5).atan();
        let expected_required =
            (200.0f64 * 0.5 + COMPARISON_MARGIN_SPRINT) / (horiz_half.tan() * COMPARISON_FILL);
        let expected_base = rig_bike.back + GHOST_PULLBACK;
        let expected_back = expected_required.max(expected_base);
        // Focus midpoint is (0, 30); back along -x (tangent +x) so the
        // camera sits at focus.x - back + lateral_x. Height caps at 2.5
        // (span 200 * 0.16 = 32, well above the cap).
        let mid_x: f64 = 0.0;
        let mid_z: f64 = 30.0;
        let lateral = rig_bike.lateral;
        let radial_len = mid_x.hypot(mid_z);
        let lateral_x = mid_x / radial_len * lateral;
        assert!(
            (state.position[0] - (mid_x - expected_back + lateral_x)).abs() < 1e-6,
            "{} vs {}",
            state.position[0],
            mid_x - expected_back + lateral_x,
        );
        assert!(
            (state.position[1] - (rig_bike.height + COMPARISON_HEIGHT_CAP)).abs() < 1e-9,
            "height {} did not cap at {}",
            state.position[1],
            rig_bike.height + COMPARISON_HEIGHT_CAP
        );
    }
}
