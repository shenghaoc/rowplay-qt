// SPDX-License-Identifier: GPL-3.0-or-later
//! Stroke-analysis view model: the downsampled chart series behind the
//! detail screen's "Split Focus" panel.
//!
//! A port of Studio's `WorkoutStrokeAnalysisView` statics:
//! `downsampleStrokes` (bounded 500-point sample keeping both endpoints),
//! `computePaceChartDomain` (negated pace, 12% padding with a 3 s floor and
//! the `-180…-60` placeholder) and `computeSplitBoundaryDistances`
//! (cumulative split distances excluding the last). Missing strokes are
//! Studio's empty state; synthesised strokes (mapper fallback that clears
//! `has_stroke_data`) are charted exactly like recorded ones — the web
//! replay does the same.
//!
//! Series are flat `[x0, y0, x1, y1, …]` vectors so the bridge carries one
//! `Vec<f64>` per line and Qt Graphs loads it with a single `replace`.

use rowplay_core::analytics::{StrokeSummary, stroke_summary};
use rowplay_core::formatting::fmt_pace;
use rowplay_core::models::{DistanceUnit, Split, Stroke};

/// Studio's chart sample bound.
pub const STROKE_CHART_LIMIT: usize = 500;

/// Metres per mile, shared with the dashboard charts.
pub use crate::dashboard::{METRES_PER_MILE, distance_axis_label};

/// Studio's `chartDistance`: kilometres or miles.
#[must_use]
pub fn chart_distance(metres: f64, unit: DistanceUnit) -> f64 {
    match unit {
        DistanceUnit::Metric => metres / 1_000.0,
        DistanceUnit::Imperial => metres / METRES_PER_MILE,
    }
}

/// Studio's `downsampleStrokes`: an even index sweep that retains both
/// endpoints, returning the sampled strokes.
#[must_use]
pub fn downsample_strokes(strokes: &[Stroke], limit: usize) -> Vec<Stroke> {
    if limit == 0 {
        return Vec::new();
    }
    if strokes.len() <= limit {
        return strokes.to_vec();
    }
    if limit == 1 {
        return vec![strokes[0]];
    }
    let last_index = strokes.len() - 1;
    (0..limit)
        .map(|sample_index| {
            let stroke_index = sample_index * last_index / (limit - 1);
            strokes[stroke_index]
        })
        .collect()
}

/// Studio's `computePaceChartDomain`: negated pace bounds with 12% padding
/// (minimum 3 s); `(-180, -60)` when no stroke has a usable pace.
#[must_use]
pub fn pace_chart_domain(strokes: &[Stroke]) -> (f64, f64) {
    let mut fastest = f64::INFINITY;
    let mut slowest = f64::NEG_INFINITY;
    let mut has_valid_pace = false;
    for stroke in strokes {
        let pace = stroke.pace;
        if !pace.is_finite() || pace <= 0.0 {
            continue;
        }
        has_valid_pace = true;
        fastest = fastest.min(pace);
        slowest = slowest.max(pace);
    }
    if !has_valid_pace {
        return (-180.0, -60.0);
    }
    let padding = ((slowest - fastest) * 0.12).max(3.0);
    (-(slowest + padding), -(fastest - padding))
}

/// Studio's `computeSplitBoundaryDistances`: cumulative distances of every
/// split except the last, in chart units; empty for a single split.
#[must_use]
pub fn split_boundary_distances(splits: &[Split], unit: DistanceUnit) -> Vec<f64> {
    if splits.len() <= 1 {
        return Vec::new();
    }
    let mut cumulative = 0.0;
    splits[..splits.len() - 1]
        .iter()
        .map(|split| {
            cumulative += split.distance;
            chart_distance(cumulative, unit)
        })
        .collect()
}

