# Phase 5c — Quality tiers and polish: design

## Tier application

`rowplay_viewmodel::replay::quality::TierSettings` maps
`RenderQuality` + sport to a plain-data struct the QML scene binds:
`{ shadows: bool, shadow_map: u16, msaa: u8, env_texture_sets: Vec<&'static
str>, normal_maps: bool, buoys_per_ring: u16, lane_markers: u16, wake_cap:
u16, spray_cap: u16 }`, sourced from `replay::quality::budgets()` and the
web `QUALITY` table's portable fields (the browser-only fields — `dprCap`,
`bodySegments` — are recorded as divergences: Qt controls DPR through the
window, and the V4 athlete has no segment LOD).

`ReplayScene.qml` binds `environment.textureSets` (a list of qrc paths built
by the view-model from the environments README table) and creates `Texture`
objects only for the listed sets; Low/Medium produce an empty list so no
image is decoded. Shadow map size maps to `DirectionalLight.shadowFactor` /
`ExtendedSceneEnvironment` settings plus `View3D` MSAA via
`renderSettings.effectiveAASamples`.

## Governor

The `Replay` singleton feeds `PerfGovernor::sample(frame_ms)` from the tick
delta; when `governor.level()` changes and the mode is Automatic, the scene's
effective tier = `preferences.tier.degraded(level)`. Pinned tier ⇒ governor
samples but never applies. Diagnostics strip reads `Replay.diagnosticsText`
(one Rust-formatted string, updated at 4 Hz, not per frame).

## Ghosts

A second athlete/equipment node group shares the loaded meshes (Model
instances), material = ghost variant from 5a's resolver, driven by
`ghost_frame(player_state, rival_trace, t)`; the gap overlay is two Labels
bound to `Replay.gapText` / `Replay.verdictText`.

## Measurements

`tools/measure-replay-frames.py` is *not* added; instead a gate-adjacent
opt-in (`ROWPLAY_REPLAY_BENCH=1`) runs 600 ticks per sport per tier, logs
median/p95 through `PrivacySafeLogger`, and writes
`artifacts/replay-bench-<gpu>.json`. The PR quotes the file. The renderer
string is captured from `QQuickWindow`/GL info at startup and included.
