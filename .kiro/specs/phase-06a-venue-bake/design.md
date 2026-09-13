# Phase 6a — Venue baking pipeline: design

## Pipeline overview

```
reference/rowplay (pinned)                tools/bake-venues/
  renderer3dEnvironment.ts   ── bake.mjs ─▶ build/venues-staging/
  renderer3d.ts (tables)                    ├ rowplay-venue-<sport>-<tier>.glb
  static/replay-assets/…                   ├ rowplay-venue-<sport>-<tier>.json  (contract)
  node_modules/three@0.184.0               └ procedural/*.png
                     ── cleanup.py (Blender) ─▶ hygiene pass, round-trip verified
                     ── vendor ─▶ assets/replay/venues/  (+ hashes, provenance)
                     ── build.rs ─▶ validate_venue drift gate (viewmodel)
```

## What the baker replicates (and where from)

The web venue as the renderer assembles it is `buildEnvironment` minus
`buildSky` (`renderer3d.ts:1751-1876`). The baker reproduces:

| Piece | Source (pinned `rowplay@011e8303`) |
| --- | --- |
| `QualityConfig` per tier | `renderer3d.ts:65-138` (`QUALITY`) |
| `EnvironmentStyle` per sport | `renderer3d.ts:414-528` (`ENVIRONMENTS`), copied with source comment |
| helper closures (`mat`, `track`, `trackInstanced`, `environmentStandardMat`, `environmentBasicMat`, `makeVerticalArc`) | `renderer3d.ts:1247-1308`, `1722-1748` |
| horizon rings (row/ski only: far r116 detail≥1, mid r84 always) | `renderer3d.ts:1759-1795` |
| infield circle (r `innerR-0.8`, hidden row/ski) + apron ring (`outerR+0.2`→55) incl. procedural map slots | `renderer3d.ts:1797-1856` |
| world dispatch (`addRowerRegattaWorld` / `addSkiStadiumWorld` / `addBikeCircuitWorld`) | `renderer3d.ts:1858-1876` |
| `innerR` 22, `outerR` 34 | `renderer3d.ts:2025-2026` |

The sky, the course ground plane, lanes, buoys and particles stay Qt-side
(Phase 5 scene + Phase 6b bindings); the venue GLB is the static world.

Node mechanics (ADR 0010): `--experimental-transform-types` (constructor
parameter property in the builder), a `module.register` resolve hook that
appends `.ts` to extensionless relative imports and maps bare `three` to the
reference checkout's `node_modules`, `Math.random = mulberry32(20260913)`
before the builder import, and a `FileReader` shim for GLTFExporter.

## Contract JSON (format 1)

One per GLB, next to it:

```jsonc
{
  "format": 1,
  "seed": 20260913,
  "source": { "repo": "rowplay", "commit": "…", "entry": "renderer3dEnvironment.ts" },
  "sport": "skierg", "quality": "ultra", "environmentDetail": 3,
  "innerR": 22, "outerR": 34,
  "materials": {
    "environment:skierg:mountain-material": {
      "type": "standard",          // standard | basic | physical
      "color": { "light": "#7897a8", "dark": "#23454a" },
      "roughness": 0.92, "metalness": 0.0,
      "opacity": 1.0, "alphaMode": "opaque", "doubleSided": true,
      "vertexColors": true, "depthWrite": true,
      "textures": {                 // present only for slots the web binds
        "map": { "kind": "set", "path": "/replay-assets/…-diffuse-512.jpg", "repeat": [8, 8] }
      },
      "normalScale": [0.22, 0.22]
    }
  },
  "instancing": {
    "environment:skierg:mountain-peaks": [
      { "p": [x,y,z], "q": [x,y,z,w], "s": [x,y,z], "c": [r,g,b] }   // c optional
    ]
  },
  "hidden": ["environment:skierg:infield"],
  "proceduralMaps": { "water-normal": { "size": 128, "repeat": [10,10], "png": "procedural/water-normal-128.png" } },
  "inventory": { "nodes": 74, "meshes": 52, "instancedMeshes": 17, "instances": 641, "materials": 34 },
  "bounds": { "min": [.., .., ..], "max": [.., .., ..] }
}
```

