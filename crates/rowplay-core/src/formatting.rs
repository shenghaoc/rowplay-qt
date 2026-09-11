// SPDX-License-Identifier: GPL-3.0-or-later
//! Time, pace, distance and power formatting.
//!
//! Port of the web app's `src/lib/format.ts` (canonical) plus the imperial
//! distance helpers from rowplay-studio's `RowPlayFormatting.swift`, which have
//! no web equivalent. String output mirrors the web app character for
//! character, including `toFixed` rounding.

use crate::models::{DistanceUnit, Sport, Workout};
use crate::num::{js_round, js_to_fixed, pad_int};

/// Placeholder for an unformattable time or pace.
pub const TIME_PLACEHOLDER: &str = "--:--";
/// Placeholder for an unformattable distance.
pub const DISTANCE_PLACEHOLDER: &str = "--";

/// BikeErg stroke pace in the API is per 1000 m; we store sec/500 m for display.
/// The PM still applies the cubic formula on the 1000 m split, so normalised
/// sec/500 m through the rower formula overstates power by 2³ = 8.
pub const BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR: f64 = 8.0;

/// 1000 feet in metres: Studio switches from feet to miles at this distance.
const FEET_THRESHOLD_METRES: f64 = 304.8;
const FEET_PER_METRE: f64 = 3.28084;
const METRES_PER_MILE: f64 = 1_609.344;

/// Seconds → `M:SS` (`M:SS.t` with `tenths`) or `H:MM:SS` above an hour (web `fmtTime`).
///
/// Negative, NaN or infinite input renders the placeholder. Hours that would
/// not print as a plain integer (beyond 1e15) also render the placeholder,
/// where the web app would print exponent notation.
#[must_use]
pub fn fmt_time(seconds: f64, tenths: bool) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return TIME_PLACEHOLDER.to_owned();
    }
    let hours = (seconds / 3600.0).floor();
    if hours >= 1e15 {
        return TIME_PLACEHOLDER.to_owned();
    }
    let minutes = ((seconds % 3600.0) / 60.0).floor();
    let secs = seconds % 60.0;
    if hours > 0.0 {
        return format!(
            "{}:{}:{}",
            hours as i64,
            pad_int(minutes, 2),
            pad_int(secs, 2)
        );
    }
    let secs_str = if tenths {
        let fixed = js_to_fixed(secs, 1);
        if fixed.len() < 4 {
            format!("{}{}", "0".repeat(4 - fixed.len()), fixed)
        } else {
            fixed
        }
    } else {
        pad_int(secs, 2)
    };
    format!("{}:{}", minutes as i64, secs_str)
}

/// Pace (sec/500 m) → `M:SS.t/500m` (web `fmtPace`).
#[must_use]
pub fn fmt_pace(pace: f64) -> String {
    if !pace.is_finite() || pace <= 0.0 {
        return TIME_PLACEHOLDER.to_owned();
    }
    format!("{}/500m", fmt_time(pace, true))
}

/// Pace without the `/500m` suffix, for gauges, charts and split labels (web `fmtPaceBare`).
///
/// Zero is invalid unless `allow_zero` is set, which is useful for pace deltas.
#[must_use]
pub fn fmt_pace_bare(pace: f64, allow_zero: bool) -> String {
    let invalid = if allow_zero { pace < 0.0 } else { pace <= 0.0 };
    if !pace.is_finite() || invalid {
        return TIME_PLACEHOLDER.to_owned();
    }
    fmt_time(pace, true)
}

/// Metres → `N m` below 1 km, `X.XX km` from 1 km (web `fmtDistance`).
///
/// Non-finite input renders `--` (the web app would print `NaN m`).
#[must_use]
pub fn fmt_distance(metres: f64) -> String {
    if !metres.is_finite() {
        return DISTANCE_PLACEHOLDER.to_owned();
    }
    if metres >= 1000.0 {
        return format!("{} km", js_to_fixed(metres / 1000.0, 2));
    }
    let rounded = js_round(metres);
    if rounded.abs() >= 1e15 {
        return DISTANCE_PLACEHOLDER.to_owned();
    }
    format!("{} m", rounded as i64)
}

