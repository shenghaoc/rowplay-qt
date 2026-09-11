// SPDX-License-Identifier: GPL-3.0-or-later
//! Dashboard view model: metric tiles, personal-best cards and the two trend
//! chart series.
//!
//! Sources: Studio's `DashboardView` (tile set, PB card, chart shapes and the
//! `recentPaceChartDomain` statics) over the web-canonical `analytics`
//! derivations (`dashboard_summary`, `dashboard_personal_bests`,
//! `recent_pace_workouts`). The "Challenge" tile from Studio is dropped: the
//! web dashboard has no challenge tile and no locale key for one
//! (`docs/source-map.md`). PB labels reuse the web's `distance_band` labels
//! ("2k", "Half", "Full" …), which are untranslated in the web too.

use rowplay_core::analytics::{
    DashboardPersonalBest, DashboardSummary, dashboard_personal_bests, dashboard_summary,
    distance_band, recent_pace_workouts,
};
use rowplay_core::formatting::{fmt_distance, fmt_distance_in, fmt_pace, fmt_time};
use rowplay_core::models::{DistanceUnit, Sport, Workout};
use rowplay_core::workout_query::pb_workout_ids;

use crate::dates::fmt_date;
use crate::role::ColorRole;
use crate::settings::Language;

/// Metres per mile (the web chart divisor for imperial).
pub const METRES_PER_MILE: f64 = 1_609.344;

/// One dashboard metric tile (Studio `MetricTile`).
#[derive(Debug, Clone, PartialEq)]
pub struct MetricTile {
    /// Locale message id for the label.
    pub label_id: &'static str,
    /// Rendered value (core formatting).
    pub value_text: String,
    /// Semantic colour.
    pub role: ColorRole,
    /// Screen-reader text: "<value> <label-source>" is composed in QML from
    /// the translated label; this is the value half plus context.
    pub accessible_value: String,
}

/// The tile row: sessions, total distance, total time, average pace.
#[must_use]
pub fn tiles(summary: &DashboardSummary, unit: DistanceUnit) -> Vec<MetricTile> {
    vec![
        MetricTile {
            label_id: "dashboard.sessions",
            value_text: summary.sessions.to_string(),
            role: ColorRole::Neutral,
            accessible_value: summary.sessions.to_string(),
        },
        MetricTile {
            label_id: "dashboard.totalDistance",
            value_text: fmt_distance_in(summary.total_distance, unit),
            role: ColorRole::Distance,
            accessible_value: fmt_distance(summary.total_distance),
        },
        MetricTile {
            label_id: "dashboard.totalTime",
            value_text: fmt_time(summary.total_time, false),
            role: ColorRole::Duration,
            accessible_value: fmt_time(summary.total_time, false),
        },
        MetricTile {
            label_id: "dashboard.avgPace",
            value_text: fmt_pace(summary.average_pace),
            role: ColorRole::Pace,
            accessible_value: fmt_pace(summary.average_pace),
        },
    ]
}

/// One personal-best card.
#[derive(Debug, Clone, PartialEq)]
pub struct PbCard {
    /// Workout id (tap target for 4b+ navigation).
    pub id: i64,
    /// Web `distanceBand` label ("500m", "2k", "Half", "Full", …).
    pub label: String,
    /// Untranslated sport display name.
    pub sport_name: &'static str,
    /// Machine key for badge styling.
    pub sport_key: &'static str,
    /// `fmt_time(tenths)` — Studio shows tenths on PB cards.
    /// `fmt_time(tenths)` value text.
    pub time_text: String,
    /// `fmt_pace` value text.
    pub pace_text: String,
    /// Locale short date.
    pub date_text: String,
}

/// PB cards for the dashboard grid.
#[must_use]
pub fn pb_cards(
    workouts: &[Workout],
    sport_filter: Option<Sport>,
    unit: DistanceUnit,
    language: Language,
    home_timezone: Option<&str>,
) -> Vec<PbCard> {
    let _ = unit; // PB distances are shown via the band label, not the unit.
    let pb_ids = pb_workout_ids(workouts, sport_filter);
    let bests: Vec<DashboardPersonalBest> = dashboard_personal_bests(workouts, &pb_ids);
    bests
        .iter()
        .map(|pb| PbCard {
            id: pb.id,
            label: distance_band(pb.distance).label,
            sport_name: pb.sport.display_name(),
            sport_key: crate::library::sport_key(pb.sport),
            time_text: fmt_time(pb.time, true),
            pace_text: fmt_pace(pb.pace),
            date_text: fmt_date(&pb.date, language, home_timezone),
        })
        .collect()
}

/// One bar of the distance-by-sport chart.
#[derive(Debug, Clone, PartialEq)]
pub struct SportBar {
    /// Untranslated sport display name (axis category label).
    pub sport_name: &'static str,
    /// Machine key for colour styling.
    pub sport_key: &'static str,
    /// Kilometres or miles (the web chart divisor).
    pub value: f64,
    /// Semantic colour for the bar.
    pub role: ColorRole,
}

