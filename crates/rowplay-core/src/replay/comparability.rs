// SPDX-License-Identifier: GPL-3.0-or-later
//! Comparability guard (web `replay/comparabilityGuard.ts`).

use crate::analytics::{distance_band, duration_band};
use crate::models::Sport;

/// The axis a workout is measured along: fixed-distance or fixed-time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparabilityAxis {
    /// Fixed-distance piece (the Concept2 default).
    Distance,
    /// Explicitly timed piece (`JustRow`, `FixedTime*`).
    Time,
}

/// Concept2 `workout_type` is machine-agnostic: `JustRow` is the only "Just*"
/// value (it covers every erg, recording elapsed time as its axis) and
/// `FixedTime` catches `FixedTimeSplits` / `FixedTimeInterval`. There is no
/// `JustSki` / `JustBike` in the API.
const TIME_AXIS_MARKERS: [&str; 2] = ["JUSTROW", "FIXEDTIME"];

/// Map a Concept2 `workout_type` string to its comparability axis.
///
/// Time-axis types are explicitly timed workouts; everything else — including
/// unknown strings and the calorie / watt-minute / variable-interval types —
/// falls through to distance-axis, a safe default since Concept2's default
/// piece is fixed-distance.
#[must_use]
pub fn classify_axis(workout_type: Option<&str>) -> ComparabilityAxis {
    let Some(workout_type) = workout_type else {
        return ComparabilityAxis::Distance;
    };
    let upper = workout_type.to_uppercase();
    if TIME_AXIS_MARKERS
        .iter()
        .any(|marker| upper.contains(marker))
    {
        ComparabilityAxis::Time
    } else {
        ComparabilityAxis::Distance
    }
}

/// Context required to evaluate comparability between two workouts
/// (web `ComparableContext`).
#[derive(Debug, Clone, PartialEq)]
pub struct ComparableContext {
    /// Machine family.
    pub sport: Sport,
    /// Total distance in metres.
    pub distance: f64,
    /// Total elapsed time in seconds.
    pub time: f64,
    /// Concept2 `workout_type` string (may be absent from a list payload).
    pub workout_type: Option<String>,
}

/// Hard-block predicate: true only when `a` and `b` are genuinely
/// like-for-like — same sport, same axis, same axis-band.
#[must_use]
pub fn are_comparable(a: &ComparableContext, b: &ComparableContext) -> bool {
    if a.sport != b.sport {
        return false;
    }
    let axis_a = classify_axis(a.workout_type.as_deref());
    let axis_b = classify_axis(b.workout_type.as_deref());
    if axis_a != axis_b {
        return false;
    }
    match axis_a {
        ComparabilityAxis::Distance => {
            distance_band(a.distance).key == distance_band(b.distance).key
        }
        ComparabilityAxis::Time => duration_band(a.time).key == duration_band(b.time).key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(
        sport: Sport,
        distance: f64,
        time: f64,
        workout_type: Option<&str>,
    ) -> ComparableContext {
        ComparableContext {
            sport,
            distance,
            time,
            workout_type: workout_type.map(String::from),
        }
    }

    #[test]
    fn classifies_time_axis_markers() {
        assert_eq!(classify_axis(None), ComparabilityAxis::Distance);
        assert_eq!(
            classify_axis(Some("2000m test")),
            ComparabilityAxis::Distance
        );
        assert_eq!(classify_axis(Some("JustRow")), ComparabilityAxis::Time);
        assert_eq!(
            classify_axis(Some("FixedTimeSplits")),
            ComparabilityAxis::Time
        );
        assert_eq!(
            classify_axis(Some("fixedtimeinterval")),
            ComparabilityAxis::Time
        );
        // Calorie / variable types fall through to distance.
        assert_eq!(
            classify_axis(Some("FixedCalorieSplits")),
            ComparabilityAxis::Distance
        );
        assert_eq!(
            classify_axis(Some("VariableInterval")),
            ComparabilityAxis::Distance
        );
    }

    #[test]
    fn comparability_requires_same_sport_axis_and_band() {
        let a = ctx(Sport::Rower, 2000.0, 480.0, None);
        assert!(are_comparable(&a, &ctx(Sport::Rower, 2010.0, 470.0, None)));
        assert!(!are_comparable(
            &a,
            &ctx(Sport::Skierg, 2000.0, 480.0, None)
        ));
        assert!(!are_comparable(
            &a,
            &ctx(Sport::Rower, 7500.0, 1800.0, None)
        ));
        assert!(!are_comparable(
            &a,
            &ctx(Sport::Rower, 2000.0, 1800.0, Some("JustRow"))
        ));
        // Time axis: duration bands.
        let t = ctx(Sport::Rower, 7500.0, 1800.0, Some("JustRow"));
        assert!(are_comparable(
            &t,
            &ctx(Sport::Rower, 6500.0, 1790.0, Some("JustRow30min"))
        ));
        assert!(!are_comparable(
            &t,
            &ctx(Sport::Rower, 500.0, 120.0, Some("FixedTimeInterval"))
        ));
    }
}
