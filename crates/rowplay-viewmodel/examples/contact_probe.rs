// SPDX-License-Identifier: GPL-3.0-or-later
//! Reproduce the Phase 5 native capture's exact debug-clock samples in Rust.
use rowplay_core::models::Sport;
use rowplay_core::replay::hand_grip::solve_hand_grip_closure;
use rowplay_core::replay::rig_pose::solve_rig_pose;
use rowplay_core::replay::stroke_model::fallback_stroke_pose;
use rowplay_viewmodel::replay::athlete::read_v4;
use rowplay_viewmodel::replay::grip::{
    closure_options, collect_hand_chains, grip_frames, warped_cycle,
};
use rowplay_viewmodel::replay::pose::{PoseSolver, clip_fraction, rig_targets};
use serde_json::json;

fn main() {
    let athlete = read_v4(
        include_bytes!("../../../assets/replay/rowplay-athlete-v4.glb"),
        include_str!("../../../assets/replay/rowplay-athlete-v4.contract.json"),
    )
    .expect("vendored V4");
    let solver = PoseSolver::new(&athlete).expect("solver");
    let mut reports = Vec::new();
    for (sport, name, steps) in [
        (Sport::Rower, "rower", &[0, 380, 760, 1380][..]),
        (Sport::Skierg, "skierg", &[1900, 0, 300, 540, 1100][..]),
        (Sport::Bike, "bike", &[0, 500][..]),
    ] {
        let closure: Vec<_> = [-1.0,1.0].into_iter().map(|side| {
            let chains = collect_hand_chains(&athlete, side).expect("chains");
            let solved = solve_hand_grip_closure(&chains, &closure_options(sport,side));
            json!({"side":side,
                "chains": chains.iter().map(|c| json!({"digit":c.digit,"tip_length":c.tip_length,
                    "joints":c.joints.iter().map(|j|json!({"name":j.helper,"position":j.position})).collect::<Vec<_>>() })).collect::<Vec<_>>(),
                "contacts":solved.contacts.iter().map(|c|json!({"digit":c.digit,"distance":c.surface_distance,
                    "segment_distance":c.segment_surface_distance,"contact":c.contact,"tip":c.tip})).collect::<Vec<_>>(),
                "flex":solved.poses.iter().map(|p|json!({"helper":p.helper,"flex":p.flex,"oppose":p.oppose})).collect::<Vec<_>>()})
        }).collect();
        let clip = athlete.clip_for(name).expect("clip");
        let dense: Vec<i32> = if std::env::args().any(|arg| arg == "--dense") {
            (0..2000).collect()
        } else {
            steps.to_vec()
        };
        let samples:Vec<_> = dense.iter().map(|&step| {
            let stroke=fallback_stroke_pose(sport,f64::from(step)/2000.0*std::f64::consts::TAU,30.0);
            let rig=solve_rig_pose(sport,&stroke,f64::from(step)*3.0,false);
            let targets=rig_targets(&rig);
            let fraction=clip_fraction(stroke.cycle_frac,stroke.phase,stroke.drive_frac,clip.drive_end);
            let posed=solver.pose(&athlete,sport,clip,fraction*f64::from(clip.duration),&targets.contacts,
                &grip_frames(sport,&rig,targets.poles,warped_cycle(stroke.warped_phase)),targets.oar);
            json!({"step":step,"cycle":stroke.cycle_frac,"drive_frac":stroke.drive_frac,
                "pelvis":targets.contacts.pelvis,"hand_targets":[targets.contacts.left_hand,targets.contacts.right_hand],
                "hand_residuals":[posed.residuals.left_hand,posed.residuals.right_hand],
                "oar_yaw":posed.oar_yaw,
                "wrist":posed.wrist.map(|w|json!({"twist":w.twist,"flexion":w.flexion,"deviation":w.deviation,
                    "requested_twist":w.requested_twist,"clamped_swing":w.clamped_swing})),
                "locals":athlete.joints.iter().zip(&posed.locals).map(|(j,p)|json!({"name":j.name,
                    "translation":p.translation,"rotation":p.rotation})).collect::<Vec<_>>()})
        }).collect();
        reports.push(json!({"sport":name,"closure":closure,"samples":samples}));
    }
    println!("{}", serde_json::to_string_pretty(&reports).expect("JSON"));
}
