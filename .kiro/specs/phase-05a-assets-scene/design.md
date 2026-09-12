# Phase 5a — Assets and scene: design

## Overview

5a is three independent layers glued by one backend singleton:

1. **Data** — vendored binaries under `assets/replay/` plus a Qt-free
   contract validator that reads the GLB JSON chunk directly.
2. **View-model** — `rowplay_viewmodel::replay`: the material-role enum, the
   palette resolver (Theme → role colours, light/dark, lane paint, ghost), the
   sky/ground palette bridge to `replay::theme`, and the anchor table.
3. **App** — `crates/rowplay-app/src/replay/`: the Qt loader (RuntimeLoader or
   balsam meshes), the `Replay` QML singleton that exposes load state and
   resolved colours, and `qml/RowPlay/Replay/` (scene, materials, equipment
   containers).

Dependency direction stays app → viewmodel → platform → core (ADR 0006). No
new crate and no new workspace dependency: the GLB reader is a small parser in
the app crate's test scope and in a `replay::glb` module with no dependencies
beyond `serde_json` (already present for the Concept2 mapper rationale; here
it parses the glTF JSON chunk only).

## D1 — Vendoring and hash pin

`tools/vendor-replay-assets.py` (Python, stdlib only, mirroring
`tools/vendor-fixtures.py`) copies the selected files from
`reference/rowplay/static/replay-assets/` into `assets/replay/`, computes
SHA-256 for each, and rewrites the vendored table in `ASSET_PROVENANCE.md`.
It refuses to run when `reference/` is absent and fails on any hash mismatch
against its embedded expectation table, so a refresh is an explicit act.

`crates/rowplay-app/tests/asset_hashes.rs` holds the same expectation table as
Rust literals and hashes the committed bytes. It is Qt-free (pure file IO +
`rowplay_fixtures::sha256_hex`) so it runs in the default `cargo test`.

## D2 — The glTF JSON-chunk reader (`rowplay-app/src/replay/glb.rs`)

A minimal, bounded reader: parse the 12-byte header, locate the JSON chunk,
`serde_json` it into a `serde_json::Value`, then walk `nodes` / `meshes` /
`materials` / `extras`. It validates the V3 contract (R2.3) over the node
tree *as the contract defines it* — `replayAssetKind`,
`replayAssetTemplateSlot`, `replayAssetVersion`, `replayAssetPartCount`,
`replayMaterialRoles`, `replayAssetSlot`, `replayMaterialRole` — and returns
either a `TemplateManifest { version, templates: [ { template, part_count,
material_roles } ] }` plus the leaf-slot list, or an `AssetError` that names
the offending node path (`equipment:row:oar-rig/Shaft`, …). Finiteness of
accessor min/max for every mesh attribute is checked from the accessors
(cheaper and context-free; the Qt-side pass re-checks the loaded geometry).

The reader never loads buffers or images: it is a metadata validator. Bounds:
the JSON chunk size is read from the header and capped at the file length
(privacy-invariant style bounded scanning), files above 64 MiB are rejected
(the athlete GLB is 4.6 MiB).

The Qt-side loader (`assets.rs`) runs the *same* checks against the
`RuntimeLoader` node tree after status becomes Ready, using
`Node.name`/custom-property access that qtbridge exposes where available;
where qtbridge cannot read glTF extras from a loaded node (likely — see
notes), the JSON-chunk validation over the same bytes is authoritative and the
Qt-side pass asserts only presence of the named roots in the loaded scene.
This is recorded as a qtbridge note with a repro.

## D3 — Loader strategy (R2.2)

`build.rs` looks for `balsam` exactly like `rcc` (`ROWPLAY_BALSAM` override,
then `qmake -query QT_INSTALL_LIBEXECS/BINS/HOST_*`). Decision recorded at
build time into `OUT_DIR/replay_asset_mode.txt` (`balsam` or `runtime`), read
by `src/replay/assets.rs` via `include_str!` and exposed to QML as
`Replay.assetMode`.

- **runtime** (default, expected in CI): `RuntimeLoader` with `source` =
  `qrc:/qt/qml/RowPlay/replay/rowplay-rigs-v3.glb` (or the env-var directory
  in development). The GLB is listed in `qml/rowplay.qrc` under the
  `/qt/qml/RowPlay/replay` prefix (uncompressed; rcc `--no-compress` already
  applies to the whole blob).
- **balsam**: `balsam -o OUT_DIR/balsam assets/replay/rowplay-rigs-v3.glb`
  produces `*.mesh` + material/texture files; they are bundled through a
  generated qrc and `Model { mesh: Mesh { source: ... } }` replaces the
  RuntimeLoader per template root.

