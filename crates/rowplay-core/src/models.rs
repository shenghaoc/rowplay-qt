// SPDX-License-Identifier: GPL-3.0-or-later
//! Concept2 domain models.
//!
//! Port of the web app's `src/lib/types.ts` (canonical shape and field names)
//! with the display helpers from rowplay-studio's `Models/Sport.swift`.
//! Numeric measurements are `f64` because every TypeScript `number` is one;
//! identifiers are integers. JSON field names follow the web app (camelCase),
//! and the Studio spellings `cadence` / `heartRate` are accepted as aliases on
//! strokes so Studio's parity fixtures load unchanged.

use serde::{Deserialize, Serialize};

/// Concept2 machine families. All three report comparable results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sport {
    /// Concept2 RowErg.
    Rower,
    /// Concept2 SkiErg.
    Skierg,
    /// Concept2 BikeErg.
    Bike,
}

impl Sport {
    /// Every sport, in the web app's declaration order.
    pub const ALL: [Sport; 3] = [Sport::Rower, Sport::Skierg, Sport::Bike];

    /// Maps Concept2 result `type` values onto the sport (web `toSport`).
    ///
    /// The match is exact and case-sensitive like the web app; anything else,
    /// including `None`, is a RowErg.
    #[must_use]
    pub fn from_concept2_type(kind: Option<&str>) -> Sport {
        match kind {
            Some("ski" | "skierg") => Sport::Skierg,
            Some("bike" | "bikeerg") => Sport::Bike,
            _ => Sport::Rower,
        }
    }

    /// Parses the canonical wire spelling (`rower`, `skierg`, `bike`) used by
    /// list-query parameters. Unknown values yield `None`.
    #[must_use]
    pub fn parse(value: &str) -> Option<Sport> {
        match value {
            "rower" => Some(Sport::Rower),
            "skierg" => Some(Sport::Skierg),
            "bike" => Some(Sport::Bike),
            _ => None,
        }
    }

    /// Canonical wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Sport::Rower => "rower",
            Sport::Skierg => "skierg",
            Sport::Bike => "bike",
        }
    }

    /// Concept2 trademark name (web `SPORT_LABEL`). Never translated.
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Sport::Rower => "RowErg",
            Sport::Skierg => "SkiErg",
            Sport::Bike => "BikeErg",
        }
    }

    /// Short label used in compact UI (Studio `shortName`).
    #[must_use]
    pub const fn short_name(self) -> &'static str {
        match self {
            Sport::Rower => "Row",
            Sport::Skierg => "Ski",
            Sport::Bike => "Bike",
        }
    }

    /// Cadence unit: strokes per minute, or rpm on the BikeErg (Studio `cadenceUnit`).
    #[must_use]
    pub const fn cadence_unit(self) -> &'static str {
        match self {
            Sport::Bike => "rpm",
            Sport::Rower | Sport::Skierg => "spm",
        }
    }
}

impl std::fmt::Display for Sport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Distance display unit (Studio `DistanceUnit`; the web app is metric only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DistanceUnit {
    /// Metres and kilometres.
    #[default]
    Metric,
    /// Feet and miles.
    Imperial,
}

impl DistanceUnit {
    /// Parses the preference spelling (`metric` / `imperial`).
    #[must_use]
    pub fn parse(value: &str) -> Option<DistanceUnit> {
        match value {
            "metric" => Some(DistanceUnit::Metric),
            "imperial" => Some(DistanceUnit::Imperial),
            _ => None,
        }
    }

    /// Preference spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DistanceUnit::Metric => "metric",
            DistanceUnit::Imperial => "imperial",
        }
    }
}

/// Heart-rate summary attached to a workout or split.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeartRateDetail {
    /// Average bpm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average: Option<f64>,
    /// Minimum bpm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Maximum bpm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// bpm at the end of the effort.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<f64>,
    /// bpm during rest (split-level).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest: Option<f64>,
    /// bpm after the recovery window.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery: Option<f64>,
}

/// Targets the athlete set on the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutTargets {
    /// Target stroke rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_rate: Option<f64>,
    /// Heart-rate zone 0–5.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heart_rate_zone: Option<f64>,
    /// Target pace, seconds per 500 m (normalised from API tenths).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pace: Option<f64>,
    /// Target watts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watts: Option<f64>,
    /// Target calories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calories: Option<f64>,
}

