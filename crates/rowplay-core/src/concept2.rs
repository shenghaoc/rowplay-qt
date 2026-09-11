// SPDX-License-Identifier: GPL-3.0-or-later
//! Concept2 Logbook raw payloads and their mapper.
//!
//! Port of the web app's `src/lib/server/concept2.ts` raw shapes and mapping
//! helpers (`mapResult`, `mapStrokes`, `mapSplits`, `mapHeartRate`,
//! `mapTargets`, `mapMetadata`, `synthStrokes` and `getWorkout`), with
//! rowplay-studio's `Concept2/Concept2Models.swift` and `Concept2Mapper.swift`
//! as the second reference. The web app is canonical; see
//! `docs/source-map.md` for the divergences the Rust port records.
//!
//! Everything here is pure: byte slices in, existing core models out. No I/O,
//! no network, no clock. Parsers bound their input before decoding
//! ([`MAX_PAYLOAD_BYTES`]) and report failures without echoing the payload.
//!
//! Wire units, all of which the mapper normalises:
//!
//! | Field | Wire unit | Core unit |
//! | --- | --- | --- |
//! | `time`, `split.time`, `stroke.t` | tenths of a second | seconds |
//! | `stroke.d` | decimetres | metres |
//! | `stroke.p`, `targets.pace` | tenths of a second per 500 m (per 1000 m on the BikeErg) | seconds per 500 m |
//! | `rest_time` | tenths of a second | seconds |

use serde::Deserialize;

use crate::formatting::pace_to_watts_for_sport;
use crate::models::{
    HeartRateDetail, LoggingMetadata, Split, SplitIntervalType, Sport, Stroke, WeightClass,
    Workout, WorkoutDetail, WorkoutTargets,
};

/// Largest Concept2 payload the mapper accepts, in bytes.
///
/// Mirrors the HTTP client's response cap: nothing parsed here can be larger
/// than what the transport is willing to read.
pub const MAX_PAYLOAD_BYTES: usize = 25 * 1024 * 1024;

/// Why a raw Concept2 payload could not be turned into core models.
///
/// Deliberately opaque: the message never echoes the payload or any part of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Concept2PayloadError {
    /// The payload exceeded [`MAX_PAYLOAD_BYTES`].
    #[error("Concept2 payload is {size} bytes, over the {limit}-byte limit")]
    TooLarge {
        /// Actual payload size.
        size: usize,
        /// Allowed maximum.
        limit: usize,
    },
    /// The payload is not the expected JSON shape.
    #[error("Concept2 payload could not be decoded")]
    Decode,
}

/// Top-level response of `GET /api/users/me/results`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SummaryResponse {
    /// The page of results.
    pub data: Vec<RawResult>,
    /// Pagination envelope; the API omits it on some responses.
    #[serde(default)]
    pub meta: Option<Meta>,
}

/// Pagination envelope.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Meta {
    /// Page bookkeeping.
    #[serde(default)]
    pub pagination: Option<Pagination>,
}

/// Page bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Pagination {
    /// Total number of pages, when the API reports it.
    #[serde(default, rename = "total_pages")]
    pub total_pages: Option<u32>,
}

/// Top-level response of `GET /api/users/me/results/{id}?include=metadata`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DetailResponse {
    /// The workout result.
    pub data: RawResult,
    /// Logging-device metadata, returned only when `include=metadata` was asked for.
    #[serde(default)]
    pub metadata: Option<RawMetadata>,
}

/// Top-level response of `GET /api/users/me/results/{id}/strokes`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct StrokesResponse {
    /// The per-stroke samples.
    pub data: Vec<RawStroke>,
}

