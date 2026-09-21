# Phase 7 — Motion: tasks

- [x] T1 `rowplay_core::replay::hand_grip`: geometry channel constants +
  side accessors, asserted against the grips fixture's `channel` section
  (R1.1).
- [x] T2 Digit-chain collection semantics ported; viewmodel collector from
  the vendored contract cross-checked against the fixture's `hands` section
  (R1.2).
- [x] T3 `solve_hand_grip_closure` (staged flexion, thumb end-press, cup
  pivot, `wrap_finger_stages`) + contact reports; defect-class unit tests
  (R1.1, R1.3, R1.4).
- [x] T4 Enable the grips half of `grip_and_equipment_parity` (drop the
  `#[ignore]`, poses + contacts vs fixture at tolerance) (R1.3).
- [x] T5 `orientHandToGripChannel` + `constrainWristFrame` budgets +
  `refineGripSpin/Tilt` port; enable the equipment half of the parity test
  (R2.1, R2.2).
- [x] T6 `PoseSolver` applies grip-channel hand orientation per frame with
  wrist budgets; ghost consistency (R2.3, R3.3).
- [x] T7 Runtime wiring: per-sport `Replay.gripPoses` table, QML application
  to finger helper joints on the sport walk, gate grip-contact assertion
  (R3.1, R3.2, R3.4).
- [ ] T8 Per-stroke variation end-to-end test + source-map architecture note
  (R4.1, R4.2). The wrist budgets (75°/150°, SkiErg 30° keep) are exact on
  paper and can still look broken at the stroke extremes: run the athlete
  and look at the catch and the finish on all three sports before trusting
  the numbers — plus the ghost, which runs the same path, so any
  orientation error doubles on screen.
  The first T8 review found a bigger upstream defect before the wrist
  question: the rower rig phase was inverted against the web (Studio's
  calibration ported op-for-op — seat farthest from the feet at the catch,
  catch grips behind the torso), making the athlete stroke backwards
  against the equipment. Fixed fixture-first: `replay-row-phase-parity.json`
  (generated from the web repo by `tools/gen-row-phase-parity.mjs`) pins the
  web's seat/sweep/roll mapping,
  `rower_rig_phase_parity` consumes it, and `solve_rower` now follows the
  web avatar (divergence row in source-map). The corpus lesson is recorded
  in AGENTS.md: the equipment fixture was web-generated but phase-agnostic —
  nothing covered the rig's phase calibration, which is how the inversion
  ported cleanly past a green parity suite.
  That inversion also resolves what the earlier 47°/step whip analysis got
  half right: the mechanism was correctly traced to the flat-wrist/tilt
  weights slewing 1→0→1 through the feather window, but the cause was those
  weights interacting with the inverted sweep, not independent authored
  behaviour — the pre-fix note calling it "not a port bug" was right about
  the mechanism and wrong about the cause. Post-fix the unclamped demand
  sweeps −41°…+178° continuously (it was all-negative −35°…−116° and
  clamped most of the cycle), the in-window rate is ~14°/step (was 47°),
  and the worst adjacent jump anywhere is 36.7° — a flexion step at the
  extraction seam, which is where the web authors the feather transition
  on purpose. The held-phase close-ups show flat wrists with no snap at
  either window edge, so feathering-vs-snap is answered at stills; what
  remains of T8's visual is the full-speed read — does the feather look
  natural in motion, which held frames cannot settle.
  Ownership (recorded after this was misassigned twice): the reviewer of
  uploaded captures has no display, no GPU and no way to run the app —
  every eyes-on item belongs to the development-machine side, never to
  "whoever reads this note next". The at-speed feather read is assigned
  there: run the app (`cargo run -p rowplay-app`), or assemble a
  true-frame-rate video from deterministic seeks and watch that.
  Ski demand swings +77°…−134°…+131° with the keep pegged; the elbow-seam
  excess now splits 50/50 into the humerus like production (ported, with the
  forearm-world bit-exact invariant pinned — see `seam_split_*`), so the
  forearm no longer corkscrews double. Bike demand sits at ~67° all cycle:
  static frame, frozen wrist, in budget — planted hands, but also proof of
  nothing beyond no-misfire when idle.
  On judging: with the demand now crossing the budget mid-drive instead of
  pegging it, a stiff-looking wrist at the extremes would indict the 75°
  budget itself — widen the budget, don't chase the frame. The close-ups
  show no stiffness, so the budget stands as ported.
  Baseline kept: `artifacts/phase-baseline/` holds the twelve ghost-loaded
  captures (catch/mid-drive/finish/mid-recovery × 3 sports) on the finished
  slice-3 code, with SHA256SUMS and a README (regen command, determinism
  caveat: gate recipe only — local prefs render different pixels). The walk
  steps (Main.qml cases 66–81, behind `ROWPLAY_PHASE_SHOTS=1`) and the gate
  assertions travel with the repo; the PNGs stay local (renderer-dependent).
  Add `ROWPLAY_PHASE_CLOSEUPS=1` for torso-and-hands close-up twins of every
  phase grab (`Replay.setCloseupCamera`) — the wide shots cannot resolve
  wrist detail, which is how the first review misread a animating athlete
  as static. The pre-inversion-fix wide set is preserved under
  `artifacts/phase-baseline/before-phase-fix/` for before/after comparison.
  Visual verdict on the post-fix set (this session could display the
  captures): row catch compressed forward with arms extended ahead onto the
  handles, finish in layback with the hands drawn to the lower ribs, blades
  buried through the drive (the `bladeWater` dip working), mid-recovery
  wrists flat with no snap at the window edges; ski forearms straight with
  the shoulder split in; bike hands planted, wrists level. The gross
  anatomy questions are settled — what remains is the subjective full-speed
  feather read above, on the development machine (see the ownership note).
