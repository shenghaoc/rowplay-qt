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
- [ ] T5 `orientHandToGripChannel` + `constrainWristFrame` budgets +
  `refineGripSpin/Tilt` port; enable the equipment half of the parity test
  (R2.1, R2.2).
- [ ] T6 `PoseSolver` applies grip-channel hand orientation per frame with
  wrist budgets; ghost consistency (R2.3, R3.3).
- [ ] T7 Runtime wiring: per-sport `Replay.gripPoses` table, QML application
  to finger helper joints on the sport walk, gate grip-contact assertion
  (R3.1, R3.2, R3.4).
- [ ] T8 Per-stroke variation end-to-end test + source-map architecture note
  (R4.1, R4.2).
- [ ] T9 Docs (roadmap, source-map, qt-bridges-notes) + full validation +
  phase PR (R5.1, R5.2).

## Slice status

Slice 1 (this PR): T1–T4 — the core grip closure port with full parity
(3 sports × 2 hands, poses + contacts at 1e-9) plus 9 module invariant tests.
Remaining: T5 (wrist budgets + equipment projections, its own parity half),
T6–T7 (per-frame hand orientation, runtime posing, gate assertion), T8
(per-stroke verification), T9 (docs/validation).