/// Logging-device metadata. `serial_number` and `device` identify hardware and
/// are stripped from anything shared or exported.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingMetadata {
    /// Performance Monitor generation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pm_version: Option<f64>,
    /// Monitor firmware version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
    /// Sensitive — hardware identifying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    /// Sensitive — hardware identifying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// Logging device OS.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_os: Option<String>,
    /// Logging device OS version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_os_version: Option<String>,
    /// Concept2 `erg_model_type` (0 = D/E/RowErg/Dynamic, 1 = C/B, 2 = A).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erg_model_type: Option<f64>,
    /// Heart-rate transport (BT, ANT, Apple, …) as a plain string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hr_type: Option<String>,
}

/// Concept2 weight class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WeightClass {
    /// Heavyweight.
    H,
    /// Lightweight.
    L,
}

/// A summary row as returned by the Concept2 results list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workout {
    /// Concept2 result id.
    pub id: i64,
    /// Logbook wall-clock timestamp, `YYYY-MM-DD HH:MM:SS` (monitor local, no offset).
    pub date: String,
    /// Machine family.
    pub sport: Sport,
    /// Total distance in metres.
    pub distance: f64,
    /// Elapsed time in seconds.
    pub time: f64,
    /// Average pace, seconds per 500 m.
    pub pace: f64,
    /// Average stroke rate (spm, or rpm on the BikeErg).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_rate: Option<f64>,
    /// Number of strokes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke_count: Option<f64>,
    /// Average heart rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heart_rate_avg: Option<f64>,
    /// Minimum heart rate over the piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hr_min: Option<f64>,
    /// Maximum heart rate over the piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hr_max: Option<f64>,
    /// Full heart-rate detail when the logbook reports it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heart_rate: Option<HeartRateDetail>,
    /// Total calories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calories_total: Option<f64>,
    /// Total watt-minutes; divided by elapsed minutes gives average power.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watt_minutes: Option<f64>,
    /// Drag factor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drag_factor: Option<f64>,
    /// Concept2 workout type label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workout_type: Option<String>,
    /// Athlete comments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comments: Option<String>,
    /// IANA time zone of the monitor, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// True UTC instant of the workout end (Concept2 `date_utc`), when the API provides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_utc: Option<String>,
    /// Weight class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weight_class: Option<WeightClass>,
    /// Concept2 privacy level (`everyone`, `logged_in`, `partners`, `private`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy: Option<String>,
    /// Logging app/channel from Concept2 `source`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Whether Concept2 verified the result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
    /// Total rest time in seconds (interval pieces).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_time: Option<f64>,
    /// Total rest distance in metres (interval pieces).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_distance: Option<f64>,
    /// Monitor targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub targets: Option<WorkoutTargets>,
    /// Logging-device metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<LoggingMetadata>,
    /// Whether per-stroke detail is available for a real-time replay.
    #[serde(default)]
    pub has_stroke_data: bool,
    /// Whether the splits are work intervals rather than even splits.
    #[serde(default)]
    pub is_interval: bool,
}

impl Workout {
    /// A minimal workout with every optional field empty — the same defaults as
    /// the web test fixture helper (`tests/unit/fixtures.ts` → `workout()`).
    #[must_use]
    pub fn new(
        id: i64,
        date: impl Into<String>,
        sport: Sport,
        distance: f64,
        time: f64,
        pace: f64,
    ) -> Self {
        Workout {
            id,
            date: date.into(),
            sport,
            distance,
            time,
            pace,
            stroke_rate: None,
            stroke_count: None,
            heart_rate_avg: None,
            hr_min: None,
            hr_max: None,
            heart_rate: None,
            calories_total: None,
            watt_minutes: None,
            drag_factor: None,
            workout_type: None,
            comments: None,
            timezone: None,
            date_utc: None,
            weight_class: None,
            privacy: None,
            source: None,
            verified: None,
            rest_time: None,
            rest_distance: None,
            targets: None,
            metadata: None,
            has_stroke_data: true,
            is_interval: false,
        }
    }

