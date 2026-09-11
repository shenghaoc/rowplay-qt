// SPDX-License-Identifier: GPL-3.0-or-later
//! Workout list query: parsing, serialising, filtering and sorting.
//!
//! Port of the web app's `src/lib/workoutQuery.ts`. Queries mirror the web
//! app's URL search parameters so deep links stay shareable; filtering and
//! sorting are pure and run in memory (demo mode and API fallback).
//! rowplay-studio's `clearFilters` helper and chip labels are included.

use std::collections::BTreeSet;

use crate::analytics::distance_band;
use crate::models::{Sport, Workout};
use crate::personal_bests::{STANDARD_DISTANCES, matches_standard_distance};
use crate::workout_tag::{TagContext, WorkoutTag, athlete_median_pace, resolve_tag};

/// Fields the list can sort by.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum WorkoutSortField {
    /// Logbook date.
    #[default]
    Date,
    /// Distance in metres.
    Distance,
    /// Elapsed time.
    Time,
    /// Average pace.
    Pace,
    /// Average power.
    Power,
}

impl WorkoutSortField {
    /// Every field, in the web app's declaration order.
    pub const ALL: [WorkoutSortField; 5] = [
        WorkoutSortField::Date,
        WorkoutSortField::Distance,
        WorkoutSortField::Time,
        WorkoutSortField::Pace,
        WorkoutSortField::Power,
    ];

    /// Wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WorkoutSortField::Date => "date",
            WorkoutSortField::Distance => "distance",
            WorkoutSortField::Time => "time",
            WorkoutSortField::Pace => "pace",
            WorkoutSortField::Power => "power",
        }
    }

    /// Parse the wire spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<WorkoutSortField> {
        WorkoutSortField::ALL
            .into_iter()
            .find(|field| field.as_str() == value)
    }
}

/// Sort direction.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum SortDir {
    /// Ascending.
    Asc,
    /// Descending.
    #[default]
    Desc,
}

impl SortDir {
    /// Wire spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SortDir::Asc => "asc",
            SortDir::Desc => "desc",
        }
    }
}

/// Parsed list query — mirrors URL search params.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WorkoutListQuery {
    /// Sport tab.
    pub sport: Option<Sport>,
    /// Resolved rowplay workout tag.
    pub tag: Option<WorkoutTag>,
    /// Raw Concept2 workout type / source label.
    pub workout_type: Option<String>,
    /// Inclusive `YYYY-MM-DD` lower bound.
    pub date_from: Option<String>,
    /// Inclusive `YYYY-MM-DD` upper bound.
    pub date_to: Option<String>,
    /// Nominal metres for a distance chip (500, 2000, …).
    pub distance_m: Option<f64>,
    /// Coarse band key from [`distance_band`] (e.g. `r3000`).
    pub distance_band_key: Option<String>,
    /// Require (or exclude) stroke data.
    pub has_stroke: Option<bool>,
    /// Free-text match against `comments`.
    pub q: Option<String>,
    /// Only personal bests.
    pub pbs_only: bool,
    /// Minimum elapsed seconds.
    pub duration_min: Option<f64>,
    /// Maximum elapsed seconds.
    pub duration_max: Option<f64>,
    /// Sort field.
    pub sort: WorkoutSortField,
    /// Sort direction.
    pub dir: SortDir,
}

/// Distance quick chips: nominal metres, URL key and label (±2% when applied).
pub const DISTANCE_CHIPS: [(f64, &str, &str); 5] = [
    (500.0, "500", "500m"),
    (2000.0, "2000", "2k"),
    (5000.0, "5000", "5k"),
    (10000.0, "10000", "10k"),
    (42195.0, "42195", "Marathon"),
];

/// Common piece durations: seconds, URL key and label (±10% when applied).
pub const DURATION_CHIPS: [(f64, &str, &str); 3] = [
    (1200.0, "20", "20 min"),
    (1800.0, "30", "30 min"),
    (3600.0, "60", "60 min"),
];

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.is_empty())
}

