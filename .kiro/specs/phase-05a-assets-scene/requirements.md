# Phase 5a — Assets and scene: requirements

Vendors the rowplay V3 rig pack, the V4 athlete and the Poly Haven environment
textures with provenance, loads and validates them at runtime, maps their
material-role metadata to Qt Quick 3D `PrincipledMaterial`s, and turns the
Phase 0 smoke scene into a real replay scene. Playback is 5b; quality tiers and
polish are 5c.

The reference contracts are `reference/rowplay/static/replay-assets/README.md`
(asset contracts), `renderer3dAssets.ts` (V3 template and material-role
contract) and `rowplay-athlete-v4.contract.json` (V4 skeleton, clips, contacts
and surfaces). The pinned rowplay commit is `011e8303b66b4d2265a6f1ec8b3ed9d8ed497086`.

## R1 — Vendored assets with provenance and a hash pin

- R1.1 `assets/replay/` receives byte-for-byte copies of
  `rowplay-rigs-v3.glb` (733,864 bytes, SHA-256
  `31418f4808b30fa786830129b0b637fc025b6e5ddbb539d848fc8cab74806925`),
  `rowplay-athlete-v4.glb` (4,584,320 bytes, SHA-256
  `a564a4dbd4922e2ba76ef21a23f5bf0eb1b0180846548f9d7110e55ffd8f760e`),
  `rowplay-athlete-v4.contract.json`, and every `environments/` texture
  directory with its `README.md` (13 families × 3 maps plus the per-tier
  payload documentation).
- R1.2 Not vendored: `rowplay-rigs-v1.glb`, `rowplay-rigs-v2.glb`,
  `rowplay-athlete-v4.usdz`, `source/`, and the root `README.md` beyond the
  facts recorded in `ASSET_PROVENANCE.md`.
- R1.3 `ASSET_PROVENANCE.md` gains one row per vendored file with source
  repository, source path, upstream commit SHA, SHA-256 of the committed bytes
  and licence: MIT for the rowplay artifacts (`LICENSES/MIT-rowplay.txt`),
  CC0-1.0 for the Poly Haven derivatives (`LICENSES/CC0-1.0.txt`) with the
  creator credited per family exactly as `environments/README.md` records it
  (Rob Tuytel, Dimitrios Savva, Charlotte Baglioni, Amal Kumar, and the
  Savva/Barresi pair for Brushed Concrete 2). The Human Base Meshes v1.4.1
  lineage of the V4 athlete (Dan Ulrich / Blender Studio, CC0-1.0, MIT
  modifications) is recorded with the row.
- R1.4 A Rust test (`crates/rowplay-app/tests/asset_hashes.rs`, no Qt needed)
  asserts every vendored file's byte count and SHA-256, so a silent asset swap
  fails CI. The expected hashes are literals in the test; a refresh means an
  intentional commit that updates `ASSET_PROVENANCE.md` in the same change.
- R1.5 The environments `README.md` per-tier payload table (Low/Medium bind
  nothing; High binds diffuse+roughness; Ultra adds the OpenGL normal and the
  SkiErg-only timber terrace set) is honoured by 5c, not by 5a. 5a only
  vendors the files and records the table's existence.

## R2 — V3 loader with loud contract validation

- R2.1 `crates/rowplay-app/src/replay/assets.rs` loads
  `rowplay-rigs-v3.glb` through Qt Quick 3D `RuntimeLoader` in development
  (env var `ROWPLAY_REPLAY_ASSETS` pointing at `assets/replay/`, fallback
  `CARGO_MANIFEST_DIR/../../assets/replay/` in debug builds) and from the rcc
  at `:/qt/qml/RowPlay/replay/rowplay-rigs-v3.glb` otherwise. The GLB is added
  to `qml/rowplay.qrc` so the release path has no file dependency.
- R2.2 `balsam`-generated meshes for release are attempted in `build.rs`
  (found like `rcc` through `qmake -query`, override `ROWPLAY_BALSAM`). If
  balsam is unavailable or its output cannot feed `Model` without hand-written
  C++, 5a ships `RuntimeLoader` with the GLB in the rcc and records the reason
  in `docs/qt-bridges-notes.md`. No C++ is written either way (ADR 0001).
