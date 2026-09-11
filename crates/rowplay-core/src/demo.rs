// SPDX-License-Identifier: GPL-3.0-or-later
//! Deterministic demo data so rowplay-qt is fully explorable without a Concept2
//! token.
//!
//! Port of the web app's `src/lib/mockData.ts`: strokes are synthesised with
//! realistic pacing (warm-up, steady state, negative split, sprint finish) plus
//! a little noise from a seeded LCG, one row per stroke cycle. The output is
//! byte-for-byte deterministic across platforms. Live-mode workout generation
//! (`generateMockWorkout`) belongs to Phase 8.

use crate::formatting::pace_to_watts_for_sport;
use crate::models::{
    HeartRateDetail, LoggingMetadata, Split, SplitIntervalType, Sport, Stroke, WeightClass,
    Workout, WorkoutDetail, WorkoutTargets,
};
use crate::num::{avg, js_round, round1};

/// The workout the app opens on in demo mode (the latest RowErg 2k test).
pub const DEFAULT_WORKOUT_ID: i64 = 1001;

#[derive(Debug, Clone, Copy)]
struct Spec {
    id: i64,
    date: &'static str,
    sport: Sport,
    distance: f64,
    /// sec/500m
    base_pace: f64,
    base_spm: f64,
    base_hr: f64,
    workout_type: &'static str,
    comments: Option<&'static str>,
    interval: bool,
    source: Option<&'static str>,
    /// Demo-only: erg logged without HR — exercises device import.
    omit_hr: bool,
    /// IANA zone for calendar bucketing demos (cross-timezone fixture).
    timezone: Option<&'static str>,
    /// Skip stroke synthesis for lightweight calendar fixtures.
    no_strokes: bool,
    /// Demo-only: simulate the Concept2 `privacy` level (defaults to `everyone`).
    privacy: Option<&'static str>,
}

const fn spec(
    id: i64,
    date: &'static str,
    sport: Sport,
    distance: f64,
    base_pace: f64,
    base_spm: f64,
    base_hr: f64,
    workout_type: &'static str,
) -> Spec {
    Spec {
        id,
        date,
        sport,
        distance,
        base_pace,
        base_spm,
        base_hr,
        workout_type,
        comments: None,
        interval: false,
        source: None,
        omit_hr: false,
        timezone: None,
        no_strokes: false,
        privacy: None,
    }
}

const SPECS: [Spec; 17] = [
    Spec {
        comments: Some("PB attempt — held on for the sprint."),
        ..spec(
            1001,
            "2026-05-27 06:12:00",
            Sport::Rower,
            2000.0,
            108.0,
            30.0,
            168.0,
            "2000m test",
        )
    },
    Spec {
        omit_hr: true,
        privacy: Some("private"),
        ..spec(
            1002,
            "2026-05-24 07:05:00",
            Sport::Rower,
            5000.0,
            118.0,
            26.0,
            158.0,
            "5000m steady",
        )
    },
    spec(
        1003,
        "2026-05-21 18:40:00",
        Sport::Skierg,
        1000.0,
        122.0,
        42.0,
        165.0,
        "1000m SkiErg",
    ),
    Spec {
        source: Some("EXR"),
        ..spec(
            1004,
            "2026-05-19 06:30:00",
            Sport::Bike,
            8000.0,
            95.0,
            85.0,
            150.0,
            "8000m BikeErg",
        )
    },
    Spec {
        interval: true,
        ..spec(
            1005,
            "2026-05-16 06:20:00",
            Sport::Rower,
            6000.0,
            116.0,
            28.0,
            160.0,
            "4x1500m intervals",
        )
    },
    spec(
        1006,
        "2026-05-13 18:15:00",
        Sport::Rower,
        500.0,
        96.0,
        36.0,
        172.0,
        "500m sprint",
    ),
    spec(
        1007,
        "2026-05-10 06:18:00",
        Sport::Rower,
        2000.0,
        112.0,
        29.0,
        166.0,
        "2000m steady",
    ),
    spec(
        1008,
        "2026-05-06 07:00:00",
        Sport::Skierg,
        1000.0,
        126.0,
        40.0,
        162.0,
        "1000m SkiErg",
    ),
    // Extra 2k pieces so the like-for-like trend shows a clear progression.
    spec(
        1009,
        "2026-04-29 06:25:00",
        Sport::Rower,
        2000.0,
        113.0,
        28.0,
        167.0,
        "2000m test",
    ),
    spec(
        1010,
        "2026-04-22 06:30:00",
        Sport::Rower,
        2000.0,
        115.0,
        28.0,
        168.0,
        "2000m test",
    ),
    spec(
        1011,
        "2026-04-15 06:28:00",
        Sport::Rower,
        2000.0,
        117.0,
        27.0,
        169.0,
        "2000m test",
    ),
    Spec {
        timezone: Some("America/New_York"),
        no_strokes: true,
        ..spec(
            9001,
            "2024-01-14 23:30:00",
            Sport::Rower,
            5000.0,
            126.0,
            28.0,
            160.0,
            "5000m steady",
        )
    },
    // Fixed-time piece for comparability-guard demo (ghost/compare block vs 2k distance pieces).
    spec(
        1012,
        "2026-04-08 06:00:00",
        Sport::Rower,
        7500.0,
        120.0,
        26.0,
        160.0,
        "JustRow",
    ),
    spec(
        1013,
        "2026-04-05 06:00:00",
        Sport::Rower,
        1000.0,
        170.0,
        18.0,
        118.0,
        "Warm-up",
    ),
    spec(
        1014,
        "2026-04-03 06:00:00",
        Sport::Rower,
        3500.0,
        132.0,
        24.0,
        145.0,
        "Technique drills",
    ),
    spec(
        1015,
        "2026-03-28 07:10:00",
        Sport::Skierg,
        5000.0,
        132.0,
        38.0,
        155.0,
        "5000m SkiErg steady",
    ),
    spec(
        1016,
        "2026-03-22 07:10:00",
        Sport::Bike,
        2000.0,
        88.0,
        92.0,
        165.0,
        "2000m BikeErg time trial",
    ),
];

