// SPDX-License-Identifier: GPL-3.0-or-later
//! The V3 equipment anchor contract (Phase 5a spec R5.1).
//!
//! `reference/rowplay/static/replay-assets/README.md` tables where each
//! composite template attaches; 5a stores and unit-tests the table, 5b applies
//! it when cloning templates onto the rig. Coordinates are metres in the
//! space the README names per row (row avatar root, moving rower group,
//! wheel-group local, crank-group local).

/// One anchor row of the asset README table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    /// The composite template slot.
    pub template: &'static str,
    /// Translation applied when the template is cloned onto the rig.
    pub position: [f32; 3],
    /// Yaw in radians applied at clone time (π mirrors the left oar rig).
    pub yaw: f32,
    /// Clone count (skis and wheels appear per side).
    pub instances: u8,
    /// The coordinate space the README names for this row.
    pub space: &'static str,
}

/// Oarlock pivots in row avatar-root coordinates (README table).
pub const OARLOCK_PIVOT: [f32; 3] = [0.88, 0.51, 0.28];

/// Per-side ski anchor x factor: `(side × 0.15, 0, 0.16)`.
pub const SKI_ANCHOR: [f32; 3] = [0.15, 0.0, 0.16];

const PI: f32 = std::f32::consts::PI;

/// The seven rows, README order.
pub const ANCHORS: [Anchor; 7] = [
    Anchor {
        template: "equipment:row:boat-assembly",
        position: [0.0, 0.0, 0.0],
        yaw: 0.0,
        instances: 1,
        space: "row avatar root",
    },
    Anchor {
        template: "equipment:row:oar-rig",
        position: OARLOCK_PIVOT,
        yaw: 0.0,
        instances: 2,
        space: "row avatar root (left clone yaws π)",
    },
    Anchor {
        template: "equipment:row:seat-carriage",
        position: [0.0, 0.0, 0.0],
        yaw: 0.0,
        instances: 1,
        space: "moving rower group",
    },
    Anchor {
        template: "equipment:ski:ski-assembly",
        position: SKI_ANCHOR,
        yaw: 0.0,
        instances: 2,
        space: "ski avatar root (side × 0.15)",
    },
    Anchor {
        template: "equipment:bike:wheel-assembly",
        position: [0.0, 0.0, 0.0],
        yaw: 0.0,
        instances: 2,
        space: "wheel-group centre, axle along local X",
    },
    Anchor {
        template: "equipment:bike:frame-assembly",
        position: [0.0, 0.0, 0.0],
        yaw: 0.0,
        instances: 1,
        space: "bike avatar root",
    },
    Anchor {
        template: "equipment:bike:drivetrain-assembly",
        position: [0.0, 0.0, 0.0],
        yaw: 0.0,
        instances: 1,
        space: "crank-group local (renderer rotates about X)",
    },
];

/// The anchor row for one template slot.
#[must_use]
pub fn anchor(template: &str) -> Option<&'static Anchor> {
    ANCHORS.iter().find(|row| row.template == template)
}

/// The yaw of clone `index` (0-based) for a template: the left oar rig and
/// the second ski keep identity except the oar's π mirror.
#[must_use]
pub fn clone_yaw(template: &str, index: u8) -> f32 {
    if template == "equipment:row:oar-rig" && index == 1 {
        PI
    } else {
        0.0
    }
}

/// The position of clone `index`: skis mirror on x, oar locks on x.
#[must_use]
pub fn clone_position(template: &str, index: u8) -> [f32; 3] {
    let base = match anchor(template) {
        Some(row) => row.position,
        None => return [0.0, 0.0, 0.0],
    };
    let side: f32 = if index == 0 { 1.0 } else { -1.0 };
    match template {
        "equipment:ski:ski-assembly" => [SKI_ANCHOR[0] * side, SKI_ANCHOR[1], SKI_ANCHOR[2]],
        "equipment:row:oar-rig" => [OARLOCK_PIVOT[0] * side, OARLOCK_PIVOT[1], OARLOCK_PIVOT[2]],
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_matches_the_readme_rows() {
        assert_eq!(ANCHORS.len(), 7);
        let boat = anchor("equipment:row:boat-assembly").expect("boat");
        assert_eq!(boat.instances, 1);
        assert_eq!(boat.space, "row avatar root");
        // Oarlocks meet the animated pivots at (±0.88, 0.51, 0.28).
        assert_eq!(clone_position("equipment:row:oar-rig", 0), OARLOCK_PIVOT);
        assert_eq!(
            clone_position("equipment:row:oar-rig", 1),
            [-0.88, 0.51, 0.28]
        );
        assert_eq!(clone_yaw("equipment:row:oar-rig", 1), PI);
        assert_eq!(clone_yaw("equipment:row:oar-rig", 0), 0.0);
        // One measured ski per side at (side × 0.15, 0, 0.16).
        assert_eq!(clone_position("equipment:ski:ski-assembly", 0), SKI_ANCHOR);
        assert_eq!(
            clone_position("equipment:ski:ski-assembly", 1),
            [-0.15, 0.0, 0.16]
        );
        assert_eq!(
            anchor("equipment:ski:ski-assembly").expect("ski").instances,
            2
        );
        assert!(anchor("nope").is_none());
    }

    #[test]
    fn every_v3_template_has_a_row() {
        for template in crate::replay::glb::V3_TEMPLATE_ROOTS {
            assert!(anchor(template).is_some(), "{template} missing");
        }
    }
}