- `color` comes from intercepting every `environmentStandardMat` /
  `environmentBasicMat` call and evaluating the passed `ThemeColor` for both
  themes — the runtime re-tints on scheme change exactly like the web's
  `environmentThemeMats` registry.
- `textures[].repeat` is already multiplied into the GLB's UVs by the baker;
  the entry stays in the contract for documentation and re-bake checks.
- `hidden` meshes are exported (not dropped) with their web visibility, so
  the structural inventory is stable across tiers.

## Instancing delivery

`EXT_mesh_gpu_instancing` is deliberately **not** written (balsam drops it
silently; Blender expands it — ADR 0010). Each `InstancedMesh` becomes a plain
archetype `Mesh` node at identity; instances live in `instancing` keyed by
node name. Phase 6b creates `Model { source: <archetype mesh>; instancing:
InstanceList { instanceList: […] } }` per entry, carrying the per-instance
colour tint.

## Procedural maps

`makeWaterNormalTexture`, `makeSnowSurfaceTexture`, `makeWaterSurfaceTexture`
(`renderer3dEnvironment.ts:705-806`) build `THREE.DataTexture`s from
noise/sin fields — DOM-free, so the baker calls them through the real builder
code, reads `texture.image.data`, and encodes RGBA8 → PNG with a minimal
encoder (IHDR + one filtered IDAT via `node:zlib` deflateSync + IEND,
deterministic). Output names carry the size variant
(`water-normal-64/128.png`, …); the contract records which variant each
sport-tier binds (web gates: surface maps at detail ≥ 1, water normal at
detail ≥ 2, 128 px at ultra/detail 3 — the size split mirrors
`makeWaterNormalTexture(ultra)` / `makeSnowSurfaceTexture(detail)`).

## Blender hygiene pass

`cleanup.py` (`blender --background --python`, `--threads 1`,
`PYTHONHASHSEED=0`): import GLB → `delete_loose` + degenerate-face dissolve
per mesh → verify names/materials/colour layers/UV layers survived 1:1 →
export GLB (fixed option set). No merge-by-material, no AO (ADR 0010).
Triangulation changes vertex counts; that is expected and flows into the
contract inventory regeneration (the contract is re-derived from the cleaned
GLB's accessor counts, not trusted from the pre-Blender build).

## Validation and vendoring

- `rowplay_viewmodel::replay::venue`: `read_venue(bytes) -> VenueDocument`
  (bounded, like `glb.rs`) + `validate_venue(&doc, &contract_json)` checking
  R4.2's rules. Unit tests synthesise each defect class (unnamed node,
  embedded image, wrong inventory, out-of-bounds, unnamed material).
- `build.rs` runs `validate_venue` over every vendored venue pair at build
  time and panics naming the file and rule — the V3 drift-gate pattern.
- `tools/vendor-replay-assets.py` learns the `venues/` family: size+SHA
  literals, `--emit-rust` rows, provenance rows (including the generated
  PNGs and their generator + seed).
- Sizes measured before the ADR 0011 decision; the 50 MB tripwire stays
  untouched until that ADR changes it deliberately.

## Determinism argument

- One seeded `Math.random` fixed before module evaluation → one Simplex
  permutation; all scatter jitter is index-hashed in the web source.
- Export order is scene-graph insertion order; GLTFExporter writes stable
  key order; float32 buffers are exact.
- The PNG encoder and JSON writer are deterministic; Blender runs
  single-threaded with a fixed hash seed (the precedent of
  `scripts/build-replay-rig-v4.mjs`).
- The baker re-bakes one variant into a temp dir on every run and compares
  bytes; any nondeterminism fails the bake.
