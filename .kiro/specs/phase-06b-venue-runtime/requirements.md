# Phase 6b — Venue runtime: requirements

Loads the Phase 6a baked venues into the replay scene, binds the quality-tier
texture sets to them, extends the gate, and re-measures performance with the
venues present. Layered on PR 6a (branch `phase-06a-venue-bake`).

## R1 — Loading

- R1.1 `build.rs` converts all 12 venue GLBs with `balsam` into the existing
  `RowPlay.ReplayAssets` module (one component per sport per tier, meshes
  bundled in the rcc), after the 6a drift gate has validated the bytes.
- R1.2 Exactly one venue component is instantiated at a time: the active
  sport's tier variant, selected by the effective quality's
  `environmentDetail` (low→low … ultra→ultra, matching the web's
  `cfg.environmentDetail`). Sport or effective-tier changes swap the instance;
  the old one is destroyed. Instantiation is dynamic (`Qt.createComponent` +
  `createObject` under the scene root, retained reference) because a `Loader`
  item cannot live inside a `View3D` scene graph.
- R1.3 A venue that fails to load or fails its material/instance walk is a
  visible error state (`Replay.loadState = "error"`), never a silent fallback
  to the Phase 5 ground plane.
- R1.4 The venue renders relative to the course loop: the GLBs are authored in
  the web's course space (`innerR` 22 / `outerR` 34, y-up, metres) so the
  component sits at the scene origin with no offset; the Phase 5 ground plane
  remains the course surface beneath the lanes.

## R2 — Materials and instances

- R2.1 After load, the scene walks the venue subtree by `objectName` and
  applies materials built from the contract: light/dark base colour (live
  re-tint on scheme change), roughness, metalness, opacity/alpha mode,
  double-sided, vertex colours, unlit (the web's `MeshBasicMaterial`), and
  clearcoat for the two glass materials. Material objects are created
  dynamically and kept in a retained registry (qt-bridges-notes: a dynamic
  material without a parent or retained reference is garbage-collected).
- R2.2 Per-instance colour tints are applied without custom shaders: the
  view-model buckets each instance group's tints into a small fixed set of
  shade classes and emits, per bucket, one transform list plus its tint; the
  scene creates one `InstanceList` + one tinted material per bucket. The
  bucket count is fixed (4) and the quantisation is deterministic; unit tests
  pin bucket counts, tint values and transform preservation.
- R2.3 Instance groups come from the contract (`instancing`, keyed by
  archetype node name): the scene turns each archetype `Model` into an
  instanced draw (`Model.instancing: InstanceList`), replacing the single
  instance. The contract's transforms are world-space already (the baker
  premultiplied the node transform).

## R3 — Quality tiers and textures

- R3.1 The environments JPEGs and the procedural PNGs ship in the rcc
  (bundled by `build.rs`). At Low and Medium **no venue texture is
  instantiated at all** (the environments README rule, extended from the 5c
  tier resolver). High binds the sport's `textureSets` diffuse + roughness;
  Ultra adds the normal maps (`normalMaps`) and the SkiErg terrace set —
  exactly `tier_settings(quality, sport)`.
- R3.2 Texture bindings come from the contract per material and slot; the
  `kind: "set"` entries resolve to `assets/replay/environments/...` paths in
  the rcc, `kind: "procedural"` to `venues/procedural/...`. `normalScale`
  from the contract applies where a normal map is bound.
- R3.3 The 5c per-tier texture gate assertion is extended to count venue
  texture bindings actually instantiated in the scene (0 / 0 / N / N+normals),
  not just the resolver's table.
- R3.4 Tier swaps (Settings or governor step-down) rebuild the venue
  instance: new tier's component, materials, and texture set. The governor's
  5c payoff-rollback behaviour is re-verified against the venue-era frame
  profile (R5.2).

## R4 — Gate

- R4.1 The scene logs `replay venue <sport>: <nodes> nodes, <groups> instanced
  groups, <instances> instances, <materials> materials` after each successful
  load. The gate test parses these lines and requires an **exact match**
  against the vendored contracts (read from `assets/replay/venues/`), per
  sport — a missing line, a wrong count, or a `replay venue FAILED` line
  fails the walk. There is no fallback path.
- R4.2 The per-sport gate screenshots (`replay-row/-ski/-bike`) now contain
  the venue; the existing render assertions (size, colour diversity) and the
  shadow-luminance margin stay. If the venue shifts the luminance regions,
  the sample fractions are re-derived from the new deterministic framing and
  the change is recorded in the test comment (5b lesson). *Superseded
  2026-09-25 (round 2's shadow-check PR, #99):* the per-sport region margin
  measured albedo, not a shadow, because the per-sport captures are Medium,
  where nothing casts shadows. The shadow check now compares the rower at
  High (`replay-row-high`) with its twin captured with the key light's
  shadow off (`replay-row-high-unshadowed`), after checking that both
  twins' 3D viewports rendered: the shadow must darken at least 1 % of the
  capture by more than the 3D noise delta, and lose more than 5 % of its
  luminance there.
- R4.3 New gate steps capture one venue-specific assertion per sport (the
  inventory line above); no new screenshots are needed beyond the existing
  three, which now cover venue rendering.

## R5 — Measurements

- R5.1 Frame times re-measured per sport per tier with the venue present,
  ghost on, `QSG_NO_VSYNC=1`, on the 5c reference machine, reporting median,
  p95 (renderStats) and wall-clock stall counts — the 5c methodology and
  table format.
- R5.2 If the default tier (Medium) now exceeds the governor budget at p95,
  that is acted on, not documented away: in order, (a) venue geometry
  reduction at lower tiers, (b) culling, (c) changing the default tier. The
  choice and its evidence go in the PR and roadmap.
- R5.3 The governor payoff-rollback is re-checked with venues present (step
  down once → no improvement → rollback + lock); deviations are documented
  with numbers.

## R6 — Documentation and rules

- R6.1 `docs/roadmap.md` 6b entry, `docs/source-map.md` runtime rows,
  `docs/qt-bridges-notes.md` for any new bridge friction (dynamic component
  creation inside `View3D`, InstanceList behaviour).
- R6.2 All user-visible strings come from the locale pipeline; the venue adds
  no new user-visible strings (load errors reuse `replay.view3dError`).
- R6.3 No hand-written C++; no new replay maths outside `rowplay-core`/the
  view-model; one bridge crossing per frame (the venue is static — no
  per-frame venue traffic); the gate and QML member check stay green.