/// Small deterministic PRNG so demo data is stable across reloads (web `rng`).
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(self.0) / f64::from(u32::MAX)
    }
}

/// Multiplier on base pace (lower = faster): warm-up slow, steady, sprint finish.
fn pace_profile(frac: f64) -> f64 {
    if frac < 0.08 {
        return 1.06 - frac; // ease in
    }
    if frac > 0.9 {
        return 0.9 - (frac - 0.9) * 0.6; // sprint
    }
    1.0 - frac * 0.06 // gentle negative split
}

fn build_strokes(spec: &Spec) -> (Vec<Stroke>, f64) {
    let mut rand = Lcg(spec.id as u32);
    let mut strokes = Vec::new();
    let mut d = 0.0;
    let mut t = 0.0;
    while d < spec.distance {
        let frac = d / spec.distance;
        let noise = (rand.next() - 0.5) * 4.0;
        let pace = (spec.base_pace * pace_profile(frac) + noise).max(70.0);
        let speed = 500.0 / pace; // m/s
        let spm = js_round(
            spec.base_spm + if frac > 0.9 { 4.0 } else { 0.0 } + (rand.next() - 0.5) * 2.0,
        )
        .max(1.0);
        // Concept2 stroke rows represent completed cycles, not evenly spaced
        // distance samples. This keeps the demo timeline truthful at both rowing
        // cadence and BikeErg rpm.
        let dt = 60.0 / spm;
        t += dt;
        d += speed * dt;
        let mut stroke = Stroke::new(
            round1(t),
            round1(d.min(spec.distance)),
            round1(pace),
            spm,
            js_round(pace_to_watts_for_sport(spec.sport, pace)),
        );
        if !spec.omit_hr {
            let hr = (spec.base_hr * (0.8 + frac * 0.22) + (rand.next() - 0.5) * 3.0).min(192.0);
            stroke.hr = Some(js_round(hr));
        }
        strokes.push(stroke);
    }
    (strokes, t)
}

fn build_splits(spec: &Spec, strokes: &[Stroke]) -> Vec<Split> {
    let n = if spec.distance >= 5000.0 {
        js_round(spec.distance / 1000.0) as usize
    } else {
        4
    };
    let seg = spec.distance / n as f64;
    let mut splits = Vec::new();
    for i in 0..n {
        let start_d = i as f64 * seg;
        let end_d = (i + 1) as f64 * seg;
        let within: Vec<&Stroke> = strokes
            .iter()
            .filter(|s| s.d > start_d && s.d <= end_d)
            .collect();
        let Some(last) = within.last() else {
            continue;
        };
        let previous_end = if i == 0 {
            0.0
        } else {
            strokes
                .iter()
                .rfind(|s| s.d <= start_d)
                .map_or(0.0, |s| s.t)
        };
        let seg_time = last.t - previous_end;
        let pace = if seg_time > 0.0 {
            seg_time / (seg / 500.0)
        } else {
            spec.base_pace
        };
        let spm_values: Vec<f64> = within.iter().map(|s| s.spm).collect();
        let mut split = Split::new(i as u32, js_round(seg), round1(seg_time), round1(pace));
        split.spm = Some(js_round(avg(&spm_values)));
        if !spec.omit_hr {
            let hr_values: Vec<f64> = within.iter().map(|s| s.hr.unwrap_or(0.0)).collect();
            split.hr = Some(js_round(avg(&hr_values)));
        }
        splits.push(split);
    }
    splits
}

