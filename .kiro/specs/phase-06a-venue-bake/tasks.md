# Phase 6a — Venue baking pipeline: tasks

- [ ] T1 Feasibility spike recorded: Node build, GLB export, determinism,
  balsam instancing drop, Blender round-trip (R1.1, R1.2 — done in ADR 0010;
  this task closes with the qt-bridges-notes entry).
- [ ] T2 `tools/bake-venues/`: register/hooks + `bake.mjs` with the
  production-faithful context, seed pinning, texture stripping + contract
  harvest, archetype/instance split, UV repeat baking, PNG encoder,
  determinism self-test (R2.1–R2.7).
- [ ] T3 `tools/bake-venues/cleanup.py` Blender hygiene pass + round-trip
  verification; bake all 12 variants + procedural PNGs (R3.1–R3.3).
- [ ] T4 Vendor into `assets/replay/venues/`: vendor-script family, hash
  pins, provenance rows incl. generated PNGs (R4.1).
- [ ] T5 `rowplay-viewmodel` `replay::venue` reader + `validate_venue` +
  defect unit tests; `build.rs` drift gate (R4.2).
- [ ] T6 Measure vendored total; write ADR 0011 (size decision per ADR
  0009); adjust the tripwire only as that ADR decides (R4.3).
- [ ] T7 Docs: roadmap Phase 6a, source-map venue rows + naming convention,
  qt-bridges-notes balsam entry, `tools/bake-venues/README.md`; full
  validation run (fmt / clippy / test / diff --check) (R5.1, R5.2).
