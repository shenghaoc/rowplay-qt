# Phase 7 — Motion: requirements

Drive the V4 athlete's hands from geometry-closure grip solving with wrist
budgets and per-stroke variation, and enable the two `#[ignore]`d parity
fixtures (`replay-current-main-grips.json`,
`replay-current-main-equipment.json`) that define done.

## R0 — What Phase 7 is (and is not)

- R0.1 The web's production V4 architecture is "clip-contact-constrained"
  (`renderer3dV4Motion.ts`: the controller stamps
  `replayV4Architecture = "clip-contact-constrained"`): the motion graph
  drives a procedural rig that **provides contact targets**; the V4 skin
  samples its authored clip and is constrained onto those targets by two-bone
  IK. The Rust port already implements exactly this split —
  `solve_rig_pose` consumes the (parity-pinned, 1e-10) motion graph and
  derives `rig_targets`; `PoseSolver` samples the clip and solves onto the
  targets. Phase 7 does **not** rewire that; it completes the hand layer the
  port still leaves at clip identity:
  - fingers are not posed at all (the clip carries no authored digit motion);
  - hands keep "the clip's authored wrist/ankle orientation"
    (`pose.rs`), with no grip-channel orientation and no wrist budgets;
  - there is no grip contact state, so nothing proves the hands actually
    hold the equipment.
- R0.2 "Per-stroke variation" is already delivered through the graph: every
  curve window reshapes per stroke with `driveFrac`/`strokeSeconds`, accents
  scale with intensity (`intensity_only_scales_accents` pins the web
  semantics), and targets consequently move per stroke. Phase 7 verifies and
  documents that flow end-to-end rather than adding machinery.

## R1 — Grip geometry closure in `rowplay-core` (`replay::hand_grip`)

- R1.1 Port `handGrip.ts` as pure, dependency-free maths (hand-rolled
  vector/quaternion algebra, no new crates — ADR 0007): the module's geometry
  channels (curl axis, fist centre/radius, palm contact/normal, long axis,
  grip-seat flesh), `collectHandDigitChains` semantics,
  `solveHandGripClosure` (bounded per-stage flexion against an equipment
  cylinder with thumb-end press, `wrapFingerStages` final enclosure),
  `HandGripClosure` outputs (per-helper `flex`/`oppose` poses, per-digit
  contact reports with signed surface distance and contact flag), and
  `HAND_CLOSURE_CUP` cup-pivot handling.
- R1.2 The solver takes hand-local rest-space digit chains (from the V4
  contract), a `HandGripSurface` (radius, optional thumb-end axial) and
  closure options (side, `thumbOppose`, flesh radii, wrap mode) — mirroring
  the web API shape so the fixture maps 1:1.
- R1.3 Enable the grips half of `grip_and_equipment_parity`: with the
  fixture's `channel` values asserted against the ported constants (tight
  tolerance) and the fixture's per-sport `options` fed to the solver, the
  solved `poses` and `contacts` must match the fixture within its recorded
  tolerance (floats in the fixture are full f64; compare with 1e-9 like the
  motion parity, or the tolerance the fixture's Studio generator documents).
  Unit tests synthesise the defect classes: wrong flesh radius (digits stand
  off the surface), double-counted flesh (axis seated too deep), unreached
  surface (`contact == false`), and wrap-mode divergence.
- R1.4 The solver is `#![forbid(unsafe_code)]`-clean, deterministic, and
  allocation-light enough to run at install time (once per sport), not per
  frame.

## R2 — Hand orientation and wrist budgets

- R2.1 Port `orientHandToGripChannel` (wrist orientation from the grip
  channel geometry), `refineGripSpinForWrist`/`refineGripTiltForWrist`, and
  the swing–twist decomposition `constrainWristFrame` with the web's budgets
  (twist 75°, flexion/deviation 150°) and the SkiErg pronation/shoulder share
  (0.5) + 30° twist-keep rule.
- R2.2 Enable the equipment half of `grip_and_equipment_parity`:
  `replay-current-main-equipment.json` (253 samples: scull grip/oarlock/elbow
  corridor and draw-finish flexion, pole grip radius and grip shift, bike
  hood contacts, saddle, and per-sample oarYaw / elbowFlexion /
  skierElbowDirection projections) asserted within the fixture's tolerance.
- R2.3 `PoseSolver::pose` applies the grip channel orientation to the hand
  semantic joints per frame (replacing the clip's authored wrist orientation
  for the current sport), with the wrist budgets enforced; the two-bone arm
  solves keep their existing behaviour and the residual/usability gates stay
  active.

## R3 — Runtime wiring

- R3.1 Closure poses are **static per sport** (the web solves once at
  install and applies cached rotations): `rowplay_viewmodel` collects the
  digit chains from the vendored V4 contract (`collectHandDigitChains`
  equivalent), runs the closure with the per-sport surface from the anchors
  table's equipment geometry, and exposes a per-sport pose table (helper name
  → flex/oppose) through the `Replay` singleton, applied QML-side to the
  balsam-exposed finger helper joints by `objectName` — once per sport
  switch, not per frame. No `frame.rs` renegotiation for fingers.
- R3.2 Wrist orientation rides the existing 19-joint frame encoding (hands
  are semantic joints), so the one-bridge-crossing and `frame::layout_json`
  contiguity guarantees are untouched.
- R3.3 The ghost's hands get the same treatment (the ghost rigs walk mirrors
  the player's), preserving the 45 % equipment ghost-opacity rules.
- R3.4 The gate gains a grip assertion: the scene logs
  `replay grip <sport>: <n>/<m> digit contacts` from the applied table, and
  the walk test requires n == m per sport (the fixture closures are
  full-contact; an unsolved hand fails loudly, not silently).
  *(2026-09-24: the fixture's SkiErg closure is not full-contact: both
  pinkies stay short of the pole grip, 8/10. n == m passed for SkiErg only
  because every sport logged the rower's table, #85. The gate now requires
  each sport's count from the fixture.)*

## R4 — Per-stroke variation verification

- R4.1 A test (or documented existing coverage) demonstrating the end-to-end
  per-stroke flow: two strokes with different `driveFrac`/`strokeSeconds`
  produce different graph windows → different contact targets → different
  posed joint values; accents scale with intensity and nothing else.
- R4.2 `docs/source-map.md` records the architecture finding (web production
  = clip-contact-constrained; the Rust port matches it; the motion graph's
  role is choreography/targets, never direct bone driving).

## R5 — Documentation and rules

- R5.1 Roadmap Phase 7 entry, source-map rows (handGrip + wrist-budget
  provenance), qt-bridges-notes for any bridge friction, and the phase's
  `.kiro/specs/` tasks kept honest.
- R5.2 No hand-written C++; no new replay maths outside `rowplay-core`
  (solver) and `rowplay-viewmodel` (application); one bridge crossing per
  frame unchanged; all user-visible strings from the locale pipeline; the
  gate and QML member check stay green.