fn time_for_distance(metres: f64, pace_sec_per_500: f64) -> f64 {
    (metres / 500.0) * pace_sec_per_500
}

/// Demo-only: exercise full-fidelity fields on selected fixtures.
fn apply_full_fidelity_demo(spec: &Spec, detail: &mut WorkoutDetail) {
    if spec.id == 1005 && spec.interval {
        let rep_dist = 1500.0;
        let rep_time = round1(time_for_distance(rep_dist, spec.base_pace));
        let rest_time = 90.0;
        detail.splits.clear();
        for i in 0..4u32 {
            let index = detail.splits.len() as u32;
            detail.splits.push(Split {
                spm: Some(spec.base_spm),
                hr: Some(spec.base_hr + f64::from(i)),
                heart_rate: Some(HeartRateDetail {
                    average: Some(spec.base_hr + f64::from(i)),
                    ending: Some(spec.base_hr + f64::from(i) + 4.0),
                    ..HeartRateDetail::default()
                }),
                calories_total: Some(95.0 + f64::from(i) * 2.0),
                watt_minutes: Some(38.0 + f64::from(i)),
                interval_type: Some(SplitIntervalType::Distance),
                is_rest: Some(false),
                ..Split::new(index, rep_dist, rep_time, spec.base_pace)
            });
            if i < 3 {
                let index = detail.splits.len() as u32;
                detail.splits.push(Split {
                    heart_rate: Some(HeartRateDetail {
                        rest: Some(118.0),
                        recovery: Some(112.0),
                        ..HeartRateDetail::default()
                    }),
                    is_rest: Some(true),
                    ..Split::new(index, 0.0, rest_time, 0.0)
                });
            }
        }
        let w = &mut detail.workout;
        w.rest_time = Some(rest_time * 3.0);
        w.rest_distance = Some(0.0);
        w.verified = Some(true);
        w.weight_class = Some(WeightClass::H);
        w.targets = Some(WorkoutTargets {
            stroke_rate: Some(28.0),
            pace: Some(spec.base_pace + 2.0),
            watts: Some(210.0),
            ..WorkoutTargets::default()
        });
        let heart_rate = HeartRateDetail {
            average: Some(spec.base_hr),
            min: Some(142.0),
            max: Some(178.0),
            ending: Some(172.0),
            recovery: Some(128.0),
            rest: None,
        };
        w.heart_rate = Some(heart_rate);
        w.heart_rate_avg = heart_rate.average;
        w.hr_min = heart_rate.min;
        w.hr_max = heart_rate.max;
        w.watt_minutes = Some(168.0);
        w.metadata = Some(LoggingMetadata {
            pm_version: Some(5.0),
            firmware_version: Some("707".into()),
            serial_number: Some("DEMO-SN-1005".into()),
            device: Some("Demo iPhone".into()),
            device_os: Some("iOS".into()),
            device_os_version: None,
            erg_model_type: Some(0.0),
            hr_type: Some("BT".into()),
        });
    }
    if spec.id == 1007 {
        let heart_rate = HeartRateDetail {
            average: Some(spec.base_hr),
            min: Some(145.0),
            max: Some(174.0),
            ending: Some(170.0),
            recovery: Some(122.0),
            rest: None,
        };
        let w = &mut detail.workout;
        w.heart_rate = Some(heart_rate);
        w.heart_rate_avg = heart_rate.average;
        w.hr_min = heart_rate.min;
        w.hr_max = heart_rate.max;
    }
    if spec.id == 1014 {
        detail.splits = vec![
            Split {
                spm: Some(30.0),
                ..Split::new(0, 1000.0, 220.0, 110.0)
            },
            Split {
                spm: Some(22.0),
                ..Split::new(1, 1000.0, 270.0, 135.0)
            },
            Split {
                spm: Some(30.0),
                ..Split::new(2, 1000.0, 220.0, 110.0)
            },
            Split {
                spm: Some(20.0),
                ..Split::new(3, 500.0, 138.0, 138.0)
            },
        ];
    }
}

