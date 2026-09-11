// SPDX-License-Identifier: GPL-3.0-or-later
//! Golden parity tests against the fixtures vendored from rowplay-studio
//! (`tests/fixtures/`, see `PROVENANCE.md`). Each test states its tolerance.
//! Fixtures for later phases are loaded and shape-checked now; their
//! behavioural assertions are `#[ignore]`d and name the phase that enables them.

use std::collections::BTreeMap;

use rowplay_core::analytics::duration_band;
use rowplay_core::performance_predictor::{
    PredictionStatus, build_prediction_table, predict_times,
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
#[ignore = "Phase 3 (platform): Concept2 mapper (tenths, decimetres, BikeErg pace divisor, interval offsets)"]
fn concept2_mapper_matches_fixture_expectations() {
    unreachable!("enable once rowplay_platform::concept2 maps raw results and strokes");
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

#[test]
#[ignore = "Phase 2 (replay core): strokePoseAt parity (stroke-pose-parity.json)"]
fn stroke_pose_parity() {
    unreachable!("enable with rowplay_core::replay::stroke_pose");
}

#[test]
#[ignore = "Phase 2 (replay core): raceGapMeters/raceGapSeconds/ghost sampling parity (replay-race-gap-parity.json)"]
fn race_gap_parity() {
    unreachable!("enable with rowplay_core::replay::race_gap");
}

#[test]
#[ignore = "Phase 2 (replay core): finished-race outcome parity (replay-race-result-parity.json)"]
fn race_result_parity() {
    unreachable!("enable with rowplay_core::replay::race_result");
}

#[test]
#[ignore = "Phase 2 (replay core): constant-pace and CSV/TCX/FIT rival parsers (replay-rival-sources-parity.json)"]
fn rival_sources_parity() {
    unreachable!("enable with rowplay_core::replay::rivals");
}

#[test]
#[ignore = "Phase 2 (replay core): motion graph channel parity (replay-current-main-motion.json, replay-current-main-2d.json)"]
fn motion_graph_parity() {
    unreachable!("enable with rowplay_core::replay::motion_graph");
}

#[test]
#[ignore = "Phase 7 (motion) / Phase 5 (3D replay): V4 grip and equipment contact parity (replay-current-main-grips.json, replay-current-main-equipment.json)"]
fn grip_and_equipment_parity() {
    unreachable!("enable with the V4 athlete contact solver");
}
