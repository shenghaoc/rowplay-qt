// SPDX-License-Identifier: GPL-3.0-or-later
//! The replacement changes anatomy, not the semantic frame or motion graph.
use rowplay_viewmodel::replay::athlete::{LocalTransform, V4Athlete, read_v4};
use rowplay_viewmodel::replay::equipment::quat_mul;
use rowplay_viewmodel::replay::grip::collect_hand_chains;

const GLB: &[u8] = include_bytes!("../../../assets/replay/authored/rowplay-athlete-v5.glb");
const OLD_GLB: &[u8] = include_bytes!("../../../assets/replay/rowplay-athlete-v4.glb");
const CONTRACT: &str =
    include_str!("../../../assets/replay/authored/rowplay-athlete-v5.contract.json");

fn old() -> V4Athlete {
    read_v4(
        OLD_GLB,
        include_str!("../../../assets/replay/rowplay-athlete-v4.contract.json"),
    )
    .unwrap()
}

fn world(a: &V4Athlete, pose: &[LocalTransform], joint: usize) -> [f64; 4] {
    a.joints[joint].parent.map_or(pose[joint].rotation, |p| {
        quat_mul(world(a, pose, p), pose[joint].rotation)
    })
}

fn inverse(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

fn verify_old_semantic_bind_bases() {
    let size = u32::from_le_bytes(OLD_GLB[12..16].try_into().unwrap()) as usize;
    let doc: serde_json::Value = serde_json::from_slice(&OLD_GLB[20..20 + size]).unwrap();
    let skin = &doc["skins"][0];
    let accessor = &doc["accessors"][skin["inverseBindMatrices"].as_u64().unwrap() as usize];
    let view = &doc["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
    assert_eq!(accessor["componentType"], 5126);
    assert_eq!(accessor["type"], "MAT4");
    assert!(view.get("byteStride").is_none());
    let offset = 28
        + size
        + view["byteOffset"].as_u64().unwrap_or(0) as usize
        + accessor["byteOffset"].as_u64().unwrap_or(0) as usize;
    let before = old();
    for name in before.semantic.iter().map(|&i| &before.joints[i].name) {
        let index = skin["joints"]
            .as_array()
            .unwrap()
            .iter()
            .position(|node| doc["nodes"][node.as_u64().unwrap() as usize]["name"] == *name)
            .unwrap();
        for column in 0..3 {
            for row in 0..3 {
                let start = offset + index * 64 + (column * 4 + row) * 4;
                let actual = f32::from_le_bytes(OLD_GLB[start..start + 4].try_into().unwrap());
                assert!((actual - f32::from(column == row)).abs() < 1e-7, "{name}");
            }
        }
    }
}

#[test]
fn anatomical_rest_bases_preserve_inherited_world_motion() {
    verify_old_semantic_bind_bases();
    let a = read_v4(GLB, CONTRACT).unwrap();
    let before = old();
    assert_eq!(a.joints.len(), 51);
    assert_eq!(a.semantic.len(), 19);
    for (old_clip, new_clip) in before.clips.iter().zip(&a.clips) {
        assert_eq!(old_clip.name, new_clip.name);
        assert_eq!(old_clip.drive_end, new_clip.drive_end);
        assert_eq!(old_clip.duration, new_clip.duration);
        for (o, n) in old_clip.channels.iter().zip(&new_clip.channels) {
            assert_eq!(o.times, n.times);
            assert_eq!(o.interpolation, n.interpolation);
        }
        for time in [0.0, 0.08, 0.19, 0.38, 0.54, 0.72, 0.88, 0.999] {
            let op = before.sample(old_clip, time);
            let np = a.sample(new_clip, time);
            for (&o, &n) in before.semantic.iter().zip(&a.semantic) {
                assert_eq!(before.joints[o].name, a.joints[n].name);
                // V4 node TRS stores the initial animated pose. Its actual
                // semantic inverse-bind rotations, verified above, are identity.
                let oq = world(&before, &op, o);
                let nq = quat_mul(world(&a, &np, n), inverse(world(&a, &a.rest_pose(), n)));
                let dot: f64 = oq.iter().zip(nq).map(|(x, y)| x * y).sum();
                // float32 GLB channels, quaternion sign is immaterial.
                assert!(
                    (1.0 - dot.abs()).abs() < 2e-6,
                    "{} @ {time}",
                    a.joints[n].name
                );
            }
        }
    }
    for joint in &before.joints {
        let new = &a.joints[a.joint_index(&joint.name).unwrap()];
        assert_eq!(
            joint.parent.map(|p| &before.joints[p].name),
            new.parent.map(|p| &a.joints[p].name)
        );
    }
}

#[test]
fn terminal_reach_is_the_saved_mesh_measurement() {
    let a = read_v4(GLB, CONTRACT).unwrap();
    for side in [-1.0, 1.0] {
        let c = a.calibration_for_hand(side).unwrap();
        for chain in collect_hand_chains(&a, side).unwrap() {
            assert_eq!(
                chain.tip_length,
                c.terminal_lengths[&chain.joints[2].helper]
            );
        }
    }
}

#[test]
fn incomplete_calibration_and_false_counts_fail_closed() {
    let original: serde_json::Value = serde_json::from_str(CONTRACT).unwrap();
    for key in ["handCalibration", "materialParameters"] {
        let mut c = original.clone();
        c.as_object_mut().unwrap().remove(key);
        assert!(read_v4(GLB, &c.to_string()).is_err(), "missing {key}");
    }
    let mut c = original;
    c["measurements"]["triangles"] = 1.into();
    assert!(read_v4(GLB, &c.to_string()).is_err());
}
