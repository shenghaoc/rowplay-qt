# Phase 2 — Replay core: tasks

- [x] Port `replay::engine` (`sample_at`, `sample_index_at`, `Frame`,
      tick-driven `ReplayState`) with web `engine.test.ts` parity.
- [x] Port `replay::motion` (`meters_per_cycle`, `clamp_dt`, `damp_factor`,
      `stroke_surge`, `catch_events`, `ParticlePool`, `PerfGovernor`).
- [x] Make `warp_stroke_phase` C1 at the drive/recovery seam and the cycle
      boundary; add `warp_stroke_phase_rate` and the derivative-continuity
      test.
- [x] Port `replay::stroke_model` (timeline, web `stroke_pose_at` /
      fallback / `catch_transitions`, Studio `compute_at_time` path).
- [x] Port `replay::motion_graph` (all three sport graphs, timing, curve
      algebra) with evaluation order matching the web module.
- [x] Port `replay::sport_kinematics` (rower / skier / bike projections,
      ski elbow direction) and `replay::theme` (COLORS / VENUES palettes).
- [x] Port `replay::comparability` and `replay::ghost_pick` (web semantics).
- [x] Port `replay::race_gap` (web + Studio origin helpers) and
      `replay::race_result` (interpolated crossing, tie tolerances, DNF).
- [x] Port `replay::rivals`: constant-pace traces, bounded dispatch,
      quoted CSV, TCX element-stack scanner, hardened FIT decoder,
      normalisation.
- [x] Port `replay::quality` (budgets + sticky degradation ladder).
- [x] Enable the six Phase 2 `#[ignore]`d parity tests with stated
      tolerances; keep the Phase 3/5/7 stubs ignored.
- [x] Record every divergence in `docs/source-map.md`; mark Phase 2
      delivered in `docs/roadmap.md`.
- [x] `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D
      warnings` (Qt-free members), `cargo test`, `git diff --check` green.
