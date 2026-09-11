// SPDX-License-Identifier: GPL-3.0-or-later
//! Semantic colour roles for metric display.
//!
//! The palette itself lives in `Theme.qml` (DESIGN.md's Metric Mapping Rule);
//! the view-model only tags values with their role so QML never decides which
//! metric gets which colour. Bridged as the `index` integer.

/// Which semantic colour a displayed value wears.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum ColorRole {
    /// Primary text colour (no semantic mapping).
    #[default]
    Neutral = 0,
    /// Monitor Blue — distance and primary emphasis.
    Distance = 1,
    /// Duration Teal — time values.
    Duration = 2,
    /// Pace blue — pace values.
    Pace = 3,
    /// Split Orange — watts, speed, splits.
    Watts = 4,
    /// Red Zone Red — heart rate.
    HeartRate = 5,
    /// Cadence Violet — stroke rate / cadence.
    Cadence = 6,
}

impl ColorRole {
    /// The integer that crosses the bridge (`Theme.metricColor(role)`).
    #[must_use]
    pub const fn index(self) -> i32 {
        self as i32
    }

    /// Inverse of [`index`](Self::index); unknown values map to `Neutral`.
    #[must_use]
    pub const fn from_index(index: i32) -> ColorRole {
        match index {
            1 => ColorRole::Distance,
            2 => ColorRole::Duration,
            3 => ColorRole::Pace,
            4 => ColorRole::Watts,
            5 => ColorRole::HeartRate,
            6 => ColorRole::Cadence,
            _ => ColorRole::Neutral,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_round_trip() {
        for role in [
            ColorRole::Neutral,
            ColorRole::Distance,
            ColorRole::Duration,
            ColorRole::Pace,
            ColorRole::Watts,
            ColorRole::HeartRate,
            ColorRole::Cadence,
        ] {
            assert_eq!(ColorRole::from_index(role.index()), role);
        }
        assert_eq!(ColorRole::from_index(99), ColorRole::Neutral);
        assert_eq!(ColorRole::from_index(-1), ColorRole::Neutral);
    }
}
