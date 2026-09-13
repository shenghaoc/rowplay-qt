// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay sampling and playback state (web `replay/engine.ts`).
//!
//! `sample_at` / `sample_index_at` are pure so a ghost track can be sampled
//! with the same clock as the live one. The web drives playback from
//! `requestAnimationFrame`; here the clock lives outside the core, so
//! playback is Studio's tick-driven `ReplayState` that a QML `FrameAnimation`
//! advances with `tick(dt)`.

use crate::models::Stroke;

/// A single interpolated frame of workout state (web `Frame`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Seconds since workout start.
    pub t: f64,
    /// Cumulative distance in metres.
    pub d: f64,
    /// Instantaneous pace, seconds per 500 m.
    pub pace: f64,
    /// Strokes per minute (rpm on the BikeErg).
    pub spm: f64,
    /// Heart rate in bpm when recorded.
    pub hr: Option<f64>,
    /// Watts, derived from pace when not reported.
    pub watts: f64,
    /// Fraction of the workout completed, 0..1 (by time).
    pub progress: f64,
}

impl Frame {
    /// The zero frame returned for an empty timeline (web `sampleAt` on `[]`).
    #[must_use]
    pub fn zero(t: f64) -> Self {
        Frame {
            t,
            d: 0.0,
            pace: 0.0,
            spm: 0.0,
            hr: None,
            watts: 0.0,
            progress: 0.0,
        }
    }
}

/// Linearly interpolate the workout state at time `t` seconds (web `sampleAt`).
///
/// Non-finite `t` yields the zero frame (Studio's guard; the web would smear
/// `NaN` through the interpolation).
#[must_use]
pub fn sample_at(strokes: &[Stroke], t: f64) -> Frame {
    let n = strokes.len();
    if n == 0 || !t.is_finite() {
        return Frame::zero(if t.is_finite() { t } else { 0.0 });
    }
    let last = strokes[n - 1].t;
    let total = if last > 0.0 { last } else { 1.0 };
    let progress = (t / total).clamp(0.0, 1.0);

    if t <= strokes[0].t {
        return frame_from(&strokes[0], t, progress);
    }
    if t >= strokes[n - 1].t {
        return frame_from(&strokes[n - 1], t, progress);
    }

    // Binary search for the bracketing samples.
    let mut lo = 0_usize;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) >> 1;
        if strokes[mid].t <= t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let lower = &strokes[lo];
    let upper = &strokes[hi];
    let span = if upper.t - lower.t == 0.0 {
        1.0
    } else {
        upper.t - lower.t
    };
    let fraction = (t - lower.t) / span;

    Frame {
        t,
        d: lerp(lower.d, upper.d, fraction),
        pace: lerp(lower.pace, upper.pace, fraction),
        spm: lerp(lower.spm, upper.spm, fraction),
        hr: match (lower.hr, upper.hr) {
            (Some(ha), Some(hb)) => Some(lerp(ha, hb, fraction)),
            (ha, hb) => ha.or(hb),
        },
        watts: lerp(lower.watts, upper.watts, fraction),
        progress,
    }
}

/// Index of the most recent stroke at or before `t` (sample-and-hold, web
/// `sampleIndexAt`); `-1`-style "none" is `None` in Rust.
#[must_use]
pub fn sample_index_at(strokes: &[Stroke], t: f64) -> Option<usize> {
    let n = strokes.len();
    if n == 0 {
        return None;
    }
    if t <= strokes[0].t {
        return Some(0);
    }
    if t >= strokes[n - 1].t {
        return Some(n - 1);
    }

    let mut lo = 0_usize;
    let mut hi = n - 1;
    while hi - lo > 1 {
        let mid = (lo + hi) >> 1;
        if strokes[mid].t <= t {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Some(lo)
}

fn frame_from(s: &Stroke, t: f64, progress: f64) -> Frame {
    Frame {
        t,
        d: s.d,
        pace: s.pace,
        spm: s.spm,
        hr: s.hr,
        watts: s.watts,
        progress,
    }
}

fn lerp(a: f64, b: f64, f: f64) -> f64 {
    a + (b - a) * f
}

/// Available playback speed presets (Studio `ReplaySpeed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaySpeed {
    /// 0.5×
    Half,
    /// 1×
    One,
    /// 1.5×
    OneAndHalf,
    /// 2×
    Two,
    /// 4×
    Four,
}

impl ReplaySpeed {
    /// The multiplier applied to each tick.
    #[must_use]
    pub fn factor(self) -> f64 {
        match self {
            ReplaySpeed::Half => 0.5,
            ReplaySpeed::One => 1.0,
            ReplaySpeed::OneAndHalf => 1.5,
            ReplaySpeed::Two => 2.0,
            ReplaySpeed::Four => 4.0,
        }
    }

    /// UI label, identical on every locale for now (Phase 4 may translate).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ReplaySpeed::Half => "0.5×",
            ReplaySpeed::One => "1×",
            ReplaySpeed::OneAndHalf => "1.5×",
            ReplaySpeed::Two => "2×",
            ReplaySpeed::Four => "4×",
        }
    }

    fn from_factor(factor: f64) -> ReplaySpeed {
        match factor {
            0.5 => ReplaySpeed::Half,
            1.5 => ReplaySpeed::OneAndHalf,
            2.0 => ReplaySpeed::Two,
            4.0 => ReplaySpeed::Four,
            _ => ReplaySpeed::One,
        }
    }
}