/// A single logbook result as the API reports it (snake_case wire names).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawResult {
    /// Concept2 result id.
    pub id: i64,
    /// Logbook wall-clock timestamp, `YYYY-MM-DD HH:MM:SS`.
    pub date: String,
    /// Machine family (`rower`, `ski`, `bike`).
    #[serde(default, rename = "type")]
    pub sport_type: Option<String>,
    /// Distance in metres.
    pub distance: f64,
    /// Elapsed time in tenths of a second.
    pub time: f64,
    /// Average stroke rate.
    #[serde(default)]
    pub stroke_rate: Option<f64>,
    /// Number of strokes.
    #[serde(default)]
    pub stroke_count: Option<f64>,
    /// Drag factor.
    #[serde(default)]
    pub drag_factor: Option<f64>,
    /// Total calories.
    #[serde(default)]
    pub calories_total: Option<f64>,
    /// Total watt-minutes.
    #[serde(default, rename = "wattminutes_total")]
    pub watt_minutes: Option<f64>,
    /// Concept2 workout type label.
    #[serde(default)]
    pub workout_type: Option<String>,
    /// Athlete comments.
    #[serde(default)]
    pub comments: Option<String>,
    /// Whether per-stroke data is available.
    #[serde(default)]
    pub stroke_data: Option<bool>,
    /// IANA time zone of the monitor.
    #[serde(default)]
    pub timezone: Option<String>,
    /// True UTC instant of the workout end, when the API provides it.
    #[serde(default)]
    pub date_utc: Option<String>,
    /// Weight class.
    #[serde(default)]
    pub weight_class: Option<WeightClass>,
    /// Concept2 privacy level.
    #[serde(default)]
    pub privacy: Option<String>,
    /// Logging app/channel.
    #[serde(default)]
    pub source: Option<String>,
    /// Whether Concept2 verified the result.
    #[serde(default)]
    pub verified: Option<bool>,
    /// Total rest time in tenths of a second.
    #[serde(default)]
    pub rest_time: Option<f64>,
    /// Total rest distance in metres.
    #[serde(default)]
    pub rest_distance: Option<f64>,
    /// Heart rate: either a single average or a structured object.
    #[serde(default)]
    pub heart_rate: Option<RawHeartRateValue>,
    /// Splits, intervals and targets.
    #[serde(default)]
    pub workout: Option<RawWorkout>,
    /// Logging-device metadata embedded in the result.
    #[serde(default)]
    pub metadata: Option<RawMetadata>,
}

/// Heart rate as the API reports it: a bare number or a structured object.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum RawHeartRateValue {
    /// A single average value.
    Number(f64),
    /// The full object.
    Object(RawHeartRate),
}

/// Structured heart-rate detail.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct RawHeartRate {
    /// Average bpm.
    #[serde(default)]
    pub average: Option<f64>,
    /// Minimum bpm.
    #[serde(default)]
    pub min: Option<f64>,
    /// Maximum bpm.
    #[serde(default)]
    pub max: Option<f64>,
    /// bpm at the end of the effort.
    #[serde(default)]
    pub ending: Option<f64>,
    /// bpm during rest.
    #[serde(default)]
    pub rest: Option<f64>,
    /// bpm after the recovery window.
    #[serde(default)]
    pub recovery: Option<f64>,
}

/// The `workout` object inside a result.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct RawWorkout {
    /// Even splits.
    #[serde(default)]
    pub splits: Option<Vec<RawSplit>>,
    /// Work intervals with rest between them.
    #[serde(default)]
    pub intervals: Option<Vec<RawSplit>>,
    /// Monitor targets.
    #[serde(default)]
    pub targets: Option<RawTargets>,
}

/// One split or interval row.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct RawSplit {
    /// Distance in metres.
    #[serde(default)]
    pub distance: Option<f64>,
    /// Time in tenths of a second.
    #[serde(default)]
    pub time: Option<f64>,
    /// Average stroke rate.
    #[serde(default)]
    pub stroke_rate: Option<f64>,
    /// Calories.
    #[serde(default)]
    pub calories_total: Option<f64>,
    /// Watt-minutes.
    #[serde(default, rename = "wattminutes_total")]
    pub watt_minutes: Option<f64>,
    /// Heart rate for the segment.
    #[serde(default)]
    pub heart_rate: Option<RawHeartRateValue>,
    /// How the interval was defined (`time`, `distance`, `calorie`, `wattminute`).
    #[serde(default, rename = "type")]
    pub interval_type: Option<String>,
    /// Rest time after the interval, in tenths of a second.
    #[serde(default)]
    pub rest_time: Option<f64>,
    /// Rest distance after the interval, in metres.
    #[serde(default)]
    pub rest_distance: Option<f64>,
    /// Machine for mixed-machine pieces.
    #[serde(default)]
    pub machine: Option<String>,
}

/// Monitor targets.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
pub struct RawTargets {
    /// Target stroke rate.
    #[serde(default)]
    pub stroke_rate: Option<f64>,
    /// Heart-rate zone 0–5.
    #[serde(default)]
    pub heart_rate_zone: Option<f64>,
    /// Target pace in tenths of a second per 500 m (per 1000 m on the BikeErg).
    #[serde(default)]
    pub pace: Option<f64>,
    /// Target watts.
    #[serde(default)]
    pub watts: Option<f64>,
    /// Target calories.
    #[serde(default)]
    pub calories: Option<f64>,
}