/// `parseBool`: `1`/`true` → true, `0`/`false` → false, anything else → `None`.
fn parse_bool(value: Option<&str>) -> Option<bool> {
    match value {
        Some("1" | "true") => Some(true),
        Some("0" | "false") => Some(false),
        _ => None,
    }
}

/// `parseNum`: finite numbers only; empty or missing → `None`.
fn parse_num(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    value.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn first<'a>(params: &'a [(&str, &str)], key: &str) -> Option<&'a str> {
    params.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// Read list filters from search parameters such as
/// `sport=rower&sort=pace&dir=asc` (first value per key wins).
#[must_use]
pub fn parse_workout_list_query(params: &[(&str, &str)]) -> WorkoutListQuery {
    let mut q = WorkoutListQuery {
        sort: first(params, "sort")
            .and_then(WorkoutSortField::parse)
            .unwrap_or_default(),
        dir: if first(params, "dir") == Some("asc") {
            SortDir::Asc
        } else {
            SortDir::Desc
        },
        ..WorkoutListQuery::default()
    };
    q.sport = first(params, "sport").and_then(Sport::parse);
    q.tag = first(params, "tag").and_then(WorkoutTag::parse);
    q.workout_type = non_empty(first(params, "type")).map(str::to_owned);
    q.date_from = non_empty(first(params, "from")).map(str::to_owned);
    q.date_to = non_empty(first(params, "to")).map(str::to_owned);
    q.distance_m = parse_num(first(params, "dist"));
    q.distance_band_key = non_empty(first(params, "band")).map(str::to_owned);
    q.has_stroke = parse_bool(first(params, "stroke"));
    q.q = first(params, "q")
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_owned);
    q.pbs_only = first(params, "pbs") == Some("1");
    q.duration_min = parse_num(first(params, "dmin"));
    q.duration_max = parse_num(first(params, "dmax"));
    q
}

fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// Serialise for shareable/bookmarkable URLs (defaults omitted).
#[must_use]
pub fn serialize_workout_list_query(q: &WorkoutListQuery) -> Vec<(String, String)> {
    let mut p: Vec<(String, String)> = Vec::new();
    let mut set = |key: &str, value: String| p.push((key.to_owned(), value));
    if let Some(sport) = q.sport {
        set("sport", sport.as_str().to_owned());
    }
    if let Some(tag) = q.tag {
        set("tag", tag.as_str().to_owned());
    }
    if let Some(wt) = non_empty(q.workout_type.as_deref()) {
        set("type", wt.to_owned());
    }
    if let Some(from) = non_empty(q.date_from.as_deref()) {
        set("from", from.to_owned());
    }
    if let Some(to) = non_empty(q.date_to.as_deref()) {
        set("to", to.to_owned());
    }
    if let Some(dist) = q.distance_m {
        set("dist", fmt_num(dist));
    }
    if let Some(band) = non_empty(q.distance_band_key.as_deref()) {
        set("band", band.to_owned());
    }
    match q.has_stroke {
        Some(true) => set("stroke", "1".to_owned()),
        Some(false) => set("stroke", "0".to_owned()),
        None => {}
    }
    if let Some(text) = non_empty(q.q.as_deref()) {
        set("q", text.to_owned());
    }
    if q.pbs_only {
        set("pbs", "1".to_owned());
    }
    if let Some(dmin) = q.duration_min {
        set("dmin", fmt_num(dmin));
    }
    if let Some(dmax) = q.duration_max {
        set("dmax", fmt_num(dmax));
    }
    if q.sort != WorkoutSortField::Date {
        set("sort", q.sort.as_str().to_owned());
    }
    if q.dir != SortDir::Desc {
        set("dir", q.dir.as_str().to_owned());
    }
    p
}

