# ADR 0010 — Venue baking runs the web builder headless in Node; instances live in a contract, not the GLB

Status: accepted (2026-09-13)

## Context

ADR 0005 decides Phase 6 bakes the web's procedural venue worlds
(`reference/rowplay/src/lib/replay/renderer3dEnvironment.ts`, 3,203 lines) to
`.glb` instead of porting them, and requires a feasibility check first: the
builder calls `document.createElementNS` for texture decoding, so it was not
known whether Node alone could run it or whether a headless browser would be
needed.

A spike (`tools/bake-venues/`, results below) against the pinned reference
(`rowplay@011e8303`) answered every open question.

## Findings

1. **Node runs the builder directly.** The only DOM references in the file are
   the two guarded lines in `loadEnvironmentTexture`
   (`renderer3dEnvironment.ts:652-656`): outside a browser it substitutes an
   empty `THREE.Texture` and records `userData.sourcePath`. Geometry
   generation is DOM-free — confirmed by the web repo's own vitest suite,
   which builds all three worlds under `environment: "node"`
   (`renderer3dEnvironment.test.ts`), and by our spike. A headless browser is
   not needed. What Node *does* need:
   - `--experimental-transform-types` (the builder class uses a constructor
     parameter property, which strip-only mode rejects) plus a resolve hook
     that appends `.ts` to the builder's extensionless relative imports;
   - a `FileReader` shim for `GLTFExporter`'s binary path (same shim as the
     web repo's `scripts/build-replay-rig-v4.mjs`);
   - all texture bindings stripped before export, because the exporter's image
     paths need a 2D canvas (we keep bindings as data instead — see below).
2. **Determinism is reachable.** The builder's randomness is a module-private
   unseeded `SimplexNoise` (`renderer3dEnvironment.ts:229-233`) plus
   index-hashed scatter jitter that is already deterministic. The singleton
   seeds its permutation table from `Math.random` at module evaluation, so
   pinning `Math.random` (mulberry32, seed 20260913) before importing the
   module fixes every noise-driven silhouette. A double bake produced
   byte-identical GLBs.
3. **Balsam silently drops `EXT_mesh_gpu_instancing`.** three's exporter
   writes instanced nodes with `TRANSLATION/ROTATION/SCALE/_COLOR_0`
   attributes (marked required), but Qt 6.11.2's balsam converts them to plain
   single-instance `Model`s — no warning. Vendoring a GLB that relies on the
   extension would ship venues with every tree line collapsed onto one
   instance.
4. **Blender's glTF importer expands instances** (a 52-mesh ski venue became
   507 meshes), and its exporter does not recompress them. A Blender pass over
   an instanced GLB is therefore destructive.

## Decision

- The baker (`tools/bake-venues/bake.mjs`) drives `EnvironmentBuilder`
  directly with a context that replicates the production wiring in
  `renderer3d.ts` (`QUALITY` table, `ENVIRONMENTS` palettes, the
  `environmentStandardMat`/`environmentBasicMat`/`makeVerticalArc` helper
  closures, `innerR` 22 / `outerR` 34, horizon rings, infield and apron —
  everything `buildEnvironment` adds except the procedural sky).
- **Instances are contract data, not GLB data.** Before export, each
  `InstancedMesh` is replaced by a plain archetype mesh (same geometry,
  material and name, identity transform); its per-instance matrices and
  `instanceColor` tints are recorded in a sidecar contract JSON. The Phase 6b
  runtime builds Qt `InstanceList`s from the contract. This keeps the GLB
  small (one geometry per archetype), Blender-safe (no expansion), immune to
  balsam's dropped extension, and keeps draw calls low via Qt's own
  instancing.
- One GLB per sport per quality tier (12 total: geometry genuinely differs at
  every `environmentDetail` level — instance counts, feature zones, and
  `laneSegments`-driven segment counts). All four tiers bake in one process
  so they share one noise permutation: Low's mountains are Ultra's mountains,
  minus dressing.
- **No textures are baked into the GLB.** Materials keep their names, base
  colours (light theme), roughness/metalness/alpha/unlit semantics; every
  texture binding — Poly Haven set slot or procedural
  water-normal/snow-groomed/water-surface map — is recorded in the contract
  with its slot, source path and `repeat`. The baker multiplies `repeat` into
  the geometry UVs (Qt Quick 3D textures have no UV transform), and encodes
  the procedural `DataTexture`s to PNG itself (its own deterministic
  `node:zlib` encoder — no canvas).
- The Blender pass (`cleanup.py`) is geometry hygiene only: import, drop
  degenerate/dissolvable geometry, verify names/materials/colours/UVs
  survived, re-export single-threaded (`--threads 1`, `PYTHONHASHSEED=0`).
  **Merging by material is rejected** — it would destroy the archetype +
  contract structure that keeps venue files small and draw calls low.
  **Baking ambient occlusion is rejected** — it would embed baked textures,
  contradicting the no-embedded-textures contract, and ambient light already
  comes from the procedural-sky IBL (ADR 0004).

## Consequences

- The bake is reproducible from the pinned reference with one command
  (`node --experimental-transform-types --import tools/bake-venues/register.mjs
  tools/bake-venues/bake.mjs`); re-baking after a reference bump requires
  re-reviewing the copied `ENVIRONMENTS`/`QUALITY` tables in the baker
  against the new source.
- Vendoring trades the web's per-session variation for a fixed, seeded set —
  recorded here and in the contract (`seed: 20260913`).
- Balsam's silent `EXT_mesh_gpu_instancing` drop is logged in
  `docs/qt-bridges-notes.md` for upstream.
- File-size consequences are measured and decided in the ADR 0009 follow-up
  (ADR 0011) on real numbers.