fn detail_for(spec: &Spec) -> WorkoutDetail {
    let (strokes, time) = if spec.no_strokes {
        (Vec::new(), time_for_distance(spec.distance, spec.base_pace))
    } else {
        build_strokes(spec)
    };
    let splits = if spec.no_strokes {
        Vec::new()
    } else {
        build_splits(spec, &strokes)
    };
    let pace = time / (spec.distance / 500.0);
    let mut workout = Workout::new(
        spec.id,
        spec.date,
        spec.sport,
        spec.distance,
        round1(time),
        round1(pace),
    );
    workout.stroke_rate = Some(if strokes.is_empty() {
        spec.base_spm
    } else {
        js_round(avg(&strokes.iter().map(|s| s.spm).collect::<Vec<_>>()))
    });
    workout.stroke_count = Some(strokes.len() as f64);
    workout.calories_total = Some(js_round((time / 60.0) * 12.0));
    workout.drag_factor = match spec.sport {
        Sport::Rower => Some(130.0),
        Sport::Skierg => Some(110.0),
        Sport::Bike => None,
    };
    workout.workout_type = Some(spec.workout_type.to_owned());
    workout.comments = spec.comments.map(str::to_owned);
    workout.has_stroke_data = !spec.no_strokes;
    workout.is_interval = spec.interval;
    workout.timezone = spec.timezone.map(str::to_owned);
    if !spec.omit_hr {
        workout.heart_rate_avg = Some(js_round(avg(&strokes
            .iter()
            .map(|s| s.hr.unwrap_or(0.0))
            .collect::<Vec<_>>())));
    }
    workout.source = spec.source.map(str::to_owned);
    // Demo workouts simulate a public Concept2 privacy level so the share flow
    // works out of the box; specific specs override to exercise the block path.
    workout.privacy = Some(spec.privacy.unwrap_or("everyone").to_owned());
    let mut detail = WorkoutDetail {
        workout,
        strokes,
        splits,
    };
    apply_full_fidelity_demo(spec, &mut detail);
    detail
}

/// Every demo workout summary, newest first (web `mockWorkouts`).
#[must_use]
pub fn mock_workouts() -> Vec<Workout> {
    let mut list: Vec<Workout> = SPECS
        .iter()
        .map(|spec| detail_for(spec).summary())
        .collect();
    list.sort_by(|a, b| b.date.cmp(&a.date));
    list
}

/// Full detail for a demo workout id, or `None` for an unknown id (web `mockWorkoutDetail`).
#[must_use]
pub fn mock_workout_detail(id: i64) -> Option<WorkoutDetail> {
    SPECS.iter().find(|spec| spec.id == id).map(detail_for)
}

