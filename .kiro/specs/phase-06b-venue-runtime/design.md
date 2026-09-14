# Phase 6b — Venue runtime: design

## Data flow

```
vendored contracts (assets/replay/venues/*.json)
        │ build.rs (embeds, after validate_venue)
        ▼
replay_venues_meta.json  ── include_str! ──▶ ReplayBackend (venue_meta)
        │ viewmodel: replay::venue_runtime::InstancePlan      │
        ▼                                                     ▼
Replay.venuePlan (JSON: materials, instance buckets,      component URL
texture bindings per material, hidden)                      per sport+tier
        │
        ▼
ReplayScene.qml: createObject(venue root) → walk → materials + InstanceLists
                 → textures only when tier ≥ High
```

## Build-time embedding

`build.rs` already validates every pair; 6b extends the embedded meta from
`inventory_json()` to the full runtime plan per variant:

```jsonc
"rowplay-venue-rower-high": {
  "component": "Rowplay_venue_rower_high",        // balsam component name
  "materials": { "<name>": { "type": "standard|basic|physical",
      "colorLight": "#rrggbb", "colorDark": "#rrggbb", "roughness": …,
      "metalness": …, "opacity": …, "blend": bool, "doubleSided": bool,
      "vertexColors": bool, "depthWrite": bool,
      "normalScale": [u,v],
      "textures": { "map": {"kind":"set|procedural","path":"…"}, … } } },
  "instanceGroups": { "<node name>": [ { "tint": [r,g,b], "transforms":
      [[px,py,pz,qx,qy,qz,qw,sx,sy,sz], …] }, … ] },   // shade buckets
  "bounds": {"min": […], "max": […]}
}
```

Bucketing (R2.2) happens in the **view-model** (`replay::venue_runtime`),
Qt-free and unit-tested: for each group, k-means-free fixed quantisation of
the tints onto a 2×2 lightness × warmth grid (matching `scatterTint`'s
`spread`/`warmth` semantics), emitting at most 4 buckets with each instance's
full transform preserved. The embed step calls the view-model so the JSON in
the binary is the bucketed plan; a unit test cross-checks the plan against the
vendored contract (every instance appears in exactly one bucket; tints within
a bucket differ by less than the quantisation step).

Total embedded size ≈ 0.8 MB of JSON (biggest contract 163 KB); the athlete
precedent embeds ~100 KB — same pattern, ADR 0008's "balsam drops extras, so
the runtime needs a sidecar" reasoning.

## Balsam packs

`PACKS` grows to 14 rows: the 12 venue GLBs → components
`Rowplay_venue_<sport>_<tier>`, exported as `Venue<Rower|Skierg|Bike><Tier>`,
subdirectory `venues/<sport>-<tier>/`. `--removeComponentAnimations` is
harmless for venues (no animations). The 12 components live in the same
`RowPlay.ReplayAssets` module and rcc blob (~9.6 MB uncompressed; rcc is
`--no-compress` for the module, accepted for a desktop app, recorded in the
PR).

## Scene integration (ReplayScene.qml)

- **All twelve variants instantiate statically** inside
  `Node { id: venueRoot }` beside the Phase 5 ground plane, and exactly the
  (sport, effective tier) match is `visible`. The design originally planned a
  dynamic `Qt.createComponent` + `createObject` loader; that was built first
  and **did not render**: the object tree is complete and walkable, but no
  dynamically created Quick 3D component ever rasterises (recorded in
  qt-bridges-notes). Static instantiation is the rigs' own pattern, and a
  tier swap becomes an instant visibility flip — which matters for governor
  step-downs (no reload hitch on degradation).
- Each variant is walked once on first show (`syncVenue`): contract
  materials, bucketed `InstanceList`s, tier-gated textures. Walk registries
  (materials, instance lists, texture cache) live for the session — bounded
  (~350 materials, ~70 lists across all twelve variants; textures shared by
  source). Errors call `Replay.reportError("venue load failed: …")` (R1.3)
  and are also a gate failure via the log line.
