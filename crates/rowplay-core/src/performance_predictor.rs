// SPDX-License-Identifier: GPL-3.0-or-later
//! Paul's Law distance prediction engine.
//!
//! Port of the web app's `src/lib/performancePredictor.ts`. Paul's Law is the
//! Concept2 community standard for estimating race times across distances:
//! `time₂ = time₁ × (distance₂ / distance₁)^1.06`.

use std::collections::BTreeMap;

/// Paul's Law exponent (Concept2 community standard).
pub const PAUL_EXPONENT: f64 = 1.06;

/// Standard Concept2 race distances in metres.
pub const PREDICTOR_DISTANCES: [u32; 7] = [500, 1000, 2000, 5000, 6000, 10000, 21097];

/// How a personal best compares with the prediction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PredictionStatus {
    /// The athlete's PB is faster than the prediction.
    Beaten,
    /// The athlete's PB is equal to or slower than the prediction.
    Behind,
    /// No PB at this distance.
    Untried,
}

/// One row of the prediction table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PredictionRow {
    /// Standard distance in metres.
    pub distance: u32,
    /// Predicted seconds.
    pub predicted_seconds: f64,
    /// Fastest known time at this distance, if any.
    pub actual_best_seconds: Option<f64>,
    /// Comparison status.
    pub status: PredictionStatus,
}

/// Apply Paul's Law from one known (distance, time) pair.
///
/// Returns predicted seconds for each standard distance in ascending order; the
/// source distance maps to `known_seconds` exactly. Non-positive inputs yield
/// an empty map.
#[must_use]
pub fn predict_times(known_distance: f64, known_seconds: f64) -> BTreeMap<u32, f64> {
    let mut out = BTreeMap::new();
    if known_distance <= 0.0 || known_seconds <= 0.0 {
        return out;
    }
    for d in PREDICTOR_DISTANCES {
        let predicted = if f64::from(d) == known_distance {
            known_seconds
        } else {
            known_seconds * (f64::from(d) / known_distance).powf(PAUL_EXPONENT)
        };
        out.insert(d, predicted);
    }
    out
}

fn classify_status(predicted_seconds: f64, actual_best_seconds: Option<f64>) -> PredictionStatus {
    match actual_best_seconds {
        None => PredictionStatus::Untried,
        Some(actual) if actual < predicted_seconds => PredictionStatus::Beaten,
        Some(_) => PredictionStatus::Behind,
    }
}

