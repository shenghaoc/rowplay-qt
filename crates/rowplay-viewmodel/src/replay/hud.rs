// SPDX-License-Identifier: GPL-3.0-or-later
//! HUD strings for the replay route (Phase 5b spec R5): the web's transport
//! clock, distance and gauges, formatted in Rust so QML shows text only.
//!
//! The web page (`src/routes/replay/[id]/+page.svelte`) shows
//! `fmtTime(frame.t, true) / fmtTime(detail.time)`, `fmtDistance(frame.d)`
//! and gauges labelled `replay.gPace` (`fmtPace` without its `/500m`),
//! `replay.gRate`, `replay.gPower` and `replay.gHeart` (when the workout has
//! heart rate), and beside a ghost the race gap (`replay.ahead` /
//! `replay.behind`). Locale ids stay the web's; only the values are built
//! here.

use rowplay_core::formatting::{fmt_distance_in, fmt_pace_bare, fmt_time};
use rowplay_core::models::DistanceUnit;
use rowplay_core::num::{js_round, js_to_fixed};
use rowplay_core::replay::engine::Frame;
use rowplay_core::replay::race_gap::{race_gap_metres, race_gap_seconds};

/// The pre-formatted HUD values for one frame.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HudStrings {
    /// Elapsed playback time with tenths (web `fmtTime(frame.t, true)`).
    pub clock: String,
    /// The workout's total time (web `fmtTime(detail.time)`).
    pub total: String,
    /// Distance covered in the preferred unit (web `fmtDistance(frame.d)`).
    pub distance: String,
    /// Pace without the `/500m` suffix (web gauge display).
    pub pace: String,
    /// Stroke rate, whole strokes (or rpm) per minute.
    pub rate: String,
    /// Power, whole watts.
    pub watts: String,
    /// Heart rate, whole bpm; empty when the frame carries none.
    pub heart: String,
}

/// Build the HUD strings for `frame` of a workout lasting `total_seconds`.
#[must_use]
pub fn hud_strings(frame: &Frame, total_seconds: f64, unit: DistanceUnit) -> HudStrings {
    HudStrings {
        clock: fmt_time(frame.t, true),
        total: fmt_time(total_seconds, false),
        distance: fmt_distance_in(frame.d, unit),
        pace: fmt_pace_bare(frame.pace, false),
        rate: whole(frame.spm),
        watts: whole(frame.watts),
        heart: frame.hr.map(whole).unwrap_or_default(),
    }
}

/// The five HUD strings joined for the bridge, in a fixed order, so the
/// per-frame crossing stays one property (spec R5.2): `clock|total|distance|
/// pace|rate|watts|heart`. QML splits on `|` for placement only.
#[must_use]
pub fn hud_bundle(strings: &HudStrings) -> String {
    [
        strings.clock.as_str(),
        strings.total.as_str(),
        strings.distance.as_str(),
        strings.pace.as_str(),
        strings.rate.as_str(),
        strings.watts.as_str(),
        strings.heart.as_str(),
    ]
    .join("|")
}

/// The numeric HUD block of the frame bundle (`frame::HUD_DISTANCE` ..
/// `frame::HUD_PROGRESS`), in the layout's order and units: distance (m),
/// pace (s/500 m), rate, elapsed (s), speed (m/s from the pace), ghost gap
/// (NaN: no ghost in Phase 5b), finish ETA (s) and progress (0..1).
#[must_use]
pub fn hud_numbers(frame: &Frame, total_seconds: f64) -> [f32; 8] {
    let speed = if frame.pace.is_finite() && frame.pace > 0.0 {
        500.0 / frame.pace
    } else {
        0.0
    };
    let remaining = if total_seconds.is_finite() {
        (total_seconds - frame.t).max(0.0)
    } else {
        0.0
    };
    [
        frame.d as f32,
        frame.pace as f32,
        frame.spm as f32,
        frame.t as f32,
        speed as f32,
        f32::NAN,
        remaining as f32,
        frame.progress as f32,
    ]
}

