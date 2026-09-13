// SPDX-License-Identifier: GPL-3.0-or-later
//! Quality-tier settings resolver (Phase 5c spec R1).
//!
//! Maps `RenderQuality` + sport to a plain-data struct the QML scene binds:
//! shadow on/off, shadow map size, MSAA samples, the list of environment
//! texture sets to load, whether to bind normal maps, and the entity budgets.
//! The texture-set lists derive from the environments README table and are
//! tested against it.

use rowplay_core::models::Sport;
use rowplay_core::replay::quality::{QualityBudgets, RenderQuality};

/// The resolved scene settings for one tier + sport combination.
#[derive(Debug, Clone, PartialEq)]
pub struct TierSettings {
    /// Whether directional-light shadows are enabled.
    pub shadows: bool,
    /// Shadow map size (0 when shadows are off; 1024 High, 2048 Ultra).
    pub shadow_map_size: u16,
    /// MSAA sample count (0 = None for Low, 2 = Medium, 4 = High/Ultra).
    pub msaa_samples: u8,
    /// Environment texture set directory names to load (empty for Low/Medium).
    pub texture_sets: Vec<&'static str>,
    /// Whether to bind OpenGL normal maps (Ultra only).
    pub normal_maps: bool,
    /// The portable entity budgets from `RenderQuality::budgets()`.
    pub budgets: QualityBudgets,
}

/// Resolve the scene settings for a quality tier and sport.
#[must_use]
pub fn tier_settings(quality: RenderQuality, sport: Sport) -> TierSettings {
    let budgets = quality.budgets();
    let (shadows, shadow_map_size, msaa_samples) = match quality {
        RenderQuality::Low => (false, 0, 0),
        RenderQuality::Medium => (false, 0, 2),
        RenderQuality::High => (true, 1024, 4),
        RenderQuality::Ultra => (true, 2048, 4),
    };
    let normal_maps = quality == RenderQuality::Ultra;
    let texture_sets = texture_sets_for(quality, sport);
    TierSettings {
        shadows,
        shadow_map_size,
        msaa_samples,
        texture_sets,
        normal_maps,
        budgets,
    }
}

/// The environment texture set directories to load for this tier + sport,
/// derived from the environments README table. Low and Medium load nothing.
fn texture_sets_for(quality: RenderQuality, sport: Sport) -> Vec<&'static str> {
    match quality {
        RenderQuality::Low | RenderQuality::Medium => Vec::new(),
        RenderQuality::High => high_sets(sport),
        RenderQuality::Ultra => ultra_sets(sport),
    }
}

/// High-tier sets: diffuse + roughness per set.
fn high_sets(sport: Sport) -> Vec<&'static str> {
    match sport {
        Sport::Rower => vec![
            "aerial-grass-rock",
            "forrest-ground-01",
            "dry-river-pebbles",
            "leafy-grass",
            "forest-leaves-04",
            "bark-brown-01",
            "brown-planks-03",
            "cobblestone-floor-03",
        ],
        Sport::Skierg => vec!["snow-02", "rock-01", "bark-brown-01", "forest-leaves-04"],
        Sport::Bike => vec![
            "brushed-concrete-2",
            "concrete-floor-painted",
            "brown-planks-03",
            "wood-floor",
        ],
    }
}

/// Ultra-tier sets: High sets + normal maps, SkiErg adds the timber terrace.
fn ultra_sets(sport: Sport) -> Vec<&'static str> {
    let mut sets = high_sets(sport);
    if sport == Sport::Skierg {
        sets.push("brown-planks-03"); // Ultra-only spectator terrace
    }
    sets
}

