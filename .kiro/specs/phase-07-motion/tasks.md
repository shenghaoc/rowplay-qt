# Phase 7 — Motion: tasks

- [ ] T1 `rowplay_core::replay::hand_grip`: geometry channel constants +
  side accessors, asserted against the grips fixture's `channel` section
  (R1.1).
- [ ] T2 Digit-chain collection semantics ported; viewmodel collector from
  the vendored contract cross-checked against the fixture's `hands` section
  (R1.2).
- [ ] T3 `solve_hand_grip_closure` (staged flexion, thumb end-press, cup
  pivot, `wrap_finger_stages`) + contact reports; defect-class unit tests
  (R1.1, R1.3, R1.4).
- [ ] T4 Enable the grips half of `grip_and_equipment_parity` (drop the
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