/// Logging-device metadata.
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct RawMetadata {
    /// Performance Monitor generation.
    #[serde(default)]
    pub pm_version: Option<f64>,
    /// Monitor firmware version.
    #[serde(default)]
    pub firmware_version: Option<String>,
    /// Hardware-identifying; stripped from anything shared or exported.
    #[serde(default)]
    pub serial_number: Option<String>,
    /// Hardware-identifying; stripped from anything shared or exported.
    #[serde(default)]
    pub device: Option<String>,
    /// Logging device OS.
    #[serde(default)]
    pub device_os: Option<String>,
    /// Logging device OS version.
    #[serde(default)]
    pub device_os_version: Option<String>,
    /// Concept2 `erg_model_type`.
    #[serde(default)]
    pub erg_model_type: Option<f64>,
    /// Heart-rate transport (BT, ANT, Apple, …).
    #[serde(default)]
    pub hr_type: Option<String>,
}

/// One per-stroke sample.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct RawStroke {
    /// Time in tenths of a second.
    pub t: f64,
    /// Distance in decimetres.
    pub d: f64,
    /// Pace in tenths of a second per 500 m (per 1000 m on the BikeErg).
    pub p: f64,
    /// Strokes per minute.
    pub spm: f64,
    /// Heart rate in bpm.
    #[serde(default)]
    pub hr: Option<f64>,
}

/// Bound a payload before decoding it.
fn check_size(bytes: &[u8]) -> Result<(), Concept2PayloadError> {
    if bytes.len() > MAX_PAYLOAD_BYTES {
        return Err(Concept2PayloadError::TooLarge {
            size: bytes.len(),
            limit: MAX_PAYLOAD_BYTES,
        });
    }
    Ok(())
}

/// Decode a bounded JSON payload into `T`.
fn decode<'de, T: Deserialize<'de>>(bytes: &'de [u8]) -> Result<T, Concept2PayloadError> {
    check_size(bytes)?;
    serde_json::from_slice(bytes).map_err(|_| Concept2PayloadError::Decode)
}

impl SummaryResponse {
    /// Decode a `/results` page from raw response bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Concept2PayloadError> {
        decode(bytes)
    }
}

impl DetailResponse {
    /// Decode a `/results/{id}` response from raw response bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Concept2PayloadError> {
        decode(bytes)
    }
}

impl StrokesResponse {
    /// Decode a `/results/{id}/strokes` response from raw response bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Concept2PayloadError> {
        decode(bytes)
    }
}

impl RawResult {
    /// Decode a single result from raw response bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, Concept2PayloadError> {
        decode(bytes)
    }
}

/// Map a result to the domain summary (web `mapResult`).
#[must_use]
pub fn map_workout(raw: &RawResult) -> Workout {
    map_workout_with_metadata(raw, None)
}

/// Map a result to the domain summary, preferring explicit `metadata`
/// (the detail endpoint's top-level envelope) over the embedded copy.
#[must_use]
pub fn map_workout_with_metadata(raw: &RawResult, metadata: Option<&RawMetadata>) -> Workout {
    let sport = Sport::from_concept2_type(raw.sport_type.as_deref());
    let time = raw.time / 10.0;
    let pace = if raw.distance > 0.0 {
        time / (raw.distance / 500.0)
    } else {
        0.0
    };
    let heart_rate = map_heart_rate_value(raw.heart_rate.as_ref());

    Workout {
        id: raw.id,
        date: raw.date.clone(),
        sport,
        distance: raw.distance,
        time,
        pace,
        stroke_rate: raw.stroke_rate,
        stroke_count: raw.stroke_count,
        heart_rate_avg: heart_rate.and_then(|hr| hr.average),
        hr_min: heart_rate.and_then(|hr| hr.min),
        hr_max: heart_rate.and_then(|hr| hr.max),
        heart_rate,
        calories_total: raw.calories_total,
        watt_minutes: raw.watt_minutes,
        drag_factor: raw.drag_factor,
        workout_type: raw.workout_type.clone(),
        comments: raw.comments.clone(),
        timezone: raw.timezone.clone(),
        date_utc: raw.date_utc.clone(),
        weight_class: raw.weight_class,
        privacy: raw.privacy.clone(),
        source: raw.source.clone(),
        verified: raw.verified,
        rest_time: raw.rest_time.map(|tenths| tenths / 10.0),
        rest_distance: raw.rest_distance,
        targets: map_targets(raw.workout.as_ref().and_then(|w| w.targets.as_ref()), sport),
        metadata: map_metadata(metadata.or(raw.metadata.as_ref())),
        has_stroke_data: raw.stroke_data.unwrap_or(false),
        // The summary endpoint never reports intervals; the detail path sets this.
        is_interval: false,
    }
}

/// Map a heart-rate value (number or object) to its detail (web `mapHeartRate`).
///
/// An object with no recognised key maps to `None`, like the web helper.
#[must_use]
pub fn map_heart_rate_value(raw: Option<&RawHeartRateValue>) -> Option<HeartRateDetail> {
    match raw? {
        RawHeartRateValue::Number(average) => Some(HeartRateDetail {
            average: Some(*average),
            ..HeartRateDetail::default()
        }),
        RawHeartRateValue::Object(hr) => map_heart_rate(hr),
    }
}