/// Distance in the athlete's preferred unit (Studio `distance(_:unit:)`).
///
/// Metric output is [`fmt_distance`]. Imperial output switches from whole feet
/// to `X.XX mi` at 1000 ft, using the absolute value for the threshold so
/// negative margins keep their unit.
#[must_use]
pub fn fmt_distance_in(metres: f64, unit: DistanceUnit) -> String {
    match unit {
        DistanceUnit::Metric => fmt_distance(metres),
        DistanceUnit::Imperial => {
            if !metres.is_finite() {
                return DISTANCE_PLACEHOLDER.to_owned();
            }
            if metres.abs() >= FEET_THRESHOLD_METRES {
                return format!("{} mi", js_to_fixed(metres / METRES_PER_MILE, 2));
            }
            let feet = (metres * FEET_PER_METRE).round();
            if feet.abs() >= 1e15 {
                return DISTANCE_PLACEHOLDER.to_owned();
            }
            format!("{} ft", feet as i64)
        }
    }
}

/// Distance formatting for race margins (Studio `distanceMargin`).
///
/// Unlike general workout distance, positive sub-unit margins keep one decimal
/// so a non-tie never renders as a misleading zero.
#[must_use]
pub fn fmt_distance_margin(metres: f64, unit: DistanceUnit) -> String {
    if !metres.is_finite() || metres < 0.0 {
        return DISTANCE_PLACEHOLDER.to_owned();
    }
    match unit {
        DistanceUnit::Metric if metres > 0.0 && metres < 1.0 => {
            format!("{} m", js_to_fixed(metres.max(0.1), 1))
        }
        DistanceUnit::Imperial => {
            let feet = metres * FEET_PER_METRE;
            if metres > 0.0 && feet < 1.0 {
                return format!("{} ft", js_to_fixed(feet.max(0.1), 1));
            }
            fmt_distance_in(metres, unit)
        }
        DistanceUnit::Metric => fmt_distance_in(metres, unit),
    }
}

/// Concept2 RowErg/SkiErg power model: watts = 2.8 / pace³ with pace in sec/m
/// (pace-per-metre = pace/500). Invalid pace yields 0 (web `paceToWatts`).
#[must_use]
pub fn pace_to_watts(pace_per_500: f64) -> f64 {
    if !pace_per_500.is_finite() || pace_per_500 <= 0.0 {
        return 0.0;
    }
    let per_metre = pace_per_500 / 500.0;
    2.8 / per_metre.powf(3.0)
}

/// Watts from stored sec/500 m pace, accounting for sport-specific PM math (web `paceToWattsForSport`).
#[must_use]
pub fn pace_to_watts_for_sport(sport: Sport, pace_per_500: f64) -> f64 {
    let watts = pace_to_watts(pace_per_500);
    if sport == Sport::Bike {
        watts / BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR
    } else {
        watts
    }
}

/// Inverse of [`pace_to_watts`]: watts → sec/500 m on the RowErg/SkiErg PM basis (web `wattsToPace`).
#[must_use]
pub fn watts_to_pace(watts: f64) -> f64 {
    if !watts.is_finite() || watts <= 0.0 {
        return 0.0;
    }
    let per_metre = (2.8 / watts).powf(1.0 / 3.0);
    per_metre * 500.0
}

/// Inverse of [`pace_to_watts_for_sport`]: normalised sec/500 m from watts (web `wattsToPaceForSport`).
#[must_use]
pub fn watts_to_pace_for_sport(sport: Sport, watts: f64) -> f64 {
    if !watts.is_finite() || watts <= 0.0 {
        return 0.0;
    }
    if sport == Sport::Bike {
        watts_to_pace(watts * BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR)
    } else {
        watts_to_pace(watts)
    }
}

/// Metres credited toward Concept2 logbook challenges: the BikeErg counts at half (web `challengeDistanceMetres`).
#[must_use]
pub fn challenge_distance_metres(sport: Sport, distance: f64) -> f64 {
    if sport == Sport::Bike {
        distance / 2.0
    } else {
        distance
    }
}

