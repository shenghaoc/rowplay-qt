# Phase 5b — Replay playback: requirements

Plays a workout back in the 5a scene: transport, the skinned V4 athlete driven
by the Phase 2 motion graph and stroke pose, equipment cloned onto its rig
anchors, the web's chase camera, and a HUD whose numbers are formatted in
Rust. All replay maths comes from `rowplay-core::replay`; 5b adds none.

## R1 — Transport

- R1.1 Play / pause / seek / speed over `replay::engine::ReplayState`, owned by
  a `Replay` backend singleton. QML drives it with one `FrameAnimation`
  calling exactly one `#[qslot] tick(dt)` per frame (AGENTS.md rule,
  qt-bridges-notes #4); `dt` is clamped with `replay::motion::clamp_dt` inside
  Rust.
- R1.2 Speed comes from `ReplaySpeed` (0.5×–8×, web labels); seek is a
  normalized 0..1 slider value mapped to the workout duration; both are slots.
- R1.3 The whole per-frame pose crosses the bridge as **one** flat
  `Vec<f32>` property (`poseFrame`) plus a `frameSeq` counter and a compact
  HUD string bundle; never per-bone properties. A Rust test asserts the
  per-frame bridge crossing count is exactly one (a counting proxy in the
  singleton increments on `tick` and on every notify emitted from within it;
  the test drives 600 ticks and asserts the counts).
- R1.4 Playback works with demo data and no token; seeking a workout with no
  stroke data uses the split-derived timeline exactly like the web.

## R2 — Athlete

- R2.1 The skinned `rowplay-athlete-v4.glb` is posed from
  `replay::motion_graph` + `replay::stroke_model`: clip phase from
  `warp_stroke_phase`-shaped cycle phase, per-sport clip
  (`rowplay-v4-row-cycle` / `-ski-cycle` / `-bike-cycle`, drive ends
  0.38 / 0.34 / 0.5 from the contract JSON), then the analytic contact pass
  (hand/foot local offsets from the contract) applied after the authored
  pose, as the web does (Phase 2 divergence rows; the web pipeline is
  canonical, Studio's frame-based path stays a test oracle only).
- R2.2 Bone targets are the 19 semantic bones in contract order; the 32 grip
  helpers are never direct animation targets (contract). Skeleton bone names
  and contact offsets are read from the vendored contract JSON at load and
  cross-checked against the loaded GLB's skin (named-slot errors on drift).
- R2.3 Skinning is applied through Qt Quick 3D `Skeleton`/`Joint` poses built
  in Rust and exposed as one flat `Vec<f32>` of joint translations +
  quaternion components (19 × (3+4)); QML writes them onto the loaded
  skeleton in one pass. If qtbridge/Qt Quick 3D cannot retarget a loaded
  glTF skeleton without C++, 5b documents the blocker in an ADR and falls
  back to the V3 leaf-slot athlete (bbox-fitted shells) for this PR, keeping
  the V4 load + validation in place for 5c/Phase 7. (Decision gate, recorded
  either way.)
- R2.4 `reduce_replay_motion` (Phase 4 preference) freezes articulation and
  camera damping when set — the deferred Phase 4 toggle lands here in
  Settings (R6.3).

## R3 — Equipment

- R3.1 Each of the 7 templates is cloned onto its anchor exactly as
  `assets/README.md` tables it: boat assembly at the row avatar root with
  oarlocks meeting the animated pivots at `(±0.88, 0.51, 0.28)`; oar-rig
  identity on the right, yaw π on the left; seat carriage translating with
  the pelvis over the static rails; ski assembly cloned per side at
  `(side × 0.15, 0, 0.16)`; bike wheel assembly at the wheel-group centre
  (axle local X, two clones); frame assembly at the bike avatar root;
  drivetrain assembly rotating about X in crank-group-local coordinates.
- R3.2 Moving parts follow the contacts the rig provides: seat carriage from
  `seat_travel`, oars from the oarlock pivots + `blade_water`/`blade_feather`,
  poles from the ski hand contacts with planted-pole anchoring, crank from
  `PedalMotion` circular channels, wheels from course speed.
- R3.3 Cloning uses Qt `InstanceList`/`Model.instancing` where a template is
  repeated (ski pair, wheels, buoys in 5c); single-instance templates are
  plain `Model` children positioned by the anchor.

## R4 — Camera

- R4.1 The chase camera ports `renderer3d.ts`: per-sport framing constants
  (`rower {back 4.05, height 1.78, ahead 0.88, lateral 2.16, aimY 0.84}`,
  `skierg {3.15, 2.3, 0.9, 1.86, 1.14}`, `bike {3.12, 1.96, 0.58, 1.92,
  0.92}`), base FOV 40/42/42, speed FOV gain 2, and the damping rates
  (`positionRate = 8 + min(18, smoothedSpeed × 0.55)`,
  `aimRate = 6 + speedFollow × 0.65`) via `replay::motion::damp_factor`.
  Constants live in the view-model; the camera maths runs in Rust inside
  `tick` and crosses as part of the flat frame.
- R4.2 Paused renders snap only when the target jumped (seek, workout
  change), exactly like the web's sub-metre trailing-lag rule; reduced motion
  disables lag entirely.

## R5 — HUD

- R5.1 Distance, pace, rate and elapsed time render from `rowplay-core`
  formatters through the view-model (`hud_strings(frame, prefs) ->
  HudStrings`), exposed as locale ids + pre-formatted strings; QML contains
  no number formatting. Existing locale ids only (`replay.gPace`,
  `replay.gRate`, …); any genuinely new string is recorded in
  `docs/source-map.md` as a substitution, never invented in the catalogue.
- R5.2 The HUD updates from the same single per-frame crossing (part of the
  frame bundle); no second timer.

## R6 — Verification

- R6.1 The gate plays the default demo workout per sport, samples at fixed
  times (0 s, 25 %, 50 %, 75 %, end), and screenshots each sport into
  `ROWPLAY_SMOKE_SCREENSHOT_DIR`; artifacts upload in CI.
- R6.2 `crates/rowplay-app/tests/bridge_crossings.rs`: 600 driven ticks
  assert exactly one notify bundle per tick (R1.3).
- R6.3 Settings gains the reduce-motion toggle bound to
  `Preferences.reduce_replay_motion` (deferred from Phase 4).
- R6.4 Parity: a view-model test asserts the camera target for a fixed frame
  equals a golden tuple captured from the web renderer at the same inputs
  (tolerance 1e-6), and the pose layout length is stable (19 × 7 + extras).