/// The race gap between the player and the ghost, joined for the bridge as
/// `ahead|<metres>|<seconds>` or `behind|<metres>|<seconds>`. QML puts the
/// metres into the web's `replay.ahead` / `replay.behind` ("▲ ahead by
/// {m}m") and the seconds after them in brackets, so no word is built here.
///
/// The web page takes `gapMeters = raceGapMetres(frame.d, ghostFrame.d)`
/// and `gapSeconds = raceGapSeconds(gapMeters, frame.pace)`, counts a level
/// race as ahead (`gapMeters >= 0`), and shows
/// `Math.abs(Math.round(gapMeters))` and
/// `({Math.abs(gapSeconds).toFixed(1)}s)`. Both numbers are rounded here as
/// JavaScript rounds them, and the seconds carry their `s`.
#[must_use]
pub fn race_gap_bundle(player: &Frame, ghost: &Frame) -> String {
    let metres = race_gap_metres(player.d, ghost.d);
    let seconds = race_gap_seconds(metres, player.pace);
    let side = if metres >= 0.0 { "ahead" } else { "behind" };
    format!(
        "{side}|{}|{}s",
        js_round(metres).abs(),
        js_to_fixed(seconds.abs(), 1)
    )
}

/// A non-negative whole number, `0` for anything non-finite or negative.
fn whole(value: f64) -> String {
    if value.is_finite() && value > 0.0 {
        format!("{}", value.round())
    } else {
        "0".to_owned()
    }
}

/// How close to the end of the replay the race counts as run (the web's
/// `replayDuration - 0.05`).
pub const RACE_FINISH_TOLERANCE_SECONDS: f64 = 0.05;

