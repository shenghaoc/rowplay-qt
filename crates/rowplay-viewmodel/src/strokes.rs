// SPDX-License-Identifier: GPL-3.0-or-later
//! Workout-detail view model: header, metric strip, splits/intervals table
//! and the targets read-out.
//!
//! Sources: Studio's `WorkoutDetailView` (strip order, `powerText` static,
//! table shape) with web-canonical values; the table header and section ids
//! are the web's (`replay.th*`, `replay.splitBreakdown`). Unit suffixes
//! (W, Cal, bpm, spm/rpm) are untranslated symbols, like the web's axes.

use rowplay_core::formatting::{fmt_distance_in, fmt_pace, fmt_time, pace_to_watts_for_sport};
use rowplay_core::models::{DistanceUnit, Split, Sport, WorkoutDetail};

use crate::dates::{fmt_date, fmt_time_of_day};
use crate::role::ColorRole;
use crate::settings::Language;

/// The detail header block.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailHeader {
    /// Logbook workout type, else the sport name.
    pub title: String,
    /// Untranslated sport display name.
    pub sport_name: &'static str,
    /// Locale short date (web `fmtDate`).
    pub date_text: String,
    /// Monitor-local `HH:MM`.
    pub time_text: String,
    /// Concept2 `source` when present.
    pub source_text: String,
    /// Interval piece (splits table title changes).
    pub is_interval: bool,
    /// Athlete comments ("" when absent).
    pub comments: String,
    /// Screen-reader line: date, time, source, intervals (Studio's header
    /// accessibility label).
    pub accessible_text: String,
}

/// One metric-strip entry (Studio's `performanceMetric`).
#[derive(Debug, Clone, PartialEq)]
pub struct StripMetric {
    /// Locale message id for the (upper-cased) label.
    pub label_id: &'static str,
    /// Rendered value including its unit symbol.
    pub value_text: String,
    /// Semantic colour for the value.
    pub role: ColorRole,
    /// Override for the accessible label (Studio: "Average Pace").
    pub accessible_label_id: Option<&'static str>,
    /// Override for the accessible value (Studio: `fmt_pace` instead of the
    /// tenths-formatted strip value).
    pub accessible_value: Option<String>,
}

/// One splits/intervals table row.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitRow {
    /// Web `Split.index` is 0-based; Studio displays it verbatim.
    pub index: u32,
    /// Distance in the preferred unit.
    pub distance_text: String,
    /// Elapsed time with tenths.
    pub time_text: String,
    /// Split pace.
    pub pace_text: String,
    /// Cadence rounded to an integer, "-" when absent (Studio).
    pub cadence_text: String,
    /// `power_text` for the split pace, "-" when not derivable.
    pub power_text: String,
    /// Average HR as an integer string, "-" when absent (Studio).
    pub hr_text: String,
    /// Rest segment of an interval piece.
    pub is_rest: bool,
}

/// One targets read-out row.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetRow {
    /// Locale message id (`replay.mTarget*`).
    pub label_id: &'static str,
    /// Rendered target value with unit symbol.
    pub value_text: String,
}

/// Studio's `powerText(for:pace:)`: watts from pace, bounded and rounded,
/// "-" for anything unusable.
#[must_use]
pub fn power_text(sport: Sport, pace: f64) -> String {
    if !pace.is_finite() || pace <= 0.0 {
        return "-".to_owned();
    }
    let watts = pace_to_watts_for_sport(sport, pace);
    if !watts.is_finite() || watts < 0.0 || watts > 100_000.0 {
        return "-".to_owned();
    }
    watts.round().to_string()
}

/// The detail header for a workout.
#[must_use]
pub fn header(
    detail: &WorkoutDetail,
    language: Language,
    home_timezone: Option<&str>,
) -> DetailHeader {
    let workout = &detail.workout;
    let title = workout
        .workout_type
        .clone()
        .unwrap_or_else(|| workout.sport.display_name().to_owned());
    let date_text = fmt_date(&workout.date, language, home_timezone);
    let time_text = fmt_time_of_day(&workout.date);
    let source_text = workout.source.clone().unwrap_or_default();

    let mut parts = vec![date_text.clone(), time_text.clone()];
    if !source_text.is_empty() {
        parts.push(source_text.clone());
    }
    if workout.is_interval {
        parts.push("intervals".to_owned());
    }

    DetailHeader {
        title,
        sport_name: workout.sport.display_name(),
        date_text,
        time_text,
        source_text,
        is_interval: workout.is_interval,
        comments: workout.comments.clone().unwrap_or_default(),
        accessible_text: parts.join(", "),
    }
}