/// Distance-by-sport bars; the axis label is `km` or `mi` (unit symbols,
/// untranslated like the web chart axes).
#[must_use]
pub fn by_sport_series(summary: &DashboardSummary, unit: DistanceUnit) -> Vec<SportBar> {
    summary
        .by_sport
        .iter()
        .map(|item| SportBar {
            sport_name: item.sport.display_name(),
            sport_key: crate::library::sport_key(item.sport),
            value: match unit {
                DistanceUnit::Metric => item.distance / 1_000.0,
                DistanceUnit::Imperial => item.distance / METRES_PER_MILE,
            },
            role: ColorRole::Distance,
        })
        .collect()
}

/// Axis unit symbol for the distance charts.
#[must_use]
pub const fn distance_axis_label(unit: DistanceUnit) -> &'static str {
    match unit {
        DistanceUnit::Metric => "km",
        DistanceUnit::Imperial => "mi",
    }
}

/// One point of the recent-pace line chart. Studio plots `-pace` so faster
/// is up; `x` is the chronological index (the web plots by date order, and
/// Qt Graphs gets pre-rendered tick labels instead of a JS date axis).
#[derive(Debug, Clone, PartialEq)]
pub struct PacePoint {
    /// Chronological index (0-based).
    pub x: f64,
    /// Negative pace (seconds per 500 m), Studio's orientation.
    pub y: f64,
    /// Tooltip/accessibility date (locale short date).
    pub date_text: String,
    /// `fmt_pace` of the point's pace.
    pub pace_text: String,
}

/// The recent-pace series for one sport (web `recentPaceWorkouts`, limit 10).
#[must_use]
pub fn recent_pace_series(
    workouts: &[Workout],
    sport: Sport,
    language: Language,
    home_timezone: Option<&str>,
) -> Vec<PacePoint> {
    recent_pace_workouts(workouts, sport, 10)
        .iter()
        .enumerate()
        .map(|(index, workout)| PacePoint {
            x: index as f64,
            y: -workout.pace,
            date_text: fmt_date(&workout.date, language, home_timezone),
            pace_text: fmt_pace(workout.pace),
        })
        .collect()
}