Rationale for expecting `runtime` in CI: balsam's output needs its own qrc
prefix and its material files reference paths that rcc must preserve; until a
CI leg proves the balsam leg end-to-end (screenshot diff identical), shipping
RuntimeLoader + GLB in rcc is the documented, tested path. The note records
the exact balsam invocation tried and its output.

## D4 — Materials (R3)

`rowplay_viewmodel::replay::materials`:

```rust
pub enum MaterialRole { AthleteSkin, AthleteFabric, AthleteHair, AthleteFootwear,
    AthleteShorts, AthleteTrim, AthleteEye, AthleteFaceDetail,
    EquipmentPainted, EquipmentDark, EquipmentLight, EquipmentMetal,
    EquipmentRubber, EquipmentGrip, EquipmentTrim }
```

with `ALL: [MaterialRole; 15]` in web order (the 11 V3 roles first),
`from_id(&str) -> Option<Self>`, `as_id() -> &'static str`.

`resolve_palette(theme: &ThemePalette, scheme: Scheme) -> RolePalette` maps
each role to a `ColorSpec { base: &'static str /*theme key*/, metalness: f32,
roughness: f32, opacity: f32 }`. The view-model owns the *theme key* indirection
(e.g. `EquipmentMetal → theme.accentMetal`) so `Theme.qml` stays the single
colour source: QML reads `Theme[role.themeKey]` for the base colour and the
view-model supplies only the PBR scalars plus the opacity (1.0 live;
`GHOST_OPACITY = 0.45` for ghost equipment; the athlete ghost stays opaque per
the V4 depth contract). Lane paint colours come from
`replay::theme::venues_*` (`lane`/`laneAlt` entries) — also via theme keys
where the shell palette has them, otherwise the view-model exports the venue
palette hex strings (documented divergence: the venue palette is data, not
shell chrome).

QML (`qml/RowPlay/Replay/ReplayMaterials.qml`) instantiates one
`PrincipledMaterial` per role in a `QtObject` map; meshes get their material by
role at load. Tests: the view-model test asserts every role resolves to a
theme key that exists in `Theme.qml`'s palette (scan the QML for the keys) and
that light/dark differ exactly where Theme says they do.

## D5 — Scene (R4)

`ReplayScene.qml`: `View3D` + `ExtendedSceneEnvironment {
toneMapping: Filmic; backgroundMode: SkyBox }` with
`lightProbe: Texture { textureData: ProceduralSkyTextureData { … } }` whose
zenith/horizon/ground/sun colours bind to `Replay.skyZenith` etc. — strings the
`Replay` singleton builds from `replay::theme::venues_{light,dark}(sport)`
through the view-model (`sky_palette(sport, dark) -> SkyPalette` with the
exact web palette field mapping: sky top = `sky`, horizon = `horizon`, ground
= `ground`, sun = `sun`). `DirectionalLight { castsShadow: true }` with the
web's art-directed world direction (5a: fixed direction from
`renderer3d.ts`'s sun vector, recorded in the view-model as data). Ground
plane: `Model { source: "#Rectangle" }` scaled to 400 m, base colour from the
palette ground entry.

The scene exposes one `Loader` per sport template set (Row / Ski / Bike) with
`active` driven by the gate walk and (in 5b) by the selected workout; in 5a
the gate activates each in turn. The athlete `RuntimeLoader` loads
`rowplay-athlete-v4.glb` (4.6 MiB in the rcc — acceptable; uncompressed) and
is shown standing at the origin, unposed.

## D6 — Gate and screenshots (R6)

Gate steps 54–59: for each sport, set `Replay.gateSport`, wait for
`Replay.loadState == "ready"`, grab `artifacts/replay-<sport>.png`; then
assert pixel diversity with the existing `smoke_screenshot.rs` helpers,
generalised into a shared `tests/pixels.rs` module. The broken-asset fixture
test (`asset_contract.rs`) feeds the validator a synthesised GLB JSON chunk
with one named defect at a time (missing root, wrong part count, role
mismatch, nested root, bad version, non-finite accessor) and asserts the
error message contains the offending slot name.

## D7 — What 5a deliberately does not do

- No playback, no posing, no contacts, no camera follow (5b).
- No quality tiers, no PerfGovernor wiring, no textures on the ground (5c).
- No venue geometry (Phase 6, ADR 0005).
- No new replay maths: anything numeric lives in `rowplay-core::replay`
  already or is authored data (anchors, palette keys) in the view-model.