/// Build the full prediction table with status by comparing predictions against
/// the athlete's personal bests (`(distance, seconds)` pairs; the fastest time
/// per distance wins).
///
/// Non-positive inputs yield an empty table (rowplay-studio behaviour; the web
/// app would return rows with undefined predictions).
#[must_use]
pub fn build_prediction_table(
    known_distance: f64,
    known_seconds: f64,
    personal_bests: &[(f64, f64)],
) -> Vec<PredictionRow> {
    let predicted = predict_times(known_distance, known_seconds);
    if predicted.is_empty() {
        return Vec::new();
    }
    let mut pb_by_dist: Vec<(f64, f64)> = Vec::new();
    for &(distance, time) in personal_bests {
        match pb_by_dist.iter_mut().find(|(d, _)| *d == distance) {
            Some((_, current)) => {
                if time < *current {
                    *current = time;
                }
            }
            None => pb_by_dist.push((distance, time)),
        }
    }
    PREDICTOR_DISTANCES
        .into_iter()
        .map(|distance| {
            let predicted_seconds = predicted[&distance];
            let actual_best_seconds = pb_by_dist
                .iter()
                .find(|(d, _)| *d == f64::from(distance))
                .map(|(_, time)| *time);
            PredictionRow {
                distance,
                predicted_seconds,
                actual_best_seconds,
                status: classify_status(predicted_seconds, actual_best_seconds),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- web performancePredictor.test.ts -----------------------------------

    #[test]
    fn keeps_the_source_distance_at_the_entered_time() {
        assert_eq!(predict_times(2000.0, 424.0)[&2000], 424.0);
    }

    #[test]
    fn predicts_6k_from_a_7_04_2k_within_a_second() {
        let two_k = 7.0 * 60.0 + 4.0;
        let expected = two_k * (6000.0_f64 / 2000.0).powf(PAUL_EXPONENT);
        let predicted = predict_times(2000.0, two_k)[&6000];
        assert!((predicted - expected).abs() <= 1.0);
    }

    #[test]
    fn returns_every_standard_distance() {
        let map = predict_times(1000.0, 180.0);
        assert_eq!(
            map.keys().copied().collect::<Vec<_>>(),
            PREDICTOR_DISTANCES.to_vec()
        );
        assert!(predict_times(0.0, 420.0).is_empty());
        assert!(predict_times(2000.0, 0.0).is_empty());
    }

    #[test]
    fn prediction_table_statuses() {
        let rows = build_prediction_table(2000.0, 420.0, &[]);
        assert_eq!(rows.len(), 7);
        assert!(
            rows.iter()
                .all(|r| r.status == PredictionStatus::Untried && r.actual_best_seconds.is_none())
        );

        let rows = build_prediction_table(2000.0, 500.0, &[(5000.0, 1000.0)]);
        let five_k = rows.iter().find(|r| r.distance == 5000).unwrap();
        assert_eq!(five_k.status, PredictionStatus::Beaten);
        assert_eq!(five_k.actual_best_seconds, Some(1000.0));
        assert!(five_k.predicted_seconds > 1000.0);

        let rows = build_prediction_table(2000.0, 400.0, &[(5000.0, 9999.0)]);
        assert_eq!(
            rows.iter().find(|r| r.distance == 5000).unwrap().status,
            PredictionStatus::Behind
        );

        let predicted = predict_times(2000.0, 420.0)[&5000];
        let rows = build_prediction_table(2000.0, 420.0, &[(5000.0, predicted)]);
        assert_eq!(
            rows.iter().find(|r| r.distance == 5000).unwrap().status,
            PredictionStatus::Behind
        );

        let rows = build_prediction_table(2000.0, 500.0, &[(1000.0, 200.0), (1000.0, 180.0)]);
        assert_eq!(
            rows.iter()
                .find(|r| r.distance == 1000)
                .unwrap()
                .actual_best_seconds,
            Some(180.0)
        );
    }

    // --- Studio PerformancePredictorTests -----------------------------------

    #[test]
    fn pauls_law_formula_and_ordering() {
        let predictions = predict_times(2000.0, 420.0);
        assert_eq!(predictions.len(), 7);
        assert!((predictions[&2000] - 420.0).abs() < 0.01);
        assert!(predictions[&500] < predictions[&2000]);
        assert!(predictions[&5000] > predictions[&2000]);
        let expected_500 = 420.0 * (500.0_f64 / 2000.0).powf(1.06);
        assert!((predictions[&500] - expected_500).abs() < 0.01);
    }

    #[test]
    fn table_marks_beaten_behind_untried_and_picks_fastest_pb() {
        let table = build_prediction_table(2000.0, 420.0, &[(500.0, 80.0)]);
        assert_eq!(
            table.iter().find(|r| r.distance == 500).unwrap().status,
            PredictionStatus::Beaten
        );
        let table = build_prediction_table(2000.0, 420.0, &[(500.0, 110.0)]);
        assert_eq!(
            table.iter().find(|r| r.distance == 500).unwrap().status,
            PredictionStatus::Behind
        );
        let table = build_prediction_table(2000.0, 420.0, &[(500.0, 100.0), (500.0, 85.0)]);
        assert_eq!(
            table
                .iter()
                .find(|r| r.distance == 500)
                .unwrap()
                .actual_best_seconds,
            Some(85.0)
        );
        let row_2k = table.iter().find(|r| r.distance == 2000).unwrap();
        assert!((row_2k.predicted_seconds - 420.0).abs() < 0.01);
        assert!(build_prediction_table(0.0, 420.0, &[(500.0, 85.0)]).is_empty());
        assert!(build_prediction_table(2000.0, 0.0, &[(500.0, 85.0)]).is_empty());
    }

    #[test]
    fn status_serialises_in_lowercase() {
        assert_eq!(
            serde_json::to_string(&PredictionStatus::Beaten).unwrap(),
            "\"beaten\""
        );
        assert_eq!(
            serde_json::from_str::<PredictionStatus>("\"untried\"").unwrap(),
            PredictionStatus::Untried
        );
    }
}