/// Every demo workout detail, newest first (Studio `DemoWorkoutLibrary.details`).
#[must_use]
pub fn demo_details() -> Vec<WorkoutDetail> {
    let mut details: Vec<WorkoutDetail> = SPECS.iter().map(detail_for).collect();
    details.sort_by(|a, b| b.workout.date.cmp(&a.workout.date));
    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formatting::BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR;

    // --- web mockData.test.ts -----------------------------------------------

    #[test]
    fn returns_a_stable_non_empty_multi_sport_list() {
        let workouts = mock_workouts();
        assert!(!workouts.is_empty());
        for w in &workouts {
            assert!(w.distance > 0.0 && w.time > 0.0 && w.pace > 0.0, "{}", w.id);
            assert!(!w.date.is_empty());
        }
        let again = mock_workouts();
        assert_eq!(
            workouts.iter().map(|w| w.id).collect::<Vec<_>>(),
            again.iter().map(|w| w.id).collect::<Vec<_>>()
        );
        assert_eq!(workouts[0].time, again[0].time);
        assert_eq!(workouts, again);
        let sports: std::collections::BTreeSet<Sport> = workouts.iter().map(|w| w.sport).collect();
        assert!(sports.len() > 1);
        assert!(workouts.iter().any(|w| w.has_stroke_data));
        assert!(workouts.windows(2).all(|pair| pair[0].date >= pair[1].date));
    }

    #[test]
    fn details_exist_for_every_workout_and_not_for_unknown_ids() {
        for w in mock_workouts() {
            let detail = mock_workout_detail(w.id).expect("detail");
            assert_eq!(detail.summary(), w, "summary of {} matches the list", w.id);
        }
        assert!(mock_workout_detail(99_999).is_none());
    }

    #[test]
    fn strokes_are_finite_and_present_when_flagged() {
        let with_stroke = mock_workouts()
            .into_iter()
            .find(|w| w.has_stroke_data)
            .unwrap();
        assert!(
            !mock_workout_detail(with_stroke.id)
                .unwrap()
                .strokes
                .is_empty()
        );
        let detail = mock_workout_detail(1001).unwrap();
        for s in &detail.strokes {
            assert!(
                s.t.is_finite()
                    && s.d.is_finite()
                    && s.pace.is_finite()
                    && s.spm.is_finite()
                    && s.watts.is_finite()
            );
        }
        assert!(detail.workout.distance > 0.0 && detail.workout.time > 0.0);
    }

    #[test]
    fn generates_one_bikeerg_stroke_row_per_cadence_cycle() {
        let detail = mock_workout_detail(1004).unwrap();
        assert_eq!(detail.workout.sport, Sport::Bike);
        assert!(detail.strokes.len() > 1000);
        let mut previous_t = 0.0;
        for stroke in &detail.strokes {
            let dt = stroke.t - previous_t;
            // Stored timestamps are rounded to tenths, so two adjacent rounding
            // errors can move the observed interval by at most 0.1 s.
            assert!((dt - 60.0 / stroke.spm).abs() <= 0.11);
            previous_t = stroke.t;
        }
        let observed_cadence = detail.strokes.len() as f64 / (detail.workout.time / 60.0);
        assert!(observed_cadence > 80.0 && observed_cadence < 95.0);
    }

    #[test]
    fn interval_workout_has_full_fidelity_splits() {
        let detail = mock_workout_detail(1005).unwrap();
        assert_eq!(detail.splits.len(), 7);
        assert_eq!(detail.splits[1].is_rest, Some(true));
        assert_eq!(detail.workout.rest_time, Some(270.0));
        assert_eq!(detail.workout.weight_class, Some(WeightClass::H));
        assert_eq!(
            detail
                .workout
                .metadata
                .as_ref()
                .and_then(|m| m.serial_number.clone())
                .as_deref(),
            Some("DEMO-SN-1005")
        );
        assert_eq!(detail.workout.targets.map(|t| t.watts), Some(Some(210.0)));
        assert!(detail.workout.is_interval);
        let technique = mock_workout_detail(1014).unwrap();
        assert_eq!(technique.splits.len(), 4);
        assert_eq!(technique.splits[3].distance, 500.0);
        let steady = mock_workout_detail(1007).unwrap();
        assert_eq!(steady.workout.hr_max, Some(174.0));
    }

    // --- Studio DemoWorkoutLibraryTests -------------------------------------

    #[test]
    fn demo_library_matches_web_seed_shape() {
        let details = demo_details();
        assert_eq!(details.len(), 17);
        assert_eq!(details[0].id(), DEFAULT_WORKOUT_ID);
        assert_eq!(details[0].workout.sport, Sport::Rower);
        assert!(details[0].workout.has_stroke_data);
        assert!(details[0].strokes.len() >= 200);
        assert!((details[0].strokes.last().unwrap().d - 2000.0).abs() <= 0.1);
        assert_eq!(details[0].workout.privacy.as_deref(), Some("everyone"));
        assert_eq!(
            mock_workout_detail(1002)
                .unwrap()
                .workout
                .privacy
                .as_deref(),
            Some("private")
        );
        assert_eq!(
            mock_workout_detail(1002).unwrap().workout.heart_rate_avg,
            None
        );
    }

    #[test]
    fn no_stroke_fixture_keeps_summary_fields_only() {
        let detail = mock_workout_detail(9001).unwrap();
        assert!(!detail.workout.has_stroke_data);
        assert!(detail.strokes.is_empty());
        assert!(
            detail.splits.is_empty(),
            "the web app synthesises no splits without strokes"
        );
        assert_eq!(detail.workout.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(detail.workout.time, 1260.0);
        assert_eq!(detail.workout.stroke_rate, Some(28.0));
    }

    #[test]
    fn bike_watts_use_the_concept2_normalised_pace_divisor() {
        let rower = pace_to_watts_for_sport(Sport::Rower, 100.0);
        let bike = pace_to_watts_for_sport(Sport::Bike, 100.0);
        assert!((bike - rower / BIKE_WATTS_FROM_NORMALIZED_PACE_DIVISOR).abs() < 1e-9);
        let detail = mock_workout_detail(1016).unwrap();
        assert!(detail.strokes.iter().all(|s| s.watts < 200.0));
    }

    #[test]
    fn lcg_matches_the_web_generator() {
        // First three draws of rng(1001) in the web app: (s * 1664525 + 1013904223) >>> 0, divided by 0xffffffff.
        let mut rng = Lcg(1001);
        let mut state: u32 = 1001;
        for _ in 0..3 {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            assert_eq!(rng.next(), f64::from(state) / 4_294_967_295.0);
        }
    }
}
