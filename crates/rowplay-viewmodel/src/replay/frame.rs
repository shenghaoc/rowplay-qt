// SPDX-License-Identifier: GPL-3.0-or-later
//! The per-frame bridge bundle layout (Phase 5b spec R1.3, design "Frame
//! bundle").
//!
//! One `tick` produces one flat `Vec<f32>` that crosses the bridge once;
//! QML indexes it by the named offsets below (exported as JSON so no magic
//! number lives in QML). Every quantity is in the replay's units: metres,
//! seconds, degrees for the camera field of view, radians for rig angles and
//! joint quaternion components as `x, y, z, w` after the three translation
//! components.

use serde_json::{Map, Value, json};

/// Index of the frame sequence number (`u32` stored as `f32` bits).
pub const SEQUENCE: usize = 0;

/// HUD block: distance (m), pace (s/500 m), rate (spm), elapsed (s), speed
/// (m/s), ghost gap (m, NaN when no ghost), finish ETA (s), progress (0..1).
pub const HUD_DISTANCE: usize = 1;
/// See [`HUD_DISTANCE`].
pub const HUD_PACE: usize = 2;
/// See [`HUD_DISTANCE`].
pub const HUD_RATE: usize = 3;
/// See [`HUD_DISTANCE`].
pub const HUD_ELAPSED: usize = 4;
/// See [`HUD_DISTANCE`].
pub const HUD_SPEED: usize = 5;
/// See [`HUD_DISTANCE`].
pub const HUD_GHOST_GAP: usize = 6;
/// See [`HUD_DISTANCE`].
pub const HUD_FINISH_ETA: usize = 7;
/// See [`HUD_DISTANCE`].
pub const HUD_PROGRESS: usize = 8;

/// Camera block: position xyz, aim xyz, vertical field of view (degrees).
pub const CAMERA_POSITION: usize = 9;
/// See [`CAMERA_POSITION`].
pub const CAMERA_AIM: usize = 12;
/// See [`CAMERA_POSITION`].
pub const CAMERA_FOV: usize = 15;

/// Course block: the rig root's x and z on the loop, its yaw as a
/// quaternion `x, y, z, w`, then the profile accents bob (m) and surge (m)
/// and the roll as a quaternion.
pub const COURSE_X: usize = 16;
/// See [`COURSE_X`].
pub const COURSE_Z: usize = 17;
/// See [`COURSE_X`].
pub const COURSE_YAW: usize = 18;
/// See [`COURSE_X`].
pub const ACCENT_BOB: usize = 22;
/// See [`COURSE_X`].
pub const ACCENT_SURGE: usize = 23;
/// See [`COURSE_X`].
pub const ACCENT_ROLL: usize = 24;

/// Number of semantic athlete joints carried per frame (contract order).
pub const JOINT_COUNT: usize = 19;
/// Floats per joint: translation xyz then rotation quaternion xyzw.
pub const JOINT_STRIDE: usize = 7;
/// Start of the joint block (`JOINT_COUNT × JOINT_STRIDE` floats).
pub const JOINTS: usize = 28;

/// Equipment block, a fixed superset across sports (unused entries are the
/// identity / zero): every rotation is a quaternion `x, y, z, w` QML assigns
/// as-is, so nothing is composed per frame in QML.
pub const EQUIPMENT: usize = JOINTS + JOINT_COUNT * JOINT_STRIDE;
/// RowErg seat carriage z offset (m).
pub const EQ_SEAT_Z: usize = EQUIPMENT;
/// RowErg left oar-rig instance rotation (mirror, sweep and feather folded in).
pub const EQ_OAR_LEFT: usize = EQUIPMENT + 1;
/// RowErg right oar-rig instance rotation.
pub const EQ_OAR_RIGHT: usize = EQUIPMENT + 5;
/// RowErg blade roll about the shaft, degrees (web `(1 − bladeFeather) · 90`).
pub const EQ_BLADE_ROLL_DEG: usize = EQUIPMENT + 9;
/// SkiErg left pole: root position xyz then rotation xyzw.
pub const EQ_POLE_LEFT: usize = EQUIPMENT + 10;
/// SkiErg right pole: root position xyz then rotation xyzw.
pub const EQ_POLE_RIGHT: usize = EQUIPMENT + 17;
/// BikeErg crank rotation about local X.
pub const EQ_CRANK: usize = EQUIPMENT + 24;
/// BikeErg wheel rotation about local X (both wheels).
pub const EQ_WHEEL: usize = EQUIPMENT + 28;

// ---- Leaf transforms (computed in Rust, assigned in QML) ----

/// RowErg left blade: position xyz then rotation xyzw (7 floats).
pub const EQ_BLADE_LEFT: usize = EQUIPMENT + 32;
/// RowErg right blade: position xyz then rotation xyzw (7 floats).
pub const EQ_BLADE_RIGHT: usize = EQUIPMENT + 39;
/// SkiErg left pole leaf positions (shaft/grip/basket, 3×xyz = 9 floats).
/// The rotation is the same as `EQ_POLE_LEFT`; the scale is constant
/// (stored in `equipmentLayout.poleLeafScales`).
pub const EQ_POLE_LEAVES_LEFT: usize = EQUIPMENT + 46;
/// SkiErg right pole leaf positions (shaft/grip/basket, 3×xyz = 9 floats).
pub const EQ_POLE_LEAVES_RIGHT: usize = EQUIPMENT + 55;

/// Floats in the equipment block.
pub const EQUIPMENT_COUNT: usize = 64;

/// Total frame length in floats.
pub const LENGTH: usize = EQUIPMENT + EQUIPMENT_COUNT;

