// SPDX-License-Identifier: GPL-3.0-or-later
//! Sidebar library view model: the filtered, sorted, day-sectioned workout
//! list with every display string pre-rendered.
//!
//! Sources: Studio's `SidebarView` (rows, sort menu, PB badge, count header)
//! over the web's `workout_query` engine (canonical filtering/sorting) and
//! `workout_local_day_key` (calendar bucketing in the home time zone). The
//! day sections are a desktop behaviour (neither reference groups the list);
//! see `docs/source-map.md`.

use std::collections::BTreeSet;

use rowplay_core::datetime::workout_local_day_key;
use rowplay_core::formatting::{fmt_distance_in, fmt_pace};
use rowplay_core::models::{DistanceUnit, Sport, Workout};
use rowplay_core::workout_query::{
    SortDir, WorkoutListQuery, WorkoutSortField, filter_and_sort_workouts,
};

use crate::dates::{fmt_date, fmt_time_of_day};
use crate::settings::Language;

/// One sidebar row, fully rendered.
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarRow {
    /// Concept2 result id (the selection key).
    pub id: i64,
    /// Row title: the logbook workout type, else the sport name.
    pub title: String,
    /// Locale short date (web `fmtDate`).
    pub date_text: String,
    /// Monitor-local time of day ("18:30").
    pub time_text: String,
    /// Distance in the preferred unit (core `fmt_distance_in`).
    pub distance_text: String,
    /// Pace (core `fmt_pace`).
    pub pace_text: String,
    /// Machine key for the sport badge: `rower` / `skierg` / `bike`.
    pub sport_key: &'static str,
    /// Untranslated sport display name (Concept2 trademark).
    pub sport_name: &'static str,
    /// Personal-best badge.
    pub is_pb: bool,
    /// `YYYY-MM-DD` day key (home-zone bucketing).
    pub section: String,
    /// Locale header for `section`.
    pub section_text: String,
    /// True on the first row of its section (QML draws the header here).
    pub is_section_start: bool,
    /// Screen-reader text (Studio's accessibilityLabel/Value port).
    pub accessible_text: String,
}

/// Renders the filtered list for the sidebar.
///
/// `pb_ids` comes from `workout_query::pb_workout_ids` for the *unfiltered*
/// library so badges do not flicker with the filter (Studio keeps one set).
#[must_use]
pub fn sidebar_rows(
    workouts: &[Workout],
    query: &WorkoutListQuery,
    pb_ids: &BTreeSet<i64>,
    unit: DistanceUnit,
    language: Language,
    home_timezone: Option<&str>,
) -> Vec<SidebarRow> {
    let filtered = filter_and_sort_workouts(workouts, query, Some(pb_ids));
    let mut rows: Vec<SidebarRow> = filtered
        .iter()
        .map(|workout| sidebar_row(workout, pb_ids, unit, language, home_timezone))
        .collect();
    let mut previous_key: Option<String> = None;
    for row in &mut rows {
        let start = previous_key.as_deref() != Some(row.section.as_str());
        row.is_section_start = start;
        previous_key = Some(row.section.clone());
    }
    rows
}

fn sidebar_row(
    workout: &Workout,
    pb_ids: &BTreeSet<i64>,
    unit: DistanceUnit,
    language: Language,
    home_timezone: Option<&str>,
) -> SidebarRow {
    let section = workout_local_day_key(&workout.date, workout.timezone.as_deref(), home_timezone);
    let section_text = fmt_date(&section, language, home_timezone);
    let title = workout
        .workout_type
        .clone()
        .unwrap_or_else(|| workout.sport.display_name().to_owned());
    let date_text = fmt_date(&workout.date, language, home_timezone);
    let time_text = fmt_time_of_day(&workout.date);
    let distance_text = fmt_distance_in(workout.distance, unit);
    let pace_text = fmt_pace(workout.pace);
    let is_pb = pb_ids.contains(&workout.id);

    // Studio: "<sport> <type>[, Personal Best]" + "<date>; <distance>;
    // <time>; <pace>". The PB suffix stays the untranslated token "PB" —
    // the badge text itself is the same in Studio.
    let pb_suffix = if is_pb { " PB" } else { "" };
    let accessible_text = format!(
        "{} {}{pb_suffix}; {date_text}; {distance_text}; {pace_text}",
        workout.sport.display_name(),
        title
    );

    SidebarRow {
        id: workout.id,
        title,
        date_text,
        time_text,
        distance_text,
        pace_text,
        sport_key: sport_key(workout.sport),
        sport_name: workout.sport.display_name(),
        is_pb,
        section,
        section_text,
        is_section_start: false,
        accessible_text,
    }
}

