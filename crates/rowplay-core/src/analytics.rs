// SPDX-License-Identifier: GPL-3.0-or-later
//! Pure analysis helpers: trend fits, like-for-like bands and per-sport summaries.
//!
//! Port of the Phase 1 subset of the web app's `src/lib/analytics.ts`
//! (`linearTrend`, `distanceBand`, `durationBand`, `summariseBySport`) plus the
//! dashboard derivations rowplay-studio added in `WorkoutAnalytics.swift`
//! (`strokeSummary`, `dashboardSummary`, `dashboardPersonalBests`,
//! `recentPaceWorkouts`). Everything else in `analytics.ts` (HR zones,
//! critical power, training load, calendars, …) is later-phase work.

use std::collections::BTreeSet;

use crate::formatting::challenge_distance_metres;
use crate::models::{Sport, Stroke, Workout};
use crate::personal_bests::standard_distance_matching;

/// Milliseconds per day, used to express trend slopes per day.
const MS_PER_DAY: f64 = 86_400_000.0;

/// Ordinary least-squares fit of a metric against time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrendFit {
    /// Slope in y-units per day.
    pub slope_per_day: f64,
    /// Fitted y at the first x.
    pub y0: f64,
    /// Fitted y at the last x.
    pub y1: f64,
    /// Total change implied by the fit across the whole span.
    pub delta: f64,
    /// Number of points.
    pub n: usize,
}

/// Ordinary least-squares fit of `points` (x = epoch ms, y = metric).
///
/// Returns `None` with fewer than two points or when every point falls on the
/// same day (zero variance in x).
#[must_use]
pub fn linear_trend(points: &[(f64, f64)]) -> Option<TrendFit> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let mut x_min = points[0].0;
    for point in &points[1..] {
        if point.0 < x_min {
            x_min = point.0;
        }
    }
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut x_last = f64::NEG_INFINITY;
    for point in points {
        let x_days = (point.0 - x_min) / MS_PER_DAY;
        sum_x += x_days;
        sum_y += point.1;
        if x_days > x_last {
            x_last = x_days;
        }
    }
    let count = n as f64;
    let mean_x = sum_x / count;
    let mean_y = sum_y / count;
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for point in points {
        let x_days = (point.0 - x_min) / MS_PER_DAY;
        numerator += (x_days - mean_x) * (point.1 - mean_y);
        denominator += (x_days - mean_x) * (x_days - mean_x);
    }
    if denominator == 0.0 {
        return None;
    }
    let slope = numerator / denominator;
    let intercept = mean_y - slope * mean_x;
    let y0 = intercept;
    let y1 = intercept + slope * x_last;
    Some(TrendFit {
        slope_per_day: slope,
        y0,
        y1,
        delta: y1 - y0,
        n,
    })
}

/// A like-for-like distance bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct DistanceBand {
    /// Stable key, e.g. `"2000"` or `"r3000"`.
    pub key: String,
    /// Display label, e.g. `"2k"` or `"3k–7k"`.
    pub label: String,
    /// Nominal distance in metres, for sorting.
    pub nominal: f64,
}

