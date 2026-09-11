// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay renderer kind and 3D quality tiers
//! (web `replay/replayRenderer.ts` types + Studio `ReplayRenderQuality`
//! portable budgets).
//!
//! The web's `renderer3d.ts` `QUALITY` table also carries browser- and
//! three.js-specific fields (`dprCap`, `antialias`, `shadowMapSize`,
//! `bodySegments`, …); those are Phase 5 material where Qt Quick 3D maps them,
//! and the divergence is recorded in `docs/source-map.md`. Preference
//! persistence (web `safeStorage`) is Phase 3 with the platform preferences
//! store.

/// Which replay renderer the user picked (web `RendererKind`); defaults to 2D.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RendererKind {
    /// Canvas-style 2D replay.
    TwoD,
    /// 3D replay (Qt Quick 3D from Phase 5).
    ThreeD,
}

/// User-selected ceiling for the 3D replay's rendering budget
/// (web `RenderQuality`); adaptive degradation may move down from the
/// selected tier for the lifetime of a scene but never up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderQuality {
    /// Reduced budgets, 30 fps target, no effects.
    Low,
    /// Default tier with modest effects.
    Medium,
    /// Full effects at 60 fps.
    High,
    /// Maximum budgets.
    Ultra,
}

impl RenderQuality {
    /// Medium is the default on every platform (web `loadQualityPref`).
    pub const DEFAULT: RenderQuality = RenderQuality::Medium;

    /// Number of one-tier degradation steps available below this ceiling.
    #[must_use]
    pub fn maximum_degradation_level(self) -> u8 {
        match self {
            RenderQuality::Low => 0,
            RenderQuality::Medium => 1,
            RenderQuality::High => 2,
            RenderQuality::Ultra => 3,
        }
    }

    /// The next lower tier, with low remaining sticky at low.
    #[must_use]
    pub fn next_lower_quality(self) -> RenderQuality {
        match self {
            RenderQuality::Low | RenderQuality::Medium => RenderQuality::Low,
            RenderQuality::High => RenderQuality::Medium,
            RenderQuality::Ultra => RenderQuality::High,
        }
    }

    /// Apply a sticky degradation without permitting an upgrade.
    #[must_use]
    pub fn degraded(self, levels: u8) -> RenderQuality {
        if levels == 0 {
            return self;
        }
        let mut quality = self;
        for _ in 0..levels.min(self.maximum_degradation_level()) {
            quality = quality.next_lower_quality();
        }
        quality
    }

    /// Portable entity and geometry budgets for this tier.
    #[must_use]
    pub fn budgets(self) -> QualityBudgets {
        match self {
            RenderQuality::Low => QualityBudgets {
                course_ring_segment_count: 48,
                lane_marker_count: 24,
                wake_entry_capacity_per_participant: 0,
                spray_particle_capacity: 0,
                spray_droplets_per_side_per_catch: 0,
                buoys_per_ring: 12,
                target_frame_rate: 30,
            },
            RenderQuality::Medium => QualityBudgets {
                course_ring_segment_count: 72,
                lane_marker_count: 48,
                wake_entry_capacity_per_participant: 16,
                spray_particle_capacity: 40,
                spray_droplets_per_side_per_catch: 4,
                buoys_per_ring: 18,
                target_frame_rate: 60,
            },
            RenderQuality::High => QualityBudgets {
                course_ring_segment_count: 96,
                lane_marker_count: 64,
                wake_entry_capacity_per_participant: 28,
                spray_particle_capacity: 48,
                spray_droplets_per_side_per_catch: 4,
                buoys_per_ring: 22,
                target_frame_rate: 60,
            },
            RenderQuality::Ultra => QualityBudgets {
                course_ring_segment_count: 144,
                lane_marker_count: 96,
                wake_entry_capacity_per_participant: 44,
                spray_particle_capacity: 72,
                spray_droplets_per_side_per_catch: 6,
                buoys_per_ring: 28,
                target_frame_rate: 60,
            },
        }
    }
}