/// Whether any list-specific filter (beyond sort) is active. The sport is a
/// top-level tab, not a list filter, so it does not count.
#[must_use]
pub fn list_query_is_filtered(q: &WorkoutListQuery) -> bool {
    q.tag.is_some()
        || non_empty(q.workout_type.as_deref()).is_some()
        || non_empty(q.date_from.as_deref()).is_some()
        || non_empty(q.date_to.as_deref()).is_some()
        || q.distance_m.is_some()
        || non_empty(q.distance_band_key.as_deref()).is_some()
        || q.has_stroke.is_some()
        || non_empty(q.q.as_deref()).is_some()
        || q.pbs_only
        || q.duration_min.is_some()
        || q.duration_max.is_some()
}

/// Average watts from watt-minutes and elapsed time; `None` when either is missing.
#[must_use]
pub fn avg_power_watts(w: &Workout) -> Option<f64> {
    match w.watt_minutes {
        Some(watt_minutes) if w.time > 0.0 => Some((watt_minutes * 60.0) / w.time),
        _ => None,
    }
}

/// Workout ids that are a PB at a standard distance (±2%), optionally filtered
/// by sport. Unlike [`crate::personal_bests::pb_workout_ids`], this picks the
/// fastest workout across every sport when no sport filter is given (the web
/// list behaviour).
#[must_use]
pub fn pb_workout_ids(workouts: &[Workout], sport: Option<Sport>) -> BTreeSet<i64> {
    let mut ids = BTreeSet::new();
    for target in STANDARD_DISTANCES {
        let mut best: Option<&Workout> = None;
        for w in workouts {
            if !matches_standard_distance(w.distance, target)
                || w.time <= 0.0
                || sport.is_some_and(|s| w.sport != s)
            {
                continue;
            }
            // `reduce((a, b) => a.time <= b.time ? a : b)` keeps the earlier workout on ties.
            best = match best {
                Some(current) if current.time <= w.time => Some(current),
                _ => Some(w),
            };
        }
        if let Some(best) = best {
            ids.insert(best.id);
        }
    }
    ids
}