- R2.3 Validation ports `collectReplayAssetTemplateLibrary` from
  `renderer3dAssets.ts`: the 7 template roots
  (`equipment:row:boat-assembly`, `equipment:row:oar-rig`,
  `equipment:row:seat-carriage`, `equipment:ski:ski-assembly`,
  `equipment:bike:wheel-assembly`, `equipment:bike:frame-assembly`,
  `equipment:bike:drivetrain-assembly`) exist exactly once each with
  `replayAssetVersion: 3`, an identity transform, a positive integer
  `replayAssetPartCount` equal to the mesh count in the subtree, and a
  non-empty duplicate-free `replayMaterialRoles` list whose sorted set equals
  the sorted set of its meshes' roles. The 18 V3 leaf slots each exist exactly
  once with `replayAssetKind: "leaf"` and a known `replayMaterialRole`. Roots
  are never nested; `InstancedMesh`/`SkinnedMesh` inside a template is an
  error; every geometry position is finite.
- R2.4 Every failure is a typed error (`AssetError`) that names the slot,
  template or node path that failed — never a warning, never a silent skip,
  never a fallback that hides the problem. The renderer surfaces the error to
  the user (5b) and the gate test asserts the failure path by loading a
  deliberately broken fixture.
- R2.5 The glTF extras are read without a GL context requirement: validation
  runs in a plain Rust integration test over the GLB bytes
  (`crates/rowplay-app/tests/asset_contract.rs`, a minimal glTF JSON-chunk
  reader — no Qt, no C++, no new dependency), so CI catches a contract drift
  even where Qt cannot render. The Qt-side loader re-runs the same checks on
  the loaded node tree before any equipment is shown.

Outcome (2026-09-12): the R2.2 balsam path is the one shipped, for every
build — `build.rs` requires `balsam`, converts both packs with
`--removeComponentAnimations` into the generated `RowPlay.ReplayAssets`
module and bundles it as `rowplay_replay.rcc` (ADR 0008). `RuntimeLoader` is
not used anywhere: its scene is not addressable from QML (no `objectName`,
no traversable `children`), so neither the material walk nor 5b posing could
reach it; the GLBs are therefore not in any rcc and the R2.1 rcc path does
not exist. The reader of R2.5 lives in `rowplay_viewmodel::replay::glb` with
its defect tests as unit tests in that file (no `tests/asset_contract.rs`).
R2.5's Qt-side re-check is the build-time validation of exactly the bytes
`balsam` converts (a drift fails the build with the named slot) plus the
startup re-validation of the on-disk pack in development
(`ROWPLAY_REPLAY_ASSETS`, or `assets/replay/` in debug builds;
`Replay.validationMode` = "startup" or "build").

## R3 — Materials from Theme.qml, role list in Rust

- R3.1 The 11 `replayMaterialRole` values (`athlete-skin`, `athlete-fabric`,
  `athlete-hair`, `athlete-footwear`, `equipment-painted`, `equipment-dark`,
  `equipment-light`, `equipment-metal`, `equipment-rubber`, `equipment-grip`,
  `equipment-trim`) live as a Rust enum
  (`rowplay_viewmodel::replay::MaterialRole`) with `from_id`/`as_id`, the web
  list order preserved.
- R3.2 Each role maps to a `PrincipledMaterial` built in QML from colours the
  Rust view-model resolves out of `Theme.qml` (light and dark variants),
  including lane paint colours and the ghost transparency variant. No hex
  colour literal appears in `qml/RowPlay/Replay/` for a material; QML binds to
  the exported colour strings. The role list and its palette mapping are
  unit-tested in the view-model crate against the Theme palette keys.
- R3.3 Materials stay outside the GLB (the pack's single neutral placeholder
  material is never a product colour source): every loaded mesh's material is
  replaced by the resolved `PrincipledMaterial` for its role at load time.
- R3.4 The 8 V4 surface roles (`athlete-fabric`, `athlete-skin`,
  `athlete-shorts`, `athlete-footwear`, `athlete-hair`, `athlete-trim`,
  `athlete-eye`, `athlete-face-detail`) map onto the same palette; the two
  roles absent from the V3 list (`athlete-shorts`, `athlete-eye`,
  `athlete-face-detail`) get their own enum members so the athlete reads
  correctly, and the enum documents which pack each role belongs to.

