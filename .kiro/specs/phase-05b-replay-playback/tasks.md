# Phase 5b — Replay playback: tasks

- [ ] T1 `rowplay_viewmodel::replay::frame`: the flat frame layout table,
  offsets accessors, layout-length test (R1.3).
- [ ] T2 `Replay` singleton transport: `tick`, play/pause/seek/speed slots,
  `ReplayState` wiring, demo-data default workout on the replay route (R1.1,
  R1.2, R1.4).
- [ ] T3 glb reader extension: animation channels (accessors, LINEAR +
  CUBICSPLINE), skin joint names, skeleton cross-check against the contract
  JSON (R2.2).
- [ ] T4 Athlete posing in Rust: clip sampling, contact pass, joint frame
  pack; QML joint writer over the loaded skeleton; decision gate ADR if the
  write path needs C++ (R2.1–R2.3).
- [ ] T5 Equipment: template clones at anchors, moving parts from contacts
  (R3.1–R3.3).
- [ ] T6 Camera: `chase()` port + golden/quote test, pause-snap rule, reduced
  motion (R4.1, R4.2, R6.4).
- [ ] T7 HUD strings + `ReplayTransport.qml` controls (play/pause/seek/speed)
  with locale ids and `Accessible.name`s (R5.1, R5.2).
- [ ] T8 Settings reduce-motion toggle (R2.4, R6.3).
- [ ] T9 Gate playback steps + per-sport sampled screenshots + CI artifacts
  (R6.1); `tests/bridge_crossings.rs` (R6.2).
- [ ] T10 Docs: source map rows, roadmap status, qtbridge notes (joints,
  animation sampling), PR validation record.