/// Flat `[x, y, …]` pairs of negated pace over chart distance (the line is
/// drawn fast-up, like Studio).
#[must_use]
pub fn pace_series(strokes: &[Stroke], unit: DistanceUnit) -> Vec<f64> {
    let mut series = Vec::with_capacity(strokes.len() * 2);
    for stroke in strokes {
        series.push(chart_distance(stroke.d, unit));
        series.push(-stroke.pace);
    }
    series
}

/// Flat `[x, y, …]` pairs of watts over chart distance.
#[must_use]
pub fn power_series(strokes: &[Stroke], unit: DistanceUnit) -> Vec<f64> {
    let mut series = Vec::with_capacity(strokes.len() * 2);
    for stroke in strokes {
        series.push(chart_distance(stroke.d, unit));
        series.push(stroke.watts);
    }
    series
}

/// Flat `[x, y, …]` pairs of stroke rate over chart distance.
#[must_use]
pub fn rate_series(strokes: &[Stroke], unit: DistanceUnit) -> Vec<f64> {
    let mut series = Vec::with_capacity(strokes.len() * 2);
    for stroke in strokes {
        series.push(chart_distance(stroke.d, unit));
        series.push(stroke.spm);
    }
    series
}

/// Flat `[x, y, …]` pairs of heart rate over chart distance; strokes without
/// HR break the line into segments (callers get one series per contiguous
/// run, so Qt Graphs never plots a fake 0 bpm dip).
#[must_use]
pub fn hr_series_segments(strokes: &[Stroke], unit: DistanceUnit) -> Vec<Vec<f64>> {
    let mut segments: Vec<Vec<f64>> = Vec::new();
    let mut current: Vec<f64> = Vec::new();
    for stroke in strokes {
        match stroke.hr {
            Some(hr) if hr.is_finite() && hr > 0.0 => {
                current.push(chart_distance(stroke.d, unit));
                current.push(hr);
            }
            _ => {
                if current.len() >= 4 {
                    segments.push(std::mem::take(&mut current));
                } else {
                    current.clear();
                }
            }
        }
    }
    if current.len() >= 4 {
        segments.push(current);
    }
    segments
}

/// Largest finite y in a flat `[x, y, …]` series (0 when empty); used to
/// size the single-scale stroke charts.
#[must_use]
pub fn series_max(series: &[f64]) -> f64 {
    let mut max = 0.0_f64;
    for value in series.iter().skip(1).step_by(2) {
        if value.is_finite() && *value > max {
            max = *value;
        }
    }
    max
}

/// The rendered "Split Focus" summary values (QML composes the labels from
/// locale ids; numbers and unit symbols come from here).
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeOverview {
    /// True when the workout has chartable strokes at all.
    pub has_strokes: bool,
    /// Raw stroke count (Studio's accessibility value).
    pub count: i64,
    /// Split count for the subtitle ("{n} splits and finishing effort").
    pub split_count: i64,
    /// Mean pace, seconds/500 m (numeric, for the rule line).
    pub average_pace: f64,
    /// Mean pace over the strokes (`fmt_pace`).
    pub average_pace_text: String,
    /// Peak watts (numeric, for chart scaling).
    pub peak_watts: f64,
    /// Mean watts (Studio's average-watts rule line).
    pub average_watts: f64,
    /// Mean watts, rounded.
    pub average_watts_text: String,
    /// Peak watts, rounded.
    pub peak_watts_text: String,
}

