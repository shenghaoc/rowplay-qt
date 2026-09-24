# Phase 7 — Motion: design

## Architecture (unchanged from the web's production shape)

```
motion graph (core, 1e-10 parity) ─▶ solve_rig_pose ─▶ rig_targets (pelvis/hands/feet)
                                                            │
        clip ─▶ clip_fraction ─▶ PoseSolver.pose ───────────┤ two-bone IK (arms, legs)
                                                            ▼
   hand_grip closure (core, NEW, install-time) ─▶ digit poses (static per sport)
   orientHandToGripChannel + wrist budgets (NEW) ─▶ hand semantic-joint rotations (per frame)
```

The port already implements the web's "clip-contact-constrained" split; this
phase adds the hand layer the port leaves at clip identity, and enables the
two fixtures that define done.

## `rowplay_core::replay::hand_grip`

Port of `handGrip.ts` (976 lines) in three layers, each independently tested:

1. **Geometry channels** — the module's fitted constants
   (`HAND_CURL_AXIS`, `HAND_FIST_CENTRE`, `HAND_FIST_RADIUS`,
   `HAND_FIST_REFERENCE_GRIP_RADIUS`, `HAND_PALM_CONTACT`,
   `HAND_PALM_NORMAL_IN`, `HAND_GRIP_SEAT_FLESH`, `HAND_LONG_AXIS`,
   `HAND_PALM_NORMAL_OUT`) as `pub const`s; the side-mirroring accessors
   (`handCurlAxis(side)` etc.) as pure functions. Asserted against the
   fixture's `channel` section.
2. **Digit chains** — `collectHandDigitChains` walks the V4 hand's helper
   rest transforms into per-digit joint chains (proximal→distal) with tip
   length and the `v4*Fingers` cup node. Core receives chains as plain data
   (`DigitJoint { helper, position, quaternion }`), so the parity test feeds
   the fixture's `hands` section directly; the viewmodel's collector derives
   the same data from the vendored contract at runtime.
3. **Closure solver** — `solve_hand_grip_closure(chains, options) ->
   GripClosure { poses, contacts }`: per-digit bounded flexion marching
   (stage-wise unless `wrap_finger_stages`, which solves all three stages
   jointly for a final enclosure), thumb opposition and axial end-press with
   the separate thumb flesh allowance, cup-pivot application
   (`HAND_CLOSURE_CUP` about the cup node), and signed surface-distance
   contact reports.

Vector/quaternion algebra: minimal hand-rolled `[f64; 3]` / `[f64; 4]`
helpers in the module (rotate vec by quat, quat multiply, axis-angle), unit-
tested against known rotations; no new dependency (ADR 0007).

## Parity enablement

`parity.rs::grip_and_equipment_parity` loses its `#[ignore]`:

- Grips: fixture `channel` values ↔ ported constants (1e-12); per sport, the
  fixture `options` → `solve_hand_grip_closure` → `poses` (helper/flex/oppose)
  and `contacts` (surfaceDistance, contact, tip) compared at 1e-9 (fixture
  floats are full f64; Studio's own generator writes unscaled f64).
- Equipment: the port of `orientHandToGripChannel` + `constrainWristFrame`
  + `refineGripSpin/Tilt` behind a pure projection API consumed by both the
  fixture test and `PoseSolver`; the 253 samples compare at the fixture's
  recorded tolerance (same argument as the grips).
- The fixture loader keeps its schema/`sourceCommit` shape checks.

## Runtime application

- Install-time (sport switch): viewmodel collects chains from the vendored
  contract (`Meta::embedded_athlete` rest transforms — the same data balsam
  exposes as joints), solves the closure with the sport's surface
  (rower scull radius + thumb stop; ski pole radius; bike hood from the
  anchors/equipment table), and exposes
  `Replay.gripPoses = { "<helper objectName>": {flex, oppose} }`.
- QML (`ReplayScene`): during the scene walk (same pass as the material
  walk), for each Joint whose `objectName` is in `Replay.gripPoses`, apply
  flex (rotation about the mirrored curl axis) and oppose (local Z) —
  bake-side these are the joint's local axes, so the application is a fixed
  per-joint rotation composition, not per-frame maths. Ghost rigs get the
  same table in their walk.
- Per frame: `PoseSolver::pose` computes the hand semantic joints'
  orientation from the grip channel (web `orientHandToGripChannel`
  equivalent) subject to the wrist budgets; this rides the existing joint
  encoding (indices for `v4LeftHand`/`v4RightHand` in `semanticOrderedNames`),
  so `frame.rs` stays 225 floats.
- Gate: `replay grip <sport>: n/m digit contacts` logged from the applied
  table; `qml_runtime_gate.rs` requires n == m for each sport and adds the
  line to `gate_log_lines` filtering.
  *(2026-09-24: n == m held only because every sport logged the rower's
  table, #85. The web's own closure,
  `tests/fixtures/replay-current-main-grips.json`, closes 10/10 for the
  rower and the bike and 8/10 for SkiErg, whose pinkies stay short of the
  pole grip. The gate now requires each sport's count from that fixture.)*

## Per-stroke variation

Already flowing: per-stroke `drive_frac`/`stroke_seconds` reshape the graph
windows (verified at 1e-10 by the motion parity corpus, which sweeps
129 phases), targets follow, accents follow intensity. R4 adds one
end-to-end test pinning that two strokes differing only in stroke timing
produce different posed hand joints, and the source-map documents the
architecture (graph = choreography/targets; clip + constraints = pose;
grip closure = the layer the port was missing).

## Risks

- The fixture's expected closures came from Studio's Swift solver; if the
  Rust port diverges numerically (matrix convention, flexion sign), the
  parity tolerance will expose it immediately — the fix is convention
  alignment, never loosening the tolerance.
- Finger helper posing through balsam joints is untested territory
  (5b posed only semantic joints); if a helper joint refuses JS rotation
  composition, fall back to packing finger rotations into the frame's
  equipment block (64 floats; flex/oppose pack into 2 floats per digit × 10
  digits = 20 floats, fits) — a frame-layout renegotiation via
  `layout_json`, decided with an ADR note if it comes to that.