/// Pure playback state machine driven by external ticks (Studio `ReplayState`;
/// the web equivalent is the rAF-driven `ReplayEngine`).
///
/// The stored timeline keeps its original (possibly non-zero) timestamp
/// origin; playback time is relative to the first stroke, exactly like
/// Studio's origin-relative duration and `relativeFrame`.
#[derive(Debug, Clone)]
pub struct ReplayState {
    /// The stroke data being replayed (original timestamp origin intact).
    strokes: Vec<Stroke>,
    /// Time of the first stroke (the playback origin).
    origin_t: f64,
    /// Total workout duration in seconds, relative to the first stroke.
    duration: f64,
    /// Current playback time (seconds, relative to the origin).
    time: f64,
    /// Whether playback is active.
    playing: bool,
    /// Current playback speed preset.
    speed: ReplaySpeed,
    /// The interpolated frame at `time`.
    current_frame: Frame,
}

impl ReplayState {
    /// Build playback state over a stroke timeline.
    #[must_use]
    pub fn new(strokes: Vec<Stroke>) -> Self {
        let origin_t = strokes.first().map_or(0.0, |s| s.t);
        let last_t = strokes.last().map_or(0.0, |s| s.t);
        let duration = (last_t - origin_t).max(0.0);
        let sampled = sample_at(&strokes, origin_t);
        let current_frame = Self::relative_frame(sampled, 0.0, duration);
        ReplayState {
            strokes,
            origin_t,
            duration,
            time: 0.0,
            playing: false,
            speed: ReplaySpeed::One,
            current_frame,
        }
    }

    /// Total duration in seconds (relative to the first stroke).
    #[must_use]
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// The stroke samples driving this replay.
    #[must_use]
    pub fn strokes(&self) -> &[Stroke] {
        &self.strokes
    }

    /// Current playback time in seconds.
    #[must_use]
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Whether playback is active.
    #[must_use]
    pub fn playing(&self) -> bool {
        self.playing
    }

    /// Current speed preset.
    #[must_use]
    pub fn speed(&self) -> ReplaySpeed {
        self.speed
    }

    /// The interpolated frame at the current playback time.
    #[must_use]
    pub fn current_frame(&self) -> Frame {
        self.current_frame
    }

    /// Start playback; resets to the beginning when at the end.
    pub fn play(&mut self) {
        if self.playing || self.duration <= 0.0 {
            return;
        }
        if self.time >= self.duration {
            self.time = 0.0;
        }
        self.playing = true;
        self.emit();
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        self.playing = false;
        self.emit();
    }

    /// Toggle between play and pause.
    pub fn toggle(&mut self) {
        if self.playing {
            self.pause();
        } else {
            self.play();
        }
    }

    /// Seek to a time clamped to `[0, duration]`.
    pub fn seek(&mut self, t: f64) {
        self.time = t.clamp(0.0, self.duration);
        self.emit();
    }

    /// Set the playback speed (a raw factor is snapped to the nearest preset).
    pub fn set_speed(&mut self, factor: f64) {
        self.speed = ReplaySpeed::from_factor(factor);
    }

