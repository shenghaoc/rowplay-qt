// SPDX-License-Identifier: GPL-3.0-or-later
//! Automatic workout classification (steady state, interval, race piece, …).
//!
//! Port of the web app's `src/lib/workoutTag.ts`. Tags are derived from the
//! split / interval structure and the athlete's median pace; nothing is
//! persisted and no network is involved.

use serde::{Deserialize, Serialize};

use crate::models::{Split, Workout};

/// Every tag, in the web app's declaration order.
pub const WORKOUT_TAGS: [WorkoutTag; 6] = [
    WorkoutTag::SteadyState,
    WorkoutTag::Interval,
    WorkoutTag::RacePiece,
    WorkoutTag::TimeTrial,
    WorkoutTag::WarmupCooldown,
    WorkoutTag::Unknown,
];

/// Auto-detected workout type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkoutTag {
    /// Long, low-variance piece.
    SteadyState,
    /// Work/rest structure.
    Interval,
    /// Short hard piece.
    RacePiece,
    /// Mid-duration low-variance piece.
    TimeTrial,
    /// Short easy piece relative to the athlete's median pace.
    WarmupCooldown,
    /// No rule matched.
    Unknown,
}

impl WorkoutTag {
    /// Wire spelling (`steady-state`, `interval`, …).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            WorkoutTag::SteadyState => "steady-state",
            WorkoutTag::Interval => "interval",
            WorkoutTag::RacePiece => "race-piece",
            WorkoutTag::TimeTrial => "time-trial",
            WorkoutTag::WarmupCooldown => "warmup-cooldown",
            WorkoutTag::Unknown => "unknown",
        }
    }

    /// Parse the wire spelling.
    #[must_use]
    pub fn parse(value: &str) -> Option<WorkoutTag> {
        WORKOUT_TAGS.into_iter().find(|tag| tag.as_str() == value)
    }
}

impl std::fmt::Display for WorkoutTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Whether a string is a known tag (web `isValidWorkoutTag`).
#[must_use]
pub fn is_valid_workout_tag(tag: Option<&str>) -> bool {
    tag.is_some_and(|value| WorkoutTag::parse(value).is_some())
}

/// Context shared by the tag rules.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TagContext {
    /// Athlete median pace (sec/500 m); rules that need it are skipped when absent.
    pub median_pace_secs: Option<f64>,
}

/// Median pace across a workout list — the athlete baseline for tag rules.
#[must_use]
pub fn athlete_median_pace(workouts: &[Workout]) -> Option<f64> {
    let mut paces: Vec<f64> = workouts
        .iter()
        .map(|w| w.pace)
        .filter(|p| *p > 0.0)
        .collect();
    if paces.is_empty() {
        return None;
    }
    paces.sort_by(f64::total_cmp);
    let mid = paces.len() / 2;
    Some(if paces.len() % 2 == 1 {
        paces[mid]
    } else {
        (paces[mid - 1] + paces[mid]) / 2.0
    })
}

fn is_rest_split(split: &Split) -> bool {
    split.is_rest == Some(true) || (split.distance == 0.0 && split.time > 0.0)
}

fn count_work_and_rest(workout: &Workout, splits: Option<&[Split]>) -> (usize, usize) {
    let Some(splits) = splits.filter(|s| !s.is_empty()) else {
        if workout.rest_time.unwrap_or(0.0) > 0.0 || workout.is_interval {
            return (2, 1);
        }
        return (1, 0);
    };
    let mut work = 0;
    let mut rest = 0;
    for split in splits {
        if is_rest_split(split) {
            rest += 1;
        } else {
            work += 1;
        }
    }
    if work >= 2 && rest == 0 {
        let work_splits: Vec<&Split> = splits
            .iter()
            .filter(|s| !is_rest_split(s) && s.pace > 0.0)
            .collect();
        for pair in work_splits.windows(2) {
            if (pair[1].pace - pair[0].pace).abs() > 30.0 {
                rest += 1;
                break;
            }
        }
    }
    (work, rest)
}

fn is_interval_structure(workout: &Workout, splits: Option<&[Split]>) -> bool {
    let (work, rest) = count_work_and_rest(workout, splits);
    work >= 2 && rest >= 1
}