/// The metric strip in Studio's order: distance, time, pace, cadence, power,
/// then the optional calories and heart rate.
#[must_use]
pub fn metric_strip(detail: &WorkoutDetail, unit: DistanceUnit) -> Vec<StripMetric> {
    let workout = &detail.workout;
    let sport = workout.sport;
    let mut strip = vec![
        StripMetric {
            label_id: "dashboard.distance",
            value_text: fmt_distance_in(workout.distance, unit),
            role: ColorRole::Distance,
            accessible_label_id: None,
            accessible_value: None,
        },
        StripMetric {
            label_id: "dashboard.time",
            value_text: fmt_time(workout.time, true),
            role: ColorRole::Neutral,
            accessible_label_id: None,
            accessible_value: None,
        },
        StripMetric {
            label_id: "replay.pacePer500m",
            value_text: fmt_time(workout.pace, true),
            role: ColorRole::Pace,
            // Studio reads the pace out as "Average Pace: 1:58" rather than
            // the tenths-style strip value.
            accessible_label_id: Some("dashboard.avgPace"),
            accessible_value: Some(fmt_pace(workout.pace)),
        },
        StripMetric {
            label_id: "dashboard.avgRate",
            value_text: match workout.stroke_rate {
                Some(rate) if rate.is_finite() => {
                    format!("{} {}", rate.round(), sport.cadence_unit())
                }
                _ => "-".to_owned(),
            },
            role: ColorRole::Cadence,
            accessible_label_id: None,
            accessible_value: None,
        },
        StripMetric {
            label_id: "replay.cPower",
            value_text: match power_text(sport, workout.pace) {
                watts if watts == "-" => "-".to_owned(),
                watts => format!("{watts} W"),
            },
            role: ColorRole::Watts,
            accessible_label_id: None,
            accessible_value: None,
        },
    ];
    if let Some(calories) = workout.calories_total {
        strip.push(StripMetric {
            label_id: "replay.mCalories",
            value_text: format!("{} Cal", calories.round()),
            role: ColorRole::Neutral,
            accessible_label_id: None,
            accessible_value: None,
        });
    }
    if let Some(hr) = workout.heart_rate_avg {
        strip.push(StripMetric {
            label_id: "replay.cHeart",
            value_text: format!("{} bpm", hr.round()),
            role: ColorRole::HeartRate,
            accessible_label_id: None,
            accessible_value: None,
        });
    }
    strip
}

/// Splits/intervals rows in log order.
#[must_use]
pub fn split_rows(detail: &WorkoutDetail, unit: DistanceUnit) -> Vec<SplitRow> {
    let sport = detail.workout.sport;
    detail
        .splits
        .iter()
        .map(|split| split_row(split, sport, unit))
        .collect()
}

fn split_row(split: &Split, sport: Sport, unit: DistanceUnit) -> SplitRow {
    // Studio reads `heartRate?.average`; the web mapper also keeps the legacy
    // scalar `hr`, which the desktop falls back to (source-map divergence).
    let hr = split
        .heart_rate
        .as_ref()
        .and_then(|detail| detail.average)
        .or(split.hr);
    SplitRow {
        index: split.index,
        distance_text: fmt_distance_in(split.distance, unit),
        time_text: fmt_time(split.time, true),
        pace_text: fmt_pace(split.pace),
        cadence_text: match split.spm {
            Some(cadence) if cadence.is_finite() => cadence.round().to_string(),
            _ => "-".to_owned(),
        },
        power_text: power_text(sport, split.pace),
        hr_text: match hr {
            Some(average) if average.is_finite() => (average.round() as i64).to_string(),
            _ => "-".to_owned(),
        },
        is_rest: split.is_rest.unwrap_or(false),
    }
}

/// The splits section title id (web `replay.splitBreakdown` /
/// `replay.intervalBreakdown`).
#[must_use]
pub const fn splits_section_id(is_interval: bool) -> &'static str {
    if is_interval {
        "replay.intervalBreakdown"
    } else {
        "replay.splitBreakdown"
    }
}

/// Column header ids for the splits table (web `replay.th*`).
#[must_use]
pub const fn split_column_ids() -> [&'static str; 7] {
    [
        "replay.thNum",
        "replay.thDist",
        "replay.thTime",
        "replay.thPace",
        "replay.thRate",
        "replay.cPower",
        "replay.thHr",
    ]
}

