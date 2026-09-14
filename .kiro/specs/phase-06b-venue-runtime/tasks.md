# Phase 6b — Venue runtime: tasks

- [x] T1 `build.rs`: 12 venue rows in `PACKS` (balsam), full runtime-plan
  embedding (materials, bucketed instance groups, bounds) via the view-model
  bucketing (R1.1, R2.2, R2.3).
- [x] T2 View-model `replay::venue_runtime`: deterministic shade bucketing
  from the contract tints + unit tests cross-checked against the vendored
  contracts (R2.2).
- [x] T3 `ReplayBackend`: venue plan exposure (component URL per sport and
  effective tier, materials/instances/texture JSON), venue revision notify,
  error reporting (R1.2, R1.3, R3.4).
- [x] T4 `ReplayScene.qml`: `venueRoot` + dynamic component load, material
  walk with live scheme re-tint, bucketed `InstanceList`s, tier-gated texture
  binding from the rcc (R1.2–R2.3, R3.1–R3.3).
- [x] T5 Environments + procedural rcc bundling in `build.rs` (R3.1).
- [x] T6 Gate: exact venue inventory lines, FAILED-is-fatal, venue texture
  counts, screenshot + shadow-region recheck (R4.1–R4.3).
- [x] T7 Measurements per sport × tier with ghost, `QSG_NO_VSYNC=1`,
  governor rollback re-check, R5.2 ladder if Medium exceeds budget (R5.1–R5.3).
- [x] T8 Docs: roadmap 6b, source-map runtime rows, qt-bridges-notes entries;
  full validation; stacked PR (R6.1–R6.3).