/// Offset of joint `index`'s translation x.
#[must_use]
pub const fn joint_offset(index: usize) -> usize {
    JOINTS + index * JOINT_STRIDE
}

/// The layout as a JSON object for QML (`Replay.frameLayout`): every named
/// offset plus `jointCount`, `jointStride` and `length`.
#[must_use]
pub fn layout_json() -> Value {
    let mut map = Map::new();
    for (name, value) in [
        ("sequence", SEQUENCE),
        ("hudDistance", HUD_DISTANCE),
        ("hudPace", HUD_PACE),
        ("hudRate", HUD_RATE),
        ("hudElapsed", HUD_ELAPSED),
        ("hudSpeed", HUD_SPEED),
        ("hudGhostGap", HUD_GHOST_GAP),
        ("hudFinishEta", HUD_FINISH_ETA),
        ("hudProgress", HUD_PROGRESS),
        ("cameraPosition", CAMERA_POSITION),
        ("cameraAim", CAMERA_AIM),
        ("cameraFov", CAMERA_FOV),
        ("courseX", COURSE_X),
        ("courseZ", COURSE_Z),
        ("courseYaw", COURSE_YAW),
        ("accentBob", ACCENT_BOB),
        ("accentSurge", ACCENT_SURGE),
        ("accentRoll", ACCENT_ROLL),
        ("joints", JOINTS),
        ("jointCount", JOINT_COUNT),
        ("jointStride", JOINT_STRIDE),
        ("equipment", EQUIPMENT),
        ("seatZ", EQ_SEAT_Z),
        ("oarLeft", EQ_OAR_LEFT),
        ("oarRight", EQ_OAR_RIGHT),
        ("bladeRollDeg", EQ_BLADE_ROLL_DEG),
        ("poleLeft", EQ_POLE_LEFT),
        ("poleRight", EQ_POLE_RIGHT),
        ("crank", EQ_CRANK),
        ("wheel", EQ_WHEEL),
        ("bladeLeft", EQ_BLADE_LEFT),
        ("bladeRight", EQ_BLADE_RIGHT),
        ("poleLeavesLeft", EQ_POLE_LEAVES_LEFT),
        ("poleLeavesRight", EQ_POLE_LEAVES_RIGHT),
        ("length", LENGTH),
    ] {
        map.insert(name.to_owned(), json!(value));
    }
    Value::Object(map)
}

/// A zero-filled frame of the right length.
#[must_use]
pub fn empty_frame() -> Vec<f32> {
    vec![0.0; LENGTH]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blocks_are_contiguous_and_the_length_is_stable() {
        assert_eq!(HUD_DISTANCE, SEQUENCE + 1);
        assert_eq!(CAMERA_POSITION, HUD_PROGRESS + 1);
        assert_eq!(CAMERA_AIM, CAMERA_POSITION + 3);
        assert_eq!(CAMERA_FOV, CAMERA_AIM + 3);
        assert_eq!(COURSE_X, CAMERA_FOV + 1);
        assert_eq!(COURSE_YAW, COURSE_Z + 1);
        assert_eq!(ACCENT_BOB, COURSE_YAW + 4);
        assert_eq!(ACCENT_ROLL, ACCENT_SURGE + 1);
        assert_eq!(JOINTS, ACCENT_ROLL + 4);
        assert_eq!(EQUIPMENT, JOINTS + 19 * 7);
        assert_eq!(EQ_OAR_RIGHT, EQ_OAR_LEFT + 4);
        assert_eq!(EQ_POLE_RIGHT, EQ_POLE_LEFT + 7);
        assert_eq!(EQ_WHEEL, EQ_CRANK + 4);
        assert_eq!(EQ_WHEEL + 4, EQUIPMENT + 32);
        assert_eq!(EQ_BLADE_LEFT, EQUIPMENT + 32);
        assert_eq!(EQ_BLADE_RIGHT, EQ_BLADE_LEFT + 7);
        assert_eq!(EQ_POLE_LEAVES_LEFT, EQ_BLADE_RIGHT + 7);
        assert_eq!(EQ_POLE_LEAVES_RIGHT, EQ_POLE_LEAVES_LEFT + 9);
        assert_eq!(EQ_POLE_LEAVES_RIGHT + 9, EQUIPMENT + EQUIPMENT_COUNT);
        assert_eq!(LENGTH, 28 + 133 + 64);
        assert_eq!(joint_offset(18) + JOINT_STRIDE, EQUIPMENT);
        assert_eq!(empty_frame().len(), LENGTH);
    }

    #[test]
    fn the_json_layout_names_every_offset() {
        let layout = layout_json();
        let object = layout.as_object().expect("object");
        assert_eq!(object["length"], json!(LENGTH));
        assert_eq!(object["joints"], json!(JOINTS));
        assert_eq!(object["jointCount"], json!(19));
        assert_eq!(object["jointStride"], json!(7));
        assert_eq!(object["cameraFov"], json!(CAMERA_FOV));
        assert_eq!(object["wheel"], json!(EQ_WHEEL));
        assert_eq!(object.len(), 35);
        // Offsets are unique and inside the frame.
        let mut seen = std::collections::BTreeSet::new();
        for (name, value) in object {
            let value = value.as_u64().expect("integer") as usize;
            // Counts, and the equipment block start (which is also seatZ).
            if matches!(
                name.as_str(),
                "jointCount" | "jointStride" | "length" | "equipment"
            ) {
                continue;
            }
            assert!(value < LENGTH, "{name} past the end");
            assert!(seen.insert(value), "{name} duplicates an offset");
        }
    }
}
