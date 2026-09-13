# Baked venues

The RowErg regatta basin, SkiErg alpine stadium and BikeErg velodrome as
static glTF, baked from the pinned rowplay checkout by
`tools/bake-venues/` (ADR 0005: bake, don't port; ADR 0010: the route).

## Files

One `.glb` and one `.json` contract per sport per quality tier, plus the
generated procedural maps:

```
rowplay-venue-<sport>-<tier>.glb    sport = rower | skierg | bike
rowplay-venue-<sport>-<tier>.json   tier  = low | medium | high | ultra
procedural/<map>-<size>.png
MANIFEST.json                       reviewed size + SHA-256 of every file
```

Low/Medium/High/Ultra differ in geometry, so all four are baked. The bake is
seeded (`20260913`) and byte-reproducible:

```bash
tools/bake-venues/bake.sh --verify     # bake + Blender pass + determinism check
tools/vendor-venues.py --accept        # review hashes and install
```

## Contract JSON

The sidecar is what the runtime needs beyond geometry:

- `materials`: every material name → type, light/dark base colour,
  roughness/metalness, opacity, alpha mode, vertex-colour and depth-write
  flags, and the texture bindings per slot. Set bindings name the vendored
  Poly Haven derivative under `assets/replay/environments/`; procedural
  bindings name a map under `procedural/`. Texture `repeat` is already
  multiplied into the GLB's UVs (Qt Quick 3D textures carry no UV transform),
  and the value stays recorded here for reference.
- `instancing`: instance transforms (position, quaternion, scale) and optional
  per-instance colour tints, keyed by the archetype node name. three.js would
  write `EXT_mesh_gpu_instancing`, but balsam drops that extension silently
  and Blender expands it, so instances travel here and the GLB holds one
  archetype mesh per group (ADR 0010).
- `hidden`: names of meshes the web builds but hides (the generic infield
  underlay the RowErg/SkiErg centre replaces); they are not in the GLB.
- `environmentDetail`, `innerR` (22) and `outerR` (34): the web's tier detail
  level and the course radii the venue is placed around.
- `inventory` and `bounds`: structural facts asserted by the build-time drift
  gate and the gate tests.

## Naming convention

Every node and material is named `environment:<sport>:<thing>`, matching the
web builder (`renderer3d.ts` / `renderer3dEnvironment.ts`), so
`docs/source-map.md` stays greppable. The root node is
`venue-<sport>-<tier>`. Instance groups keep the archetype node's name, and
the contract's `instancing` keys are those names.

## Licence and provenance

Generated from rowplay's MIT-licensed venue builders; rows are in
`ASSET_PROVENANCE.md` and the byte pins in
`crates/rowplay-app/tests/asset_hashes.rs`.