## R4 — Replay scene replaces the smoke scene

- R4.1 `qml/RowPlay/Replay/ReplayScene.qml` owns a `View3D` with
  `ExtendedSceneEnvironment` (filmic tonemapping), a
  `ProceduralSkyTextureData` `lightProbe` whose zenith/horizon/ground/sun
  colours come from the sport palette in `rowplay_core::replay::theme`
  (`venues_light` / `venues_dark`, resolved through the view-model and
  exposed as QML colour strings — no palette hex in QML), a shadow-casting
  `DirectionalLight`, and a ground plane tinted from the same palette.
- R4.2 No downloaded HDRI (ADR 0004) and no port of
  `renderer3dEnvironment.ts` (ADR 0005): the venue is the ground plane plus
  the procedural sky only; venues are Phase 6.
- R4.3 The scene loads one sport's equipment template set plus the athlete
  placeholder (the skinned V4 mesh is loaded but not yet posed — 5b), arranged
  statically at the anchors of R5. The Phase 0 smoke scene stays
  (`SmokeScene.qml`) because the bootstrap smoke test pins it.
- R4.4 Light/dark follows `Theme` (the `Qt.styleHints.colorScheme` binding),
  pinnable with `ROWPLAY_FORCE_COLOR_SCHEME` like the rest of the shell.

Outcome (2026-09-12): the key light follows the web's per-sport `SUN_OFFSETS`
aimed at `SHADOW_TARGET_HEIGHT` (0.55 m) through a `LookAtNode`, both as
data in `rowplay_viewmodel::replay::palette`. Shadows required metre-scaled
Qt parameters (`shadowBias` 0.02, `pcfFactor` 0.03, `shadowMapFar` 60,
`csmNumSplits` 2 — Qt's defaults assume a scene about a hundred times
larger), and the camera needs `clipNear: 0.1` for the same reason. The
procedural sky is rebuilt per palette change because in-place colour edits
do not refresh the probe. The athlete placeholder is the balsam `Athlete`
component, unposed at (0, 0, -2.4).

## R5 — Anchor contract data

- R5.1 `rowplay_viewmodel::replay::anchors` records the README's anchor table
  verbatim as data: oarlocks `(±0.88, 0.51, 0.28)` in row avatar-root
  coordinates, the ski anchor `(side × 0.15, 0, 0.16)`, the seat carriage in
  moving rower-group coordinates, the wheel assembly at the wheel-group
  centre with its axle along local X, the frame assembly in bike avatar-root
  coordinates, and the drivetrain assembly in crank-group-local coordinates.
  5a stores and unit-tests the table; 5b applies it.

## R6 — Static verification in the gate

- R6.1 The runtime-error gate (`ROWPLAY_SMOKE_GATE=1`) walks the replay scene
  for each of the three sports: equipment templates load and validate, the
  athlete loads, and a screenshot per sport is saved to
  `ROWPLAY_SMOKE_SCREENSHOT_DIR` and uploaded as a CI artifact.
- R6.2 Each screenshot is asserted non-blank and non-single-coloured by the
  existing pixel checks in `smoke_screenshot.rs` (extended to the per-sport
  captures): at least N distinct colours and a non-trivial share of pixels
  differing from the background.
- R6.3 A Qt-free unit test asserts the gate's validation walk exercises every
  template root and every leaf slot (the same code path as R2.5).

## R7 — Invariants and docs

- R7.1 Privacy invariants untouched: no token, no user data in the replay
  path; demo mode remains fully explorable without a Concept2 token.
- R7.2 The QML member check (`ROWPLAY_GATE_MEMBER_CHECK`) stays green with the
  new `Replay` singleton members added to the scan.
- R7.3 `docs/source-map.md` gains the Phase 5a rows (web → Rust/QML) and the
  divergences 5a introduces; `docs/roadmap.md` marks Phase 5a delivered;
  every qtbridge friction point met lands in `docs/qt-bridges-notes.md` with a
  minimal repro (at minimum the balsam outcome of R2.2).
- R7.4 `.kiro/specs/phase-05a-assets-scene/tasks.md` tracks these
  requirements; every task maps to a requirement id.
