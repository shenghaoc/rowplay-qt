// SPDX-License-Identifier: GPL-3.0-or-later
//! Performance budget for the sidebar (Phase 4 ground rule 4): with a
//! synthetic 5,000-workout library, re-filtering stays under 50 ms and the
//! full display-string rebuild stays well inside a dropped-frame-free budget.
//!
//! The library is generated deterministically (an LCG over the demo
//! generator's value ranges) so timings are comparable across runs. The test
//! prints its measurements for the PR report; run with `--nocapture`.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use rowplay_core::models::{DistanceUnit, Sport, Workout};
use rowplay_core::workout_query::{
    WorkoutListQuery, WorkoutSortField, filter_and_sort_workouts, pb_workout_ids,
};
use rowplay_viewmodel::library::sidebar_rows;
use rowplay_viewmodel::settings::Language;

const LIBRARY_SIZE: usize = 5_000;
const FILTER_BUDGET: Duration = Duration::from_millis(50);
const RENDER_BUDGET: Duration = Duration::from_millis(250);

/// Deterministic xorshift-ish generator (no rand dependency).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        // Numerical Recipes LCG constants.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn synthetic_library() -> Vec<Workout> {
    let mut rng = Lcg(0x5eed_2024);
    let sports = [Sport::Rower, Sport::Skierg, Sport::Bike];
    let types = ["Steady", "Intervals", "Test", "Race", "Warmup", "Long row"];
    let mut workouts = Vec::with_capacity(LIBRARY_SIZE);
    // Ten years back from a fixed date, newest first (like the logbook).
    let base_day = 9_200; // days since epoch ~2026-03
    for index in 0..LIBRARY_SIZE {
        let day = base_day - (index as i64 * 740) / LIBRARY_SIZE as i64;
        let date = day_key(day);
        let sport = sports[rng.below(3) as usize];
        let distance = match rng.below(6) {
            0 => 500.0 + rng.below(50) as f64,
            1 => 2_000.0 + rng.below(100) as f64,
            2 => 5_000.0 + rng.below(300) as f64,
            3 => 10_000.0 + rng.below(500) as f64,
            4 => 21_097.0 + rng.below(50) as f64,
            _ => 1_000.0 * (1 + rng.below(20)) as f64,
        };
        let base_pace = match sport {
            Sport::Rower => 110.0,
            Sport::Skierg => 105.0,
            Sport::Bike => 100.0,
        };
        let pace = base_pace + rng.below(60) as f64 * 0.7;
        let time = pace * distance / 500.0;
        let mut workout = Workout::new(
            (1_000_000 - index) as i64,
            format!("{date} {:02}:{:02}:00", 5 + rng.below(16), rng.below(60)),
            sport,
            distance,
            time,
            pace,
        );
        workout.workout_type = Some(types[rng.below(types.len() as u64) as usize].to_owned());
        workout.stroke_rate = Some(18.0 + rng.below(16) as f64);
        if rng.below(3) == 0 {
            workout.comments = Some(format!("session {}", rng.below(1_000)));
        }
        if rng.below(4) == 0 {
            workout.heart_rate_avg = Some(120.0 + rng.below(60) as f64);
        }
        workouts.push(workout);
    }
    workouts
}

fn day_key(days_since_epoch: i64) -> String {
    // Civil-from-days algorithm (Howard Hinnant), good enough for fixtures.
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{m:02}-{d:02}", if m <= 2 { y } else { y + 1 })
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort();
    samples[samples.len() / 2]
}

#[test]
fn filtering_a_5000_workout_library_stays_under_50ms() {
    let workouts = synthetic_library();
    assert_eq!(workouts.len(), LIBRARY_SIZE);
    let pb_ids = pb_workout_ids(&workouts, None);

    // Representative re-filter operations: the query engine plus the PB set.
    let queries: Vec<(&str, WorkoutListQuery)> = vec![
        ("default (date desc)", WorkoutListQuery::default()),
        (
            "sport filter",
            WorkoutListQuery {
                sport: Some(Sport::Skierg),
                ..WorkoutListQuery::default()
            },
        ),
        (
            "free text",
            WorkoutListQuery {
                q: Some("session 4".to_owned()),
                ..WorkoutListQuery::default()
            },
        ),
        (
            "pace ascending",
            WorkoutListQuery {
                sort: WorkoutSortField::Pace,
                ..WorkoutListQuery::default()
            },
        ),
        (
            "date range + distance chip",
            WorkoutListQuery {
                date_from: Some(day_key(8_000)),
                date_to: Some(day_key(9_000)),
                distance_m: Some(2_000.0),
                ..WorkoutListQuery::default()
            },
        ),
    ];

    for (name, query) in &queries {
        let mut samples = Vec::new();
        for _ in 0..7 {
            let started = Instant::now();
            let filtered = filter_and_sort_workouts(&workouts, query, Some(&pb_ids));
            samples.push(started.elapsed());
            assert!(!filtered.is_empty() || name.contains("free text"));
        }
        let med = median(samples);
        eprintln!("filter [{name}]: median {med:?} over {LIBRARY_SIZE} workouts");
        assert!(
            med < FILTER_BUDGET,
            "filter [{name}] took {med:?}, budget is {FILTER_BUDGET:?}"
        );
    }
}

#[test]
fn rendering_5000_sidebar_rows_stays_interactive() {
    let workouts = synthetic_library();
    let pb_ids: BTreeSet<i64> = pb_workout_ids(&workouts, None);
    let query = WorkoutListQuery::default();

    let mut samples = Vec::new();
    let mut row_count = 0;
    for _ in 0..5 {
        let started = Instant::now();
        let rows = sidebar_rows(
            &workouts,
            &query,
            &pb_ids,
            DistanceUnit::Metric,
            Language::En,
            None,
        );
        samples.push(started.elapsed());
        row_count = rows.len();
    }
    let med = median(samples);
    eprintln!("sidebar render (filter + all display strings): median {med:?} for {row_count} rows");
    assert_eq!(row_count, LIBRARY_SIZE);
    assert!(
        med < RENDER_BUDGET,
        "full sidebar rebuild took {med:?}, budget is {RENDER_BUDGET:?}"
    );
}
