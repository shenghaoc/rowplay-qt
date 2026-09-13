# Phase 5b — Replay playback: design

## Frame bundle

`tick(dt)` in the `Replay` singleton advances `ReplayState`, then samples
`sample_at`, `stroke_pose_at`, `sample_motion_graph`, and the camera, and
packs one `Vec<f32>`:

```
[0]      sequence (f32 bits of u32)
[1..8]   hud: distance_m, pace_s_per_500, rate_spm, elapsed_s, speed_m_s,
           ghost_gap_m (NaN when no ghost), finish_eta_s
[9..16]  camera: pos xyz, aim xyz, fov
[17..]   pose: per sport — rower: seat, torso, arms, head, blade angles…;
           athlete joints 19×7 (t xyz + q xyzw); equipment scalars
```

The exact layout is a `pub const` table in `rowplay_viewmodel::replay::frame`
(with `offsets()` accessors and a layout-length test) so QML indexes it by
named constant rather than magic numbers. HUD *strings* ride alongside as one
`String` property built in the same tick (`"distance|pace|rate|elapsed|…"`,
split in QML only for display placement — no parsing of numbers).

QML reads `Replay.poseFrame` (a `var` list) inside a single
`onPoseFrameChanged` handler that writes joints/materials/camera in one pass;
the handler is the only consumer. `FrameAnimation { onTriggered:
Replay.tick(dt) }` is the only clock.

## Athlete posing (R2)

Phase → clip time: `clip_t = warp-shaped cycle phase × clip.duration`; the
Qt side samples the loaded glTF animation by setting
`Skeleton`/joint transforms computed in Rust from the clip's keyframe tracks
*read out of the GLB at load* (the glb reader of 5a also parses animation
channels: input/output accessors, cubic/linear interpolation) — i.e. Rust
evaluates the authored clip, not Qt's animation system, so the contact pass
composes exactly. Contact pass: hand/foot world targets from the sport
kinematics (`solve_rower/skier/bike_kinematics`), then the analytic correction
rotates the terminal bone so the contract's local offset lands on the target
(bounded rotation, web's clamp constants).

Decision gate (R2.3), resolved before the first 5b commit: the `balsam`
component (ADR 0008) exposes every bone as a named `Node` inside
`Skin { joints: [...] }`, so the joint write path is plain QML property
assignment; no ADR, no V3-shell fallback. The clip data is read from the GLB
(three LINEAR clips of 20 channels — 19 rotations plus the hips translation —
over 1 s, 14/14/9 keyframes) and evaluated in Rust; the component itself
carries no animation. Contact targets: `rowplay_core::replay::rig_pose`, a
port of Studio's `ReplayRigPose` solver (per-sport calibration ranges over the
motion-graph channels: seat/handle/oar for the rower, the shoulder-arc hand
path and planted basket for the skier, crank/pedal/wheel for the bike),
supplies the pelvis, hand and foot targets the two-bone contact pass closes
on, using the contract's per-bone local offsets.

## Equipment and anchors

Anchors are the 5a view-model table. Each template root is a named `Node` of
the `balsam` `Rigs` component (ADR 0008); repeated templates (ski pair,
wheels) are cloned as `Model` children sharing the generated mesh source, or
through `instancing`. The whole rig rides a course `Node` placed by the
web's loop (R3.4: `a = metres / 1000 · 2π`, live radius 30 m, tangent yaw,
+π for the row shell) with the profile accents (bob, surge, roll) from the
motion graph. Per-frame equipment scalars from the frame bundle:
`seat_travel` → carriage Z; oarlock pivot angles from `blade_water` +
`oarlock_load` envelope; ski poles from hand contacts with the planted-pole
anchor from `catch_events`; crank angle from `PedalMotion.left/right`
circular channels; wheel spin integrated from speed (circular, no wrap pop).

## Camera

`rowplay_viewmodel::replay::camera::chase(frame, prev, dt, reduce_motion) ->
CameraState` ports the web block verbatim (constants in `framing(sport)`),
using `damp_factor` from core. Golden test R6.4 captures web values via a
small Node harness against the pinned checkout (like the dates golden test)
— if the harness is infeasible, the constants + rates are asserted literally
against `renderer3d.ts` lines with the source line quoted in the test.

## HUD and Settings

`hud_strings` reuses `formatting::{fmt_distance, fmt_pace, fmt_time}` plus
`dates` where relevant; the reduce-motion toggle is a `CheckBox` in
`SettingsScreen.qml` bound to `Settings.reduceReplayMotion` (new qproperty +
slot persisting through `AppState::update_prefs`).

## Verification

Gate steps 60–74: per sport, load, play, sample at the five fixed times
(pausing between samples for a deterministic grab), screenshot
`replay-play-<sport>-<i>.png`. `bridge_crossings.rs` drives `tick` in a
headless Qt test (the singleton is constructible without a window) and
asserts `notify_count == ticks` and `poseFrame` length == layout length.