/// Whether the player's replay has reached its finish line, where the race
/// verdict shows. The web page's `raceFinished` is
/// `ghostActive && replayDuration > 0 && frame.t >= replayDuration - 0.05`;
/// the caller holds the ghost, and both times here are the player's,
/// relative to its first stroke (the web's are absolute, which moves both
/// sides of the comparison by the same origin).
#[must_use]
pub fn race_finished(elapsed_seconds: f64, duration_seconds: f64) -> bool {
    duration_seconds > 0.0 && elapsed_seconds >= duration_seconds - RACE_FINISH_TOLERANCE_SECONDS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            t: 72.34,
            d: 1234.5,
            pace: 118.4,
            spm: 27.6,
            hr: Some(151.2),
            watts: 212.49,
            progress: 0.3,
        }
    }

    #[test]
    fn strings_come_from_the_core_formatters() {
        let hud = hud_strings(&frame(), 480.0, DistanceUnit::Metric);
        assert_eq!(hud.clock, fmt_time(72.34, true));
        assert_eq!(hud.total, fmt_time(480.0, false));
        assert_eq!(hud.distance, fmt_distance_in(1234.5, DistanceUnit::Metric));
        assert_eq!(hud.pace, fmt_pace_bare(118.4, false));
        assert!(!hud.pace.contains("/500m"));
        assert_eq!(hud.rate, "28");
        assert_eq!(hud.watts, "212");
        assert_eq!(hud.heart, "151");
        assert!(hud.clock.contains('.'), "tenths: {}", hud.clock);
    }

    #[test]
    fn missing_heart_rate_and_bad_values_render_empty_or_zero() {
        let mut f = frame();
        f.hr = None;
        f.spm = f64::NAN;
        f.watts = -5.0;
        let hud = hud_strings(&f, 0.0, DistanceUnit::Imperial);
        assert_eq!(hud.heart, "");
        assert_eq!(hud.rate, "0");
        assert_eq!(hud.watts, "0");
        assert_eq!(
            hud.distance,
            fmt_distance_in(1234.5, DistanceUnit::Imperial)
        );
    }

    #[test]
    fn the_numeric_block_follows_the_layout_order() {
        let numbers = hud_numbers(&frame(), 1200.0);
        assert_eq!(numbers.len(), super::super::frame::HUD_PROGRESS);
        assert!((numbers[4] - 500.0 / frame().pace as f32).abs() < 1e-4);
        assert!(numbers[5].is_nan(), "no ghost in Phase 5b");
        assert!((numbers[6] - (1200.0 - frame().t) as f32).abs() < 1e-3);
    }

    /// The web page's gap line, evaluated under Node at the pinned commit
    /// with its own `raceGapMetres` / `raceGapSeconds` and the page's
    /// expressions: `gapMeters >= 0` picks `replay.ahead`, then
    /// `Math.abs(Math.round(gapMeters))` and
    /// `Math.abs(gapSeconds).toFixed(1)`. Exact strings, no tolerance.
    #[test]
    fn the_race_gap_follows_the_webs_side_and_rounding() {
        let at = |d: f64, pace: f64| Frame { d, pace, ..frame() };
        for (player, ghost, expected) in [
            (at(520.4, 120.0), at(500.0, 120.0), "ahead|20|4.9s"),
            (at(485.0, 120.0), at(500.0, 120.0), "behind|15|3.6s"),
            // A level race counts as ahead, the start line included.
            (at(0.0, 120.0), at(0.0, 120.0), "ahead|0|0.0s"),
            // Under half a metre the gap keeps its side.
            (at(499.7, 120.0), at(500.0, 120.0), "behind|0|0.1s"),
            // `Math.round` takes a half up: -2.5 m is 2 m behind.
            (at(502.5, 120.0), at(500.0, 120.0), "ahead|3|0.6s"),
            (at(497.5, 120.0), at(500.0, 120.0), "behind|2|0.6s"),
            (at(100.0, 110.0), at(1350.5, 110.0), "behind|1250|275.1s"),
            // `toFixed` takes an exact tie away from zero: 0.25 s is 0.3.
            (at(501.25, 100.0), at(500.0, 100.0), "ahead|1|0.3s"),
            // The seconds come from the player's pace; none, no seconds.
            (at(510.0, 0.0), at(500.0, 120.0), "ahead|10|0.0s"),
            (at(2000.0, 105.2), at(1987.95, 105.2), "ahead|12|2.5s"),
        ] {
            assert_eq!(
                race_gap_bundle(&player, &ghost),
                expected,
                "player at {} m, ghost at {} m",
                player.d,
                ghost.d
            );
        }
    }

    #[test]
    fn the_bundle_keeps_a_fixed_field_order() {
        let hud = hud_strings(&frame(), 480.0, DistanceUnit::Metric);
        let bundle = hud_bundle(&hud);
        let fields: Vec<&str> = bundle.split('|').collect();
        assert_eq!(fields.len(), 7);
        assert_eq!(fields[0], hud.clock);
        assert_eq!(fields[2], hud.distance);
        assert_eq!(fields[6], hud.heart);
        for field in &fields {
            assert!(!field.contains('|'));
        }
    }

    /// Re-expressed from the web page's `raceFinished`
    /// (`replayDuration > 0 && frame.t >= replayDuration - 0.05`): exact
    /// bounds, so the tolerance is the web's 0.05 s with no float margin.
    #[test]
    fn the_race_is_finished_within_the_webs_tolerance_of_the_end() {
        assert!(!race_finished(0.0, 420.0), "the start line");
        assert!(!race_finished(210.0, 420.0), "halfway");
        assert!(!race_finished(419.9, 420.0), "a tenth short");
        assert!(race_finished(419.95, 420.0), "within 0.05 s");
        assert!(race_finished(420.0, 420.0), "the end");
        assert!(!race_finished(0.0, 0.0), "no duration, no race");
        assert!(!race_finished(f64::NAN, 420.0), "no clock, no finish");
    }
}