/// Portable entity and geometry budgets for one replay quality tier
/// (Studio `ReplayRenderConfiguration`; counts are clamped at construction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityBudgets {
    /// Course ring geometry segment count.
    pub course_ring_segment_count: u16,
    /// Lane marker count.
    pub lane_marker_count: u16,
    /// Wake trail entries retained per participant.
    pub wake_entry_capacity_per_participant: u16,
    /// Spray droplet pool capacity.
    pub spray_particle_capacity: u16,
    /// Spray droplets spawned per side at each catch.
    pub spray_droplets_per_side_per_catch: u8,
    /// Course buoys per ring.
    pub buoys_per_ring: u16,
    /// 30 (low) or 60 (every other tier).
    pub target_frame_rate: u8,
}

impl QualityBudgets {
    /// Ceiling for [`Self::course_ring_segment_count`] (the ultra tier).
    pub const MAXIMUM_COURSE_RING_SEGMENT_COUNT: u16 = 144;
    /// Ceiling for [`Self::lane_marker_count`].
    pub const MAXIMUM_LANE_MARKER_COUNT: u16 = 96;
    /// Ceiling for [`Self::wake_entry_capacity_per_participant`].
    pub const MAXIMUM_WAKE_ENTRY_CAPACITY_PER_PARTICIPANT: u16 = 44;
    /// Ceiling for [`Self::spray_particle_capacity`].
    pub const MAXIMUM_SPRAY_PARTICLE_CAPACITY: u16 = 72;
    /// Ceiling for [`Self::spray_droplets_per_side_per_catch`].
    pub const MAXIMUM_SPRAY_DROPLETS_PER_SIDE_PER_CATCH: u8 = 6;
    /// Ceiling for [`Self::buoys_per_ring`].
    pub const MAXIMUM_BUOYS_PER_RING: u16 = 28;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Studio's exact per-tier budgets (docs/source-map of rowplay-studio):
    /// course/marker/wake/spray/spray-per-side/buoy-per-ring/timeline frame
    /// rate = low 48/24/0/0/0/12/30, medium 72/48/16/40/4/18/60,
    /// high 96/64/28/48/4/22/60, ultra 144/96/44/72/6/28/60.
    #[test]
    fn budgets_match_the_documented_tiers() {
        let b = RenderQuality::Low.budgets();
        assert_eq!(
            (
                b.course_ring_segment_count,
                b.lane_marker_count,
                b.wake_entry_capacity_per_participant,
                b.spray_particle_capacity,
                b.spray_droplets_per_side_per_catch,
                b.buoys_per_ring,
                b.target_frame_rate
            ),
            (48, 24, 0, 0, 0, 12, 30)
        );
        let b = RenderQuality::Medium.budgets();
        assert_eq!(
            (
                b.course_ring_segment_count,
                b.lane_marker_count,
                b.wake_entry_capacity_per_participant,
                b.spray_particle_capacity,
                b.spray_droplets_per_side_per_catch,
                b.buoys_per_ring,
                b.target_frame_rate
            ),
            (72, 48, 16, 40, 4, 18, 60)
        );
        let b = RenderQuality::High.budgets();
        assert_eq!(
            (
                b.course_ring_segment_count,
                b.lane_marker_count,
                b.wake_entry_capacity_per_participant,
                b.spray_particle_capacity,
                b.spray_droplets_per_side_per_catch,
                b.buoys_per_ring,
                b.target_frame_rate
            ),
            (96, 64, 28, 48, 4, 22, 60)
        );
        let b = RenderQuality::Ultra.budgets();
        assert_eq!(
            (
                b.course_ring_segment_count,
                b.lane_marker_count,
                b.wake_entry_capacity_per_participant,
                b.spray_particle_capacity,
                b.spray_droplets_per_side_per_catch,
                b.buoys_per_ring,
                b.target_frame_rate
            ),
            (144, 96, 44, 72, 6, 28, 60)
        );
    }

    #[test]
    fn default_is_medium_and_degradation_is_sticky() {
        assert_eq!(RenderQuality::DEFAULT, RenderQuality::Medium);
        assert_eq!(RenderQuality::Ultra.degraded(0), RenderQuality::Ultra);
        assert_eq!(RenderQuality::Ultra.degraded(1), RenderQuality::High);
        assert_eq!(RenderQuality::Ultra.degraded(3), RenderQuality::Low);
        assert_eq!(RenderQuality::Ultra.degraded(9), RenderQuality::Low);
        assert_eq!(RenderQuality::Low.degraded(2), RenderQuality::Low);
        assert_eq!(RenderQuality::Medium.maximum_degradation_level(), 1);
        assert_eq!(RenderQuality::Ultra.maximum_degradation_level(), 3);
    }
}