/// JSON representation of the tier settings for the QML scene.
#[must_use]
pub fn tier_settings_json(quality: RenderQuality, sport: Sport) -> serde_json::Value {
    let ts = tier_settings(quality, sport);
    serde_json::json!({
        "shadows": ts.shadows,
        "shadowMapSize": ts.shadow_map_size,
        "msaaSamples": ts.msaa_samples,
        "textureSets": ts.texture_sets,
        "normalMaps": ts.normal_maps,
        "buoysPerRing": ts.budgets.buoys_per_ring,
        "laneMarkerCount": ts.budgets.lane_marker_count,
        "wakeCap": ts.budgets.wake_entry_capacity_per_participant,
        "sprayCap": ts.budgets.spray_particle_capacity,
        "targetFrameRate": ts.budgets.target_frame_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The environments README documents the per-sport High/Ultra set counts:
    /// RowErg 8/8, SkiErg 4/5, BikeErg 4/4. These are the numbers the gate
    /// test's tier inventory assertion will check against.
    #[test]
    fn texture_set_counts_match_the_readme() {
        // RowErg: 8 at High, 8 at Ultra (same sets, Ultra adds normals)
        assert_eq!(high_sets(Sport::Rower).len(), 8);
        assert_eq!(ultra_sets(Sport::Rower).len(), 8);
        // SkiErg: 4 at High, 5 at Ultra (adds brown-planks-03 terrace)
        assert_eq!(high_sets(Sport::Skierg).len(), 4);
        assert_eq!(ultra_sets(Sport::Skierg).len(), 5);
        // BikeErg: 4 at High, 4 at Ultra
        assert_eq!(high_sets(Sport::Bike).len(), 4);
        assert_eq!(ultra_sets(Sport::Bike).len(), 4);
    }

    #[test]
    fn low_and_medium_load_no_textures() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            assert!(
                tier_settings(RenderQuality::Low, sport)
                    .texture_sets
                    .is_empty()
            );
            assert!(
                tier_settings(RenderQuality::Medium, sport)
                    .texture_sets
                    .is_empty()
            );
        }
    }

    #[test]
    fn shadows_only_at_high_and_ultra() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            assert!(!tier_settings(RenderQuality::Low, sport).shadows);
            assert!(!tier_settings(RenderQuality::Medium, sport).shadows);
            assert!(tier_settings(RenderQuality::High, sport).shadows);
            assert!(tier_settings(RenderQuality::Ultra, sport).shadows);
        }
    }

    #[test]
    fn normal_maps_only_at_ultra() {
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            assert!(!tier_settings(RenderQuality::High, sport).normal_maps);
            assert!(tier_settings(RenderQuality::Ultra, sport).normal_maps);
        }
    }

    #[test]
    fn shadow_map_sizes() {
        let s = tier_settings(RenderQuality::High, Sport::Rower);
        assert_eq!(s.shadow_map_size, 1024);
        let s = tier_settings(RenderQuality::Ultra, Sport::Rower);
        assert_eq!(s.shadow_map_size, 2048);
    }

    #[test]
    fn msaa_samples_per_tier() {
        assert_eq!(
            tier_settings(RenderQuality::Low, Sport::Rower).msaa_samples,
            0
        );
        assert_eq!(
            tier_settings(RenderQuality::Medium, Sport::Rower).msaa_samples,
            2
        );
        assert_eq!(
            tier_settings(RenderQuality::High, Sport::Rower).msaa_samples,
            4
        );
        assert_eq!(
            tier_settings(RenderQuality::Ultra, Sport::Rower).msaa_samples,
            4
        );
    }

    #[test]
    fn every_texture_set_directory_exists() {
        // This test only runs when the assets directory is present (development).
        let assets_dir = std::path::Path::new("assets/replay/environments");
        if !assets_dir.exists() {
            // Running from a different working directory or CI without assets.
            return;
        }
        for sport in [Sport::Rower, Sport::Skierg, Sport::Bike] {
            for quality in [RenderQuality::High, RenderQuality::Ultra] {
                for set in &tier_settings(quality, sport).texture_sets {
                    assert!(
                        assets_dir.join(set).is_dir(),
                        "texture set directory {set} does not exist under {}/",
                        assets_dir.display()
                    );
                }
            }
        }
    }

    #[test]
    fn tier_json_carries_all_fields() {
        let j = tier_settings_json(RenderQuality::High, Sport::Rower);
        assert_eq!(j["shadows"], true);
        assert_eq!(j["shadowMapSize"], 1024);
        assert_eq!(j["msaaSamples"], 4);
        assert_eq!(j["textureSets"].as_array().unwrap().len(), 8);
        assert_eq!(j["normalMaps"], false);
        assert_eq!(j["buoysPerRing"], 22);
    }
}