- [ ] T9 Docs (roadmap, source-map, qt-bridges-notes) + full validation +
  phase PR (R5.1, R5.2).

## Slice status

Slice 1 (landed): T1–T4 — the core grip closure port with full parity
(3 sports × 2 hands, poses + contacts at 1e-9) plus 9 module invariant tests.

Slice 2 (this PR): T5 — the equipment contracts (`row_equipment`,
`ski_equipment`, `bike_equipment`, `bike_saddle`) with the equipment parity
half enabled at 1e-12 (253 samples: oar yaw, elbow/reach flexion, knee
flexion, saddle grid, skier elbow direction), plus the wrist layer
(`wrist.rs`: grip-channel orientation, spin/tilt relief, swing–twist budgets
with the SkiErg keep/share rules, 12 unit tests re-expressing the web
orientation suite). Enabling the fixture first caught one slice-1 defect:
`hand_palm_normal_out` returned the negated construction ray instead of the
shipped rig's measured outward normal — corrected, with the constant pinned.

Remaining: T8 (per-stroke verification + the catch/finish visual on all
three sports), T9 (docs/validation).

## Post-phase follow-up: issue #40 (2026-09-21)

The two SkiErg hand-position discontinuities the phase shipped with (steps
529 and 1377 of the dense guard's sweep — the wrist refinements' faithful
±π snaps, coupled into position by the port's contact chord) are fixed in a
follow-up PR, not a phase:

- `PoseSolver::pose` now closes each hand chain on the **sport's effective
  contact offset** (`effective_hand_offset`, the web's `gripEffectorOffsets`:
  the SkiErg fist-channel centre, the rower/bike channel centre) rather than
  the V4 contract's authored palm point, and `solve_limb_measured` follows
  the web's `solvePositionTowardTarget` — the chain aims the terminal
  *origin* at `anchor − R·offset` with bone-length segments, the web's
  absolute 2 mm reach margins at both ends, and the terminal counter-rotation.
- The dense guard's carve-out windows and their `(0.15, 1.45)` budgets are
  deleted. The SkiErg branch compares every step against a recorded oracle
  (`tests/fixtures/replay-v4-hand-parity.json`, generated by
  `tools/gen-v4-hand-parity.mjs` from the real web V4 controller), asserts
  the hand stays on its grip (`GRIP_CONTACT_BUDGET`), and asserts parity
  two-sidedly for window 1 (the port may snap where the web snaps, and must
  not smooth a snap away) while pinning window 2's divergence as an absence
  (issue #43).
- The coupling is **reduced ~11x, not eliminated** (issue #44): the port's
  snap still reaches the elbow 2.1x further than the web's. Ranking 14
  stays open (issue #43).

T8's SkiErg captures were re-taken through the step walk
(`Replay.setGuardCycleStep`, steps 520–540); on this macOS host the 3D
viewport grab is black (qt-bridges-notes #17), so the visual read stays on
a Linux session's gate walk.