/// In-memory filter + sort. Returns references in the sorted order.
#[must_use]
pub fn filter_and_sort_workouts<'a>(
    workouts: &'a [Workout],
    q: &WorkoutListQuery,
    pb_ids: Option<&BTreeSet<i64>>,
) -> Vec<&'a Workout> {
    let computed_pbs = if pb_ids.is_none() && q.pbs_only {
        Some(pb_workout_ids(workouts, q.sport))
    } else {
        None
    };
    let pbs = pb_ids.or(computed_pbs.as_ref());
    let tag_ctx = q.tag.map(|_| TagContext {
        median_pace_secs: athlete_median_pace(workouts),
    });
    let workout_type = non_empty(q.workout_type.as_deref());
    let date_from = non_empty(q.date_from.as_deref());
    let date_to = non_empty(q.date_to.as_deref());
    let band_key = non_empty(q.distance_band_key.as_deref());
    let text = non_empty(q.q.as_deref()).map(str::to_lowercase);

    let mut out: Vec<&Workout> = workouts
        .iter()
        .filter(|w| {
            if q.sport.is_some_and(|sport| w.sport != sport) {
                return false;
            }
            if q.tag
                .is_some_and(|tag| resolve_tag(w, None, tag_ctx.as_ref()) != tag)
            {
                return false;
            }
            if workout_type.is_some_and(|wt| w.workout_type.as_deref() != Some(wt)) {
                return false;
            }
            let day = w.day_key();
            if date_from.is_some_and(|from| day.as_str() < from) {
                return false;
            }
            if date_to.is_some_and(|to| day.as_str() > to) {
                return false;
            }
            if q.distance_m
                .is_some_and(|nominal| !matches_standard_distance(w.distance, nominal))
            {
                return false;
            }
            if band_key.is_some_and(|key| distance_band(w.distance).key != key) {
                return false;
            }
            if q.has_stroke == Some(true) && !w.has_stroke_data {
                return false;
            }
            if q.has_stroke == Some(false) && w.has_stroke_data {
                return false;
            }
            if let Some(text) = &text {
                let hay = w.comments.as_deref().unwrap_or("").to_lowercase();
                if !hay.contains(text.as_str()) {
                    return false;
                }
            }
            if q.pbs_only && pbs.is_some_and(|ids| !ids.contains(&w.id)) {
                return false;
            }
            if q.duration_min.is_some_and(|min| w.time < min) {
                return false;
            }
            if q.duration_max.is_some_and(|max| w.time > max) {
                return false;
            }
            true
        })
        .collect();

    let dir: f64 = if q.dir == SortDir::Asc { 1.0 } else { -1.0 };
    out.sort_by(|a, b| {
        let cmp = match q.sort {
            WorkoutSortField::Date => match a.date.cmp(&b.date) {
                std::cmp::Ordering::Less => -1.0,
                std::cmp::Ordering::Equal => 0.0,
                std::cmp::Ordering::Greater => 1.0,
            },
            WorkoutSortField::Distance => a.distance - b.distance,
            WorkoutSortField::Time => a.time - b.time,
            WorkoutSortField::Pace => {
                let sentinel = if dir > 0.0 { f64::INFINITY } else { -1.0 };
                let pa = if a.pace > 0.0 { a.pace } else { sentinel };
                let pb = if b.pace > 0.0 { b.pace } else { sentinel };
                pa - pb
            }
            WorkoutSortField::Power => {
                avg_power_watts(a).unwrap_or(-1.0) - avg_power_watts(b).unwrap_or(-1.0)
            }
        };
        (cmp * dir)
            .partial_cmp(&0.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

/// Apply a distance quick chip (toggle off if already set); clears band and duration chips.
#[must_use]
pub fn toggle_distance_chip(q: &WorkoutListQuery, metres: f64) -> WorkoutListQuery {
    let on = q.distance_m == Some(metres);
    WorkoutListQuery {
        distance_m: if on { None } else { Some(metres) },
        distance_band_key: None,
        duration_min: None,
        duration_max: None,
        ..q.clone()
    }
}

/// Apply a duration quick chip (±10%, toggle off if already set); clears distance chips.
#[must_use]
pub fn toggle_duration_chip(q: &WorkoutListQuery, seconds: f64) -> WorkoutListQuery {
    let tol = seconds * 0.1;
    let on = duration_chip_active(q, seconds);
    WorkoutListQuery {
        duration_min: if on { None } else { Some(seconds - tol) },
        duration_max: if on { None } else { Some(seconds + tol) },
        distance_m: None,
        distance_band_key: None,
        ..q.clone()
    }
}

/// Whether a duration chip's ±10% bounds are currently applied.
#[must_use]
pub fn duration_chip_active(q: &WorkoutListQuery, seconds: f64) -> bool {
    let tol = seconds * 0.1;
    q.duration_min == Some(seconds - tol) && q.duration_max == Some(seconds + tol)
}

/// Clear every filter, preserving sort field and direction (Studio `clearFilters`).
#[must_use]
pub fn clear_filters(q: &WorkoutListQuery) -> WorkoutListQuery {
    WorkoutListQuery {
        sort: q.sort,
        dir: q.dir,
        ..WorkoutListQuery::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workout(id: i64) -> Workout {
        Workout::new(
            id,
            "2026-05-01 06:00:00",
            Sport::Rower,
            2000.0,
            480.0,
            120.0,
        )
    }

    fn base() -> WorkoutListQuery {
        WorkoutListQuery::default()
    }

    fn ids(list: &[&Workout]) -> Vec<i64> {
        list.iter().map(|w| w.id).collect()
    }

    // --- web workoutQuery.test.ts -------------------------------------------

    #[test]
    fn parse_defaults_and_fields() {
        let q = parse_workout_list_query(&[]);
        assert_eq!(q.sort, WorkoutSortField::Date);
        assert_eq!(q.dir, SortDir::Desc);
        assert_eq!(q.sport, None);
        assert_eq!(
            parse_workout_list_query(&[("sport", "rower")]).sport,
            Some(Sport::Rower)
        );
        assert_eq!(parse_workout_list_query(&[("sport", "kayak")]).sport, None);
        let q = parse_workout_list_query(&[("sort", "pace"), ("dir", "asc")]);
        assert_eq!((q.sort, q.dir), (WorkoutSortField::Pace, SortDir::Asc));
        assert_eq!(
            parse_workout_list_query(&[("sort", "invalid")]).sort,
            WorkoutSortField::Date
        );
        let q = parse_workout_list_query(&[("from", "2026-01-01"), ("to", "2026-06-01")]);
        assert_eq!(
            (q.date_from.as_deref(), q.date_to.as_deref()),
            (Some("2026-01-01"), Some("2026-06-01"))
        );
        assert_eq!(
            parse_workout_list_query(&[("dist", "2000")]).distance_m,
            Some(2000.0)
        );
        assert_eq!(
            parse_workout_list_query(&[("dist", "abc")]).distance_m,
            None
        );
        assert_eq!(
            parse_workout_list_query(&[("band", "r2000")])
                .distance_band_key
                .as_deref(),
            Some("r2000")
        );
        assert_eq!(
            parse_workout_list_query(&[("stroke", "1")]).has_stroke,
            Some(true)
        );
        assert_eq!(
            parse_workout_list_query(&[("stroke", "0")]).has_stroke,
            Some(false)
        );
        assert_eq!(
            parse_workout_list_query(&[("stroke", "true")]).has_stroke,
            Some(true)
        );
        assert_eq!(
            parse_workout_list_query(&[("stroke", "false")]).has_stroke,
            Some(false)
        );
        assert_eq!(
            parse_workout_list_query(&[("stroke", "maybe")]).has_stroke,
            None
        );
        assert!(parse_workout_list_query(&[("pbs", "1")]).pbs_only);
        let q = parse_workout_list_query(&[("dmin", "600"), ("dmax", "1200")]);
        assert_eq!(
            (q.duration_min, q.duration_max),
            (Some(600.0), Some(1200.0))
        );
        assert_eq!(
            parse_workout_list_query(&[("q", "morning row")])
                .q
                .as_deref(),
            Some("morning row")
        );
        assert_eq!(parse_workout_list_query(&[("q", "   ")]).q, None);
        assert_eq!(
            parse_workout_list_query(&[("type", "JustRow")])
                .workout_type
                .as_deref(),
            Some("JustRow")
        );
        assert_eq!(
            parse_workout_list_query(&[("tag", "race-piece")]).tag,
            Some(WorkoutTag::RacePiece)
        );
        assert_eq!(parse_workout_list_query(&[("tag", "sprint")]).tag, None);
    }

    #[test]
    fn serialize_omits_defaults_and_round_trips() {
        let p = serialize_workout_list_query(&base());
        assert!(p.is_empty());
        let p = serialize_workout_list_query(&WorkoutListQuery {
            sort: WorkoutSortField::Pace,
            dir: SortDir::Asc,
            ..base()
        });
        assert_eq!(
            p,
            vec![
                ("sort".to_owned(), "pace".to_owned()),
                ("dir".to_owned(), "asc".to_owned())
            ]
        );

        let original = parse_workout_list_query(&[
            ("sport", "bike"),
            ("tag", "interval"),
            ("type", "JustRow"),
            ("sort", "pace"),
            ("dir", "asc"),
            ("from", "2026-01-01"),
            ("dist", "2000"),
            ("pbs", "1"),
        ]);
        let serialized = serialize_workout_list_query(&original);
        let borrowed: Vec<(&str, &str)> = serialized
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let parsed = parse_workout_list_query(&borrowed);
        assert_eq!(parsed, original);
        assert!(serialized.iter().any(|(k, v)| k == "dist" && v == "2000"));
    }

    #[test]
    fn list_query_is_filtered_cases() {
        assert!(!list_query_is_filtered(&base()));
        assert!(!list_query_is_filtered(&WorkoutListQuery {
            sport: Some(Sport::Rower),
            ..base()
        }));
        assert!(list_query_is_filtered(&WorkoutListQuery {
            date_from: Some("2026-01-01".into()),
            ..base()
        }));
        assert!(list_query_is_filtered(&WorkoutListQuery {
            distance_m: Some(2000.0),
            ..base()
        }));
        assert!(list_query_is_filtered(&WorkoutListQuery {
            pbs_only: true,
            ..base()
        }));
        assert!(list_query_is_filtered(&WorkoutListQuery {
            tag: Some(WorkoutTag::TimeTrial),
            ..base()
        }));
        assert!(!list_query_is_filtered(&WorkoutListQuery {
            q: Some(String::new()),
            ..base()
        }));
    }

    fn sample() -> Vec<Workout> {
        let w2k = Workout {
            comments: None,
            ..Workout::new(1, "2026-05-01 06:00:00", Sport::Rower, 2000.0, 480.0, 120.0)
        };
        let w5k = Workout::new(
            2,
            "2026-04-01 06:00:00",
            Sport::Rower,
            5000.0,
            1200.0,
            120.0,
        );
        let bike = Workout::new(3, "2026-03-01 06:00:00", Sport::Bike, 2000.0, 500.0, 125.0);
        vec![w2k, w5k, bike]
    }

    #[test]
    fn filter_and_sort_cases() {
        let all = sample();
        assert_eq!(filter_and_sort_workouts(&all, &base(), None).len(), 3);

        let rower = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                sport: Some(Sport::Rower),
                ..base()
            },
            None,
        );
        assert_eq!(rower.len(), 2);
        assert!(rower.iter().all(|w| w.sport == Sport::Rower));

        assert_eq!(
            filter_and_sort_workouts(
                &all,
                &WorkoutListQuery {
                    distance_m: Some(2000.0),
                    ..base()
                },
                None
            )
            .len(),
            2
        );

        let dated = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                date_from: Some("2026-04-01".into()),
                date_to: Some("2026-04-30".into()),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&dated), vec![2]);

        let with_no_stroke = [
            all[0].clone(),
            Workout {
                has_stroke_data: false,
                ..workout(4)
            },
        ];
        let stroked = filter_and_sort_workouts(
            &with_no_stroke,
            &WorkoutListQuery {
                has_stroke: Some(true),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&stroked), vec![1]);

        let with_comment = [
            all[0].clone(),
            Workout {
                comments: Some("morning row session".into()),
                ..workout(5)
            },
        ];
        let found = filter_and_sort_workouts(
            &with_comment,
            &WorkoutListQuery {
                q: Some("Morning".into()),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&found), vec![5]);

        let sorted = filter_and_sort_workouts(&all, &base(), None);
        assert!(sorted[0].date > sorted[1].date);
        let asc = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                dir: SortDir::Asc,
                ..base()
            },
            None,
        );
        assert!(asc[0].date < asc[1].date);

        let by_distance = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                sort: WorkoutSortField::Distance,
                dir: SortDir::Asc,
                sport: Some(Sport::Rower),
                ..base()
            },
            None,
        );
        assert!(by_distance[0].distance <= by_distance[1].distance);
        let by_time = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                sort: WorkoutSortField::Time,
                dir: SortDir::Asc,
                sport: Some(Sport::Rower),
                ..base()
            },
            None,
        );
        assert!(by_time[0].time <= by_time[1].time);

        let duration = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                duration_min: Some(1000.0),
                duration_max: Some(1400.0),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&duration), vec![2]);

        let with_type = [
            all[0].clone(),
            Workout {
                workout_type: Some("JustRow".into()),
                ..workout(6)
            },
        ];
        let by_type = filter_and_sort_workouts(
            &with_type,
            &WorkoutListQuery {
                workout_type: Some("JustRow".into()),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&by_type), vec![6]);

        let taggable = [
            Workout::new(6, "2026-05-01 06:00:00", Sport::Rower, 500.0, 100.0, 100.0),
            Workout::new(
                7,
                "2026-05-01 06:00:00",
                Sport::Rower,
                30_000.0,
                2400.0,
                120.0,
            ),
        ];
        let race = filter_and_sort_workouts(
            &taggable,
            &WorkoutListQuery {
                tag: Some(WorkoutTag::RacePiece),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&race), vec![6]);

        let provided = BTreeSet::from([1]);
        let pbs = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                pbs_only: true,
                ..base()
            },
            Some(&provided),
        );
        assert_eq!(ids(&pbs), vec![1]);
        let none = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                pbs_only: true,
                ..base()
            },
            Some(&BTreeSet::new()),
        );
        assert!(none.is_empty());

        let banded = filter_and_sort_workouts(
            &all,
            &WorkoutListQuery {
                distance_band_key: Some("5000".into()),
                ..base()
            },
            None,
        );
        assert_eq!(ids(&banded), vec![2]);
    }

    #[test]
    fn pb_workout_ids_cases() {
        let faster = workout(1);
        let slower = Workout {
            time: 500.0,
            ..workout(2)
        };
        let ids = pb_workout_ids(&[faster, slower], None);
        assert!(ids.contains(&1) && !ids.contains(&2));

        let w5k = Workout {
            distance: 5000.0,
            time: 1200.0,
            ..workout(2)
        };
        let ids = pb_workout_ids(&[workout(1), w5k], None);
        assert!(ids.contains(&1) && ids.contains(&2));

        assert!(
            pb_workout_ids(
                &[Workout {
                    distance: 2020.0,
                    ..workout(1)
                }],
                None
            )
            .contains(&1)
        );
        assert!(
            !pb_workout_ids(
                &[Workout {
                    distance: 2100.0,
                    ..workout(1)
                }],
                None
            )
            .contains(&1)
        );

        let rower = workout(1);
        let bike = Workout {
            sport: Sport::Bike,
            time: 460.0,
            ..workout(2)
        };
        let ids = pb_workout_ids(&[rower.clone(), bike.clone()], Some(Sport::Rower));
        assert!(ids.contains(&1) && !ids.contains(&2));
        // Unfiltered, the fastest workout across sports wins (web list semantics).
        assert_eq!(pb_workout_ids(&[rower, bike], None), BTreeSet::from([2]));
        assert!(pb_workout_ids(&[], None).is_empty());
    }

    #[test]
    fn avg_power_watts_cases() {
        let w = Workout {
            watt_minutes: Some(80.0),
            ..workout(1)
        };
        assert!((avg_power_watts(&w).unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(avg_power_watts(&workout(1)), None);
        let zero = Workout {
            watt_minutes: Some(80.0),
            time: 0.0,
            ..workout(1)
        };
        assert_eq!(avg_power_watts(&zero), None);
        let w = Workout {
            watt_minutes: Some(200.0),
            time: 400.0,
            ..workout(1)
        };
        assert!((avg_power_watts(&w).unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn chip_toggles() {
        let on = toggle_distance_chip(&base(), 2000.0);
        assert_eq!(on.distance_m, Some(2000.0));
        assert_eq!(toggle_distance_chip(&on, 2000.0).distance_m, None);
        let banded = WorkoutListQuery {
            distance_band_key: Some("r2000".into()),
            ..base()
        };
        assert_eq!(
            toggle_distance_chip(&banded, 2000.0).distance_band_key,
            None
        );

        let on = toggle_duration_chip(&base(), 1800.0);
        assert_eq!(
            (on.duration_min, on.duration_max),
            (Some(1620.0), Some(1980.0))
        );
        assert_eq!(on.distance_m, None);
        assert!(duration_chip_active(&on, 1800.0));
        assert!(!duration_chip_active(&on, 3600.0));
        assert!(!duration_chip_active(&base(), 1800.0));
        let off = toggle_duration_chip(&on, 1800.0);
        assert_eq!((off.duration_min, off.duration_max), (None, None));
    }

    // --- Studio WorkoutQueryTests -------------------------------------------

    #[test]
    fn sorts_by_pace_and_power() {
        let a = Workout {
            pace: 130.0,
            ..workout(1)
        };
        let b = Workout {
            pace: 110.0,
            ..workout(2)
        };
        let list = vec![a, b];
        let asc = filter_and_sort_workouts(
            &list,
            &WorkoutListQuery {
                sort: WorkoutSortField::Pace,
                dir: SortDir::Asc,
                ..base()
            },
            None,
        );
        assert_eq!(ids(&asc), vec![2, 1]);
        let desc = filter_and_sort_workouts(
            &list,
            &WorkoutListQuery {
                sort: WorkoutSortField::Pace,
                dir: SortDir::Desc,
                ..base()
            },
            None,
        );
        assert_eq!(ids(&desc), vec![1, 2]);

        let low = Workout {
            time: 400.0,
            watt_minutes: Some(100.0),
            ..workout(1)
        };
        let high = Workout {
            time: 400.0,
            watt_minutes: Some(200.0),
            ..workout(2)
        };
        let list = vec![low, high];
        let desc = filter_and_sort_workouts(
            &list,
            &WorkoutListQuery {
                sort: WorkoutSortField::Power,
                dir: SortDir::Desc,
                ..base()
            },
            None,
        );
        assert_eq!(ids(&desc), vec![2, 1]);
        let asc = filter_and_sort_workouts(
            &list,
            &WorkoutListQuery {
                sort: WorkoutSortField::Power,
                dir: SortDir::Asc,
                ..base()
            },
            None,
        );
        assert_eq!(ids(&asc), vec![1, 2]);
    }

    #[test]
    fn pbs_only_respects_sport_filter_and_whitespace_search_does_not_bypass_filters() {
        let list = vec![
            Workout {
                time: 400.0,
                ..workout(1)
            },
            Workout {
                sport: Sport::Skierg,
                time: 430.0,
                ..workout(2)
            },
            Workout {
                sport: Sport::Skierg,
                time: 460.0,
                ..workout(3)
            },
        ];
        let result = filter_and_sort_workouts(
            &list,
            &WorkoutListQuery {
                sport: Some(Sport::Skierg),
                pbs_only: true,
                ..base()
            },
            None,
        );
        assert_eq!(ids(&result), vec![2]);

        let list = vec![
            Workout {
                time: 400.0,
                ..workout(1)
            },
            Workout {
                time: 600.0,
                ..workout(2)
            },
        ];
        let q = WorkoutListQuery {
            q: Some(" \n\t ".into()),
            pbs_only: true,
            duration_min: Some(500.0),
            ..base()
        };
        assert!(filter_and_sort_workouts(&list, &q, None).is_empty());
    }

    #[test]
    fn clear_filters_preserves_sort() {
        let q = WorkoutListQuery {
            sport: Some(Sport::Rower),
            pbs_only: true,
            sort: WorkoutSortField::Pace,
            dir: SortDir::Asc,
            ..base()
        };
        let cleared = clear_filters(&q);
        assert_eq!(cleared.sport, None);
        assert!(!cleared.pbs_only);
        assert_eq!(
            (cleared.sort, cleared.dir),
            (WorkoutSortField::Pace, SortDir::Asc)
        );
        assert!(filter_and_sort_workouts(&[], &base(), None).is_empty());
    }
}