    /// Advance the clock by `dt` seconds of wall time while playing. Returns
    /// whether the frame changed; playback stops at the end.
    pub fn tick(&mut self, dt: f64) -> bool {
        if !self.playing || !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        self.time += dt * self.speed.factor();
        if self.time >= self.duration {
            self.time = self.duration;
            self.playing = false;
            self.emit();
            return true;
        }
        self.emit();
        true
    }

    fn emit(&mut self) {
        let sampled = sample_at(&self.strokes, self.time + self.origin_t);
        self.current_frame = Self::relative_frame(sampled, self.time, self.duration);
    }

    fn relative_frame(mut frame: Frame, playback_time: f64, duration: f64) -> Frame {
        frame.t = playback_time;
        frame.progress = if duration > 0.0 {
            (playback_time / duration).clamp(0.0, 1.0)
        } else {
            0.0
        };
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ladder_strokes() -> Vec<Stroke> {
        // Web `tests/unit/fixtures.ts` → `ladderStrokes()`.
        let mk = |t, d, pace, spm, hr, watts| Stroke {
            t,
            d,
            pace,
            spm,
            hr: Some(hr),
            watts,
            raw_t: None,
            raw_d: None,
        };
        vec![
            mk(0.0, 0.0, 120.0, 28.0, 140.0, 100.0),
            mk(10.0, 50.0, 110.0, 30.0, 150.0, 120.0),
            mk(20.0, 100.0, 100.0, 32.0, 160.0, 140.0),
        ]
    }

    #[test]
    fn sample_at_returns_zeros_for_empty_strokes() {
        let f = sample_at(&[], 5.0);
        assert_eq!(f.d, 0.0);
        assert_eq!(f.pace, 0.0);
        assert_eq!(f.progress, 0.0);
    }

    #[test]
    fn sample_at_clamps_before_first_sample() {
        let strokes = ladder_strokes();
        let f = sample_at(&strokes, -1.0);
        assert_eq!(f.pace, strokes[0].pace);
        assert_eq!(f.d, strokes[0].d);
        assert_eq!(f.progress, 0.0);
    }

    #[test]
    fn sample_at_clamps_after_last_sample() {
        let strokes = ladder_strokes();
        let f = sample_at(&strokes, 100.0);
        let last = strokes[strokes.len() - 1];
        assert_eq!(f.pace, last.pace);
        assert_eq!(f.d, last.d);
        assert_eq!(f.progress, 1.0);
    }

    #[test]
    fn sample_at_returns_exact_values_on_stroke_timestamps() {
        let strokes = ladder_strokes();
        let mid = strokes[1];
        let f = sample_at(&strokes, mid.t);
        assert_eq!(f.pace, mid.pace);
        assert_eq!(f.d, mid.d);
        assert_eq!(f.spm, mid.spm);
        assert_eq!(f.hr, mid.hr);
    }

    #[test]
    fn sample_at_interpolates_mid_stroke() {
        let strokes = ladder_strokes();
        let f = sample_at(&strokes, 15.0);
        assert_eq!(f.t, 15.0);
        assert!(f.pace > 100.0 && f.pace < 120.0);
        assert!(f.d > 50.0 && f.d < 100.0);
        assert!((f.progress - 0.75).abs() < 1e-5);
    }

    #[test]
    fn sample_at_interpolates_heart_rate_when_both_ends_have_hr() {
        let strokes = ladder_strokes();
        let f = sample_at(&strokes, 15.0);
        let hr = f.hr.expect("hr");
        assert!(hr > 150.0 && hr < 160.0);
    }

    #[test]
    fn sample_at_carries_a_single_sided_hr() {
        let mk = |t, hr| Stroke {
            t,
            d: 0.0,
            pace: 120.0,
            spm: 28.0,
            hr,
            watts: 0.0,
            raw_t: None,
            raw_d: None,
        };
        let strokes = vec![mk(0.0, Some(140.0)), mk(10.0, None)];
        assert_eq!(sample_at(&strokes, 5.0).hr, Some(140.0));
        let strokes = vec![mk(0.0, None), mk(10.0, Some(150.0))];
        assert_eq!(sample_at(&strokes, 5.0).hr, Some(150.0));
    }

    #[test]
    fn sample_index_at_mirrors_the_bracketing_search() {
        let strokes = ladder_strokes();
        assert_eq!(sample_index_at(&[], 5.0), None);
        assert_eq!(sample_index_at(&strokes, -1.0), Some(0));
        assert_eq!(sample_index_at(&strokes, 100.0), Some(strokes.len() - 1));
        assert_eq!(sample_index_at(&strokes, strokes[1].t), Some(1));
        assert_eq!(sample_index_at(&strokes, 15.0), Some(1));
    }

    #[test]
    fn player_and_ghost_share_the_engine_clock() {
        let player = vec![
            Stroke {
                t: 0.0,
                d: 0.0,
                pace: 120.0,
                spm: 28.0,
                hr: None,
                watts: 200.0,
                raw_t: None,
                raw_d: None,
            },
            Stroke {
                t: 60.0,
                d: 250.0,
                pace: 118.0,
                spm: 29.0,
                hr: None,
                watts: 210.0,
                raw_t: None,
                raw_d: None,
            },
            Stroke {
                t: 120.0,
                d: 500.0,
                pace: 116.0,
                spm: 30.0,
                hr: None,
                watts: 220.0,
                raw_t: None,
                raw_d: None,
            },
        ];
        let ghost = vec![
            Stroke {
                t: 0.0,
                d: 0.0,
                pace: 125.0,
                spm: 26.0,
                hr: None,
                watts: 180.0,
                raw_t: None,
                raw_d: None,
            },
            Stroke {
                t: 60.0,
                d: 230.0,
                pace: 124.0,
                spm: 27.0,
                hr: None,
                watts: 185.0,
                raw_t: None,
                raw_d: None,
            },
            Stroke {
                t: 120.0,
                d: 480.0,
                pace: 122.0,
                spm: 28.0,
                hr: None,
                watts: 190.0,
                raw_t: None,
                raw_d: None,
            },
        ];
        for t in [0.0, 30.0, 60.0, 90.0, 120.0, 150.0] {
            let pf = sample_at(&player, t);
            let gf = sample_at(&ghost, t);
            assert_eq!(pf.t, gf.t);
            assert_eq!(pf.t, t);
        }
    }

    #[test]
    fn replay_state_plays_seeks_and_stops_at_the_end() {
        let mut state = ReplayState::new(ladder_strokes());
        assert_eq!(state.duration(), 20.0);
        assert!(!state.playing());
        state.play();
        assert!(state.playing());
        assert!(state.tick(1.0));
        assert!((state.time() - 1.0).abs() < 1e-9);
        state.set_speed(2.0);
        state.tick(2.0);
        assert!((state.time() - 5.0).abs() < 1e-9);
        state.seek(-5.0);
        assert_eq!(state.time(), 0.0);
        state.seek(1e9);
        assert_eq!(state.time(), 20.0);
        state.pause();
        assert!(!state.playing());
        state.tick(1.0); // paused: no advance
        assert_eq!(state.time(), 20.0);
        state.play(); // at the end: restarts
        assert_eq!(state.time(), 0.0);
        state.set_speed(4.0);
        for _ in 0..10 {
            state.tick(1.0);
        }
        assert!(!state.playing());
        assert_eq!(state.time(), 20.0);
    }

    #[test]
    fn replay_state_handles_non_zero_origins() {
        let strokes: Vec<Stroke> = ladder_strokes()
            .into_iter()
            .map(|mut s| {
                s.t += 1000.0;
                s
            })
            .collect();
        let mut state = ReplayState::new(strokes);
        assert_eq!(state.duration(), 20.0);
        state.play();
        state.tick(10.0);
        let frame = state.current_frame();
        assert_eq!(frame.t, 10.0);
        assert!((frame.d - 50.0).abs() < 1e-9);
        assert!((frame.progress - 0.5).abs() < 1e-9);
    }

    #[test]
    fn replay_state_empty_timeline_never_plays() {
        let mut state = ReplayState::new(Vec::new());
        assert_eq!(state.duration(), 0.0);
        state.play();
        assert!(!state.playing());
        assert!(!state.tick(1.0));
    }

    #[test]
    fn replay_speed_presets_round_trip() {
        assert_eq!(ReplaySpeed::from_factor(2.0), ReplaySpeed::Two);
        assert_eq!(ReplaySpeed::from_factor(3.0), ReplaySpeed::One);
        assert_eq!(ReplaySpeed::Four.label(), "4×");
    }
}