/// Map a structured heart-rate object, or `None` when it carries no known key.
#[must_use]
pub fn map_heart_rate(raw: &RawHeartRate) -> Option<HeartRateDetail> {
    let detail = HeartRateDetail {
        average: raw.average,
        min: raw.min,
        max: raw.max,
        ending: raw.ending,
        rest: raw.rest,
        recovery: raw.recovery,
    };
    let empty = detail == HeartRateDetail::default();
    if empty { None } else { Some(detail) }
}

/// Map monitor targets, normalising the BikeErg target pace to sec/500 m.
#[must_use]
pub fn map_targets(raw: Option<&RawTargets>, sport: Sport) -> Option<WorkoutTargets> {
    let raw = raw?;
    let pace_div = if sport == Sport::Bike { 2.0 } else { 1.0 };
    let targets = WorkoutTargets {
        stroke_rate: raw.stroke_rate,
        heart_rate_zone: raw.heart_rate_zone,
        pace: raw.pace.map(|pace| pace / 10.0 / pace_div),
        watts: raw.watts,
        calories: raw.calories,
    };
    let empty = targets == WorkoutTargets::default();
    if empty { None } else { Some(targets) }
}

/// Map logging-device metadata, or `None` when it carries no known key.
#[must_use]
pub fn map_metadata(raw: Option<&RawMetadata>) -> Option<LoggingMetadata> {
    let raw = raw?;
    let metadata = LoggingMetadata {
        pm_version: raw.pm_version,
        firmware_version: raw.firmware_version.clone(),
        serial_number: raw.serial_number.clone(),
        device: raw.device.clone(),
        device_os: raw.device_os.clone(),
        device_os_version: raw.device_os_version.clone(),
        erg_model_type: raw.erg_model_type,
        hr_type: raw.hr_type.clone(),
    };
    let empty = metadata == LoggingMetadata::default();
    if empty { None } else { Some(metadata) }
}

/// Map the API's interval `type` onto the domain enum (web `mapSplitType`).
#[must_use]
pub fn map_split_type(raw: Option<&str>) -> Option<SplitIntervalType> {
    match raw? {
        "time" => Some(SplitIntervalType::Time),
        "distance" => Some(SplitIntervalType::Distance),
        "calorie" => Some(SplitIntervalType::Calorie),
        "wattminute" => Some(SplitIntervalType::Wattminute),
        _ => None,
    }
}

/// Map raw strokes onto the timeline, normalising units (web `mapStrokes`).
///
/// The BikeErg reports stroke pace per 1000 m, so its pace is halved to the
/// app-wide sec/500 m basis. Interval workouts restart the monitor's `t`/`d`
/// counters at zero for each rep, so a stroke that goes backwards carries the
/// previous value into a running offset and the returned timeline stays
/// monotonic. `raw_t` / `raw_d` keep the as-logged values.
#[must_use]
pub fn map_strokes(raw: &[RawStroke], sport: Sport) -> Vec<Stroke> {
    let pace_div = if sport == Sport::Bike { 2.0 } else { 1.0 };
    let mut t_offset = 0.0;
    let mut d_offset = 0.0;
    let mut prev_t = 0.0;
    let mut prev_d = 0.0;

    raw.iter()
        .map(|stroke| {
            let raw_t = stroke.t / 10.0;
            let raw_d = stroke.d / 10.0;
            if raw_t < prev_t {
                t_offset += prev_t;
            }
            if raw_d < prev_d {
                d_offset += prev_d;
            }
            prev_t = raw_t;
            prev_d = raw_d;

            let pace = stroke.p / 10.0 / pace_div;
            Stroke {
                t: raw_t + t_offset,
                d: raw_d + d_offset,
                pace,
                spm: stroke.spm,
                hr: stroke.hr,
                watts: pace_to_watts_for_sport(sport, pace),
                raw_t: Some(raw_t),
                raw_d: Some(raw_d),
            }
        })
        .collect()
}