/// The targets read-out (web replay gauge set); empty when the logbook entry
/// carried no targets.
#[must_use]
pub fn target_rows(detail: &WorkoutDetail) -> Vec<TargetRow> {
    let Some(targets) = &detail.workout.targets else {
        return Vec::new();
    };
    let sport = detail.workout.sport;
    let mut rows = Vec::new();
    if let Some(pace) = targets.pace {
        if pace.is_finite() && pace > 0.0 {
            rows.push(TargetRow {
                label_id: "replay.mTargetPace",
                value_text: fmt_pace(pace),
            });
        }
    }
    if let Some(watts) = targets.watts {
        if watts.is_finite() && watts > 0.0 {
            rows.push(TargetRow {
                label_id: "replay.mTargetWatts",
                value_text: format!("{} W", watts.round()),
            });
        }
    }
    if let Some(rate) = targets.stroke_rate {
        if rate.is_finite() && rate > 0.0 {
            rows.push(TargetRow {
                label_id: "replay.mTargetRate",
                value_text: format!("{} {}", rate.round(), sport.cadence_unit()),
            });
        }
    }
    if let Some(zone) = targets.heart_rate_zone {
        if zone.is_finite() && zone > 0.0 {
            rows.push(TargetRow {
                label_id: "replay.mTargetHrZone",
                value_text: zone.round().to_string(),
            });
        }
    }
    if let Some(calories) = targets.calories {
        if calories.is_finite() && calories > 0.0 {
            rows.push(TargetRow {
                label_id: "replay.mTargetCalories",
                value_text: format!("{} Cal", calories.round()),
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use rowplay_core::demo::{demo_details, mock_workout_detail};

    use super::*;

    fn demo_2000m() -> WorkoutDetail {
        mock_workout_detail(1001).expect("demo 1001")
    }

    /// Studio's `powerText`: rounded core watts, with bounds and non-finite
    /// inputs falling back to "-".
    #[test]
    fn power_text_matches_studio() {
        let rower = pace_to_watts_for_sport(Sport::Rower, 120.0);
        assert_eq!(power_text(Sport::Rower, 120.0), rower.round().to_string());
        // BikeErg uses the 8.0 divisor: same nominal pace, different watts.
        let bike = pace_to_watts_for_sport(Sport::Bike, 120.0);
        assert_eq!(power_text(Sport::Bike, 120.0), bike.round().to_string());
        assert_ne!(
            power_text(Sport::Rower, 120.0),
            power_text(Sport::Bike, 120.0)
        );
        assert_eq!(power_text(Sport::Rower, 0.0), "-");
        assert_eq!(power_text(Sport::Rower, -5.0), "-");
        assert_eq!(power_text(Sport::Rower, f64::NAN), "-");
        assert_eq!(power_text(Sport::Rower, f64::INFINITY), "-");
    }

    #[test]
    fn header_renders_the_demo_piece() {
        let detail = demo_2000m();
        let header = header(&detail, Language::En, None);
        assert!(!header.title.is_empty());
        assert_eq!(header.sport_name, "RowErg");
        assert!(!header.date_text.is_empty());
        assert!(header.time_text.contains(':'));
        assert!(header.accessible_text.contains(&header.date_text));
    }

    #[test]
    fn metric_strip_carries_roles_and_optional_entries() {
        let detail = demo_2000m();
        let strip = metric_strip(&detail, DistanceUnit::Metric);
        assert!(strip.len() >= 5);
        assert_eq!(strip[0].label_id, "dashboard.distance");
        assert_eq!(strip[0].role, ColorRole::Distance);
        assert_eq!(strip[2].label_id, "replay.pacePer500m");
        assert_eq!(
            strip[2].accessible_value.as_deref(),
            Some(fmt_pace(detail.workout.pace).as_str())
        );
        assert!(strip[4].value_text.ends_with('W') || strip[4].value_text == "-");

        // A demo piece with heart rate and calories gets the extra entries.
        let with_extras = demo_details()
            .into_iter()
            .find(|d| d.workout.heart_rate_avg.is_some() && d.workout.calories_total.is_some())
            .expect("demo piece with HR and calories");
        let strip = metric_strip(&with_extras, DistanceUnit::Metric);
        assert!(strip.iter().any(|m| m.label_id == "replay.mCalories"));
        let hr = strip
            .iter()
            .find(|m| m.label_id == "replay.cHeart")
            .expect("hr entry");
        assert!(hr.value_text.ends_with("bpm"));
        assert_eq!(hr.role, ColorRole::HeartRate);
    }

    #[test]
    fn split_rows_render_every_demo_split() {
        let detail = demo_2000m();
        let rows = split_rows(&detail, DistanceUnit::Metric);
        assert_eq!(rows.len(), detail.splits.len());
        for row in &rows {
            assert!(row.time_text.contains(':'));
            assert!(row.pace_text.contains(':') || row.pace_text == "--:--");
            assert!(!row.distance_text.is_empty());
        }
        assert_eq!(splits_section_id(false), "replay.splitBreakdown");
        assert_eq!(splits_section_id(true), "replay.intervalBreakdown");
        assert_eq!(split_column_ids().len(), 7);
    }

    #[test]
    fn interval_demo_pieces_flag_rest_rows() {
        let interval = demo_details()
            .into_iter()
            .find(|d| d.workout.is_interval)
            .expect("demo interval piece");
        let rows = split_rows(&interval, DistanceUnit::Metric);
        assert!(!rows.is_empty());
        assert!(rows.iter().any(|r| r.is_rest));
    }

    #[test]
    fn targets_render_when_present() {
        let with_targets = demo_details()
            .into_iter()
            .find(|d| d.workout.targets.is_some())
            .expect("demo piece with targets");
        let rows = target_rows(&with_targets);
        assert!(!rows.is_empty());
        for row in &rows {
            assert!(row.label_id.starts_with("replay.mTarget"));
            assert!(!row.value_text.is_empty());
        }
        // Without targets: no rows, no crash.
        let without = demo_details()
            .into_iter()
            .find(|d| d.workout.targets.is_none())
            .expect("demo piece without targets");
        assert!(target_rows(&without).is_empty());
    }
}