/// Average watts from cached watt-minutes when present, else the Concept2 pace model (web `avgWatts`).
#[must_use]
pub fn avg_watts(workout: &Workout) -> f64 {
    match workout.watt_minutes {
        Some(watt_minutes) if watt_minutes != 0.0 && workout.time > 0.0 => {
            js_round(watt_minutes / (workout.time / 60.0))
        }
        _ => js_round(pace_to_watts_for_sport(workout.sport, workout.pace)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workout(id: i64) -> Workout {
        Workout::new(
            id,
            "2026-05-01 06:00:00",
            Sport::Rower,
            2000.0,
            480.0,
            120.0,
        )
    }

    // --- web format.test.ts -------------------------------------------------

    #[test]
    fn fmt_time_web_cases() {
        assert_eq!(fmt_time(125.3, true), "2:05.3");
        assert_eq!(fmt_time(3661.0, false), "1:01:01");
        assert_eq!(fmt_time(f64::NAN, false), "--:--");
        assert_eq!(fmt_time(-1.0, false), "--:--");
    }

    #[test]
    fn fmt_pace_web_cases() {
        assert_eq!(fmt_pace(120.0), "2:00.0/500m");
        assert_eq!(fmt_pace_bare(0.0, true), "0:00.0");
        assert_eq!(fmt_pace_bare(0.0, false), "--:--");
        assert_eq!(fmt_pace(0.0), "--:--");
    }

    #[test]
    fn fmt_distance_web_cases() {
        assert_eq!(fmt_distance(500.0), "500 m");
        assert_eq!(fmt_distance(2500.0), "2.50 km");
    }

    #[test]
    fn pace_to_watts_web_cases() {
        assert!((pace_to_watts(120.0) - 202.55).abs() < 0.05);
        assert_eq!(pace_to_watts(0.0), 0.0);
        let mut w = workout(1);
        w.watt_minutes = Some(600.0);
        w.time = 600.0;
        w.pace = 130.0;
        assert_eq!(avg_watts(&w), 60.0);
        let w2 = workout(2);
        assert_eq!(avg_watts(&w2), js_round(pace_to_watts(120.0)));
    }

    #[test]
    fn bike_pace_normalisation() {
        // API reports 190.0 s/1000m → 95.0 s/500m (same speed as rower 1:35/500m).
        let normalized = 1900.0 / 10.0 / 2.0;
        assert_eq!(normalized, 95.0);
        assert!(
            (pace_to_watts_for_sport(Sport::Bike, normalized) - pace_to_watts(95.0) / 8.0).abs()
                < 0.05
        );
    }

    #[test]
    fn watts_to_pace_round_trips() {
        for sport in [Sport::Rower, Sport::Skierg] {
            let watts = pace_to_watts_for_sport(sport, 120.0);
            assert!((watts_to_pace_for_sport(sport, watts) - 120.0).abs() < 1e-5);
        }
        assert!((watts_to_pace_for_sport(Sport::Bike, 51.03) - 95.0).abs() < 0.5);
        let watts = pace_to_watts_for_sport(Sport::Bike, 95.0);
        assert!((watts_to_pace_for_sport(Sport::Bike, watts) - 95.0).abs() < 1e-5);
        assert_eq!(watts_to_pace(0.0), 0.0);
        assert_eq!(watts_to_pace_for_sport(Sport::Bike, -3.0), 0.0);
    }

    #[test]
    fn challenge_distance_counts_bike_at_half() {
        assert_eq!(challenge_distance_metres(Sport::Bike, 8000.0), 4000.0);
        assert_eq!(challenge_distance_metres(Sport::Rower, 8000.0), 8000.0);
        assert_eq!(challenge_distance_metres(Sport::Skierg, 5000.0), 5000.0);
    }

    // --- Studio RowPlayFormattingTests --------------------------------------

    #[test]
    fn fmt_time_boundaries() {
        assert_eq!(fmt_time(0.0, false), "0:00");
        assert_eq!(fmt_time(45.0, false), "0:45");
        assert_eq!(fmt_time(125.0, false), "2:05");
        assert_eq!(fmt_time(65.3, true), "1:05.3");
        assert_eq!(fmt_time(0.0, true), "0:00.0");
        assert_eq!(fmt_time(f64::INFINITY, false), "--:--");
        assert_eq!(fmt_time(f64::MAX, false), "--:--");
        assert_eq!(fmt_time(1.0, false), "0:01");
        assert_eq!(fmt_time(59.0, false), "0:59");
        assert_eq!(fmt_time(60.0, false), "1:00");
        assert_eq!(fmt_time(61.0, false), "1:01");
        assert_eq!(fmt_time(3599.0, false), "59:59");
        assert_eq!(fmt_time(3600.0, false), "1:00:00");
        assert_eq!(fmt_time(125.0, true), "2:05.0");
        // Web quirk kept on purpose: tenths can round up to "60.0".
        assert_eq!(fmt_time(119.96, true), "1:60.0");
    }

    #[test]
    fn fmt_pace_boundaries() {
        assert_eq!(fmt_pace(300.0), "5:00.0/500m");
        assert_eq!(fmt_pace(-1.0), "--:--");
        assert_eq!(fmt_pace(f64::INFINITY), "--:--");
        assert_eq!(fmt_pace(f64::NAN), "--:--");
    }

    #[test]
    fn fmt_distance_boundaries() {
        assert_eq!(fmt_distance(5000.0), "5.00 km");
        assert_eq!(fmt_distance(1000.0), "1.00 km");
        assert_eq!(fmt_distance(0.0), "0 m");
        assert_eq!(fmt_distance(f64::INFINITY), "--");
        assert_eq!(fmt_distance(f64::NAN), "--");
        assert_eq!(fmt_distance(1.0), "1 m");
        assert_eq!(fmt_distance(999.0), "999 m");
        assert_eq!(fmt_distance(1234.0), "1.23 km");
        assert_eq!(fmt_distance(1_000_000.0), "1000.00 km");
        assert_eq!(fmt_distance(-1.0), "-1 m");
        // The web app compares the signed value against 1 km, so negative
        // distances stay in metres (Studio printed "-1.50 km").
        assert_eq!(fmt_distance(-1500.0), "-1500 m");
        assert_eq!(
            fmt_distance_in(5000.0, DistanceUnit::Metric),
            fmt_distance(5000.0)
        );
    }

    #[test]
    fn fmt_distance_imperial() {
        assert_eq!(fmt_distance_in(100.0, DistanceUnit::Imperial), "328 ft");
        assert_eq!(
            fmt_distance_in(1_609.344, DistanceUnit::Imperial),
            "1.00 mi"
        );
        assert_eq!(fmt_distance_in(-500.0, DistanceUnit::Imperial), "-0.31 mi");
        assert_eq!(fmt_distance_in(304.8, DistanceUnit::Imperial), "0.19 mi");
        assert_eq!(fmt_distance_in(304.0, DistanceUnit::Imperial), "997 ft");
        let five_k = fmt_distance_in(5000.0, DistanceUnit::Imperial);
        assert!(five_k.ends_with("mi") && five_k.starts_with("3."));
        assert_eq!(fmt_distance_in(0.0, DistanceUnit::Imperial), "0 ft");
        assert_eq!(fmt_distance_in(f64::INFINITY, DistanceUnit::Imperial), "--");
        assert_eq!(fmt_distance_in(f64::NAN, DistanceUnit::Imperial), "--");
        assert_eq!(fmt_distance_in(1.0, DistanceUnit::Imperial), "3 ft");
        assert_eq!(
            fmt_distance_in(1_000_000.0, DistanceUnit::Imperial),
            "621.37 mi"
        );
        let metric = fmt_distance_in(5000.0, DistanceUnit::Metric);
        assert_ne!(metric, five_k);
        assert!(metric.ends_with("km"));
    }

    #[test]
    fn fmt_distance_margin_keeps_sub_unit_values() {
        assert_eq!(fmt_distance_margin(0.3, DistanceUnit::Metric), "0.3 m");
        assert_eq!(fmt_distance_margin(0.01, DistanceUnit::Metric), "0.1 m");
        assert_eq!(fmt_distance_margin(0.1, DistanceUnit::Imperial), "0.3 ft");
        assert_eq!(fmt_distance_margin(0.001, DistanceUnit::Imperial), "0.1 ft");
        assert_eq!(fmt_distance_margin(12.0, DistanceUnit::Metric), "12 m");
        assert_eq!(fmt_distance_margin(-1.0, DistanceUnit::Metric), "--");
        assert_eq!(fmt_distance_margin(f64::NAN, DistanceUnit::Imperial), "--");
    }

    #[test]
    fn pace_to_watts_edge_cases() {
        assert!(pace_to_watts(90.0) > pace_to_watts(120.0));
        assert_eq!(pace_to_watts(-1.0), 0.0);
        assert_eq!(pace_to_watts(f64::INFINITY), 0.0);
        assert_eq!(pace_to_watts(f64::NAN), 0.0);
        assert!((pace_to_watts(600.0) - 1.62).abs() < 0.01);
        assert_eq!(
            pace_to_watts_for_sport(Sport::Rower, 120.0),
            pace_to_watts(120.0)
        );
        assert_eq!(
            pace_to_watts_for_sport(Sport::Skierg, 120.0),
            pace_to_watts(120.0)
        );
        assert_eq!(
            pace_to_watts_for_sport(Sport::Bike, 120.0),
            pace_to_watts(120.0) / BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR
        );
        for sport in Sport::ALL {
            assert_eq!(pace_to_watts_for_sport(sport, -1.0), 0.0);
        }
        assert_eq!(BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR, 8.0);
    }

    #[test]
    fn avg_watts_falls_back_when_watt_minutes_are_zero() {
        let mut w = workout(1);
        w.watt_minutes = Some(0.0);
        assert_eq!(avg_watts(&w), js_round(pace_to_watts(120.0)));
        w.watt_minutes = Some(100.0);
        w.time = 0.0;
        assert_eq!(avg_watts(&w), js_round(pace_to_watts(120.0)));
    }
}