/// Map the result's splits (or intervals) onto domain splits (web `mapSplits`).
#[must_use]
pub fn map_splits(raw: &RawResult) -> Vec<Split> {
    let empty = Vec::new();
    let raw_splits = raw
        .workout
        .as_ref()
        .and_then(|workout| workout.splits.as_ref().or(workout.intervals.as_ref()))
        .unwrap_or(&empty);

    raw_splits
        .iter()
        .enumerate()
        .map(|(index, split)| {
            let time = split.time.unwrap_or(0.0) / 10.0;
            let distance = split.distance.unwrap_or(0.0);
            let pace = if distance > 0.0 {
                time / (distance / 500.0)
            } else {
                0.0
            };
            let heart_rate = map_heart_rate_value(split.heart_rate.as_ref());
            Split {
                index: index as u32,
                distance,
                time,
                pace,
                spm: split.stroke_rate,
                hr: heart_rate.and_then(|hr| hr.average),
                heart_rate,
                calories_total: split.calories_total,
                watt_minutes: split.watt_minutes,
                interval_type: map_split_type(split.interval_type.as_deref()),
                rest_time: split.rest_time.map(|tenths| tenths / 10.0),
                rest_distance: split.rest_distance,
                machine: split
                    .machine
                    .as_deref()
                    .filter(|machine| !machine.is_empty())
                    .map(|machine| Sport::from_concept2_type(Some(machine))),
                is_rest: Some(distance == 0.0 && time > 0.0),
            }
        })
        .collect()
}

/// Build a lower-resolution stroke timeline when the logbook has no per-stroke
/// rows (web `synthStrokes`): one segment per split, or a 60-step ramp across
/// the summary when there are no splits either.
#[must_use]
pub fn synth_strokes(workout: &Workout, splits: &[Split]) -> Vec<Stroke> {
    let make = |t: f64, d: f64, pace: f64, spm: f64, hr: Option<f64>| Stroke {
        t,
        d,
        pace,
        spm,
        hr,
        watts: pace_to_watts_for_sport(workout.sport, pace),
        raw_t: None,
        raw_d: None,
    };

    if let Some(first) = splits.first() {
        let mut out = Vec::with_capacity(splits.len() + 1);
        out.push(make(
            0.0,
            0.0,
            first.pace,
            first.spm.unwrap_or(0.0),
            first.hr,
        ));
        let mut t = 0.0;
        let mut d = 0.0;
        for split in splits {
            t += split.time;
            d += split.distance;
            out.push(make(t, d, split.pace, split.spm.unwrap_or(0.0), split.hr));
        }
        return out;
    }

    // Single segment from the summary.
    const STEPS: u32 = 60;
    (0..=STEPS)
        .map(|i| {
            let f = f64::from(i) / f64::from(STEPS);
            make(
                workout.time * f,
                workout.distance * f,
                workout.pace,
                workout.stroke_rate.unwrap_or(0.0),
                workout.heart_rate_avg,
            )
        })
        .collect()
}