/// Bucket a workout distance so 2k compares with 2k rather than with a 5k.
///
/// Standard erg distances get a tight ±6% window; anything else falls into a
/// coarse range band. Mirrors the web app, including the lowest range band's
/// nominal of 375 (the web fixed `(0 + min(750, 0)) / 2 = 0` upstream in
/// rowplay#202 — the port adopted the fix when the reference pin moved past
/// it; `docs/source-map.md` records the history).
#[must_use]
pub fn distance_band(metres: f64) -> DistanceBand {
    const STANDARDS: [(f64, &str); 9] = [
        (100.0, "100m"),
        (500.0, "500m"),
        (1000.0, "1k"),
        (2000.0, "2k"),
        (5000.0, "5k"),
        (6000.0, "6k"),
        (10000.0, "10k"),
        (21097.0, "Half"),
        (42195.0, "Full"),
    ];
    for (d, label) in STANDARDS {
        if (metres - d).abs() <= d * 0.06 {
            return DistanceBand {
                key: format!("{}", d as i64),
                label: label.to_owned(),
                nominal: d,
            };
        }
    }
    const RANGES: [(f64, f64, &str); 6] = [
        (0.0, 750.0, "<750m"),
        (750.0, 1500.0, "750m–1.5k"),
        (1500.0, 3000.0, "1.5k–3k"),
        (3000.0, 7000.0, "3k–7k"),
        (7000.0, 15000.0, "7k–15k"),
        (15000.0, f64::INFINITY, "15k+"),
    ];
    for (lo, hi, label) in RANGES {
        if metres >= lo && metres < hi {
            // Web (rowplay#202): for the lowest band `lo == 0` so
            // `min(hi, lo * 2)` would collapse the nominal to 0 — use `hi`
            // as the upper bound in that case (nominal 375 for `<750m`).
            let upper = if lo == 0.0 { hi } else { hi.min(lo * 2.0) };
            return DistanceBand {
                key: format!("r{}", lo as i64),
                label: label.to_owned(),
                nominal: (lo + upper) / 2.0,
            };
        }
    }
    DistanceBand {
        key: "other".to_owned(),
        label: "Other".to_owned(),
        nominal: metres,
    }
}

/// A like-for-like duration bucket.
#[derive(Debug, Clone, PartialEq)]
pub struct DurationBand {
    /// Stable key, e.g. `"1800"` or `"r900"`.
    pub key: String,
    /// Display label, e.g. `"30 min"` or `"15–40m"`.
    pub label: String,
    /// Nominal duration in seconds, for sorting.
    pub nominal: f64,
}

/// Bucket a workout duration so 30min-vs-30min compares correctly and a 20 min
/// piece is not raced against a 60 min piece. Mirrors [`distance_band`] for
/// fixed-time pieces (standard targets snap within ±10%).
#[must_use]
pub fn duration_band(seconds: f64) -> DurationBand {
    const STANDARDS: [(f64, &str); 5] = [
        (60.0, "1 min"),
        (240.0, "4 min"),
        (1200.0, "20 min"),
        (1800.0, "30 min"),
        (3600.0, "60 min"),
    ];
    for (s, label) in STANDARDS {
        if (seconds - s).abs() <= s * 0.1 {
            return DurationBand {
                key: format!("{}", s as i64),
                label: label.to_owned(),
                nominal: s,
            };
        }
    }
    const RANGES: [(f64, f64, &str); 6] = [
        (0.0, 90.0, "<90s"),
        (90.0, 360.0, "90s–6m"),
        (360.0, 900.0, "6–15m"),
        (900.0, 2400.0, "15–40m"),
        (2400.0, 4800.0, "40–80m"),
        (4800.0, f64::INFINITY, "80m+"),
    ];
    for (lo, hi, label) in RANGES {
        if seconds >= lo && seconds < hi {
            // For the lowest band lo == 0, so min(hi, lo * 2) would collapse the
            // nominal to 0 — use hi as the upper bound in that case (web fix).
            let upper = if lo == 0.0 { hi } else { hi.min(lo * 2.0) };
            return DurationBand {
                key: format!("r{}", lo as i64),
                label: label.to_owned(),
                nominal: (lo + upper) / 2.0,
            };
        }
    }
    DurationBand {
        key: "other".to_owned(),
        label: "Other".to_owned(),
        nominal: seconds,
    }
}

/// Aggregate of one sport's sessions.
#[derive(Debug, Clone, PartialEq)]
pub struct SportSummary {
    /// Machine family.
    pub sport: Sport,
    /// Session count.
    pub sessions: usize,
    /// Total metres.
    pub distance: f64,
    /// Total seconds.
    pub time: f64,
    /// Distance-weighted average pace (sec/500 m); 0 when no distance.
    pub avg_pace: f64,
    /// Best (lowest) average pace across this sport's sessions; `None` when no
    /// session has a positive pace (the web app keeps an `Infinity` sentinel).
    pub best_pace: Option<f64>,
    /// Longest single session in metres.
    pub longest: f64,
}

