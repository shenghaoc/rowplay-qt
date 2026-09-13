# Phase 6a — Venue baking pipeline: requirements

Bakes the web app's procedural replay venues (RowErg regatta basin, SkiErg
alpine stadium, BikeErg velodrome) to vendored `.glb` assets per ADR 0005, via
a deterministic headless-Node pipeline (ADR 0010), with contract sidecars,
a scripted Blender hygiene pass, hash pinning, provenance and structural
validation. No runtime changes in this PR.

## R1 — Feasibility first (done, recorded)

- R1.1 The exporter route was proven before exporter code: the builder runs
  under plain Node (guarded DOM lines only), determinism is reachable by
  pinning `Math.random` before import, and a double bake is byte-identical.
  Recorded in ADR 0010 with the pinned reference SHAs.
- R1.2 Balsam's silent `EXT_mesh_gpu_instancing` drop and Blender's instance
  expansion are recorded in ADR 0010 and `docs/qt-bridges-notes.md`; the
  pipeline is designed around them (archetype GLB + contract instances).

## R2 — The baker

- R2.1 `tools/bake-venues/bake.mjs` runs from the repo root with one command,
  reads the pinned `reference/rowplay` checkout (fails with a clear error if
  missing) and writes 12 GLBs — one per sport (`rower`, `skierg`, `bike`) per
  quality tier (`low`, `medium`, `high`, `ultra`) — plus one contract JSON per
  GLB, into a staging directory.
- R2.2 The baker replicates the production context exactly from the pinned
  web source, with the source lines cited in comments: the `QUALITY` table
  (`renderer3d.ts:65-138`), the `ENVIRONMENTS` palettes (`renderer3d.ts:414-528`),
  the material/geometry helper closures (`renderer3d.ts:1247-1308`,
  `1722-1748`), horizon rings + infield + apron (`renderer3d.ts:1751-1876`),
  with `innerR` 22 and `outerR` 34 (`renderer3d.ts:2025-2026`). Everything
  `buildEnvironment` adds except `buildSky`.
- R2.3 The bake is seeded: `Math.random` is replaced with mulberry32(20260913)
  before the builder module is imported (its `SimplexNoise` singleton seeds
  from `Math.random` at module evaluation). The seed is recorded in every
  contract JSON. All tiers bake in one process so silhouettes are consistent
  across tiers.
- R2.4 Deterministic output: baking twice with the same seed and reference
  produces byte-identical GLBs, contracts and PNGs. The baker asserts this
  itself on every run (bake one variant twice into a temp dir and compare).
- R2.5 **No textures are baked in.** All texture bindings are stripped before
  export; the contract records, per material and slot: the source path (web
  `/replay-assets/...` set derivative or procedural map name), `repeat`, and
  `normalScale`. `repeat` is multiplied into the geometry UVs so the runtime
  needs no UV transform. The baker fails loudly if one material binds slots
  with differing repeats (UV baking could not represent that).
- R2.6 Procedural venue maps (`water-normal`, `snow-groomed`,
  `water-surface`, both 64 px and 128 px variants) are encoded to PNG by the
  baker's own deterministic encoder (`node:zlib`, no canvas) and vendored
  next to the venues with generated-asset provenance.
- R2.7 Instancing: each `InstancedMesh` is exported as its archetype mesh
  (identity transform, same geometry/material/name); per-instance
  decomposed matrices and `instanceColor` tints are recorded in the contract.
  Meshes hidden by the web (e.g. the generic infield for row/ski) keep their
  visibility recorded in the contract rather than being dropped.

## R3 — Blender hygiene pass

- R3.1 `tools/bake-venues/cleanup.py` runs under
  `blender --background --python`, single-threaded with `PYTHONHASHSEED=0`
  (the reproducibility guards the web's own rig authoring uses), over every
  staged GLB. It drops degenerate geometry (zero-area faces, loose verts) and
  re-exports.
- R3.2 The script verifies the round-trip itself: node names, material names,
  vertex-colour layers and UV layers must survive 1:1; any loss fails the
  bake. Merge-by-material and AO baking are explicitly rejected in ADR 0010.
- R3.3 The script prints per-file before/after stats (meshes, verts, bytes)
  which the bake command surfaces.

## R4 — Vendoring and verification

- R4.1 Baked venues land in `assets/replay/venues/` with SHA-256 pins in
  `crates/rowplay-app/tests/asset_hashes.rs` (extended vendor flow), rows in
  `ASSET_PROVENANCE.md` (source repo/path/commit, seed, licence), and the
  procedural PNGs marked as generated from the web code at the pinned SHA.
- R4.2 A structural validator in `rowplay-viewmodel`
  (`replay::venue`, modelled on `replay::glb::validate_v3`) cross-checks each
  vendored GLB against its contract: bounded read, every node/mesh named with
  the `environment:<sport>:` prefix, fixed per-sport-per-tier inventory
  (node/mesh/material/instance counts), all materials named and present in
  the contract, bounds plausible for the course (within radius ~170),
  **no embedded images**, finite accessor bounds. Synthesised defects are
  unit-tested; `build.rs` panics on drift, exactly like the V3 pack.
- R4.3 The asset-size decision: measure the vendored total first, then write
  the ADR 0009 follow-up (ADR 0011) — stay in plain Git with a raised
  ceiling, or move to Git LFS (with `lfs: true` in CI) — decided on measured
  numbers.

## R5 — Documentation

- R5.1 ADR 0010 (route, seed, instancing), ADR 0011 (size decision),
  `docs/roadmap.md` Phase 6a entry, `docs/source-map.md` venue rows (naming
  convention `environment:<sport>:<thing>`, contract format, the copied
  `ENVIRONMENTS`/`QUALITY` tables with their source lines), and
  `docs/qt-bridges-notes.md` for the balsam instancing drop.
- R5.2 `tools/bake-venues/README.md` documents the one-command bake, the
  environment requirements (Node ≥ 24, `pnpm install` in the reference
  checkout, Blender path override) and the determinism guarantees.