/// Studio's `recentPaceChartDomain`: ±12% padding (≥ 3 s) around the valid
/// paces, negated; the `-180…-60` placeholder when nothing is plottable.
#[must_use]
pub fn recent_pace_domain(workouts: &[Workout]) -> (f64, f64) {
    let mut fastest = f64::INFINITY;
    let mut slowest = f64::NEG_INFINITY;
    let mut has_valid_pace = false;
    for workout in workouts {
        let pace = workout.pace;
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

/// Y-axis tick labels for the negated pace axis: `fmt_pace(|value|)` at
/// evenly spaced ticks (Studio formats ticks through the same helper).
#[must_use]
pub fn pace_axis_labels(domain: (f64, f64), tick_count: usize) -> Vec<(f64, String)> {
    let (low, high) = domain;
    if tick_count < 2 || !(low.is_finite() && high.is_finite()) || high <= low {
        return Vec::new();
    }
    (0..tick_count)
        .map(|i| {
            let value = low + (high - low) * (i as f64) / ((tick_count - 1) as f64);
            (value, fmt_pace(-value))
        })
        .collect()
}

/// The dashboard's derived state in one call (the app layer recomputes on
/// library / filter / preference changes).
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardView {
    /// Metric tile row.
    pub tiles: Vec<MetricTile>,
    /// Personal-best cards.
    pub pb_cards: Vec<PbCard>,
    /// Distance-by-sport bars.
    pub sport_bars: Vec<SportBar>,
    /// Recent-pace line points (selected sport).
    pub pace_points: Vec<PacePoint>,
    /// Negated-pace axis domain.
    pub pace_domain: (f64, f64),
    /// Pre-rendered axis ticks `(value, label)`.
    pub pace_axis: Vec<(f64, String)>,
}

/// Full dashboard derivation for `workouts` (already filtered by the shell's
/// sport filter, like Studio's `library.filtered*` properties).
#[must_use]
pub fn dashboard_view(
    workouts: &[Workout],
    sport: Sport,
    unit: DistanceUnit,
    language: Language,
    home_timezone: Option<&str>,
) -> DashboardView {
    let summary = dashboard_summary(workouts);
    let pace_source = recent_pace_workouts(workouts, sport, 10)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let pace_points = recent_pace_series(workouts, sport, language, home_timezone);
    let pace_domain = recent_pace_domain(&pace_source);
    DashboardView {
        tiles: tiles(&summary, unit),
        pb_cards: pb_cards(workouts, None, unit, language, home_timezone),
        sport_bars: by_sport_series(&summary, unit),
        pace_points,
        pace_domain,
        pace_axis: pace_axis_labels(pace_domain, 4),
    }
}

#[cfg(test)]
mod tests {
    use rowplay_core::demo::mock_workouts;
    use rowplay_core::models::Workout;

    use super::*;

    #[test]
    fn tiles_render_the_demo_summary() {
        let workouts = mock_workouts();
        let summary = dashboard_summary(&workouts);
        let tiles = tiles(&summary, DistanceUnit::Metric);
        assert_eq!(tiles.len(), 4);
        assert_eq!(tiles[0].label_id, "dashboard.sessions");
        assert_eq!(tiles[0].value_text, "17");
        assert_eq!(tiles[1].label_id, "dashboard.totalDistance");
        assert!(tiles[1].value_text.ends_with("km"));
        assert_eq!(tiles[2].role, ColorRole::Duration);
        assert!(tiles[2].value_text.contains(':'));
        assert_eq!(tiles[3].role, ColorRole::Pace);
        assert!(tiles[3].value_text.contains(':'));
    }

    #[test]
    fn pb_cards_use_web_band_labels() {
        let workouts = mock_workouts();
        let cards = pb_cards(&workouts, None, DistanceUnit::Metric, Language::En, None);
        assert!(!cards.is_empty());
        for card in &cards {
            assert!(
                ["500m", "1k", "2k", "5k", "6k", "10k", "Half", "Full"]
                    .contains(&card.label.as_str()),
                "unexpected PB label {}",
                card.label
            );
            assert!(card.time_text.contains(':'));
            assert!(!card.date_text.is_empty());
        }
        // Sorted by distance ascending (Studio's dashboardPersonalBests).
        let distances: Vec<f64> = cards
            .iter()
            .map(|c| match c.label.as_str() {
                "500m" => 500.0,
                "1k" => 1000.0,
                "2k" => 2000.0,
                _ => f64::MAX,
            })
            .collect();
        let mut sorted = distances.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(distances, sorted);
    }

    #[test]
    fn by_sport_bars_convert_units() {
        let workouts = mock_workouts();
        let summary = dashboard_summary(&workouts);
        let metric = by_sport_series(&summary, DistanceUnit::Metric);
        let imperial = by_sport_series(&summary, DistanceUnit::Imperial);
        assert!(!metric.is_empty());
        for (m, i) in metric.iter().zip(imperial.iter()) {
            assert_eq!(m.sport_key, i.sport_key);
            let expected = m.value * 1_000.0 / METRES_PER_MILE;
            assert!(
                (i.value - expected).abs() < 1e-9,
                "imperial bar {} vs expected {expected}",
                i.value
            );
        }
        assert_eq!(distance_axis_label(DistanceUnit::Metric), "km");
        assert_eq!(distance_axis_label(DistanceUnit::Imperial), "mi");
    }

    /// Studio's domain statics, re-expressed.
    #[test]
    fn pace_domain_pads_twelve_percent_with_a_three_second_floor() {
        let make = |pace: f64| {
            Workout::new(
                1,
                "2024-01-01 00:00:00",
                Sport::Rower,
                2000.0,
                pace * 4.0,
                pace,
            )
        };
        let workouts = vec![make(120.0), make(130.0)];
        let (low, high) = recent_pace_domain(&workouts);
        // padding = max((130-120)*0.12, 3) = 3
        assert!((low - (-133.0)).abs() < 1e-9, "low {low}");
        assert!((high - (-117.0)).abs() < 1e-9, "high {high}");

        let single = vec![make(120.0)];
        let (low, high) = recent_pace_domain(&single);
        assert!((low - (-123.0)).abs() < 1e-9);
        assert!((high - (-117.0)).abs() < 1e-9);

        // No valid pace: the placeholder domain.
        let none = vec![Workout::new(
            1,
            "2024-01-01 00:00:00",
            Sport::Rower,
            0.0,
            0.0,
            0.0,
        )];
        assert_eq!(recent_pace_domain(&none), (-180.0, -60.0));
        assert_eq!(recent_pace_domain(&[]), (-180.0, -60.0));
    }

    #[test]
    fn pace_series_is_negated_and_chronological() {
        let workouts = mock_workouts();
        let points = recent_pace_series(&workouts, Sport::Rower, Language::En, None);
        assert!(!points.is_empty());
        assert!(points.len() <= 10);
        for (index, point) in points.iter().enumerate() {
            assert_eq!(point.x, index as f64);
            assert!(point.y <= 0.0);
            assert!(point.pace_text.contains(':'));
            assert!(!point.date_text.is_empty());
        }
    }

    #[test]
    fn pace_axis_labels_format_through_core() {
        let labels = pace_axis_labels((-133.0, -117.0), 4);
        assert_eq!(labels.len(), 4);
        assert_eq!(labels[0].0, -133.0);
        assert_eq!(labels[3].0, -117.0);
        assert_eq!(labels[0].1, fmt_pace(133.0));
        assert!(pace_axis_labels((-133.0, -117.0), 1).is_empty());
        assert!(pace_axis_labels((0.0, 0.0), 4).is_empty());
    }

    #[test]
    fn dashboard_view_bundles_everything() {
        let workouts = mock_workouts();
        let view = dashboard_view(
            &workouts,
            Sport::Rower,
            DistanceUnit::Metric,
            Language::En,
            None,
        );
        assert_eq!(view.tiles.len(), 4);
        assert!(!view.pb_cards.is_empty());
        assert!(!view.sport_bars.is_empty());
        assert!(!view.pace_points.is_empty());
        assert_eq!(view.pace_axis.len(), 4);
    }
}
