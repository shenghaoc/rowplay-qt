// SPDX-License-Identifier: GPL-3.0-or-later
//! SkiErg equipment contract (Phase 7): the port of the web's
//! `skiEquipment.ts` human-scale proportions, pole-grip contact numbers and
//! per-tier equipment detail.
//!
//! Pure data plus one tier lookup; Qt-free and I/O-free like the rest of
//! `rowplay-core`. Compared against the `ski` tree of
//! `replay-current-main-equipment.json` at 1e-12.

use crate::replay::hand_grip::HAND_FIST_REFERENCE_GRIP_RADIUS;
use crate::replay::quality::RenderQuality;

/// Measured rest-pose stature of the shipped V4 athlete (m).
pub const STANDING_HEIGHT: f64 = 1.64;
/// Shoulder half-width (m).
pub const SHOULDER_HALF_WIDTH: f64 = 0.25;
/// Hip half-width (m).
pub const HIP_HALF_WIDTH: f64 = 0.12;
/// Upper-arm length (m).
pub const UPPER_ARM_LENGTH: f64 = 0.49;
/// Forearm length (m).
pub const FOREARM_LENGTH: f64 = 0.47;
/// Thigh length (m).
pub const THIGH_LENGTH: f64 = 0.4;
/// Shin length (m).
pub const SHIN_LENGTH: f64 = 0.39;
/// Half of ski centre-to-centre stance (m).
pub const SKI_CENTER_OFFSET: f64 = 0.15;
/// Runner length (m, ~116% of stature).
pub const SKI_LENGTH: f64 = 1.9;
/// Maximum shovel width (m).
pub const SKI_WIDTH: f64 = 0.072;
/// Double-pole length (m, ~83.5% of stature).
pub const POLE_LENGTH: f64 = 1.37;
/// Basket plant lateral offset outside the ski pair (m).
pub const POLE_PLANT_LATERAL_OFFSET: f64 = 0.46;
/// Basket plant forward offset ahead of the athlete's centre (m).
pub const POLE_PLANT_FORWARD_OFFSET: f64 = 0.24;
/// Palm ride down the shaft from the grip top (m).
pub const GRIP_SHIFT: f64 = 0.042;
/// Physical radius of the rendered pole-grip capsule: the rubber the fist
/// channel was calibrated against, so this *is*
/// [`HAND_FIST_REFERENCE_GRIP_RADIUS`] and the two cannot drift apart.
pub const POLE_GRIP_RADIUS: f64 = HAND_FIST_REFERENCE_GRIP_RADIUS;
/// Thumb opposition closing the fist onto the pole rubber (rad, thumb-root
/// bone frame).
pub const POLE_THUMB_OPPOSE: f64 = 1.75;

/// Per-tier SkiErg equipment detail (web `SkiEquipmentDetail`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkiEquipmentDetail {
    /// Radial resolution for the visible hard-surface fallback.
    pub radial_segments: u32,
    /// Detail earning geometry only when the tier can display it.
    pub top_sheet: bool,
    /// Detail earning geometry only when the tier can display it.
    pub metal_edges: bool,
    /// Detail earning geometry only when the tier can display it.
    pub binding_rails: bool,
    /// Detail earning geometry only when the tier can display it.
    pub boot_closures: bool,
    /// Detail earning geometry only when the tier can display it.
    pub grip_straps: bool,
    /// Detail earning geometry only when the tier can display it.
    pub basket_ribs: bool,
}

/// Equipment detail for a render tier (web `skiEquipmentDetail`): geometry
/// is progressive, not only pixel density.
#[must_use]
pub const fn ski_equipment_detail(quality: RenderQuality) -> SkiEquipmentDetail {
    match quality {
        RenderQuality::Low => SkiEquipmentDetail {
            radial_segments: 8,
            top_sheet: false,
            metal_edges: false,
            binding_rails: false,
            boot_closures: false,
            grip_straps: false,
            basket_ribs: false,
        },
        RenderQuality::Medium => SkiEquipmentDetail {
            radial_segments: 12,
            top_sheet: true,
            metal_edges: false,
            binding_rails: true,
            boot_closures: true,
            grip_straps: true,
            basket_ribs: false,
        },
        RenderQuality::High => SkiEquipmentDetail {
            radial_segments: 16,
            top_sheet: true,
            metal_edges: true,
            binding_rails: true,
            boot_closures: true,
            grip_straps: true,
            basket_ribs: true,
        },
        RenderQuality::Ultra => SkiEquipmentDetail {
            radial_segments: 20,
            top_sheet: true,
            metal_edges: true,
            binding_rails: true,
            boot_closures: true,
            grip_straps: true,
            basket_ribs: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pole_rubber_matches_fist_calibration() {
        assert_eq!(POLE_GRIP_RADIUS, HAND_FIST_REFERENCE_GRIP_RADIUS);
    }

    #[test]
    fn ski_and_pole_sit_inside_the_classic_bands() {
        // Skis at ~116% (112–117% band), poles at ~83.5% (83–84% band).
        let ski_ratio = SKI_LENGTH / STANDING_HEIGHT;
        assert!(
            (112.0..=117.0).contains(&(ski_ratio * 100.0)),
            "{ski_ratio}"
        );
        let pole_ratio = POLE_LENGTH / STANDING_HEIGHT;
        assert!(
            (83.0..=84.0).contains(&(pole_ratio * 100.0)),
            "{pole_ratio}"
        );
    }

    #[test]
    fn detail_grows_monotonically_with_tier() {
        let tiers = [
            RenderQuality::Low,
            RenderQuality::Medium,
            RenderQuality::High,
            RenderQuality::Ultra,
        ];
        let mut previous = 0;
        for tier in tiers {
            let detail = ski_equipment_detail(tier);
            assert!(detail.radial_segments >= previous);
            previous = detail.radial_segments;
        }
        assert!(ski_equipment_detail(RenderQuality::Low).radial_segments < previous);
    }
}