/// Validates and canonicalises a `YYYY-MM-DD` day-key filter input
/// (the date-range fields); `None` when the text is empty, and an
/// unparseable value is reported back to the field as invalid.
#[must_use]
pub fn normalize_day_key(text: &str) -> Option<Option<String>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(None);
    }
    rowplay_core::datetime::naive_date_from_key(trimmed)
        .map(|date| Some(date.format("%Y-%m-%d").to_string()))
}

/// Stable machine key for QML badge styling.
#[must_use]
pub const fn sport_key(sport: Sport) -> &'static str {
    match sport {
        Sport::Rower => "rower",
        Sport::Skierg => "skierg",
        Sport::Bike => "bike",
    }
}

/// Studio's `toggleSort`: same field flips the direction, a new field starts
/// descending except pace and time, which start ascending (best first).
#[must_use]
pub fn toggle_sort(query: &WorkoutListQuery, field: WorkoutSortField) -> WorkoutListQuery {
    let mut next = query.clone();
    if next.sort == field {
        next.dir = match next.dir {
            SortDir::Asc => SortDir::Desc,
            SortDir::Desc => SortDir::Asc,
        };
    } else {
        next.sort = field;
        next.dir = match field {
            WorkoutSortField::Pace | WorkoutSortField::Time => SortDir::Asc,
            _ => SortDir::Desc,
        };
    }
    next
}

/// Locale message id for a sort-field menu entry (web `workoutList.sort*`).
#[must_use]
pub const fn sort_field_id(field: WorkoutSortField) -> &'static str {
    match field {
        WorkoutSortField::Date => "workoutList.sortDate",
        WorkoutSortField::Distance => "workoutList.sortDistance",
        WorkoutSortField::Time => "workoutList.sortTime",
        WorkoutSortField::Pace => "workoutList.sortPace",
        WorkoutSortField::Power => "workoutList.sortPower",
    }
}

/// The sort fields in Studio's menu order.
#[must_use]
pub const fn sort_fields() -> [WorkoutSortField; 5] {
    [
        WorkoutSortField::Date,
        WorkoutSortField::Distance,
        WorkoutSortField::Time,
        WorkoutSortField::Pace,
        WorkoutSortField::Power,
    ]
}

#[cfg(test)]
mod tests {
    use rowplay_core::demo::mock_workouts;
    use rowplay_core::models::Sport;
    use rowplay_core::workout_query::pb_workout_ids;

    use super::*;

    fn demo_rows(unit: DistanceUnit) -> Vec<SidebarRow> {
        let workouts = mock_workouts();
        let pb_ids = pb_workout_ids(&workouts, None);
        sidebar_rows(
            &workouts,
            &WorkoutListQuery::default(),
            &pb_ids,
            unit,
            Language::En,
            None,
        )
    }

    #[test]
    fn renders_every_demo_workout_with_display_strings() {
        let rows = demo_rows(DistanceUnit::Metric);
        assert_eq!(rows.len(), mock_workouts().len());
        let first = &rows[0];
        // Default query sorts by date descending; every row is pre-rendered.
        assert!(!first.date_text.is_empty());
        assert!(first.distance_text.ends_with("km") || first.distance_text.ends_with(" m"));
        assert!(first.pace_text.contains(':'));
        assert!(!first.accessible_text.is_empty());
        assert!(matches!(first.sport_key, "rower" | "skierg" | "bike"));
    }

    #[test]
    fn day_sections_mark_only_the_first_row() {
        let rows = demo_rows(DistanceUnit::Metric);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for row in &rows {
            if row.is_section_start {
                assert!(
                    seen.insert(row.section.clone()),
                    "section {} started twice",
                    row.section
                );
                assert!(!row.section_text.is_empty());
            }
        }
        assert!(rows[0].is_section_start);
    }