/// Summarise workouts by sport, sorted by total distance descending. Sports
/// stay in first-seen order when distances tie (stable sort, like the web app).
#[must_use]
pub fn summarise_by_sport(workouts: &[Workout]) -> Vec<SportSummary> {
    let mut groups: Vec<(Sport, Vec<&Workout>)> = Vec::new();
    for workout in workouts {
        match groups.iter_mut().find(|(sport, _)| *sport == workout.sport) {
            Some((_, list)) => list.push(workout),
            None => groups.push((workout.sport, vec![workout])),
        }
    }
    let mut out: Vec<SportSummary> = groups
        .into_iter()
        .map(|(sport, list)| {
            let mut distance = 0.0;
            let mut time = 0.0;
            let mut best_pace = f64::INFINITY;
            let mut longest = f64::NEG_INFINITY;
            for w in &list {
                distance += w.distance;
                time += w.time;
                if w.pace > 0.0 && w.pace < best_pace {
                    best_pace = w.pace;
                }
                if w.distance > longest {
                    longest = w.distance;
                }
            }
            let avg_pace = if distance > 0.0 {
                time / (distance / 500.0)
            } else {
                0.0
            };
            SportSummary {
                sport,
                sessions: list.len(),
                distance,
                time,
                avg_pace,
                best_pace: if best_pace.is_finite() {
                    Some(best_pace)
                } else {
                    None
                },
                longest,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.distance
            .partial_cmp(&a.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Aggregate of a workout's strokes (Studio `StrokeSummary`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StrokeSummary {
    /// Stroke count.
    pub count: usize,
    /// Mean pace (sec/500 m).
    pub average_pace: f64,
    /// Mean watts.
    pub average_watts: f64,
    /// Maximum watts.
    pub peak_watts: f64,
}

/// Single-pass stroke aggregate (Studio `strokeSummary(for:)`).
#[must_use]
pub fn stroke_summary(strokes: &[Stroke]) -> StrokeSummary {
    if strokes.is_empty() {
        return StrokeSummary::default();
    }
    let mut total_pace = 0.0;
    let mut total_watts = 0.0;
    let mut peak_watts = f64::NEG_INFINITY;
    for stroke in strokes {
        total_pace += stroke.pace;
        total_watts += stroke.watts;
        if stroke.watts > peak_watts {
            peak_watts = stroke.watts;
        }
    }
    let count = strokes.len();
    StrokeSummary {
        count,
        average_pace: total_pace / count as f64,
        average_watts: total_watts / count as f64,
        peak_watts,
    }
}

/// Dashboard totals (Studio `DashboardSummary`).
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardSummary {
    /// Session count.
    pub sessions: usize,
    /// Total metres.
    pub total_distance: f64,
    /// Metres credited toward Concept2 challenges (BikeErg at half).
    pub challenge_distance: f64,
    /// Total seconds.
    pub total_time: f64,
    /// Distance-weighted average pace (sec/500 m).
    pub average_pace: f64,
    /// Per-sport breakdown.
    pub by_sport: Vec<SportSummary>,
}

/// Dashboard totals in one pass (Studio `dashboardSummary(for:)`).
#[must_use]
pub fn dashboard_summary(workouts: &[Workout]) -> DashboardSummary {
    let mut total_distance = 0.0;
    let mut challenge_distance = 0.0;
    let mut total_time = 0.0;
    for workout in workouts {
        total_distance += workout.distance;
        challenge_distance += challenge_distance_metres(workout.sport, workout.distance);
        total_time += workout.time;
    }
    let average_pace = if total_distance > 0.0 {
        total_time / (total_distance / 500.0)
    } else {
        0.0
    };
    DashboardSummary {
        sessions: workouts.len(),
        total_distance,
        challenge_distance,
        total_time,
        average_pace,
        by_sport: summarise_by_sport(workouts),
    }
}

/// A personal best row for the dashboard (Studio `DashboardPersonalBest`).
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardPersonalBest {
    /// Workout id.
    pub id: i64,
    /// Machine family.
    pub sport: Sport,
    /// Standard distance the workout matched (metres).
    pub distance: f64,
    /// Elapsed seconds.
    pub time: f64,
    /// Average pace (sec/500 m).
    pub pace: f64,
    /// Logbook date string.
    pub date: String,
}

/// Rows for the PB card: workouts in `pb_ids` that match a standard distance,
/// sorted by distance ascending then date descending (Studio `dashboardPersonalBests`).
#[must_use]
pub fn dashboard_personal_bests(
    workouts: &[Workout],
    pb_ids: &BTreeSet<i64>,
) -> Vec<DashboardPersonalBest> {
    let mut rows: Vec<DashboardPersonalBest> = workouts
        .iter()
        .filter(|w| pb_ids.contains(&w.id))
        .filter_map(|w| {
            standard_distance_matching(w.distance).map(|distance| DashboardPersonalBest {
                id: w.id,
                sport: w.sport,
                distance,
                time: w.time,
                pace: w.pace,
                date: w.date.clone(),
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.date.cmp(&a.date))
    });
    rows
}

/// The latest `limit` workouts of a sport in date order (Studio `recentPaceWorkouts`).
#[must_use]
pub fn recent_pace_workouts(workouts: &[Workout], sport: Sport, limit: usize) -> Vec<&Workout> {
    if limit == 0 {
        return Vec::new();
    }
    let mut matching: Vec<&Workout> = workouts.iter().filter(|w| w.sport == sport).collect();
    matching.sort_by(|a, b| a.date.cmp(&b.date));
    let skip = matching.len().saturating_sub(limit);
    matching.split_off(skip)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datetime::logbook_epoch_millis;
    use crate::demo::mock_workouts;

    fn workout(id: i64, sport: Sport, distance: f64, time: f64) -> Workout {
        let pace = if distance > 0.0 {
            time / (distance / 500.0)
        } else {
            0.0
        };
        Workout::new(id, "2023-11-14 22:13:20", sport, distance, time, pace)
    }

    // --- web analytics.test.ts ----------------------------------------------

    #[test]
    fn linear_trend_needs_two_points_and_a_span() {
        assert!(linear_trend(&[]).is_none());
        assert!(linear_trend(&[(0.0, 1.0)]).is_none());
        assert!(linear_trend(&[(5.0, 1.0), (5.0, 2.0)]).is_none());
    }

    #[test]
    fn linear_trend_fits_improving_2k_pieces_from_demo_data() {
        let mut two_ks: Vec<Workout> = mock_workouts()
            .into_iter()
            .filter(|w| w.sport == Sport::Rower && (w.distance - 2000.0).abs() < 50.0)
            .collect();
        two_ks.sort_by(|a, b| a.date.cmp(&b.date));
        let points: Vec<(f64, f64)> = two_ks
            .iter()
            .map(|w| (logbook_epoch_millis(&w.date), w.pace))
            .collect();
        let fit = linear_trend(&points).expect("fit");
        assert!(fit.n >= 2);
        assert!(fit.delta < 0.0);
    }

    #[test]
    fn linear_trend_uses_the_largest_x_for_the_endpoint() {
        let day = 86_400_000.0;
        let fit = linear_trend(&[(2.0 * day, 140.0), (0.0, 120.0), (day, 130.0)]).unwrap();
        assert!((fit.y0 - 120.0).abs() < 1e-9);
        assert!((fit.y1 - 140.0).abs() < 1e-9);
        assert!((fit.delta - 20.0).abs() < 1e-9);
        assert!((fit.slope_per_day - 10.0).abs() < 1e-9);
    }

    #[test]
    fn linear_trend_reports_a_daily_slope() {
        let start = 0.0;
        let day = 86_400_000.0;
        let fit = linear_trend(&[
            (start, 120.0),
            (start + day, 118.0),
            (start + 2.0 * day, 116.0),
        ])
        .unwrap();
        assert!((fit.slope_per_day + 2.0).abs() < 1e-9);
        assert!((fit.delta + 4.0).abs() < 1e-9);
        assert_eq!(fit.n, 3);
    }

    #[test]
    fn distance_band_buckets() {
        assert_eq!(distance_band(2000.0).key, "2000");
        assert_eq!(distance_band(2003.0).key, "2000");
        assert_eq!(distance_band(2003.0).label, "2k");
        assert_eq!(distance_band(3500.0).label, "3k–7k");
        assert_eq!(distance_band(8000.0).label, "7k–15k");
        assert_eq!(distance_band(8000.0).key, "r7000");
        assert_eq!(distance_band(8000.0).nominal, 10500.0);
        assert_eq!(distance_band(300.0).key, "r0");
        assert_eq!(
            distance_band(300.0).nominal,
            375.0,
            "web rowplay#202: lowest band nominal is hi/2, not 0"
        );
        assert_eq!(distance_band(20000.0).label, "Half", "within 6% of 21097");
        assert_eq!(distance_band(17000.0).label, "15k+");
        assert_eq!(distance_band(17000.0).nominal, 22500.0);
        assert_eq!(distance_band(-5.0).key, "other");
        assert_eq!(distance_band(f64::NAN).label, "Other");
    }

    #[test]
    fn duration_band_buckets() {
        assert_eq!(duration_band(1750.0).key, "1800");
        assert_eq!(duration_band(600.0).key, "r360");
        assert_eq!(duration_band(90.0).key, "r90");
        assert_eq!(duration_band(30.0).key, "r0");
        assert_eq!(duration_band(30.0).nominal, 45.0);
        assert_eq!(duration_band(-1.0).key, "other");
        assert_eq!(duration_band(-1.0).nominal, -1.0);
        assert!(duration_band(f64::NAN).nominal.is_nan());
        assert_eq!(duration_band(f64::INFINITY).label, "Other");
    }

    #[test]
    fn summarise_by_sport_aggregates_demo_history() {
        let rows = summarise_by_sport(&mock_workouts());
        assert!(rows.len() >= 2);
        let rower = rows.iter().find(|r| r.sport == Sport::Rower).unwrap();
        assert!(rower.sessions > 0 && rower.distance > 0.0 && rower.avg_pace > 0.0);
        assert!(rows.windows(2).all(|w| w[0].distance >= w[1].distance));
    }

    #[test]
    fn summarise_by_sport_without_positive_pace() {
        let rows = summarise_by_sport(&[
            Workout::new(1, "2026-05-01 06:00:00", Sport::Rower, 2000.0, 500.0, 0.0),
            Workout::new(2, "2026-05-01 06:00:00", Sport::Rower, 1000.0, 260.0, -1.0),
        ]);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].sessions, 2);
        assert_eq!(rows[0].best_pace, None);
        assert!((rows[0].avg_pace - 760.0 / 6.0).abs() < 1e-9);
        assert_eq!(rows[0].longest, 2000.0);
    }

    // --- Studio WorkoutAnalyticsTests ---------------------------------------

    #[test]
    fn summarise_by_sport_best_pace_and_longest() {
        let rows = summarise_by_sport(&[
            workout(1, Sport::Rower, 2000.0, 480.0),
            workout(2, Sport::Rower, 5000.0, 1250.0),
            workout(3, Sport::Rower, 3000.0, 690.0),
            workout(4, Sport::Skierg, 1000.0, 250.0),
        ]);
        let rower = rows.iter().find(|r| r.sport == Sport::Rower).unwrap();
        assert_eq!(rower.sessions, 3);
        assert!((rower.distance - 10_000.0).abs() < 1e-9);
        assert!((rower.time - 2420.0).abs() < 1e-9);
        assert!((rower.best_pace.unwrap() - 115.0).abs() < 1e-9);
        assert_eq!(rower.longest, 5000.0);
        let ski = rows.iter().find(|r| r.sport == Sport::Skierg).unwrap();
        assert_eq!(ski.sessions, 1);
        assert!((ski.best_pace.unwrap() - 125.0).abs() < 1e-9);
        assert_eq!(ski.longest, 1000.0);
    }

    #[test]
    fn dashboard_summary_accounts_for_bike_challenge_distance() {
        let workouts = mock_workouts();
        let summary = dashboard_summary(&workouts);
        let bike: f64 = workouts
            .iter()
            .filter(|w| w.sport == Sport::Bike)
            .map(|w| w.distance)
            .sum();
        assert_eq!(summary.sessions, workouts.len());
        assert!((summary.challenge_distance - (summary.total_distance - bike / 2.0)).abs() < 1e-6);
        assert_eq!(
            summary.by_sport.first().map(|s| s.sport),
            Some(Sport::Rower)
        );
        assert!(summary.average_pace > 0.0);
        assert_eq!(dashboard_summary(&[]).average_pace, 0.0);
    }

    #[test]
    fn stroke_summary_single_pass() {
        let strokes = [
            Stroke {
                hr: Some(150.0),
                ..Stroke::new(0.0, 0.0, 120.0, 28.0, 200.0)
            },
            Stroke {
                hr: Some(152.0),
                ..Stroke::new(2.0, 10.0, 130.0, 29.0, 240.0)
            },
            Stroke {
                hr: Some(154.0),
                ..Stroke::new(4.0, 20.0, 110.0, 30.0, 220.0)
            },
        ];
        let summary = stroke_summary(&strokes);
        assert_eq!(summary.count, 3);
        assert!((summary.average_pace - 120.0).abs() < 1e-9);
        assert!((summary.average_watts - 220.0).abs() < 1e-9);
        assert_eq!(summary.peak_watts, 240.0);
        assert_eq!(stroke_summary(&[]), StrokeSummary::default());
    }

    #[test]
    fn dashboard_personal_bests_uses_supplied_ids() {
        let workouts = [
            workout(1, Sport::Rower, 2000.0, 420.0),
            workout(2, Sport::Rower, 2000.0, 430.0),
            workout(3, Sport::Skierg, 5000.0, 1100.0),
        ];
        let pbs = dashboard_personal_bests(&workouts, &BTreeSet::from([1, 3]));
        assert_eq!(pbs.iter().map(|p| p.id).collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(
            pbs.iter().map(|p| p.sport).collect::<Vec<_>>(),
            vec![Sport::Rower, Sport::Skierg]
        );
        assert_eq!(
            pbs.iter().map(|p| p.distance).collect::<Vec<_>>(),
            vec![2000.0, 5000.0]
        );
    }

    #[test]
    fn recent_pace_workouts_returns_latest_in_date_order() {
        let mut workouts = vec![
            workout(1, Sport::Rower, 2000.0, 480.0),
            workout(2, Sport::Skierg, 2000.0, 480.0),
            workout(3, Sport::Rower, 2000.0, 480.0),
            workout(4, Sport::Rower, 2000.0, 480.0),
        ];
        for (i, w) in workouts.iter_mut().enumerate() {
            w.date = format!("2026-01-0{} 06:00:00", i + 2);
        }
        let recent = recent_pace_workouts(&workouts, Sport::Rower, 2);
        assert_eq!(recent.iter().map(|w| w.id).collect::<Vec<_>>(), vec![3, 4]);
        assert!(recent_pace_workouts(&workouts, Sport::Rower, 0).is_empty());
    }
}
