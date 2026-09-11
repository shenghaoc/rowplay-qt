// SPDX-License-Identifier: GPL-3.0-or-later
//! Replay core: the pure, renderer-neutral replay domain ported from the
//! rowplay web app (`src/lib/replay/*.ts`), with rowplay-studio
//! (`Sources/RowPlayCore/Replay/*.swift`) as the second reference.
//!
//! The module tree keeps the web file names so `docs/source-map.md` stays
//! greppable: `engine`, `motion`, `stroke_model`, `motion_graph`,
//! `sport_kinematics`, `comparability`, `ghost_pick`, `race_gap`,
//! `race_result`, `rivals`, `quality` and the 2D `theme` palettes.
//! Everything here is Qt-free and side-effect-free: playback is driven by
//! externally supplied ticks, parsers consume byte slices, and the motion
//! graph is a deterministic function of a [`stroke_model::StrokePose`].

pub mod comparability;
pub mod engine;
pub mod ghost_pick;
pub mod motion;
pub mod motion_graph;
pub mod quality;
pub mod race_gap;
pub mod race_result;
pub mod rivals;
pub mod sport_kinematics;
pub mod stroke_model;
pub mod theme;

pub use comparability::{ComparabilityAxis, ComparableContext, are_comparable, classify_axis};
pub use engine::{Frame, ReplaySpeed, ReplayState, sample_at, sample_index_at};
pub use ghost_pick::{GhostPickContext, pick_default_ghost_candidate};
pub use motion::{
    ParticlePool, PerfGovernor, catch_events, clamp_dt, damp_factor, meters_per_cycle,
    stroke_surge, warp_stroke_phase, warp_stroke_phase_rate,
};
pub use motion_graph::{ReplayMotionGraph, sample_motion_graph};
pub use quality::{QualityBudgets, RenderQuality, RendererKind};
pub use race_gap::{
    absolute_time, finish_delta_sec, ghost_dist_at_player_finish, ghost_distance, ghost_frame,
    player_dist_at_ghost_finish, race_gap_metres, race_gap_seconds, relative_duration,
};
pub use race_result::{RaceOutcome, RaceResult, race_result, time_crossing_target};
pub use rivals::{
    ParsedTrace, RivalParseError, constant_pace_ghost, constant_pace_strokes, parse_rival_file,
};
pub use sport_kinematics::{
    BikeKinematics, RowerKinematics, SkierElbowDirection, SkierKinematics, solve_bike_kinematics,
    solve_rower_kinematics, solve_skier_elbow_direction, solve_skier_kinematics,
};
pub use stroke_model::{
    PoseContext, StrokePose, StrokeTimeline, StrokeTimelineEntry, build_stroke_timeline,
    catch_transitions, compute_at_time, fallback_stroke_pose, reduced_motion, stroke_pose_at,
};
