# ADR 0008 — Replay packs load as `balsam` components in every build; `RuntimeLoader` is not used

Status: accepted (2026-09-12)

## Context

[ADR 0003](0003-gltf-runtime-assets-authored-in-blender.md) expected the
vendored `.glb` packs to load through `QtQuick3D.AssetUtils` `RuntimeLoader`
in development, with `balsam` output reserved for release builds. The replay
has to do two things to the loaded packs from QML, with no C++
([ADR 0001](0001-rust-core-with-qt-bridges-and-qt-quick-3d.md)): in Phase 5a,
replace every rig mesh's material with the `PrincipledMaterial` for its
`replayMaterialRole` (following `renderer3dAssets.ts`); in Phase 5b, pose
the V4 athlete's joints from Rust.

`RuntimeLoader` cannot serve either. In Qt 6.11.2 it exposes only `source`,
`status`, `errorString`, `bounds`, `instancing` and the two supported-format
lists (`qquick3druntimeloader_p.h`); the nodes it imports carry no
`objectName` and are not reachable through `children` from QML. Walking a
`RuntimeLoader { source: ".../rowplay-rigs-v3.glb" }` recursively for
`objectName === "equipment:row:boat-assembly"` never finds it, so neither a
material-role walk nor joint posing can address the scene.

`balsam`, by contrast, generates one QML component per pack in which every
node carries its glTF name as `objectName` and the V4 skin becomes a `Skin`
whose 51 joints are `Node`s named after the contract's bones (56 named
`v4*` nodes in all, the four contact helpers included). It also collapses
each pack to its single
placeholder material, drops the glTF `extras` (`replayAsset*`,
`replayMaterialRole`) and, unless told otherwise, turns the athlete's three
authored clips into auto-running `Timeline`s whose `.qad` keyframe files are
not part of the bundle.

## Decision

- `build.rs` runs `balsam --removeComponentAnimations` on both vendored
  packs (`rowplay-rigs-v3.glb`, `rowplay-athlete-v4.glb`) in **every** build.
  `balsam` is found through `qmake -query` like `rcc` (override
  `ROWPLAY_BALSAM`). The generated `RowPlay.ReplayAssets` module (`Rigs`,
  `Athlete` and their meshes) is bundled as its own `rcc --binary` blob and
  registered next to the QML and i18n bundles.
- The V3 contract (`rowplay_viewmodel::replay::glb::validate_v3`) is
  enforced at build time on the exact bytes `balsam` converts; a drift fails
  the build with the named slot, template or node path.
- Material roles are re-applied at runtime from the map `validate_v3`
  produces — embedded at build time as `replay_assets_meta.json`
  (`include_str!`), and recomputed from the on-disk pack whenever the
  startup re-validation below runs: the scene walks the `Rigs` component by
  `objectName` and assigns each mesh the material for its role. The
  `Athlete` component keeps balsam's placeholder material over the pack's
  vertex colours until Phase 5b poses it and applies the V4 surface roles.
- Development builds (`ROWPLAY_REPLAY_ASSETS`, or the repository's
  `assets/replay/` in debug builds) re-validate the on-disk pack at startup,
  so an edited asset fails loudly with the slot name.
- The `.glb` files are not shipped in the binary; `RuntimeLoader` is used
  nowhere; no C++ is written.

This supersedes only the "`RuntimeLoader` at development time" consequence of
ADR 0003. The rest of ADR 0003 — `.glb` as the only runtime format, Blender
authoring, no USDZ, provenance for every vendored file — stands.

## Consequences

- `balsam` (part of the Qt Quick 3D module) becomes a build dependency of
  `rowplay-app` on every platform, next to `rcc` and `lrelease`.
- Because `balsam` collapses materials and drops the glTF extras, the role
  map from `validate_v3` (build-time or startup) is the source of material
  roles at runtime; the packs' placeholder material is never a product
  colour source.
- The athlete's authored clips are stripped from the component: Phase 5b
  evaluates them in Rust and Qt's animation system must never drive the
  joints. The `Skin` and every named joint are kept and the mesh files are
  identical with or without the flag.
- Editing an asset needs a rebuild (`cargo build -p rowplay-app`) rather than
  a restart; the startup re-validation only reports contract drift, it does
  not reload geometry.
- The scene is in metres, as the web's is, so Qt Quick 3D's unit defaults
  (`clipNear` 10, `shadowBias` 10, `pcfFactor` 2.0, `shadowMapFar` 5000)
  assume a scene roughly a hundred times larger and must be set explicitly
  (`clipNear: 0.1`, `shadowBias: 0.02`, `pcfFactor: 0.03`, `shadowMapFar:
  60`, two cascades).
