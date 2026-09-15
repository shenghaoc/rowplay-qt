// SPDX-License-Identifier: GPL-3.0-or-later
//! Saddle shape contract (Phase 7): the port of the web's `bikeSaddle.js`.
//!
//! One table, three consumers: the procedural renderer, the authored V3
//! equipment package and the penetration guards all read the same stations.
//! Local frame: origin at the bike rig's saddle centre, +Z forward, +Y up;
//! the top surface at the widest station is the pad top. Pure geometry,
//! Qt-free and I/O-free like the rest of `rowplay-core`. Compared against
//! the `saddle` tree and `saddleDrop` grid of
//! `replay-current-main-equipment.json` at 1e-12.

/// How sharply the shell climbs from the channel to the wing.
pub const CHANNEL_FALLOFF: f64 = 0.22;
/// Shell thickness below the top surface (m).
pub const SHELL_THICKNESS: f64 = 0.016;

/// One measured saddle station along the saddle's own Z, nose-ward positive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SaddleStation {
    /// Fore-aft position (m).
    pub z: f64,
    /// Shell half-width (m).
    pub half_width: f64,
    /// Top surface below the pad top at the widest point (m).
    pub drop_outer: f64,
    /// Top surface below the pad top at the channel edge (m).
    pub drop_channel: f64,
    /// Half-width of the through cut-out; 0 where the shell is solid (m).
    pub cutout: f64,
}

/// Stations from the rear (z = −0.05) to the nose (z = 0.17): the measured
/// skin envelope minus clearance, not styling.
pub const STATIONS: [SaddleStation; 14] = [
    SaddleStation {
        z: -0.05,
        half_width: 0.03,
        drop_outer: 0.012,
        drop_channel: 0.012,
        cutout: 0.0,
    },
    SaddleStation {
        z: -0.03,
        half_width: 0.06,
        drop_outer: 0.002,
        drop_channel: 0.004,
        cutout: 0.0,
    },
    SaddleStation {
        z: -0.015,
        half_width: 0.073,
        drop_outer: 0.0,
        drop_channel: 0.004,
        cutout: 0.018,
    },
    SaddleStation {
        z: 0.0,
        half_width: 0.075,
        drop_outer: 0.0,
        drop_channel: 0.007,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.015,
        half_width: 0.068,
        drop_outer: 0.001,
        drop_channel: 0.014,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.028,
        half_width: 0.056,
        drop_outer: 0.002,
        drop_channel: 0.021,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.045,
        half_width: 0.046,
        drop_outer: 0.018,
        drop_channel: 0.021,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.06,
        half_width: 0.038,
        drop_outer: 0.02,
        drop_channel: 0.022,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.08,
        half_width: 0.032,
        drop_outer: 0.024,
        drop_channel: 0.026,
        cutout: 0.024,
    },
    SaddleStation {
        z: 0.1,
        half_width: 0.027,
        drop_outer: 0.028,
        drop_channel: 0.029,
        cutout: 0.022,
    },
    SaddleStation {
        z: 0.115,
        half_width: 0.024,
        drop_outer: 0.04,
        drop_channel: 0.041,
        cutout: 0.014,
    },
    SaddleStation {
        z: 0.13,
        half_width: 0.022,
        drop_outer: 0.046,
        drop_channel: 0.046,
        cutout: 0.0,
    },
    SaddleStation {
        z: 0.15,
        half_width: 0.02,
        drop_outer: 0.053,
        drop_channel: 0.053,
        cutout: 0.0,
    },
    SaddleStation {
        z: 0.17,
        half_width: 0.017,
        drop_outer: 0.062,
        drop_channel: 0.062,
        cutout: 0.0,
    },
];

/// Rear end of the saddle (m).
pub const REAR_Z: f64 = -0.05;
/// Nose end of the saddle (m).
pub const NOSE_Z: f64 = 0.17;
/// Saddle length, rear to nose (m).
pub const LENGTH: f64 = 0.22;
/// Widest shell half-width (m).
pub const MAX_HALF_WIDTH: f64 = 0.075;

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Station values interpolated at an arbitrary local Z, or `None` beyond the
/// saddle's own ends (web `bikeSaddleStationAt`).
#[must_use]
pub fn station_at(z: f64) -> Option<SaddleStation> {
    if z < STATIONS[0].z || z > STATIONS[STATIONS.len() - 1].z {
        return None;
    }
    for pair in STATIONS.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if z > b.z {
            continue;
        }
        let span = b.z - a.z;
        let t = if span <= 0.0 { 0.0 } else { (z - a.z) / span };
        return Some(SaddleStation {
            z,
            half_width: lerp(a.half_width, b.half_width, t),
            drop_outer: lerp(a.drop_outer, b.drop_outer, t),
            drop_channel: lerp(a.drop_channel, b.drop_channel, t),
            cutout: lerp(a.cutout, b.cutout, t),
        });
    }
    None
}

/// Top surface of the saddle at a local (x, z) as a drop below the pad top —
/// or `None` where there is no material: outside the shell, or inside the
/// cut-out (web `bikeSaddleDropAt`). The single definition of "is the saddle
/// here" for penetration guards and mesh builders alike.
#[must_use]
pub fn drop_at(x: f64, z: f64) -> Option<f64> {
    let station = station_at(z)?;
    let ax = x.abs();
    if ax > station.half_width || ax < station.cutout {
        return None;
    }
    let inner = station.cutout.max(0.0);
    let span = station.half_width - inner;
    let t = if span <= 1e-6 {
        1.0
    } else {
        ((ax - inner) / span).clamp(0.0, 1.0)
    };
    Some(
        station.drop_outer
            + (station.drop_channel - station.drop_outer) * (1.0 - t).powf(CHANNEL_FALLOFF),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ends_match_the_station_table() {
        assert_eq!(REAR_Z, STATIONS[0].z);
        assert_eq!(NOSE_Z, STATIONS[STATIONS.len() - 1].z);
        assert!((LENGTH - (NOSE_Z - REAR_Z)).abs() < 1e-15);
        let widest = STATIONS.iter().map(|s| s.half_width).fold(0.0, f64::max);
        assert_eq!(MAX_HALF_WIDTH, widest);
    }

    #[test]
    fn pad_top_is_zero_drop_at_the_widest_station() {
        // The widest station (z = 0) is the pad-top reference.
        assert_eq!(drop_at(0.075, 0.0), Some(0.0));
    }

    #[test]
    fn cutout_has_no_material_on_the_centreline() {
        assert_eq!(drop_at(0.0, 0.0), None);
        assert_eq!(drop_at(0.01, 0.0), None);
    }

    #[test]
    fn outside_the_shell_is_none() {
        assert_eq!(drop_at(0.0, -0.06), None);
        assert_eq!(drop_at(0.0, 0.18), None);
        assert_eq!(drop_at(0.09, 0.0), None);
    }
}
