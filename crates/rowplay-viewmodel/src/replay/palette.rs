// SPDX-License-Identifier: GPL-3.0-or-later
//! The venue palette bridge for the 3D scene (Phase 5a spec R4.1).
//!
//! `rowplay_core::replay::theme` owns the web's per-sport venue palettes
//! (golden-tested against the 2D fixture); this module selects the handful of
//! entries the Qt Quick 3D scene needs — procedural-sky zenith/horizon/sun,
//! the ground plane tint and the lane-paint colours — so QML binds exported
//! strings instead of palette hex literals. The palettes are *data* (the web
//! stylesheet mirrored), which is why they bypass `Theme.qml` (recorded
//! divergence, Phase 5a).

use rowplay_core::models::Sport;
use rowplay_core::replay::theme::{COLORS_DARK, COLORS_LIGHT, venues_dark, venues_light};

fn canvas(dark: bool) -> &'static rowplay_core::replay::theme::CanvasColors {
    if dark { &COLORS_DARK } else { &COLORS_LIGHT }
}

/// The four procedural-sky inputs (ADR 0004: no HDRI, ever).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkyPalette {
    /// `ProceduralSkyTextureData.zenithColor`.
    pub zenith: &'static str,
    /// `ProceduralSkyTextureData.horizonColor`.
    pub horizon: &'static str,
    /// `ProceduralSkyTextureData.groundColor`.
    pub ground: &'static str,
    /// The art-directed sun colour of the directional key.
    pub sun: &'static str,
}

/// Maps a sport palette onto the procedural sky (web `VENUES_*` sky family).
#[must_use]
pub fn sky_palette(sport: Sport, dark: bool) -> SkyPalette {
    let venue = if dark {
        venues_dark(sport)
    } else {
        venues_light(sport)
    };
    SkyPalette {
        zenith: venue.sky_top,
        horizon: venue.sky_horizon,
        ground: venue.ground_mid,
        sun: venue.sun,
    }
}

/// The ground plane tint: the venue's mid ground band.
#[must_use]
pub fn ground_color(sport: Sport, dark: bool) -> &'static str {
    let venue = if dark {
        venues_dark(sport)
    } else {
        venues_light(sport)
    };
    venue.ground_top
}

/// Lane paint for the live participant (`equipment-painted` role).
#[must_use]
pub fn live_paint(dark: bool) -> &'static str {
    canvas(dark).live
}

/// Lane paint for a ghost participant.
#[must_use]
pub fn ghost_paint(dark: bool) -> &'static str {
    canvas(dark).ghost
}

/// The venue accent (distance markers, lane furniture).
#[must_use]
pub fn marker(sport: Sport, dark: bool) -> &'static str {
    let venue = if dark {
        venues_dark(sport)
    } else {
        venues_light(sport)
    };
    venue.marker
}

/// Buoy / safety base colour (5c instance caps draw these).
#[must_use]
pub fn safety(sport: Sport, dark: bool) -> &'static str {
    let venue = if dark {
        venues_dark(sport)
    } else {
        venues_light(sport)
    };
    venue.safety
}

/// The key light's position relative to its target, in metres (web
/// `SUN_OFFSETS` in `renderer3d.ts`): a low golden-hour key for RowErg, a
/// higher alpine key for SkiErg and a dusk side-key for the BikeErg
/// velodrome. The scene aims a `DirectionalLight` from this offset at the
/// shadow target, exactly like the web's `sunLight` / `sunLight.target`.
#[must_use]
pub fn sun_offset(sport: Sport) -> [f32; 3] {
    match sport {
        Sport::Rower => [-22.0, 18.0, 14.0],
        Sport::Skierg => [16.0, 28.0, 10.0],
        Sport::Bike => [12.0, 20.0, -10.0],
    }
}

/// Height of the key light's target above the ground plane, in metres (web
/// `SHADOW_TARGET_HEIGHT`).
pub const SHADOW_TARGET_HEIGHT: f32 = 0.55;

/// The sun's elevation above the horizon in degrees, derived from
/// [`sun_offset`] for the procedural sky's `sunLatitude` (the web draws its
/// sun disc along the same offset, `renderer3d.ts` `sun.position`).
#[must_use]
pub fn sun_elevation_degrees(sport: Sport) -> f32 {
    let [x, y, z] = sun_offset(sport);
    (y / (x * x + y * y + z * z).sqrt()).asin().to_degrees()
}

/// The sun's azimuth in degrees for the procedural sky's `sunLongitude`,
/// derived from [`sun_offset`]. Qt's generator (`proceduralskytexturedata.cpp`,
/// Qt 6.11) starts the sun at `(0, 0, -1)`, tilts it by the latitude about
/// X and then turns it by the longitude about Y, so a longitude `b` puts the
/// sun along `(-sin b, ·, -cos b)`; solving for the web offset gives
/// `atan2(-x, -z)`.
#[must_use]
pub fn sun_azimuth_degrees(sport: Sport) -> f32 {
    let [x, _, z] = sun_offset(sport);
    (-x).atan2(-z).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_palettes_follow_the_sport_and_scheme() {
        let light = sky_palette(Sport::Rower, false);
        let dark = sky_palette(Sport::Rower, true);
        assert_eq!(light.zenith, "#4d86a8");
        assert_eq!(dark.zenith, "#071724");
        assert_ne!(light.horizon, dark.horizon);
        assert_eq!(sky_palette(Sport::Bike, false).zenith, "#edf3f4");
        for dark in [false, true] {
            for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
                let sky = sky_palette(sport, dark);
                for colour in [sky.zenith, sky.horizon, sky.ground, sky.sun] {
                    assert!(colour.starts_with('#') && colour.len() == 7, "{colour}");
                }
            }
        }
    }

    #[test]
    fn the_key_light_offsets_are_the_web_constants() {
        assert_eq!(sun_offset(Sport::Rower), [-22.0, 18.0, 14.0]);
        assert_eq!(sun_offset(Sport::Skierg), [16.0, 28.0, 10.0]);
        assert_eq!(sun_offset(Sport::Bike), [12.0, 20.0, -10.0]);
        assert_eq!(SHADOW_TARGET_HEIGHT, 0.55);
        // asin(18 / sqrt(1004)), asin(28 / sqrt(1140)), asin(20 / sqrt(644)).
        for (sport, expected) in [
            (Sport::Rower, 34.6),
            (Sport::Skierg, 56.0),
            (Sport::Bike, 52.0),
        ] {
            let elevation = sun_elevation_degrees(sport);
            assert!(
                (elevation - expected).abs() < 0.1,
                "{sport:?}: {elevation} vs {expected}"
            );
        }
        // atan2(22, -14), atan2(-16, -10), atan2(-12, 10).
        for (sport, expected) in [
            (Sport::Rower, 122.5),
            (Sport::Skierg, -122.0),
            (Sport::Bike, -50.2),
        ] {
            let azimuth = sun_azimuth_degrees(sport);
            assert!(
                (azimuth - expected).abs() < 0.1,
                "{sport:?}: {azimuth} vs {expected}"
            );
        }
    }

    #[test]
    fn paints_stay_distinct_for_live_and_ghost() {
        for dark in [false, true] {
            assert_ne!(live_paint(dark), ghost_paint(dark));
        }
    }
}
