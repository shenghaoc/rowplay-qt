// SPDX-License-Identifier: GPL-3.0-or-later
//! HUD strings for the replay route (Phase 5b spec R5): the web's transport
//! clock, distance and gauges, formatted in Rust so QML shows text only.
//!
//! The web page (`src/routes/replay/[id]/+page.svelte`) shows
//! `fmtTime(frame.t, true) / fmtTime(detail.time)`, `fmtDistance(frame.d)`
//! and gauges labelled `replay.gPace` (`fmtPace` without its `/500m`),
//! `replay.gRate`, `replay.gPower` and `replay.gHeart` (when the workout has
//! heart rate). Locale ids stay the web's; only the values are built here.

use rowplay_core::formatting::{fmt_distance_in, fmt_pace_bare, fmt_time};
use rowplay_core::models::DistanceUnit;
use rowplay_core::replay::engine::Frame;

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

/// A non-negative whole number, `0` for anything non-finite or negative.
fn whole(value: f64) -> String {
    if value.is_finite() && value > 0.0 {
        format!("{}", value.round())
    } else {
        "0".to_owned()
    }
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
}