    /// The `YYYY-MM-DD` part of the logbook date (web `date.slice(0, 10)`).
    #[must_use]
    pub fn day_key(&self) -> String {
        self.date.chars().take(10).collect()
    }
}

/// One sample on the workout timeline. Distances in metres, time in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stroke {
    /// Seconds since workout start.
    pub t: f64,
    /// Cumulative distance in metres.
    pub d: f64,
    /// Instantaneous pace, seconds per 500 m.
    pub pace: f64,
    /// Strokes per minute (or rpm for the bike). Studio spells this `cadence`.
    #[serde(alias = "cadence")]
    pub spm: f64,
    /// Heart rate in bpm, if recorded. Studio spells this `heartRate`.
    #[serde(default, alias = "heartRate", skip_serializing_if = "Option::is_none")]
    pub hr: Option<f64>,
    /// Watts, derived from pace when not reported.
    pub watts: f64,
    /// As-logged time before interval cumulative offsets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_t: Option<f64>,
    /// As-logged distance before interval cumulative offsets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_d: Option<f64>,
}

impl Stroke {
    /// A stroke without heart rate or raw values.
    #[must_use]
    pub const fn new(t: f64, d: f64, pace: f64, spm: f64, watts: f64) -> Self {
        Stroke {
            t,
            d,
            pace,
            spm,
            hr: None,
            watts,
            raw_t: None,
            raw_d: None,
        }
    }
}

/// How a split or interval was defined on the monitor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitIntervalType {
    /// Fixed time.
    Time,
    /// Fixed distance.
    Distance,
    /// Fixed calories.
    Calorie,
    /// Fixed watt-minutes.
    Wattminute,
}

/// A split/interval summary inside a workout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Split {
    /// Zero-based position in the workout (the web app's numbering).
    pub index: u32,
    /// Distance in metres.
    pub distance: f64,
    /// Time in seconds.
    pub time: f64,
    /// Pace, seconds per 500 m.
    pub pace: f64,
    /// Average stroke rate. Studio spells this `cadence`.
    #[serde(default, alias = "cadence", skip_serializing_if = "Option::is_none")]
    pub spm: Option<f64>,
    /// Legacy scalar average HR — prefer `heart_rate.average` when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hr: Option<f64>,
    /// Heart-rate detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heart_rate: Option<HeartRateDetail>,
    /// Calories.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calories_total: Option<f64>,
    /// Watt-minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watt_minutes: Option<f64>,
    /// Interval definition.
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub interval_type: Option<SplitIntervalType>,
    /// Rest time after the interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_time: Option<f64>,
    /// Rest distance after the interval.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_distance: Option<f64>,
    /// Machine for mixed-machine pieces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine: Option<Sport>,
    /// True when this row is a rest segment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_rest: Option<bool>,
}

impl Split {
    /// A work split with only the core fields set.
    #[must_use]
    pub const fn new(index: u32, distance: f64, time: f64, pace: f64) -> Self {
        Split {
            index,
            distance,
            time,
            pace,
            spm: None,
            hr: None,
            heart_rate: None,
            calories_total: None,
            watt_minutes: None,
            interval_type: None,
            rest_time: None,
            rest_distance: None,
            machine: None,
            is_rest: None,
        }
    }
}

/// Full detail needed to drive a replay: the summary plus strokes and splits.
///
/// Serialises to the web app's flat `WorkoutDetail` shape (summary fields,
/// `strokes`, `splits`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkoutDetail {
    /// Summary row.
    #[serde(flatten)]
    pub workout: Workout,
    /// Per-stroke samples with cumulative time and distance.
    #[serde(default)]
    pub strokes: Vec<Stroke>,
    /// Splits or intervals.
    #[serde(default)]
    pub splits: Vec<Split>,
}

impl WorkoutDetail {
    /// Result id.
    #[must_use]
    pub fn id(&self) -> i64 {
        self.workout.id
    }