/// Summary numbers for the panel header and screen readers.
#[must_use]
pub fn overview(detail_strokes: &[Stroke], splits: &[Split]) -> StrokeOverview {
    let summary: StrokeSummary = stroke_summary(detail_strokes);
    StrokeOverview {
        has_strokes: !detail_strokes.is_empty(),
        count: summary.count as i64,
        split_count: splits.len() as i64,
        average_pace: summary.average_pace,
        average_pace_text: fmt_pace(summary.average_pace),
        average_watts: summary.average_watts,
        average_watts_text: summary.average_watts.round().to_string(),
        peak_watts: summary.peak_watts,
        peak_watts_text: summary.peak_watts.round().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use rowplay_core::demo::{demo_details, mock_workout_detail};

    use super::*;

    fn stroke(t: f64, d: f64, pace: f64) -> Stroke {
        Stroke::new(t, d, pace, 24.0, 200.0)
    }

    /// Studio's downsampling contract: bounded, endpoints retained.
    #[test]
    fn downsample_keeps_endpoints_and_bound() {
        let strokes: Vec<Stroke> = (0..2000)
            .map(|i| stroke(f64::from(i), f64::from(i) * 5.0, 120.0))
            .collect();
        let sampled = downsample_strokes(&strokes, STROKE_CHART_LIMIT);
        assert_eq!(sampled.len(), STROKE_CHART_LIMIT);
        assert_eq!(sampled.first().unwrap().t, 0.0);
        assert_eq!(sampled.last().unwrap().t, 1999.0);

        // Below the limit: untouched.
        let short: Vec<Stroke> = (0..10)
            .map(|i| stroke(f64::from(i), f64::from(i), 120.0))
            .collect();
        assert_eq!(downsample_strokes(&short, STROKE_CHART_LIMIT).len(), 10);
        // Degenerate limits match Studio.
        assert!(downsample_strokes(&short, 0).is_empty());
        assert_eq!(downsample_strokes(&short, 1).len(), 1);
    }

    /// Studio's domain statics, re-expressed (12% padding, 3 s floor,
    /// placeholder domain).
    #[test]
    fn pace_domain_pads_and_falls_back() {
        let strokes = vec![stroke(0.0, 0.0, 120.0), stroke(10.0, 50.0, 130.0)];
        let (low, high) = pace_chart_domain(&strokes);
        assert!((low - (-133.0)).abs() < 1e-9);
        assert!((high - (-117.0)).abs() < 1e-9);
        assert_eq!(pace_chart_domain(&[]), (-180.0, -60.0));
        // Zero/negative/non-finite paces are skipped.
        let junk = vec![stroke(0.0, 0.0, 0.0), stroke(1.0, 1.0, f64::NAN)];
        assert_eq!(pace_chart_domain(&junk), (-180.0, -60.0));
    }

    /// The detail pace chart's ticks (`Detail.paceAxisValues/Labels`): the
    /// negated, fast-up stroke domain maps onto formatted paces spanning it
    /// end to end — never the raw negative axis numbers the chart printed
    /// before — with the slowest pace on the bottom tick.
    #[test]
    fn pace_axis_ticks_over_the_stroke_domain_read_as_paces() {
        let strokes = vec![
            stroke(0.0, 0.0, 118.0),
            stroke(10.0, 50.0, 125.0),
            stroke(20.0, 100.0, 121.0),
        ];
        let domain = pace_chart_domain(&strokes);
        let ticks = crate::dashboard::pace_axis_labels(domain, 4);
        assert_eq!(ticks.len(), 4);
        assert_eq!(ticks[0].0, domain.0);
        assert_eq!(ticks[3].0, domain.1);
        for (value, label) in &ticks {
            assert!(*value < 0.0, "axis values stay negated (fast is up)");
            assert_eq!(*label, fmt_pace(-value));
            assert!(!label.starts_with('-'), "{label} reads as a pace");
        }
        assert!(-ticks[0].0 > -ticks[3].0, "the bottom tick is the slowest");
        // The stroke-less placeholder domain still yields pace labels.
        let empty = crate::dashboard::pace_axis_labels(pace_chart_domain(&[]), 4);
        assert_eq!(
            empty.first().map(|(_, label)| label.as_str()),
            Some("3:00.0/500m")
        );
    }

    #[test]
    fn split_boundaries_accumulate_except_the_last() {
        let splits = vec![
            Split::new(0, 500.0, 120.0, 120.0),
            Split::new(1, 500.0, 121.0, 121.0),
            Split::new(2, 1000.0, 122.0, 122.0),
        ];
        let metric = split_boundary_distances(&splits, DistanceUnit::Metric);
        assert_eq!(metric.len(), 2);
        assert!((metric[0] - 0.5).abs() < 1e-9);
        assert!((metric[1] - 1.0).abs() < 1e-9);
        let imperial = split_boundary_distances(&splits, DistanceUnit::Imperial);
        assert!((imperial[0] - 500.0 / METRES_PER_MILE).abs() < 1e-9);
        // A single split has no interior boundaries.
        assert!(split_boundary_distances(&splits[..1], DistanceUnit::Metric).is_empty());
    }

    #[test]
    fn flat_series_interleave_axes() {
        let strokes = vec![stroke(0.0, 0.0, 120.0), stroke(10.0, 500.0, 130.0)];
        let pace = pace_series(&strokes, DistanceUnit::Metric);
        assert_eq!(pace, vec![0.0, -120.0, 0.5, -130.0]);
        let power = power_series(&strokes, DistanceUnit::Metric);
        assert_eq!(power, vec![0.0, 200.0, 0.5, 200.0]);
        let rate = rate_series(&strokes, DistanceUnit::Metric);
        assert_eq!(rate, vec![0.0, 24.0, 0.5, 24.0]);
    }

    #[test]
    fn hr_segments_break_on_missing_samples() {
        let mut strokes = vec![
            stroke(0.0, 0.0, 120.0),
            stroke(5.0, 25.0, 120.0),
            stroke(10.0, 50.0, 120.0),
        ];
        strokes[0].hr = Some(140.0);
        strokes[1].hr = Some(145.0);
        strokes[2].hr = Some(150.0);
        let mut gap = stroke(15.0, 75.0, 120.0); // no HR
        gap.hr = None;
        strokes.push(gap);
        let mut after = stroke(20.0, 100.0, 120.0);
        after.hr = Some(152.0);
        strokes.push(after);
        let mut after2 = stroke(25.0, 125.0, 120.0);
        after2.hr = Some(154.0);
        strokes.push(after2);

        let segments = hr_series_segments(&strokes, DistanceUnit::Metric);
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0], vec![0.0, 140.0, 0.025, 145.0, 0.05, 150.0]);
        assert_eq!(segments[1], vec![0.1, 152.0, 0.125, 154.0]);
        // Without any HR: nothing to draw.
        assert!(hr_series_segments(&[stroke(0.0, 0.0, 120.0)], DistanceUnit::Metric).is_empty());
    }

    #[test]
    fn series_max_scans_the_y_lane() {
        assert_eq!(series_max(&[]), 0.0);
        assert_eq!(series_max(&[0.0, 120.0, 1.0, 350.5, 2.0, 90.0]), 350.5);
        assert_eq!(series_max(&[0.0, f64::NAN, 1.0, -5.0]), 0.0);
    }

    #[test]
    fn overview_handles_stroke_less_and_synthesised_workouts() {
        // Stroke-less demo piece: Studio's "No Stroke Detail" empty state.
        let strokeless = demo_details()
            .into_iter()
            .find(|d| d.strokes.is_empty())
            .expect("demo piece without strokes");
        let empty = overview(&strokeless.strokes, &strokeless.splits);
        assert!(!empty.has_strokes);
        assert_eq!(empty.count, 0);

        // A normal piece: summary numbers rendered, never formatted in QML.
        let detail = mock_workout_detail(1001).expect("demo 1001");
        let summary = overview(&detail.strokes, &detail.splits);
        assert!(summary.has_strokes);
        assert!(summary.count > 0);
        assert!(summary.average_watts > 0.0);
        assert!(summary.average_pace_text.contains(':'));
        assert!(
            summary
                .average_watts_text
                .chars()
                .all(|c| c.is_ascii_digit())
        );
        assert_eq!(summary.split_count, detail.splits.len() as i64);
        assert_eq!(distance_axis_label(DistanceUnit::Metric), "km");
    }
}