- The walk mirrors `applySceneRules`: for each `Model` found by `objectName`:
  - materials: one dynamically created `PrincipledMaterial` per contract
    entry (created once per venue load, stored in a `venueMaterials` JS
    object — the retained reference), `baseColor` from the scheme's colour,
    `lighting: NoLighting` for `basic`, `blend`/opacity for alpha,
    `vertexColorsEnabled`, `clearcoatAmount` for physical glass,
    `cullMode: DisableCulling` for double-sided.
  - instances: if the name is in `instanceGroups`, create the bucketed
    `InstanceList`s (`Instance { position, rotation, scale }` per transform)
    parented under `venueRoot` (retained), one per bucket, each `Model` gets
    `instancing` set and its archetype draw otherwise suppressed by the list
    (the single-vertex-offset archetype draw is avoided by giving every
    bucket list the archetype mesh and never rendering the bare Model).
  - textures: only when `Replay.tierSettings.normalMaps || textureSets.length`
    says the tier binds them; `Texture { source: qrc path }` created per bound
    slot (also retained), assigned to the material. Low/Medium create none.
- Scheme change re-tints in place (walk the retained materials) without
  reloading the venue — mirroring how the rig materials bind to `Theme`.
- No per-frame venue work: nothing in `applyFrame` touches the venue (static
  world), preserving one-bridge-crossing-per-frame.

## Texture rcc

A fourth generated resource `rowplay_environments.qrc` (build.rs) bundling
`assets/replay/environments/**` (39 JPEGs) and
`assets/replay/venues/procedural/*.png` under `/qt/qml/RowPlay/Environments/`
(~4.6 MB). The tier resolver's `texture_sets` already names the directories;
the contract's `textures` entries carry the file suffix, so the QML resolves
`qrc:/qt/qml/RowPlay/Environments/<set>/<file>` from
`kind: "set" path: "/replay-assets/environments/<set>/<file>"` by suffix
substitution (the web's `/replay-assets/` prefix maps to the rcc root).

## Gate

- `Main.qml` replay steps already switch sports and grab; `loadVenue()` logs
  `replay venue <sport>: …` on success, `replay venue FAILED <sport>: …` on
  any failure. `qml_runtime_gate.rs` reads the vendored contracts
  (`assets/replay/venues/*.json`, serde_json is already an app dependency)
  and asserts the exact node/group/instance/material counts per sport, and
  that no FAILED line appears.
- Tier cycling steps (62–65) now also exercise venue reloads; the extended
  texture assertion counts venue textures in the scene (logged by
  `replay venue textures <tier>: n`), 0 at Low/Medium.
- `assert_shadows` fractions re-checked against the new captures; re-derived
  and commented if the venue moved them.

## Performance expectations

The venue adds ~25–80k vertices per sport-tier (contracts' inventory) as
static, non-instanced-until-bucketed geometry. Draw calls rise by
(meshes − groups + groups×buckets): the bucket scheme keeps tree/berm
scatters at ≤4 draws per group. Medium (the default) is expected to stay near
the 5c profile; Ultra was already at/over budget and is the likely finding —
R5.2's ladder applies, starting with tier-dependent venue variants that
already exist (the Low bake is ~6× smaller than Ultra).

## Test plan

- viewmodel: bucketing unit tests (deterministic, contract-cross-checked).
- app: `build.rs` gate runs in every build; venue contract tests from 6a keep
  covering the vendored bytes.
- gate: venue inventory lines (exact), FAILED never, venue texture counts,
  existing screenshot/shadow/diversity assertions with re-derived regions if
  needed.
- bench: per sport × tier with ghost, `QSG_NO_VSYNC=1`, on hardware GL;
  governor rollback re-check; numbers in the PR and roadmap.