/// Assemble the replay-ready detail for one result (web `getWorkout`).
///
/// `raw_strokes` is the `/strokes` payload when it was fetched successfully;
/// it is used only when the result advertises `stroke_data`, like the web
/// client. When no strokes are available the timeline is synthesised from the
/// splits and `has_stroke_data` is cleared so the pose model treats the
/// workout as split-derived instead of drawing one cycle per synthesised point.
#[must_use]
pub fn assemble_detail(
    response: &DetailResponse,
    raw_strokes: Option<&[RawStroke]>,
) -> WorkoutDetail {
    let raw = &response.data;
    let mut workout = map_workout_with_metadata(raw, response.metadata.as_ref());

    let advertise_strokes = raw.stroke_data.unwrap_or(false);
    let mut strokes = match raw_strokes {
        Some(strokes) if advertise_strokes => map_strokes(strokes, workout.sport),
        _ => Vec::new(),
    };

    let splits = map_splits(raw);
    let synthesised = strokes.is_empty();
    if synthesised {
        strokes = synth_strokes(&workout, &splits);
    }

    workout.is_interval = raw
        .workout
        .as_ref()
        .and_then(|detail| detail.intervals.as_ref())
        .is_some_and(|intervals| !intervals.is_empty());
    workout.has_stroke_data = workout.has_stroke_data && !synthesised;

    WorkoutDetail {
        workout,
        strokes,
        splits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_result() -> RawResult {
        RawResult {
            id: 1,
            date: "2026-05-15 07:30:00".into(),
            sport_type: Some("rower".into()),
            distance: 2000.0,
            time: 4500.0,
            stroke_rate: Some(32.0),
            stroke_count: Some(220.0),
            drag_factor: Some(120.0),
            calories_total: Some(310.0),
            watt_minutes: Some(1500.0),
            workout_type: Some("FixedDistance".into()),
            comments: Some("steady".into()),
            stroke_data: Some(true),
            timezone: Some("Europe/London".into()),
            date_utc: Some("2026-05-15T06:30:00Z".into()),
            weight_class: Some(WeightClass::H),
            privacy: Some("everyone".into()),
            source: Some("ErgData".into()),
            verified: Some(true),
            rest_time: Some(200.0),
            rest_distance: Some(30.0),
            heart_rate: Some(RawHeartRateValue::Object(RawHeartRate {
                average: Some(150.0),
                min: Some(120.0),
                max: Some(175.0),
                ..RawHeartRate::default()
            })),
            workout: None,
            metadata: None,
        }
    }

    #[test]
    fn map_workout_normalises_units_and_carries_every_field() {
        let workout = map_workout(&raw_result());
        assert_eq!(workout.id, 1);
        assert_eq!(workout.sport, Sport::Rower);
        assert_eq!(workout.time, 450.0);
        assert_eq!(workout.pace, 112.5);
        assert_eq!(workout.stroke_rate, Some(32.0));
        assert_eq!(workout.stroke_count, Some(220.0));
        assert_eq!(workout.heart_rate_avg, Some(150.0));
        assert_eq!(workout.hr_min, Some(120.0));
        assert_eq!(workout.hr_max, Some(175.0));
        assert_eq!(workout.workout_type.as_deref(), Some("FixedDistance"));
        assert_eq!(workout.timezone.as_deref(), Some("Europe/London"));
        assert_eq!(workout.weight_class, Some(WeightClass::H));
        assert_eq!(workout.rest_time, Some(20.0));
        assert_eq!(workout.rest_distance, Some(30.0));
        assert_eq!(workout.verified, Some(true));
        assert!(workout.has_stroke_data);
        assert!(!workout.is_interval);
    }

    #[test]
    fn map_workout_keeps_absent_optionals_absent_like_the_web() {
        let raw = RawResult {
            sport_type: None,
            workout_type: None,
            verified: None,
            stroke_data: None,
            distance: 0.0,
            ..raw_result()
        };
        let workout = map_workout(&raw);
        assert_eq!(workout.sport, Sport::Rower);
        assert_eq!(workout.workout_type, None);
        assert_eq!(workout.verified, None);
        assert!(!workout.has_stroke_data);
        assert_eq!(workout.pace, 0.0);
    }

    #[test]
    fn heart_rate_maps_numbers_and_rejects_empty_objects() {
        let number = RawHeartRateValue::Number(148.0);
        assert_eq!(
            map_heart_rate_value(Some(&number)).and_then(|hr| hr.average),
            Some(148.0)
        );
        let empty = RawHeartRateValue::Object(RawHeartRate::default());
        assert_eq!(map_heart_rate_value(Some(&empty)), None);
        assert_eq!(map_heart_rate_value(None), None);
        let full = RawHeartRateValue::Object(RawHeartRate {
            ending: Some(160.0),
            recovery: Some(110.0),
            ..RawHeartRate::default()
        });
        let detail = map_heart_rate_value(Some(&full)).unwrap();
        assert_eq!(detail.average, None);
        assert_eq!(detail.ending, Some(160.0));
        assert_eq!(detail.recovery, Some(110.0));
    }

    #[test]
    fn targets_normalise_bike_pace_and_collapse_when_empty() {
        let targets = RawTargets {
            stroke_rate: Some(24.0),
            pace: Some(2000.0),
            ..RawTargets::default()
        };
        let bike = map_targets(Some(&targets), Sport::Bike).unwrap();
        assert_eq!(bike.pace, Some(100.0));
        let rower = map_targets(Some(&targets), Sport::Rower).unwrap();
        assert_eq!(rower.pace, Some(200.0));
        assert_eq!(
            map_targets(Some(&RawTargets::default()), Sport::Rower),
            None
        );
        assert_eq!(map_targets(None, Sport::Rower), None);
    }

    #[test]
    fn metadata_collapses_when_empty_and_prefers_the_envelope() {
        let embedded = RawMetadata {
            pm_version: Some(5.0),
            ..RawMetadata::default()
        };
        assert_eq!(
            map_metadata(Some(&embedded)).and_then(|m| m.pm_version),
            Some(5.0)
        );
        assert_eq!(map_metadata(Some(&RawMetadata::default())), None);
        assert_eq!(map_metadata(None), None);

        let mut raw = raw_result();
        raw.metadata = Some(embedded);
        let envelope = RawMetadata {
            firmware_version: Some("1.2.3".into()),
            ..RawMetadata::default()
        };
        let workout = map_workout_with_metadata(&raw, Some(&envelope));
        assert_eq!(
            workout.metadata.and_then(|m| m.firmware_version).as_deref(),
            Some("1.2.3")
        );
    }

    #[test]
    fn strokes_normalise_tenths_decimetres_and_the_bike_divisor() {
        let raw = [
            RawStroke {
                t: 0.0,
                d: 0.0,
                p: 1080.0,
                spm: 32.0,
                hr: None,
            },
            RawStroke {
                t: 12.0,
                d: 85.0,
                p: 1075.0,
                spm: 32.0,
                hr: Some(140.0),
            },
        ];
        let rower = map_strokes(&raw, Sport::Rower);
        assert_eq!(rower[1].t, 1.2);
        assert_eq!(rower[1].d, 8.5);
        assert_eq!(rower[1].pace, 107.5);
        assert_eq!(rower[1].raw_t, Some(1.2));
        assert_eq!(rower[1].raw_d, Some(8.5));
        assert_eq!(rower[1].hr, Some(140.0));
        assert!(rower[1].watts > 0.0);

        let bike = map_strokes(&raw, Sport::Bike);
        assert_eq!(bike[0].pace, 54.0);
        assert_eq!(bike[0].watts, pace_to_watts_for_sport(Sport::Bike, 54.0));
    }

    #[test]
    fn strokes_accumulate_interval_offsets() {
        let raw = [
            RawStroke {
                t: 0.0,
                d: 0.0,
                p: 1080.0,
                spm: 34.0,
                hr: None,
            },
            RawStroke {
                t: 32.0,
                d: 160.0,
                p: 1080.0,
                spm: 34.0,
                hr: None,
            },
            RawStroke {
                t: 0.0,
                d: 0.0,
                p: 1090.0,
                spm: 34.0,
                hr: None,
            },
            RawStroke {
                t: 8.0,
                d: 40.0,
                p: 1085.0,
                spm: 35.0,
                hr: None,
            },
        ];
        let strokes = map_strokes(&raw, Sport::Rower);
        assert_eq!(strokes[1].t, 3.2);
        assert_eq!(strokes[2].t, 3.2);
        assert_eq!(strokes[2].raw_t, Some(0.0));
        assert_eq!(strokes[3].t, 4.0);
        assert_eq!(strokes[3].d, 20.0);
        assert_eq!(strokes[2].pace, 109.0);
    }

    #[test]
    fn splits_prefer_splits_then_intervals_and_flag_rests() {
        let mut raw = raw_result();
        raw.workout = Some(RawWorkout {
            splits: Some(vec![RawSplit {
                distance: Some(500.0),
                time: Some(1125.0),
                stroke_rate: Some(32.0),
                interval_type: Some("distance".into()),
                machine: Some("bike".into()),
                rest_time: Some(200.0),
                ..RawSplit::default()
            }]),
            intervals: Some(vec![RawSplit {
                distance: Some(1.0),
                ..RawSplit::default()
            }]),
            targets: None,
        });
        let splits = map_splits(&raw);
        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].index, 0);
        assert_eq!(splits[0].time, 112.5);
        assert_eq!(splits[0].distance, 500.0);
        assert_eq!(splits[0].pace, 112.5);
        assert_eq!(splits[0].spm, Some(32.0));
        assert_eq!(splits[0].interval_type, Some(SplitIntervalType::Distance));
        assert_eq!(splits[0].rest_time, Some(20.0));
        assert_eq!(splits[0].machine, Some(Sport::Bike));
        assert_eq!(splits[0].is_rest, Some(false));

        // A zero-distance segment with time is a rest row.
        raw.workout = Some(RawWorkout {
            splits: None,
            intervals: Some(vec![RawSplit {
                distance: Some(0.0),
                time: Some(600.0),
                ..RawSplit::default()
            }]),
            targets: None,
        });
        let rests = map_splits(&raw);
        assert_eq!(rests.len(), 1);
        assert_eq!(rests[0].is_rest, Some(true));
        assert_eq!(rests[0].pace, 0.0);
        assert_eq!(rests[0].machine, None);
    }

    #[test]
    fn split_types_accept_only_the_four_known_values() {
        assert_eq!(map_split_type(Some("time")), Some(SplitIntervalType::Time));
        assert_eq!(
            map_split_type(Some("distance")),
            Some(SplitIntervalType::Distance)
        );
        assert_eq!(
            map_split_type(Some("calorie")),
            Some(SplitIntervalType::Calorie)
        );
        assert_eq!(
            map_split_type(Some("wattminute")),
            Some(SplitIntervalType::Wattminute)
        );
        assert_eq!(map_split_type(Some("other")), None);
        assert_eq!(map_split_type(None), None);
    }

    #[test]
    fn synth_strokes_walks_splits_then_falls_back_to_the_summary() {
        let workout = Workout::new(1, "2026-05-15 07:30:00", Sport::Rower, 2000.0, 450.0, 112.5);
        let splits = vec![
            Split::new(0, 500.0, 112.5, 112.5),
            Split::new(1, 500.0, 115.0, 115.0),
        ];
        let from_splits = synth_strokes(&workout, &splits);
        assert_eq!(from_splits.len(), 3);
        assert_eq!(from_splits[0].t, 0.0);
        assert_eq!(from_splits[1].d, 500.0);
        assert_eq!(from_splits[2].t, 227.5);
        assert_eq!(from_splits[2].d, 1000.0);
        assert_eq!(from_splits[2].pace, 115.0);
        assert_eq!(from_splits[2].spm, 0.0);
        assert_eq!(from_splits[2].raw_t, None);

        let from_summary = synth_strokes(&workout, &[]);
        assert_eq!(from_summary.len(), 61);
        assert_eq!(from_summary[0].t, 0.0);
        assert_eq!(from_summary[60].t, 450.0);
        assert_eq!(from_summary[60].d, 2000.0);
        assert_eq!(from_summary[60].pace, 112.5);
    }

    #[test]
    fn assemble_detail_clears_stroke_data_when_synthesising() {
        let mut raw = raw_result();
        raw.workout = Some(RawWorkout {
            splits: Some(vec![RawSplit {
                distance: Some(500.0),
                time: Some(1125.0),
                ..RawSplit::default()
            }]),
            intervals: Some(vec![RawSplit {
                distance: Some(500.0),
                time: Some(1125.0),
                ..RawSplit::default()
            }]),
            targets: None,
        });
        let response = DetailResponse {
            data: raw,
            metadata: None,
        };

        let with_strokes = assemble_detail(
            &response,
            Some(&[RawStroke {
                t: 0.0,
                d: 0.0,
                p: 1080.0,
                spm: 32.0,
                hr: None,
            }]),
        );
        assert_eq!(with_strokes.strokes.len(), 1);
        assert!(with_strokes.workout.has_stroke_data);
        assert!(with_strokes.workout.is_interval);
        assert_eq!(with_strokes.splits.len(), 1);

        let without = assemble_detail(&response, None);
        assert_eq!(without.strokes.len(), 2, "split-derived timeline");
        assert!(!without.workout.has_stroke_data);
        assert!(without.workout.is_interval);
    }

    #[test]
    fn assemble_detail_ignores_strokes_the_result_does_not_advertise() {
        let mut raw = raw_result();
        raw.stroke_data = Some(false);
        let response = DetailResponse {
            data: raw,
            metadata: None,
        };
        let detail = assemble_detail(
            &response,
            Some(&[RawStroke {
                t: 0.0,
                d: 0.0,
                p: 1080.0,
                spm: 32.0,
                hr: None,
            }]),
        );
        assert_eq!(detail.strokes.len(), 61, "summary ramp");
        assert!(!detail.workout.has_stroke_data);
        assert!(!detail.workout.is_interval);
    }

    #[test]
    fn payloads_decode_from_bounded_byte_slices() {
        let detail = br#"{"data":{"id":7,"date":"2026-01-01 00:00:00","distance":1000,"time":2250},"metadata":{"pm_version":5}}"#;
        let response = DetailResponse::from_slice(detail).unwrap();
        assert_eq!(response.data.id, 7);
        assert_eq!(response.metadata.and_then(|m| m.pm_version), Some(5.0));
        assert_eq!(response.data.sport_type, None);

        let summary = br#"{"data":[],"meta":{"pagination":{"total_pages":3}}}"#;
        let response = SummaryResponse::from_slice(summary).unwrap();
        assert!(response.data.is_empty());
        assert_eq!(
            response
                .meta
                .and_then(|m| m.pagination)
                .and_then(|p| p.total_pages),
            Some(3)
        );

        let strokes = br#"{"data":[{"t":0,"d":0,"p":1000,"spm":30}]}"#;
        assert_eq!(StrokesResponse::from_slice(strokes).unwrap().data.len(), 1);
    }

    #[test]
    fn malformed_and_oversized_payloads_are_typed_errors() {
        assert_eq!(
            DetailResponse::from_slice(b"{not json").unwrap_err(),
            Concept2PayloadError::Decode
        );
        assert_eq!(
            SummaryResponse::from_slice(b"{}").unwrap_err(),
            Concept2PayloadError::Decode
        );
        let oversized = vec![b' '; MAX_PAYLOAD_BYTES + 1];
        assert_eq!(
            SummaryResponse::from_slice(&oversized).unwrap_err(),
            Concept2PayloadError::TooLarge {
                size: MAX_PAYLOAD_BYTES + 1,
                limit: MAX_PAYLOAD_BYTES,
            }
        );
    }

    #[test]
    fn payload_errors_never_echo_the_payload() {
        let message = Concept2PayloadError::Decode.to_string();
        assert_eq!(message, "Concept2 payload could not be decoded");
        for error in [
            Concept2PayloadError::Decode,
            Concept2PayloadError::TooLarge { size: 10, limit: 5 },
        ] {
            let text = format!("{error} {error:?}");
            assert!(!text.contains("supersecret"), "{text}");
        }
    }
}