    /// The summary row (web `summaryOf`).
    #[must_use]
    pub fn summary(&self) -> Workout {
        self.workout.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_concept2_type_is_exact_like_the_web_app() {
        assert_eq!(Sport::from_concept2_type(Some("rower")), Sport::Rower);
        assert_eq!(Sport::from_concept2_type(Some("ski")), Sport::Skierg);
        assert_eq!(Sport::from_concept2_type(Some("skierg")), Sport::Skierg);
        assert_eq!(Sport::from_concept2_type(Some("bike")), Sport::Bike);
        assert_eq!(Sport::from_concept2_type(Some("bikeerg")), Sport::Bike);
        assert_eq!(Sport::from_concept2_type(None), Sport::Rower);
        assert_eq!(Sport::from_concept2_type(Some("unknown")), Sport::Rower);
        assert_eq!(Sport::from_concept2_type(Some("")), Sport::Rower);
        // Studio lower-cases first; the web app does not, and the web app wins.
        assert_eq!(Sport::from_concept2_type(Some("SKI")), Sport::Rower);
    }

    #[test]
    fn sport_labels_keep_concept2_trademarks() {
        assert_eq!(Sport::Rower.display_name(), "RowErg");
        assert_eq!(Sport::Skierg.display_name(), "SkiErg");
        assert_eq!(Sport::Bike.display_name(), "BikeErg");
        assert_eq!(Sport::Rower.short_name(), "Row");
        assert_eq!(Sport::Skierg.short_name(), "Ski");
        assert_eq!(Sport::Bike.short_name(), "Bike");
        assert_eq!(Sport::Rower.cadence_unit(), "spm");
        assert_eq!(Sport::Skierg.cadence_unit(), "spm");
        assert_eq!(Sport::Bike.cadence_unit(), "rpm");
        assert_eq!(Sport::ALL.len(), 3);
    }

    #[test]
    fn sport_wire_round_trip() {
        for sport in Sport::ALL {
            assert_eq!(Sport::parse(sport.as_str()), Some(sport));
            let json = serde_json::to_string(&sport).unwrap();
            assert_eq!(json, format!("\"{}\"", sport.as_str()));
            assert_eq!(serde_json::from_str::<Sport>(&json).unwrap(), sport);
        }
        assert_eq!(Sport::parse("kayak"), None);
        assert_eq!(DistanceUnit::parse("metric"), Some(DistanceUnit::Metric));
        assert_eq!(
            DistanceUnit::parse("imperial"),
            Some(DistanceUnit::Imperial)
        );
        assert_eq!(DistanceUnit::parse("unknown"), None);
        assert_eq!(DistanceUnit::parse(""), None);
    }

    #[test]
    fn workout_detail_uses_the_web_json_shape() {
        let json = r#"{
            "id": 1001, "date": "2026-05-27 06:12:00", "sport": "rower",
            "distance": 2000, "time": 432.5, "pace": 108.1, "hasStrokeData": true,
            "isInterval": false, "weightClass": "H",
            "strokes": [{"t": 2, "d": 11, "pace": 120, "spm": 28, "hr": 140, "watts": 160}],
            "splits": [{"index": 0, "distance": 500, "time": 108, "pace": 108, "type": "distance"}]
        }"#;
        let detail: WorkoutDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.id(), 1001);
        assert_eq!(detail.workout.weight_class, Some(WeightClass::H));
        assert_eq!(detail.strokes[0].spm, 28.0);
        assert_eq!(detail.strokes[0].hr, Some(140.0));
        assert_eq!(
            detail.splits[0].interval_type,
            Some(SplitIntervalType::Distance)
        );
        let back: serde_json::Value = serde_json::to_value(&detail).unwrap();
        assert_eq!(back["hasStrokeData"], true);
        assert_eq!(back["strokes"][0]["spm"], 28.0);
        assert!(
            back.get("comments").is_none(),
            "absent optionals are not serialised"
        );
    }

    #[test]
    fn stroke_accepts_studio_field_spellings() {
        let stroke: Stroke = serde_json::from_str(
            r#"{"t": 2, "d": 11, "pace": 120, "cadence": 28, "heartRate": 140, "watts": 160}"#,
        )
        .unwrap();
        assert_eq!(stroke.spm, 28.0);
        assert_eq!(stroke.hr, Some(140.0));
    }

    #[test]
    fn day_key_is_the_first_ten_characters() {
        let workout = Workout::new(1, "2026-05-01 06:00:00", Sport::Rower, 2000.0, 480.0, 120.0);
        assert_eq!(workout.day_key(), "2026-05-01");
        assert!(workout.has_stroke_data);
    }
}