    #[test]
    fn home_timezone_shifts_day_sections_for_the_late_night_workout() {
        // Demo workout 9001 logs 2024-01-14 23:30:00 in America/New_York.
        let workouts = mock_workouts();
        let late = workouts.iter().find(|w| w.id == 9001).expect("demo 9001");
        assert_eq!(late.timezone.as_deref(), Some("America/New_York"));
        let pb = BTreeSet::new();
        let query = WorkoutListQuery::default();

        let utc_rows = sidebar_rows(
            &workouts,
            &query,
            &pb,
            DistanceUnit::Metric,
            Language::En,
            None,
        );
        let tokyo_rows = sidebar_rows(
            &workouts,
            &query,
            &pb,
            DistanceUnit::Metric,
            Language::En,
            Some("Asia/Tokyo"),
        );
        let utc_row = utc_rows.iter().find(|r| r.id == 9001).unwrap();
        let tokyo_row = tokyo_rows.iter().find(|r| r.id == 9001).unwrap();
        // 23:30 in New York is the next day in Tokyo.
        assert_eq!(utc_row.section, "2024-01-14");
        assert_eq!(tokyo_row.section, "2024-01-15");
    }

    #[test]
    fn personal_best_badges_survive_filtering() {
        let workouts = mock_workouts();
        let pb_ids = pb_workout_ids(&workouts, None);
        assert!(!pb_ids.is_empty(), "the demo set has PBs");

        let query = WorkoutListQuery {
            sport: Some(Sport::Skierg),
            ..WorkoutListQuery::default()
        };
        let rows = sidebar_rows(
            &workouts,
            &query,
            &pb_ids,
            DistanceUnit::Metric,
            Language::En,
            None,
        );
        assert!(rows.iter().all(|r| r.sport_key == "skierg"));
        // The badge set was computed unfiltered: SkiErg rows that are PBs
        // stay badged.
        assert!(rows.iter().any(|r| r.is_pb));
    }

    #[test]
    fn imperial_rows_use_miles() {
        let rows = demo_rows(DistanceUnit::Imperial);
        assert!(
            rows.iter()
                .any(|r| r.distance_text.contains("mi") || r.distance_text.contains("ft"))
        );
    }

    /// Studio's `toggleSort` semantics.
    #[test]
    fn toggle_sort_flips_direction_and_resets_per_field() {
        let q = WorkoutListQuery::default(); // Date, Desc (web default)
        let q2 = toggle_sort(&q, WorkoutSortField::Date);
        assert_eq!(q2.sort, WorkoutSortField::Date);
        assert_eq!(q2.dir, SortDir::Asc);
        let q3 = toggle_sort(&q2, WorkoutSortField::Date);
        assert_eq!(q3.dir, SortDir::Desc);

        let pace = toggle_sort(&q, WorkoutSortField::Pace);
        assert_eq!(pace.sort, WorkoutSortField::Pace);
        assert_eq!(pace.dir, SortDir::Asc); // best pace first
        let time = toggle_sort(&q, WorkoutSortField::Time);
        assert_eq!(time.dir, SortDir::Asc);
        let distance = toggle_sort(&q, WorkoutSortField::Distance);
        assert_eq!(distance.dir, SortDir::Desc);
        let power = toggle_sort(&q, WorkoutSortField::Power);
        assert_eq!(power.dir, SortDir::Desc);
    }

    #[test]
    fn day_key_validation_accepts_canonical_keys_only() {
        assert_eq!(normalize_day_key(""), Some(None));
        assert_eq!(normalize_day_key("   "), Some(None));
        assert_eq!(
            normalize_day_key("2024-03-05"),
            Some(Some("2024-03-05".to_owned()))
        );
        assert_eq!(normalize_day_key("2024-13-45"), None);
        assert_eq!(normalize_day_key("yesterday"), None);
    }

    #[test]
    fn sort_field_ids_are_web_keys() {
        for field in sort_fields() {
            let id = sort_field_id(field);
            assert!(id.starts_with("workoutList.sort"), "{id}");
        }
        assert_eq!(sort_fields().len(), 5);
    }
}