fn average_pace(workout: &Workout, splits: Option<&[Split]>) -> f64 {
    if workout.pace > 0.0 {
        return workout.pace;
    }
    let Some(splits) = splits.filter(|s| !s.is_empty()) else {
        return 0.0;
    };
    let mut sum = 0.0;
    let mut count = 0usize;
    for split in splits {
        if !is_rest_split(split) && split.pace > 0.0 {
            sum += split.pace;
            count += 1;
        }
    }
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn pace_std_dev(splits: Option<&[Split]>) -> f64 {
    let Some(splits) = splits.filter(|s| !s.is_empty()) else {
        return 0.0;
    };
    let mut sum = 0.0;
    let mut count = 0usize;
    for split in splits {
        if !is_rest_split(split) && split.pace > 0.0 {
            sum += split.pace;
            count += 1;
        }
    }
    if count < 2 {
        return 0.0;
    }
    let mean = sum / count as f64;
    let mut variance_sum = 0.0;
    for split in splits {
        if !is_rest_split(split) && split.pace > 0.0 {
            variance_sum += (split.pace - mean).powi(2);
        }
    }
    (variance_sum / count as f64).sqrt()
}

/// Auto-detect the workout type from split / interval structure.
///
/// `splits` is the detail payload's split list when available (a summary row
/// passes `None`).
#[must_use]
pub fn auto_detect_tag(
    workout: &Workout,
    splits: Option<&[Split]>,
    ctx: Option<&TagContext>,
) -> WorkoutTag {
    if workout.distance <= 0.0 || workout.time <= 0.0 {
        return WorkoutTag::Unknown;
    }
    if is_interval_structure(workout, splits) {
        return WorkoutTag::Interval;
    }
    let duration_min = workout.time / 60.0;
    let median = ctx.and_then(|c| c.median_pace_secs);
    let avg_pace = average_pace(workout, splits);
    let std = pace_std_dev(splits);

    if median.is_some_and(|median| median > 0.0 && duration_min < 8.0 && avg_pace > median * 1.25) {
        return WorkoutTag::WarmupCooldown;
    }
    if (duration_min < 12.0 || workout.distance <= 2000.0)
        && median.is_none_or(|median| median <= 0.0 || avg_pace <= median * 1.25)
    {
        return WorkoutTag::RacePiece;
    }
    if (12.0..=35.0).contains(&duration_min) && std < 3.0 {
        return WorkoutTag::TimeTrial;
    }
    if duration_min > 35.0 && std < 6.0 {
        return WorkoutTag::SteadyState;
    }
    WorkoutTag::Unknown
}

/// Tag derived from the live Concept2 workout payload (web `resolveTag`).
#[must_use]
pub fn resolve_tag(
    workout: &Workout,
    splits: Option<&[Split]>,
    ctx: Option<&TagContext>,
) -> WorkoutTag {
    auto_detect_tag(workout, splits, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Sport;

    fn base() -> Workout {
        Workout::new(1, "2026-01-01 06:00:00", Sport::Rower, 2000.0, 480.0, 120.0)
    }

    fn interval_splits(reps: u32, rep_dist: f64, rep_pace: f64, rest_sec: f64) -> Vec<Split> {
        let mut splits = Vec::new();
        for i in 0..reps {
            let index = splits.len() as u32;
            splits.push(Split {
                is_rest: Some(false),
                ..Split::new(index, rep_dist, (rep_dist / 500.0) * rep_pace, rep_pace)
            });
            if i < reps - 1 {
                let index = splits.len() as u32;
                splits.push(Split {
                    is_rest: Some(true),
                    ..Split::new(index, 0.0, rest_sec, 0.0)
                });
            }
        }
        splits
    }

    #[test]
    fn lists_six_tag_values() {
        assert_eq!(
            WORKOUT_TAGS.iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            vec![
                "steady-state",
                "interval",
                "race-piece",
                "time-trial",
                "warmup-cooldown",
                "unknown"
            ]
        );
        assert!(is_valid_workout_tag(Some("race-piece")));
        assert!(!is_valid_workout_tag(Some("sprint")));
        assert!(!is_valid_workout_tag(None));
        assert_eq!(
            serde_json::to_string(&WorkoutTag::WarmupCooldown).unwrap(),
            "\"warmup-cooldown\""
        );
    }

    #[test]
    fn classifies_intervals() {
        let mut w = base();
        w.distance = 4000.0;
        w.time = 2400.0;
        w.is_interval = true;
        assert_eq!(
            auto_detect_tag(&w, Some(&interval_splits(4, 1000.0, 115.0, 90.0)), None),
            WorkoutTag::Interval
        );

        let mut w = base();
        w.distance = 4000.0;
        w.time = 1800.0;
        w.rest_time = Some(240.0);
        w.is_interval = true;
        w.workout_type = Some("JustRow".into());
        assert_eq!(auto_detect_tag(&w, None, None), WorkoutTag::Interval);

        let mut w = base();
        w.distance = 0.0;
        w.time = 300.0;
        let rest = [Split {
            is_rest: Some(true),
            ..Split::new(0, 0.0, 300.0, 0.0)
        }];
        assert_ne!(auto_detect_tag(&w, Some(&rest), None), WorkoutTag::Interval);
    }

    #[test]
    fn classifies_single_pieces() {
        let ctx = TagContext {
            median_pace_secs: Some(120.0),
        };
        let mut w = base();
        w.distance = 500.0;
        w.time = 100.0;
        w.pace = 100.0;
        assert_eq!(
            auto_detect_tag(&w, Some(&[]), Some(&ctx)),
            WorkoutTag::RacePiece
        );
        assert_eq!(resolve_tag(&w, None, Some(&ctx)), WorkoutTag::RacePiece);

        let mut w = base();
        w.distance = 10000.0;
        w.time = 2400.0;
        w.pace = 120.0;
        let splits = [Split::new(0, 10000.0, 2400.0, 120.0)];
        assert_eq!(
            auto_detect_tag(&w, Some(&splits), Some(&ctx)),
            WorkoutTag::SteadyState
        );

        let splits: Vec<Split> = (0..8)
            .map(|i| Split::new(i, 5000.0, 600.0, 120.0 + f64::from(i % 2) * 0.5))
            .collect();
        let mut w = base();
        w.distance = 40000.0;
        w.time = 4800.0;
        assert_eq!(
            auto_detect_tag(&w, Some(&splits), None),
            WorkoutTag::SteadyState
        );

        let mut w = base();
        w.distance = 15000.0;
        w.time = 3600.0;
        assert_eq!(auto_detect_tag(&w, None, None), WorkoutTag::SteadyState);

        let splits = [
            Split::new(0, 3000.0, 720.0, 120.0),
            Split::new(1, 3000.0, 720.0, 121.0),
        ];
        let mut w = base();
        w.distance = 6000.0;
        w.time = 1440.0;
        w.pace = 120.5;
        assert_eq!(
            auto_detect_tag(&w, Some(&splits), None),
            WorkoutTag::TimeTrial
        );

        let mut w = base();
        w.distance = 1000.0;
        w.time = 420.0;
        w.pace = 160.0;
        assert_eq!(
            auto_detect_tag(&w, Some(&[]), Some(&ctx)),
            WorkoutTag::WarmupCooldown
        );
    }

    #[test]
    fn falls_back_to_unknown() {
        let ctx = TagContext {
            median_pace_secs: Some(120.0),
        };
        let splits = [
            Split::new(0, 2000.0, 480.0, 100.0),
            Split::new(1, 2000.0, 520.0, 130.0),
        ];
        let mut w = base();
        w.distance = 4000.0;
        w.time = 1000.0;
        w.pace = 115.0;
        assert_eq!(
            auto_detect_tag(&w, Some(&splits), Some(&ctx)),
            WorkoutTag::Unknown
        );

        let splits = [
            Split::new(0, 2000.0, 440.0, 110.0),
            Split::new(1, 2000.0, 540.0, 135.0),
            Split::new(2, 2000.0, 440.0, 110.0),
        ];
        let mut w = base();
        w.distance = 6000.0;
        w.time = 1420.0;
        w.pace = 118.0;
        assert_eq!(
            auto_detect_tag(&w, Some(&splits), Some(&ctx)),
            WorkoutTag::Unknown
        );

        let mut w = base();
        w.time = 0.0;
        assert_eq!(auto_detect_tag(&w, None, None), WorkoutTag::Unknown);
    }

    #[test]
    fn athlete_median_pace_cases() {
        assert_eq!(athlete_median_pace(&[]), None);
        let mut a = base();
        a.pace = 110.0;
        let mut b = base();
        b.pace = 130.0;
        let mut c = base();
        c.pace = 0.0;
        assert_eq!(athlete_median_pace(&[a.clone(), b.clone(), c]), Some(120.0));
        let mut d = base();
        d.pace = 125.0;
        assert_eq!(athlete_median_pace(&[a, b, d]), Some(125.0));
    }
}
