// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden parity tests against the fixtures vendored from rowplay-studio
//! (`tests/fixtures/`, see `PROVENANCE.md`). Each test states its tolerance.
//! Fixtures for later phases are loaded and shape-checked now; their
//! behavioural assertions are `#[ignore]`d and name the phase that enables them.

use std::collections::BTreeMap;

use rowplay_core::analytics::duration_band;
use rowplay_core::concept2::{DetailResponse, StrokesResponse, assemble_detail};
use rowplay_core::formatting::pace_to_watts_for_sport;
use rowplay_core::models::{Sport, Stroke, Workout};
use rowplay_core::performance_predictor::{
    PredictionStatus, build_prediction_table, predict_times,
};
use rowplay_core::replay::engine::Frame;
use rowplay_core::replay::motion_graph::{
    CircularMotion, MotionChannel, MotionTiming, PedalMotion, ReplayMotionGraph,
    sample_motion_graph,
};
use rowplay_core::replay::race_gap::{
    ghost_distance, race_gap_metres, race_gap_seconds, relative_duration,
};
use rowplay_core::replay::race_result::{RaceOutcome, race_result};
use rowplay_core::replay::rivals::{ParsedTrace, constant_pace_strokes, parse_rival_file};
use rowplay_core::replay::sport_kinematics::{
    solve_bike_kinematics, solve_rower_kinematics, solve_skier_kinematics,
};
use rowplay_core::replay::stroke_model::{PoseContext, StrokePose, compute_at_time};
use rowplay_core::replay::theme::{
    COLORS_DARK, COLORS_LIGHT, VenuePalette, venues_dark, venues_light,
};
use rowplay_fixtures::load_json;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictorFixture {
    name: String,
    input: PredictorInput,
    expected: BTreeMap<String, f64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PredictorInput {
    known_distance: f64,
    known_seconds: f64,
}

/// Tolerance: 0.5% of the expected value (Studio's `expectedValue * 0.005`);
/// the fixture stores values rounded to two decimals.
#[test]
fn performance_predictor_matches_web_verified_values() {
    let fixtures: Vec<PredictorFixture> =
        load_json("performance-predictor-parity.json").expect("fixture");
    assert!(!fixtures.is_empty(), "should load at least one fixture");
    for fixture in &fixtures {
        let predictions = predict_times(fixture.input.known_distance, fixture.input.known_seconds);
        if fixture.expected.is_empty() {
            assert!(
                predictions.is_empty(),
                "{}: expected empty predictions for zero inputs",
                fixture.name
            );
            continue;
        }
        assert_eq!(
            predictions.len(),
            fixture.expected.len(),
            "{}: distance count",
            fixture.name
        );
        for (key, expected) in &fixture.expected {
            let distance: u32 = key
                .parse()
                .unwrap_or_else(|_| panic!("{}: invalid distance key {key}", fixture.name));
            let actual = predictions
                .get(&distance)
                .copied()
                .unwrap_or_else(|| panic!("{}: missing {distance}m", fixture.name));
            let tolerance = expected * 0.005;
            assert!(
                (actual - expected).abs() <= tolerance,
                "{}: {distance}m predicted {actual}, expected {expected} (±{tolerance})",
                fixture.name
            );
        }
    }
}

#[test]
fn prediction_table_status_parity() {
    let table = build_prediction_table(2000.0, 420.0, &[(2000.0, 410.0), (5000.0, 1200.0)]);
    let status = |d: u32| table.iter().find(|r| r.distance == d).map(|r| r.status);
    assert_eq!(
        status(2000),
        Some(PredictionStatus::Beaten),
        "2k PB of 410s beats the 420s prediction"
    );
    assert_eq!(
        status(5000),
        Some(PredictionStatus::Behind),
        "5k PB of 1200s is behind the ~1109s prediction"
    );
    assert_eq!(
        status(10000),
        Some(PredictionStatus::Untried),
        "10k has no PB"
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DurationBandFixture {
    name: String,
    input_seconds: f64,
    expected_key: String,
    expected_label: String,
    expected_nominal_seconds: f64,
}

/// Tolerance: keys and labels must match exactly (including the en dash),
/// nominal seconds within 1e-4 (Studio's accuracy).
#[test]
fn duration_band_matches_web_verified_values() {
    let fixtures: Vec<DurationBandFixture> =
        load_json("duration-band-parity.json").expect("fixture");
    assert!(!fixtures.is_empty());
    for fixture in &fixtures {
        let band = duration_band(fixture.input_seconds);
        assert_eq!(
            band.key, fixture.expected_key,
            "{} [{}s]: key",
            fixture.name, fixture.input_seconds
        );
        assert_eq!(
            band.label, fixture.expected_label,
            "{} [{}s]: label",
            fixture.name, fixture.input_seconds
        );
        assert!(
            (band.nominal - fixture.expected_nominal_seconds).abs() <= 1e-4,
            "{} [{}s]: nominal {} != {}",
            fixture.name,
            fixture.input_seconds,
            band.nominal,
            fixture.expected_nominal_seconds
        );
    }
}

// --- later-phase fixtures: loaded and shape-checked now, asserted later ------

#[derive(Deserialize)]
struct Concept2Fixture {
    description: String,
    #[serde(rename = "rawResult")]
    raw_result: serde_json::Value,
    #[serde(rename = "rawStrokes", default)]
    raw_strokes: Vec<serde_json::Value>,
    expected: serde_json::Value,
}

const CONCEPT2_FIXTURES: [&str; 4] = [
    "Concept2/rower-steady.fixture.json",
    "Concept2/rower-interval.fixture.json",
    "Concept2/ski-steady.fixture.json",
    "Concept2/bike-steady.fixture.json",
];

#[test]
fn concept2_fixtures_are_redacted_and_well_formed() {
    for name in CONCEPT2_FIXTURES {
        let fixture: Concept2Fixture = load_json(name).expect(name);
        assert!(!fixture.description.is_empty(), "{name}: description");
        assert!(
            fixture.raw_result.get("id").is_some(),
            "{name}: rawResult.id"
        );
        assert!(
            fixture.expected.get("result").is_some(),
            "{name}: expected.result"
        );
        let text = rowplay_fixtures::read_string(name).expect(name);
        for forbidden in [
            "serial_number",
            "\"device\"",
            "first_name",
            "last_name",
            "username",
            "@",
            "Bearer",
            "token",
        ] {
            assert!(
                !text.contains(forbidden),
                "{name}: contains forbidden field {forbidden}"
            );
        }
        if let Some(comments) = fixture.raw_result.get("comments") {
            assert_eq!(comments, "REDACTED", "{name}: comments must be redacted");
        }
        for stroke in &fixture.raw_strokes {
            for key in ["t", "d", "p", "spm"] {
                assert!(
                    stroke.get(key).is_some(),
                    "{name}: raw stroke missing {key}"
                );
            }
        }
    }
}

#[test]
fn concept2_mapper_matches_fixture_expectations() {
    // The fixtures were computed by hand from the documented wire units
    // (tenths of a second, decimetres, the BikeErg per-1000m stroke pace), so
    // every value below is an exact quotient; 1e-9 only absorbs float noise.
    const TOLERANCE: f64 = 1e-9;

    for name in CONCEPT2_FIXTURES {
        let fixture: Concept2Fixture = load_json(name).expect(name);

        // Drive the real byte-slice parsers by re-wrapping the fixture's raw
        // values in the API's response envelopes.
        let detail_bytes =
            serde_json::to_vec(&serde_json::json!({ "data": fixture.raw_result })).expect(name);
        let detail = DetailResponse::from_slice(&detail_bytes).expect(name);

        let strokes_bytes =
            serde_json::to_vec(&serde_json::json!({ "data": fixture.raw_strokes })).expect(name);
        let raw_strokes = StrokesResponse::from_slice(&strokes_bytes).expect(name);

        let mapped = assemble_detail(&detail, Some(&raw_strokes.data));

        let expected = &fixture.expected;
        let result = &expected["result"];

        assert_eq!(
            mapped.workout.sport.as_str(),
            result["sport"].as_str().expect(name),
            "{name}: sport"
        );
        for (field, actual) in [
            ("time", mapped.workout.time),
            ("distance", mapped.workout.distance),
            ("pace", mapped.workout.pace),
        ] {
            let want = result[field]
                .as_f64()
                .unwrap_or_else(|| panic!("{name}: expected.result.{field}"));
            assert!(
                (actual - want).abs() <= TOLERANCE,
                "{name}: {field}: {actual} != {want}"
            );
        }

        let expected_strokes = expected["strokes"].as_array().expect(name);
        assert_eq!(
            mapped.strokes.len(),
            fixture.raw_strokes.len(),
            "{name}: stroke count"
        );
        assert!(
            mapped.workout.has_stroke_data,
            "{name}: a fixture with strokes keeps stroke data"
        );
        for entry in expected_strokes {
            let index = entry["_index"].as_u64().expect(name) as usize;
            let stroke = &mapped.strokes[index];
            for (field, actual) in [("t", stroke.t), ("d", stroke.d), ("pace", stroke.pace)] {
                let want = entry[field]
                    .as_f64()
                    .unwrap_or_else(|| panic!("{name}: expected.strokes[{index}].{field}"));
                assert!(
                    (actual - want).abs() <= TOLERANCE,
                    "{name}: strokes[{index}].{field}: {actual} != {want}"
                );
            }
            // The interval fixture also pins the as-logged values behind the
            // cumulative offsets.
            for (field, actual) in [("rawT", stroke.raw_t), ("rawD", stroke.raw_d)] {
                if let Some(want) = entry.get(field).and_then(serde_json::Value::as_f64) {
                    let actual = actual
                        .unwrap_or_else(|| panic!("{name}: strokes[{index}].{field} missing"));
                    assert!(
                        (actual - want).abs() <= TOLERANCE,
                        "{name}: strokes[{index}].{field}: {actual} != {want}"
                    );
                }
            }
        }

        let expected_splits = expected["splits"].as_array().expect(name);
        assert_eq!(mapped.splits.len(), expected_splits.len(), "{name}: splits");
        for entry in expected_splits {
            let index = entry["_index"].as_u64().expect(name) as usize;
            let split = &mapped.splits[index];
            for (field, actual) in [
                ("time", split.time),
                ("distance", split.distance),
                ("pace", split.pace),
            ] {
                let want = entry[field]
                    .as_f64()
                    .unwrap_or_else(|| panic!("{name}: expected.splits[{index}].{field}"));
                assert!(
                    (actual - want).abs() <= TOLERANCE,
                    "{name}: splits[{index}].{field}: {actual} != {want}"
                );
            }
        }
    }
}

#[test]
fn replay_fixtures_load() {
    let pose = rowplay_fixtures::load_value("stroke-pose-parity.json").expect("stroke-pose");
    assert!(pose["cases"].as_array().is_some_and(|c| c.len() == 3));
    let gap = rowplay_fixtures::load_value("replay-race-gap-parity.json").expect("race-gap");
    assert!(gap["cases"].as_array().is_some_and(|c| !c.is_empty()));
    let result =
        rowplay_fixtures::load_value("replay-race-result-parity.json").expect("race-result");
    assert!(result["cases"].as_array().is_some_and(|c| !c.is_empty()));
    let sources =
        rowplay_fixtures::load_value("replay-rival-sources-parity.json").expect("rival-sources");
    assert!(
        sources["constantPace"]
            .as_array()
            .is_some_and(|c| !c.is_empty())
    );
    assert!(sources["csv"].as_array().is_some_and(|c| !c.is_empty()));
    for name in [
        "replay-current-main-motion.json",
        "replay-current-main-2d.json",
    ] {
        let value = rowplay_fixtures::load_value(name).expect(name);
        assert_eq!(
            value["sourceCommit"], "4d96480e7c6fb382f800555bd3aa463d9fe5b1a6",
            "{name}"
        );
        assert!(
            value["samples"].as_array().is_some_and(|s| !s.is_empty()),
            "{name}: samples"
        );
    }
    for name in [
        "replay-current-main-grips.json",
        "replay-current-main-equipment.json",
    ] {
        let value = rowplay_fixtures::load_value(name).expect(name);
        assert!(
            value["schema"]
                .as_str()
                .is_some_and(|s| s.starts_with("rowplay.replay.current-main.")),
            "{name}"
        );
    }
}

// --- Phase 2 golden parity (each test states its tolerance) ------------------

/// Tolerance: the fixture stores hand-verified ranges; index / cycleFrac /
/// driveFrac compare at 1e-6 like Studio's fixture test, ranges inclusive.
#[test]
fn stroke_pose_parity() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        name: String,
        sport: String,
        strokes: Vec<Stroke>,
        context: Context,
        query_time: f64,
        expected: Expected,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Context {
        peak_watts: f64,
        median_watts: f64,
        #[serde(rename = "medianDPS")]
        median_dps: f64,
        #[serde(rename = "medianHR")]
        median_hr: f64,
        #[serde(rename = "maxHR")]
        max_hr: f64,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Expected {
        index: usize,
        cycle_frac: f64,
        drive: bool,
        drive_frac: Option<f64>,
        drive_frac_range: Option<Vec<f64>>,
        intensity_range: Option<Vec<f64>>,
        fatigue_range: Option<Vec<f64>>,
        amplitude_range: Option<Vec<f64>>,
    }

    #[derive(Deserialize)]
    struct Wrapper {
        cases: Vec<Case>,
    }
    let cases: Wrapper = rowplay_fixtures::load_json("stroke-pose-parity.json").expect("fixture");
    let cases = &cases.cases;
    assert!(!cases.is_empty());
    for case in cases {
        let sport = match case.sport.as_str() {
            "rower" => Sport::Rower,
            "skierg" => Sport::Skierg,
            "bike" => Sport::Bike,
            other => panic!("{}: unknown sport {other}", case.name),
        };
        // Studio's fixture harness: sample-and-hold the bracketing stroke row
        // and compute the pose from the frame (progress = query time/duration).
        let index = case
            .strokes
            .iter()
            .position(|s| case.query_time < s.t)
            .unwrap_or(case.strokes.len() - 1);
        let end = &case.strokes[index];
        let start = case
            .strokes
            .get(index.wrapping_sub(1))
            .filter(|_| index > 0);
        let duration = case.strokes.last().map_or(0.0, |s| s.t);
        let frame = Frame {
            t: case.query_time,
            d: end.d,
            pace: end.pace,
            spm: end.spm,
            hr: end.hr,
            watts: end.watts,
            progress: if duration > 0.0 {
                case.query_time / duration
            } else {
                0.0
            },
        };
        let context = PoseContext {
            sport,
            peak_watts: case.context.peak_watts,
            median_watts: case.context.median_watts,
            median_dps: case.context.median_dps,
            max_hr: case.context.max_hr,
        };
        let pose = compute_at_time(
            &frame,
            start.map_or(0.0, |s| s.t),
            end.t,
            start.map_or(0.0, |s| s.d),
            end.d,
            index,
            &context,
            case.context.median_hr,
            duration,
        );

        assert_eq!(pose.index, case.expected.index, "{}: index", case.name);
        assert!(
            (pose.cycle_frac - case.expected.cycle_frac).abs() <= 1e-6,
            "{}: cycleFrac {} != {}",
            case.name,
            pose.cycle_frac,
            case.expected.cycle_frac
        );
        assert_eq!(pose.drive, case.expected.drive, "{}: drive", case.name);
        if let Some(expected) = case.expected.drive_frac {
            assert!(
                (pose.drive_frac - expected).abs() <= 1e-6,
                "{}: driveFrac {} != {expected}",
                case.name,
                pose.drive_frac
            );
        }
        let in_range = |value: f64, range: &Option<Vec<f64>>, field: &str| {
            if let Some(range) = range {
                assert_eq!(range.len(), 2, "{}: {field} range", case.name);
                assert!(
                    value >= range[0] && value <= range[1],
                    "{}: {field} {value} outside {:?}",
                    case.name,
                    range
                );
            }
        };
        in_range(
            pose.drive_frac,
            &case.expected.drive_frac_range,
            "driveFrac",
        );
        in_range(pose.intensity, &case.expected.intensity_range, "intensity");
        in_range(pose.fatigue, &case.expected.fatigue_range, "fatigue");
        in_range(pose.amplitude, &case.expected.amplitude_range, "amplitude");
    }
}

/// Tolerance: gap metres/seconds and ghost distances at 1e-6 (exact web math).
#[test]
fn race_gap_parity() {
    #[derive(Deserialize)]
    struct Case {
        #[serde(default)]
        label: String,
        #[serde(default)]
        player_d: serde_json::Value,
        #[serde(default)]
        ghost_d: serde_json::Value,
        #[serde(default)]
        expected_gap_m: serde_json::Value,
        #[serde(default)]
        player_pace_per_500m: serde_json::Value,
        #[serde(default)]
        expected_gap_sec: serde_json::Value,
        #[serde(default)]
        elapsed: Option<f64>,
        #[serde(default)]
        ghost_strokes: Option<Vec<Stroke>>,
        #[serde(default)]
        expected_ghost_distance: Option<f64>,
        #[serde(default)]
        expected_relative_duration: Option<f64>,
    }

    fn number(value: &serde_json::Value) -> Option<f64> {
        match value {
            serde_json::Value::Number(n) => n.as_f64(),
            serde_json::Value::String(s) => match s.as_str() {
                "Infinity" => Some(f64::INFINITY),
                "-Infinity" => Some(f64::NEG_INFINITY),
                "NaN" => Some(f64::NAN),
                _ => s.parse().ok(),
            },
            _ => None,
        }
    }

    #[derive(Deserialize)]
    struct Wrapper {
        cases: Vec<Case>,
    }
    let cases: Wrapper =
        rowplay_fixtures::load_json("replay-race-gap-parity.json").expect("fixture");
    let cases = &cases.cases;
    assert!(!cases.is_empty());
    for case in cases {
        if let (Some(player), Some(ghost)) = (number(&case.player_d), number(&case.ghost_d)) {
            let gap = race_gap_metres(player, ghost);
            let expected = number(&case.expected_gap_m).unwrap();
            assert!(
                (gap - expected).abs() <= 1e-6,
                "{}: gap {gap} != {expected}",
                case.label
            );
            let pace = number(&case.player_pace_per_500m).unwrap_or(0.0);
            let seconds = race_gap_seconds(gap, pace);
            let expected = number(&case.expected_gap_sec).unwrap();
            assert!(
                (seconds - expected).abs() <= 1e-6,
                "{}: gap seconds {seconds} != {expected}",
                case.label
            );
        }
        if let (Some(elapsed), Some(strokes)) = (case.elapsed, &case.ghost_strokes) {
            let distance = ghost_distance(elapsed, strokes);
            let expected = case.expected_ghost_distance.unwrap();
            assert!(
                (distance - expected).abs() <= 1e-6,
                "{}: ghost distance {distance} != {expected}",
                case.label
            );
        }
        if let Some(strokes) = &case.ghost_strokes {
            if let Some(expected) = case.expected_relative_duration {
                let duration = relative_duration(strokes);
                assert!(
                    (duration - expected).abs() <= 1e-6,
                    "{}: relative duration {duration} != {expected}",
                    case.label
                );
            }
        }
    }
}

/// Tolerance: outcome exact; margins 0.05 s / 0.5 m and finish times 0.05 s
/// (Studio's fixture test accuracies).
#[test]
fn race_result_parity() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Case {
        label: String,
        axis: String,
        #[serde(default)]
        target_distance: serde_json::Value,
        #[serde(default)]
        target_duration: serde_json::Value,
        #[serde(default)]
        workout_type: Option<String>,
        player_strokes: Vec<Stroke>,
        rival_strokes: Vec<Stroke>,
        #[serde(default)]
        expect_outcome: Option<String>,
        #[serde(default)]
        expect_time_margin: Option<f64>,
        #[serde(default)]
        expect_distance_margin: Option<f64>,
        #[serde(default)]
        expect_player_finish_time: Option<f64>,
        #[serde(default)]
        expect_rival_finish_time: Option<f64>,
        #[serde(default)]
        expect_rival_dnf: Option<bool>,
        #[serde(default)]
        expect_time_margin_absent: Option<bool>,
        #[serde(default)]
        expect_nil: Option<bool>,
    }

    fn number(value: &serde_json::Value) -> f64 {
        match value {
            serde_json::Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
            serde_json::Value::String(s) if s == "NaN" => f64::NAN,
            _ => f64::NAN,
        }
    }

    #[derive(Deserialize)]
    struct Wrapper {
        cases: Vec<Case>,
    }
    let cases: Wrapper =
        rowplay_fixtures::load_json("replay-race-result-parity.json").expect("fixture");
    let cases = &cases.cases;
    assert!(!cases.is_empty());
    for case in cases {
        let mut workout = Workout::new(
            1,
            "2026-01-01 00:00:00",
            Sport::Rower,
            number(&case.target_distance),
            case.target_duration.as_f64().unwrap_or(480.0),
            120.0,
        );
        workout.workout_type = case
            .workout_type
            .clone()
            .or_else(|| match case.axis.as_str() {
                "time" => Some("FixedTimeInterval".to_string()),
                _ => None,
            });
        let result = race_result(&case.player_strokes, &case.rival_strokes, &workout);

        if case.expect_nil == Some(true) {
            assert!(result.is_none(), "{}: expected no result", case.label);
            continue;
        }
        let result = result.unwrap_or_else(|| panic!("{}: expected a result", case.label));
        let outcome = match case.expect_outcome.as_deref() {
            Some("playerWon") => RaceOutcome::PlayerWon,
            Some("rivalWon") => RaceOutcome::RivalWon,
            Some("tie") => RaceOutcome::Tie,
            other => panic!("{}: unexpected outcome {other:?}", case.label),
        };
        assert_eq!(result.outcome, outcome, "{}: outcome", case.label);
        if let Some(expected) = case.expect_time_margin {
            let actual = result.time_margin.unwrap_or(-1.0);
            assert!(
                (actual - expected).abs() <= 0.05,
                "{}: time margin {actual} != {expected}",
                case.label
            );
        }
        if case.expect_time_margin_absent == Some(true) {
            assert!(
                result.time_margin.is_none(),
                "{}: time margin absent",
                case.label
            );
        }
        if let Some(expected) = case.expect_distance_margin {
            let actual = result.distance_margin.unwrap_or(-1.0);
            assert!(
                (actual - expected).abs() <= 0.5,
                "{}: distance margin {actual} != {expected}",
                case.label
            );
        }
        if let Some(expected) = case.expect_player_finish_time {
            let actual = result.player_finish_time.unwrap_or(-1.0);
            assert!(
                (actual - expected).abs() <= 0.05,
                "{}: player finish {actual} != {expected}",
                case.label
            );
        }
        if let Some(expected) = case.expect_rival_finish_time {
            let actual = result.rival_finish_time.unwrap_or(-1.0);
            assert!(
                (actual - expected).abs() <= 0.05,
                "{}: rival finish {actual} != {expected}",
                case.label
            );
        }
        if let Some(expected) = case.expect_rival_dnf {
            assert_eq!(
                result.rival_did_not_finish, expected,
                "{}: rival DNF",
                case.label
            );
        }
    }
}

/// Tolerance: constant-pace endpoints at 1e-3, bike divisor exact
/// (Studio's fixture test accuracies).
#[test]
fn rival_sources_parity() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        constant_pace: Vec<ConstantPaceCase>,
        csv: Vec<TextCase>,
        tcx: Vec<TextCase>,
        fit: Vec<BinaryCase>,
        normalization: Vec<TextCase>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ConstantPaceCase {
        label: String,
        pace_per_500m: f64,
        total_distance: f64,
        sport: String,
        expected_stroke_count: usize,
        expected_end_time: Option<f64>,
        expected_end_distance: Option<f64>,
        expected_pace: Option<f64>,
        expect_positive_watts: Option<bool>,
        expect_bike_watts_divisor: Option<bool>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct TextCase {
        label: String,
        file_name: String,
        content: String,
        expect_success: bool,
        min_strokes: Option<usize>,
        expected_time_at_index0: Option<f64>,
        expected_time_at_index1: Option<f64>,
        expected_time_at_index2: Option<f64>,
        expected_distance_at_index1: Option<f64>,
        expected_pace_at_index1: Option<f64>,
        expect_derived_pace: Option<bool>,
        expect_derived_watts: Option<bool>,
        expect_hr: Option<bool>,
        expect_cadence: Option<bool>,
        expect_watts: Option<bool>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct BinaryCase {
        label: String,
        file_name: String,
        base64: String,
        expect_success: bool,
        min_strokes: Option<usize>,
        expected_time_at_index0: Option<f64>,
        expected_time_at_index1: Option<f64>,
        expected_distance_at_index1: Option<f64>,
        expect_derived_pace: Option<bool>,
        expect_derived_watts: Option<bool>,
        expect_hr: Option<bool>,
    }

    fn assert_expectations(label: &str, trace: &ParsedTrace, case: &dyn CaseExpectations) {
        assert!(
            trace.strokes.len() >= case.min_strokes().unwrap_or(2),
            "{label}: stroke count {}",
            trace.strokes.len()
        );
        if let Some(t0) = case.time_at_index0() {
            assert!(
                (trace.strokes[0].t - t0).abs() <= 1e-3,
                "{label}: t0 {}",
                trace.strokes[0].t
            );
        }
        if let Some(t1) = case.time_at_index1() {
            assert!(
                (trace.strokes[1].t - t1).abs() <= 1e-3,
                "{label}: t1 {} != {t1}",
                trace.strokes[1].t
            );
        }
        if let Some(t2) = case.time_at_index2() {
            assert!(
                (trace.strokes[2].t - t2).abs() <= 1e-3,
                "{label}: t2 {} != {t2}",
                trace.strokes[2].t
            );
        }
        if let Some(d1) = case.distance_at_index1() {
            assert!(
                (trace.strokes[1].d - d1).abs() <= 1e-2,
                "{label}: d1 {} != {d1}",
                trace.strokes[1].d
            );
        }
        if let Some(p1) = case.pace_at_index1() {
            assert!(
                (trace.strokes[1].pace - p1).abs() <= 1e-3,
                "{label}: pace1 {} != {p1}",
                trace.strokes[1].pace
            );
        }
        if case.derived_pace() == Some(true) {
            assert!(trace.strokes[1].pace > 0.0, "{label}: derived pace");
        }
        if case.derived_watts() == Some(true) {
            assert!(
                trace.strokes.iter().skip(1).all(|s| s.watts > 0.0),
                "{label}: derived watts"
            );
        }
        if case.hr() == Some(true) {
            assert!(
                trace.strokes.iter().any(|s| s.hr.is_some()),
                "{label}: heart rate"
            );
        }
        if case.cadence() == Some(true) {
            assert!(
                trace.strokes.iter().any(|s| s.spm > 0.0),
                "{label}: cadence"
            );
        }
        if case.watts() == Some(true) {
            assert!(
                trace.strokes.iter().any(|s| s.watts > 0.0),
                "{label}: watts"
            );
        }
    }

    trait CaseExpectations {
        fn min_strokes(&self) -> Option<usize>;
        fn time_at_index0(&self) -> Option<f64>;
        fn time_at_index1(&self) -> Option<f64>;
        fn time_at_index2(&self) -> Option<f64>;
        fn distance_at_index1(&self) -> Option<f64>;
        fn pace_at_index1(&self) -> Option<f64>;
        fn derived_pace(&self) -> Option<bool>;
        fn derived_watts(&self) -> Option<bool>;
        fn hr(&self) -> Option<bool>;
        fn cadence(&self) -> Option<bool>;
        fn watts(&self) -> Option<bool>;
    }

    impl CaseExpectations for TextCase {
        fn min_strokes(&self) -> Option<usize> {
            self.min_strokes
        }
        fn time_at_index0(&self) -> Option<f64> {
            self.expected_time_at_index0
        }
        fn time_at_index1(&self) -> Option<f64> {
            self.expected_time_at_index1
        }
        fn time_at_index2(&self) -> Option<f64> {
            self.expected_time_at_index2
        }
        fn distance_at_index1(&self) -> Option<f64> {
            self.expected_distance_at_index1
        }
        fn pace_at_index1(&self) -> Option<f64> {
            self.expected_pace_at_index1
        }
        fn derived_pace(&self) -> Option<bool> {
            self.expect_derived_pace
        }
        fn derived_watts(&self) -> Option<bool> {
            self.expect_derived_watts
        }
        fn hr(&self) -> Option<bool> {
            self.expect_hr
        }
        fn cadence(&self) -> Option<bool> {
            self.expect_cadence
        }
        fn watts(&self) -> Option<bool> {
            self.expect_watts
        }
    }

    impl CaseExpectations for BinaryCase {
        fn min_strokes(&self) -> Option<usize> {
            self.min_strokes
        }
        fn time_at_index0(&self) -> Option<f64> {
            self.expected_time_at_index0
        }
        fn time_at_index1(&self) -> Option<f64> {
            self.expected_time_at_index1
        }
        fn time_at_index2(&self) -> Option<f64> {
            None
        }
        fn distance_at_index1(&self) -> Option<f64> {
            self.expected_distance_at_index1
        }
        fn pace_at_index1(&self) -> Option<f64> {
            None
        }
        fn derived_pace(&self) -> Option<bool> {
            self.expect_derived_pace
        }
        fn derived_watts(&self) -> Option<bool> {
            self.expect_derived_watts
        }
        fn hr(&self) -> Option<bool> {
            self.expect_hr
        }
        fn cadence(&self) -> Option<bool> {
            None
        }
        fn watts(&self) -> Option<bool> {
            None
        }
    }

    /// Minimal standard base64 decoder for fixture payloads.
    fn decode_base64(input: &str) -> Vec<u8> {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for c in input
            .bytes()
            .filter(|c| *c != b'=' && !c.is_ascii_whitespace())
        {
            let value = TABLE.iter().position(|t| *t == c).expect("valid base64") as u32;
            buffer = (buffer << 6) | value;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push(((buffer >> bits) & 0xff) as u8);
            }
        }
        out
    }

    let fixture: Fixture =
        rowplay_fixtures::load_json("replay-rival-sources-parity.json").expect("fixture");

    for case in &fixture.constant_pace {
        let sport = Sport::parse(&case.sport).unwrap_or(Sport::Rower);
        let strokes = constant_pace_strokes(case.pace_per_500m, case.total_distance, sport);
        assert_eq!(strokes.len(), case.expected_stroke_count, "{}", case.label);
        if case.expected_stroke_count == 2 {
            assert!((strokes[0].t - 0.0).abs() <= 1e-3, "{}", case.label);
            assert!((strokes[0].d - 0.0).abs() <= 1e-3, "{}", case.label);
            if let Some(end_t) = case.expected_end_time {
                assert!((strokes[1].t - end_t).abs() <= 1e-3, "{}", case.label);
            }
            if let Some(end_d) = case.expected_end_distance {
                assert!((strokes[1].d - end_d).abs() <= 1e-3, "{}", case.label);
            }
            if let Some(pace) = case.expected_pace {
                assert!((strokes[0].pace - pace).abs() <= 1e-3, "{}", case.label);
            }
            if case.expect_positive_watts == Some(true) {
                assert!(strokes[0].watts > 0.0, "{}", case.label);
            }
            if case.expect_bike_watts_divisor == Some(true) {
                let rower =
                    constant_pace_strokes(case.pace_per_500m, case.total_distance, Sport::Rower);
                assert!(strokes[0].watts < rower[0].watts, "{}", case.label);
                let expected = pace_to_watts_for_sport(Sport::Bike, case.pace_per_500m);
                assert!(
                    (strokes[0].watts - expected).abs() <= 1e-9,
                    "{}",
                    case.label
                );
            }
            assert_eq!(strokes[0].spm, 0.0, "{}", case.label);
        }
    }

    let text_sections: [(&str, &[TextCase]); 3] = [
        ("csv", &fixture.csv),
        ("tcx", &fixture.tcx),
        ("normalization", &fixture.normalization),
    ];
    for (section, cases) in text_sections {
        for case in cases {
            if case.expect_success {
                let trace = parse_rival_file(case.content.as_bytes(), &case.file_name)
                    .unwrap_or_else(|error| panic!("{}: {error}", case.label));
                assert_eq!(trace.file_name, case.file_name, "{}", case.label);
                assert_expectations(&case.label, &trace, case);
            } else {
                assert!(
                    parse_rival_file(case.content.as_bytes(), &case.file_name).is_err(),
                    "{}: expected failure ({section})",
                    case.label
                );
            }
        }
    }

    for case in &fixture.fit {
        let data = decode_base64(&case.base64);
        if case.expect_success {
            let trace = parse_rival_file(&data, &case.file_name)
                .unwrap_or_else(|error| panic!("{}: {error}", case.label));
            assert_expectations(&case.label, &trace, case);
        } else {
            assert!(
                parse_rival_file(&data, &case.file_name).is_err(),
                "{}: expected failure",
                case.label
            );
        }
    }
}

/// Tolerance: every channel within 1e-10 across the 129-phase × 3-sport
/// corpus (Studio's accuracy for the same fixture).
#[test]
fn motion_graph_parity() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        sample_count_per_sport: usize,
        sample_count: usize,
        channels_by_sport: std::collections::BTreeMap<String, Vec<String>>,
        samples: Vec<Sample>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Sample {
        sport: String,
        phase_index: usize,
        values: Vec<f64>,
    }

    let fixture: Fixture =
        rowplay_fixtures::load_json("replay-current-main-motion.json").expect("fixture");
    assert_eq!(fixture.sample_count_per_sport, 129);
    assert_eq!(fixture.sample_count, fixture.samples.len());

    let mut counts = std::collections::BTreeMap::new();
    for sample in &fixture.samples {
        let sport = Sport::parse(&sample.sport).expect("sport");
        *counts.entry(sample.sport.clone()).or_insert(0) += 1;
        let channels = fixture
            .channels_by_sport
            .get(&sample.sport)
            .unwrap_or_else(|| panic!("channels for {}", sample.sport));
        assert_eq!(sample.values.len(), channels.len());
        let pose = generator_pose(sport, sample.phase_index, fixture.sample_count_per_sport);
        let graph = sample_motion_graph(sport, &pose);
        let actual = flatten_motion_graph(&graph);
        assert_eq!(
            actual.len(),
            channels.len(),
            "{}: channel shape at phase {}",
            sample.sport,
            sample.phase_index
        );
        for (path, expected) in channels.iter().zip(&sample.values) {
            let value = match actual.iter().find(|(p, _)| p == path) {
                Some((_, value)) => *value,
                None => panic!("{}: missing channel {path}", sample.sport),
            };
            assert!(
                (value - expected).abs() <= 1e-10,
                "{} {path} phase {}: {value} != {expected}",
                sample.sport,
                sample.phase_index
            );
        }
    }
    assert_eq!(counts["rower"], 129);
    assert_eq!(counts["skierg"], 129);
    assert_eq!(counts["bike"], 129);
}

/// Tolerance: projections within 1e-10 and palettes character-exact across
/// the 64-phase × 3-sport corpus.
#[test]
fn two_d_kinematics_parity() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        sample_count_per_sport: usize,
        samples: Vec<Sample>,
        palettes: Palettes,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Sample {
        sport: String,
        phase_index: usize,
        kinematics: serde_json::Value,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Palettes {
        venues_light:
            std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
        venues_dark: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
        colors_light: std::collections::BTreeMap<String, String>,
        colors_dark: std::collections::BTreeMap<String, String>,
    }

    let fixture: Fixture =
        rowplay_fixtures::load_json("replay-current-main-2d.json").expect("fixture");
    assert_eq!(fixture.sample_count_per_sport, 64);

    for sample in &fixture.samples {
        let sport = Sport::parse(&sample.sport).expect("sport");
        let pose = generator_pose(sport, sample.phase_index, fixture.sample_count_per_sport);
        let expected = sample.kinematics.as_object().expect("kinematics object");
        let value_of = |key: &str| {
            expected[key]
                .as_f64()
                .unwrap_or_else(|| panic!("{}: field {key}", sample.sport))
        };
        match sport {
            Sport::Rower => {
                let k = solve_rower_kinematics(&pose);
                let pairs = [
                    ("legExtension", k.leg_extension),
                    ("bodySwing", k.body_swing),
                    ("armDraw", k.arm_draw),
                    ("bladeDepth", k.blade_depth),
                    ("bladeFeather", k.blade_feather),
                    ("surge", k.surge),
                    ("vertical", k.vertical),
                ];
                for (key, value) in pairs {
                    assert!(
                        (value - value_of(key)).abs() <= 1e-10,
                        "{} rower {key}: {value} != {}",
                        sample.sport,
                        value_of(key)
                    );
                }
            }
            Sport::Skierg => {
                let k = solve_skier_kinematics(&pose);
                let pairs = [
                    ("cycle", k.cycle),
                    ("armPress", k.arm_press),
                    ("hipHinge", k.hip_hinge),
                    ("kneeFlex", k.knee_flex),
                    ("poleContact", k.pole_contact),
                    ("poleSweep", k.pole_sweep),
                    ("elbowLoad", k.elbow_load),
                    ("armExtension", k.arm_extension),
                    ("poleLift", k.pole_lift),
                    ("poleFlight", k.pole_flight),
                    ("rebound", k.rebound),
                    ("surge", k.surge),
                ];
                for (key, value) in pairs {
                    assert!(
                        (value - value_of(key)).abs() <= 1e-10,
                        "{} skierg {key}: {value} != {}",
                        sample.sport,
                        value_of(key)
                    );
                }
            }
            Sport::Bike => {
                let k = solve_bike_kinematics(&pose);
                let pairs = [
                    ("crankAngle", k.crank_angle),
                    ("torsoSway", k.torso_sway),
                    ("hipRock", k.hip_rock),
                    ("anklePitchLeft", k.ankle_pitch_left),
                    ("anklePitchRight", k.ankle_pitch_right),
                ];
                for (key, value) in pairs {
                    assert!(
                        (value - value_of(key)).abs() <= 1e-10,
                        "{} bike {key}: {value} != {}",
                        sample.sport,
                        value_of(key)
                    );
                }
            }
        }
    }

    // Palettes: exact strings.
    let colors = [
        (&fixture.palettes.colors_light, &COLORS_LIGHT, "colorsLight"),
        (&fixture.palettes.colors_dark, &COLORS_DARK, "colorsDark"),
    ];
    for (expected, actual, name) in colors {
        for key in ["live", "ghost", "skin", "skinShade", "hair", "shoe", "foam"] {
            let expected = &expected[key];
            let value = match key {
                "live" => actual.live,
                "ghost" => actual.ghost,
                "skin" => actual.skin,
                "skinShade" => actual.skin_shade,
                "hair" => actual.hair,
                "shoe" => actual.shoe,
                _ => actual.foam,
            };
            assert_eq!(value, expected.as_str(), "{name}.{key}");
        }
    }
    let venues = [
        (
            &fixture.palettes.venues_light,
            venues_light as fn(Sport) -> &'static VenuePalette,
            "venuesLight",
        ),
        (
            &fixture.palettes.venues_dark,
            venues_dark as fn(Sport) -> &'static VenuePalette,
            "venuesDark",
        ),
    ];
    for (expected, resolver, name) in venues {
        for (sport_key, sport) in [
            ("rower", Sport::Rower),
            ("skierg", Sport::Skierg),
            ("bike", Sport::Bike),
        ] {
            let palette = resolver(sport);
            for (field, expected_value) in &expected[sport_key] {
                let actual = venue_field(palette, field)
                    .unwrap_or_else(|| panic!("{name}.{sport_key}.{field}: no such field"));
                assert_eq!(actual, expected_value, "{name}.{sport_key}.{field}");
            }
        }
    }
}

/// Deterministic pose scheme shared with Studio's parity export
/// (`export_rowplay_native_parity.mjs` → `poseFor`).
fn generator_pose(sport: Sport, phase_index: usize, sample_count: usize) -> StrokePose {
    let cycle = f64::from(phase_index as u32) / f64::from(sample_count as u32);
    let phase = cycle * std::f64::consts::TAU;
    let (stroke_seconds, drive_frac, stroke_meters) = match sport {
        Sport::Bike => (0.75, 0.5, 5.0),
        Sport::Skierg => (1.875, 0.34, 8.0),
        Sport::Rower => (60.0 / 28.0, 0.38, 11.0),
    };
    let intensity = f64::from(((phase_index * 37) % sample_count) as u32)
        / f64::from((sample_count - 1) as u32);
    StrokePose {
        index: 7,
        phase,
        warped_phase: phase,
        cycle_frac: cycle,
        drive_frac,
        drive: cycle < drive_frac,
        drive_progress: if cycle < drive_frac {
            cycle / drive_frac
        } else {
            1.0
        },
        recovery_progress: if cycle < drive_frac {
            0.0
        } else {
            (cycle - drive_frac) / (1.0 - drive_frac)
        },
        stroke_seconds,
        stroke_meters,
        rate: 60.0 / stroke_seconds,
        watts: 200.0,
        intensity,
        amplitude: 1.0,
        fatigue: 0.0,
        real: true,
    }
}

/// Flatten a sampled graph into sorted dotted-path leaves, mirroring the
/// generator's `flattenNumericLeaves` naming.
#[must_use]
fn flatten_motion_graph(graph: &ReplayMotionGraph) -> Vec<(String, f64)> {
    fn channel(path: &str, c: &MotionChannel, out: &mut Vec<(String, f64)>) {
        out.push((format!("{path}.value"), c.value));
        out.push((format!("{path}.velocity"), c.velocity));
        out.push((format!("{path}.acceleration"), c.acceleration));
    }
    fn timing(t: &MotionTiming, out: &mut Vec<(String, f64)>) {
        out.push(("timing.cycle".into(), t.cycle));
        out.push(("timing.cycleIndex".into(), t.cycle_index as f64));
        out.push(("timing.driveFraction".into(), t.drive_fraction));
        out.push(("timing.driveProgress".into(), t.drive_progress));
        out.push(("timing.phase".into(), t.phase));
        out.push(("timing.phaseAcceleration".into(), t.phase_acceleration));
        out.push(("timing.phaseVelocity".into(), t.phase_velocity));
        out.push(("timing.recoveryProgress".into(), t.recovery_progress));
        out.push(("timing.secondsPerCycle".into(), t.seconds_per_cycle));
    }
    fn circular(path: &str, c: &CircularMotion, out: &mut Vec<(String, f64)>) {
        out.push((format!("{path}.angle"), c.angle));
        out.push((format!("{path}.sin"), c.sin));
        out.push((format!("{path}.cos"), c.cos));
        out.push((format!("{path}.angularVelocity"), c.angular_velocity));
        out.push((
            format!("{path}.angularAcceleration"),
            c.angular_acceleration,
        ));
    }
    fn pedal(path: &str, p: &PedalMotion, out: &mut Vec<(String, f64)>) {
        circular(&format!("{path}.rotation"), &p.rotation, out);
        channel(&format!("{path}.ankleFlex"), &p.ankle_flex, out);
        channel(&format!("{path}.drive"), &p.drive, out);
        channel(&format!("{path}.kneeLift"), &p.knee_lift, out);
        channel(&format!("{path}.legExtension"), &p.leg_extension, out);
        channel(&format!("{path}.pedalLock"), &p.pedal_lock, out);
    }

    let mut out = Vec::new();
    match graph {
        ReplayMotionGraph::Rower(g) => {
            timing(&g.timing, &mut out);
            channel("body.armDraw", &g.body.arm_draw, &mut out);
            channel("body.handleTravel", &g.body.handle_travel, &mut out);
            channel("body.headBob", &g.body.head_bob, &mut out);
            channel("body.legExtension", &g.body.leg_extension, &mut out);
            channel("body.pelvisTravel", &g.body.pelvis_travel, &mut out);
            channel("body.seatTravel", &g.body.seat_travel, &mut out);
            channel("body.shoulderSet", &g.body.shoulder_set, &mut out);
            channel("body.spineHinge", &g.body.spine_hinge, &mut out);
            channel("body.torsoReach", &g.body.torso_reach, &mut out);
            channel("body.torsoSwing", &g.body.torso_swing, &mut out);
            channel("contacts.bladeFeather", &g.contacts.blade_feather, &mut out);
            channel("contacts.bladeWater", &g.contacts.blade_water, &mut out);
            channel("contacts.footPressure", &g.contacts.foot_pressure, &mut out);
            channel("contacts.handleGrip", &g.contacts.handle_grip, &mut out);
            channel("contacts.oarlockLoad", &g.contacts.oarlock_load, &mut out);
            channel("accents.surge", &g.accents.surge, &mut out);
            channel("accents.vertical", &g.accents.vertical, &mut out);
        }
        ReplayMotionGraph::Skierg(g) => {
            timing(&g.timing, &mut out);
            channel("body.armExtension", &g.body.arm_extension, &mut out);
            channel("body.armPress", &g.body.arm_press, &mut out);
            channel("body.elbowLoad", &g.body.elbow_load, &mut out);
            channel("body.headRise", &g.body.head_rise, &mut out);
            channel("body.hipHinge", &g.body.hip_hinge, &mut out);
            channel("body.kneeFlex", &g.body.knee_flex, &mut out);
            channel("body.pelvisHinge", &g.body.pelvis_hinge, &mut out);
            channel("body.poleFlight", &g.body.pole_flight, &mut out);
            channel("body.poleLift", &g.body.pole_lift, &mut out);
            channel("body.poleSweep", &g.body.pole_sweep, &mut out);
            channel("body.reach", &g.body.reach, &mut out);
            channel("body.shoulderDrop", &g.body.shoulder_drop, &mut out);
            channel("body.spineHinge", &g.body.spine_hinge, &mut out);
            channel("body.torsoCompression", &g.body.torso_compression, &mut out);
            channel("contacts.footPressure", &g.contacts.foot_pressure, &mut out);
            channel("contacts.poleGrip", &g.contacts.pole_grip, &mut out);
            channel("contacts.poleLoad", &g.contacts.pole_load, &mut out);
            channel("contacts.polePlant", &g.contacts.pole_plant, &mut out);
            channel("accents.rebound", &g.accents.rebound, &mut out);
            channel("accents.surge", &g.accents.surge, &mut out);
        }
        ReplayMotionGraph::Bike(g) => {
            timing(&g.timing, &mut out);
            channel(
                "body.headStabilization",
                &g.body.head_stabilization,
                &mut out,
            );
            channel("body.hipRock", &g.body.hip_rock, &mut out);
            channel("body.pelvisRock", &g.body.pelvis_rock, &mut out);
            channel(
                "body.shoulderCounterRotation",
                &g.body.shoulder_counter_rotation,
                &mut out,
            );
            channel("body.spineLean", &g.body.spine_lean, &mut out);
            channel("body.torsoSway", &g.body.torso_sway, &mut out);
            circular("crank", &g.crank, &mut out);
            pedal("leftPedal", &g.left_pedal, &mut out);
            pedal("rightPedal", &g.right_pedal, &mut out);
            channel(
                "contacts.handlebarGrip",
                &g.contacts.handlebar_grip,
                &mut out,
            );
            channel(
                "contacts.saddleContact",
                &g.contacts.saddle_contact,
                &mut out,
            );
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Field accessor for palette comparison (string fields by web name).
fn venue_field(palette: &VenuePalette, field: &str) -> Option<&'static str> {
    Some(match field {
        "skyTop" => palette.sky_top,
        "skyHorizon" => palette.sky_horizon,
        "haze" => palette.haze,
        "sun" => palette.sun,
        "ridgeFar" => palette.ridge_far,
        "ridgeNear" => palette.ridge_near,
        "foliageFar" => palette.foliage_far,
        "foliageNear" => palette.foliage_near,
        "structure" => palette.structure,
        "structureShade" => palette.structure_shade,
        "structureLight" => palette.structure_light,
        "groundTop" => palette.ground_top,
        "groundMid" => palette.ground_mid,
        "groundBottom" => palette.ground_bottom,
        "surfaceLine" => palette.surface_line,
        "surfaceHighlight" => palette.surface_highlight,
        "surfaceShadow" => palette.surface_shadow,
        "marker" => palette.marker,
        "safety" => palette.safety,
        "safetyLight" => palette.safety_light,
        _ => return None,
    })
}

// --- Phase 7: grip closure parity (handGrip.ts) ---------------------------

#[derive(Deserialize)]
struct GripsFixture {
    channel: GripChannelFixture,
    hands: GripHandsFixture,
    closures: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Clone, Copy)]
struct Xyz {
    x: f64,
    y: f64,
    z: f64,
}

impl From<Xyz> for [f64; 3] {
    fn from(value: Xyz) -> [f64; 3] {
        [value.x, value.y, value.z]
    }
}

#[derive(Deserialize)]
#[allow(clippy::struct_field_names)]
struct GripChannelFixture {
    #[serde(rename = "handCurlAxis")]
    hand_curl_axis: Xyz,
    #[serde(rename = "handFistCentre")]
    hand_fist_centre: Xyz,
    #[serde(rename = "handFistRadius")]
    hand_fist_radius: f64,
    #[serde(rename = "handFistReferenceGripRadius")]
    hand_fist_reference_grip_radius: f64,
    #[serde(rename = "handPalmContact")]
    hand_palm_contact: Xyz,
    #[serde(rename = "handPalmNormalIn")]
    hand_palm_normal_in: Xyz,
    #[serde(rename = "handGripSeatFlesh")]
    hand_grip_seat_flesh: f64,
    #[serde(rename = "handLongAxis")]
    hand_long_axis: Xyz,
}

#[derive(Deserialize)]
struct GripHandsFixture {
    left: GripHandFixture,
    right: GripHandFixture,
}

#[derive(Deserialize)]
struct GripHandFixture {
    chains: Vec<GripChainFixture>,
}

#[derive(Deserialize)]
struct GripChainFixture {
    digit: String,
    #[serde(rename = "tipLength")]
    tip_length: f64,
    #[serde(rename = "cupNode", default)]
    cup_node: Option<GripJointFixture>,
    joints: Vec<GripJointFixture>,
}

#[derive(Deserialize)]
struct GripJointFixture {
    helper: String,
    position: [f64; 3],
    quaternion: [f64; 4],
}

#[derive(Deserialize)]
struct GripClosureOptionsFixture {
    radius: f64,
    #[serde(rename = "thumbEndAxial")]
    thumb_end_axial: Option<f64>,
    #[serde(rename = "thumbOppose")]
    thumb_oppose: f64,
    #[serde(rename = "wrapFingerStages", default)]
    wrap_finger_stages: bool,
}

#[derive(Deserialize)]
struct GripClosureResultFixture {
    poses: Vec<rowplay_core::replay::hand_grip::DigitStagePose>,
    contacts: Vec<rowplay_core::replay::hand_grip::DigitContact>,
}

fn fixture_digit(name: &str) -> &'static str {
    match name {
        "index" => "index",
        "middle" => "middle",
        "ring" => "ring",
        "pinky" => "pinky",
        _ => "thumb",
    }
}

fn fixture_chains(hand: &GripHandFixture) -> Vec<rowplay_core::replay::hand_grip::HandDigitChain> {
    use rowplay_core::replay::hand_grip::{DigitJoint, HandDigitChain};
    hand.chains
        .iter()
        .map(|chain| HandDigitChain {
            digit: fixture_digit(&chain.digit),
            joints: chain
                .joints
                .iter()
                .map(|joint| DigitJoint {
                    helper: joint.helper.clone(),
                    position: joint.position,
                    quaternion: joint.quaternion,
                })
                .collect(),
            tip_length: chain.tip_length,
            cup_node: chain.cup_node.as_ref().map(|cup| DigitJoint {
                helper: cup.helper.clone(),
                position: cup.position,
                quaternion: cup.quaternion,
            }),
        })
        .collect()
}

fn assert_closure_matches(
    label: &str,
    solved: &rowplay_core::replay::hand_grip::GripClosure,
    expected: &GripClosureResultFixture,
) {
    assert_eq!(
        solved.poses.len(),
        expected.poses.len(),
        "{label}: pose count"
    );
    for (solved, expected) in solved.poses.iter().zip(&expected.poses) {
        assert_eq!(solved.helper, expected.helper, "{label}: pose helper order");
        assert!(
            (solved.flex - expected.flex).abs() < 1e-9,
            "{label}: {} flex {} vs {}",
            expected.helper,
            solved.flex,
            expected.flex
        );
        assert!(
            (solved.oppose - expected.oppose).abs() < 1e-9,
            "{label}: {} oppose {} vs {}",
            expected.helper,
            solved.oppose,
            expected.oppose
        );
    }
    assert_eq!(
        solved.contacts.len(),
        expected.contacts.len(),
        "{label}: contact count"
    );
    for (solved, expected) in solved.contacts.iter().zip(&expected.contacts) {
        assert_eq!(solved.digit, expected.digit, "{label}: contact digit");
        assert!(
            (solved.surface_distance - expected.surface_distance).abs() < 1e-9,
            "{label}: {} surface distance {} vs {}",
            expected.digit,
            solved.surface_distance,
            expected.surface_distance
        );
        assert_eq!(
            solved.contact, expected.contact,
            "{label}: {} contact flag",
            expected.digit
        );
        for axis in 0..3 {
            assert!(
                (solved.tip[axis] - expected.tip[axis]).abs() < 1e-9,
                "{label}: {} tip axis {axis}",
                expected.digit
            );
        }
    }
}

#[test]
fn grip_closure_parity() {
    let fixture: GripsFixture = load_json("replay-current-main-grips.json")
        .unwrap_or_else(|error| panic!("load grips fixture: {error}"));

    // The fixture's fitted geometry channels must equal the ported constants.
    let channel = &fixture.channel;
    let close = |a: f64, b: f64| (a - b).abs() < 1e-12;
    let curl_axis: [f64; 3] = channel.hand_curl_axis.into();
    let fist_centre: [f64; 3] = channel.hand_fist_centre.into();
    let palm_contact: [f64; 3] = channel.hand_palm_contact.into();
    let palm_normal_in: [f64; 3] = channel.hand_palm_normal_in.into();
    let long_axis: [f64; 3] = channel.hand_long_axis.into();
    for axis in 0..3 {
        assert!(close(
            curl_axis[axis],
            rowplay_core::replay::hand_grip::HAND_CURL_AXIS[axis]
        ));
        assert!(close(
            fist_centre[axis],
            rowplay_core::replay::hand_grip::HAND_FIST_CENTRE[axis]
        ));
        assert!(close(
            palm_contact[axis],
            rowplay_core::replay::hand_grip::HAND_PALM_CONTACT[axis]
        ));
        assert!(close(
            palm_normal_in[axis],
            rowplay_core::replay::hand_grip::hand_palm_normal_in()[axis]
        ));
        assert!(close(
            long_axis[axis],
            rowplay_core::replay::hand_grip::HAND_LONG_AXIS[axis]
        ));
    }
    assert!(close(
        channel.hand_fist_radius,
        rowplay_core::replay::hand_grip::HAND_FIST_RADIUS
    ));
    assert!(close(
        channel.hand_fist_reference_grip_radius,
        rowplay_core::replay::hand_grip::HAND_FIST_REFERENCE_GRIP_RADIUS
    ));
    assert!(close(
        channel.hand_grip_seat_flesh,
        rowplay_core::replay::hand_grip::hand_grip_seat_flesh()
    ));

    // Per sport: the fixture's options feed the solver; the solved poses and
    // contact reports must match the fixture within 1e-9.
    for (sport, entry) in &fixture.closures {
        let options: GripClosureOptionsFixture = serde_json::from_value(entry["options"].clone())
            .unwrap_or_else(|error| panic!("{sport}: options: {error}"));
        let closure_options = rowplay_core::replay::hand_grip::ClosureOptions {
            side: -1.0,
            surface: rowplay_core::replay::hand_grip::GripSurface {
                radius: options.radius,
                thumb_end_axial: options.thumb_end_axial,
            },
            thumb_oppose: options.thumb_oppose,
            finger_flesh: None,
            thumb_flesh: None,
            wrap_finger_stages: options.wrap_finger_stages,
        };
        for (side_name, side) in [("left", -1.0), ("right", 1.0)] {
            let expected: GripClosureResultFixture =
                serde_json::from_value(entry[side_name].clone())
                    .unwrap_or_else(|error| panic!("{sport}/{side_name}: closure: {error}"));
            let mut closure_options = closure_options.clone();
            closure_options.side = side;
            let chains = fixture_chains(if side < 0.0 {
                &fixture.hands.left
            } else {
                &fixture.hands.right
            });
            let solved =
                rowplay_core::replay::hand_grip::solve_hand_grip_closure(&chains, &closure_options);
            assert_closure_matches(&format!("{sport}/{side_name}"), &solved, &expected);
        }
    }
}

#[test]
#[ignore = "Phase 7 (motion): wrist budgets and equipment projections (replay-current-main-equipment.json)"]
fn equipment_contact_parity() {
    unreachable!("enable with orientHandToGripChannel + constrainWristFrame");
}
