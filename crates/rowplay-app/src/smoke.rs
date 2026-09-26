// SPDX-License-Identifier: GPL-3.0-or-later
//! Phase 0 stack smoke-test backend.
//!
//! QML drives one `tick(dt)` slot per frame from a `FrameAnimation` and reads
//! the results back as plain properties (one crossing per frame each way).

use qtbridge::QmlElement;
use qtbridge::qobject;
use rowplay_core::demo::mock_workouts;
use rowplay_core::formatting::fmt_pace;

/// Singleton exposed to QML as `Smoke` in the `RowPlay` module.
pub struct SmokeBackend {
    elapsed: f64,
    frames: u64,
    sphere_y: f64,
    cube_angle: f64,
    status: String,
    screenshot_path: String,
    exit_after_frames: i32,
}

impl Default for SmokeBackend {
    fn default() -> Self {
        let workouts = mock_workouts();
        let status = match workouts.first() {
            Some(latest) => format!(
                "core: {} demo workouts, latest {} {} at {}",
                workouts.len(),
                latest.sport.display_name(),
                latest.workout_type.as_deref().unwrap_or("workout"),
                fmt_pace(latest.pace)
            ),
            None => "core: no demo workouts".to_owned(),
        };
        SmokeBackend {
            elapsed: 0.0,
            frames: 0,
            sphere_y: 60.0,
            cube_angle: 0.0,
            status,
            screenshot_path: std::env::var("ROWPLAY_SMOKE_SCREENSHOT").unwrap_or_default(),
            exit_after_frames: std::env::var("ROWPLAY_SMOKE_EXIT_AFTER_FRAMES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        }
    }
}

#[qobject(NoQmlElement)]
impl SmokeBackend {
    qproperty!("elapsed", Member = elapsed, Notify = frame_changed);
    qproperty!("frames", Member = frames, Notify = frame_changed);
    qproperty!("sphereY", Member = sphere_y, Notify = frame_changed);
    qproperty!("cubeAngle", Member = cube_angle, Notify = frame_changed);
    qproperty!("status", Member = status, Constant);
    qproperty!("screenshotPath", Member = screenshot_path, Constant);
    qproperty!("exitAfterFrames", Member = exit_after_frames, Constant);

    /// Emitted once per tick after every per-frame property changed.
    #[qsignal(qml_name = "frameChanged")]
    fn frame_changed(&mut self);

    /// Advance the scene by `dt` seconds (called from QML's `FrameAnimation`).
    #[qslot]
    fn tick(&mut self, dt: f64) {
        let dt = if dt.is_finite() {
            dt.clamp(0.0, 0.1)
        } else {
            0.0
        };
        self.elapsed += dt;
        self.frames += 1;
        self.sphere_y = 60.0 + 25.0 * (self.elapsed * 2.0).sin();
        self.cube_angle = (self.elapsed * 45.0) % 360.0;
        self.frame_changed();
    }
}

// qtbridge derives the module URI from the Cargo package name (`rowplay_app`);
// implementing `QmlElement` by hand keeps the QML-facing name `RowPlay`.
impl QmlElement for SmokeBackend {
    const URI: &str = "RowPlay";
    const ELEMENT_NAME: &str = "Smoke";
    const MAJOR_VERSION: u8 = 1;
    const MINOR_VERSION: u8 = 0;
    const IS_SINGLETON: bool = true;
}
