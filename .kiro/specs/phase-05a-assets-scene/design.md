# Phase 5a — Assets and scene: design

## Overview

5a is three independent layers glued by one backend singleton:

1. **Data** — vendored binaries under `assets/replay/` plus a Qt-free
   contract validator that reads the GLB JSON chunk directly
   (`rowplay_viewmodel::replay::glb`).
2. **View-model** — `rowplay_viewmodel::replay`: the material-role enum and
   its theme-key specs, the sky/ground/lane palette bridge to
   `replay::theme`, the key-light data (web sun offsets) and the anchor
   table.
3. **App** — `crates/rowplay-app/src/replay/` (the embedded asset metadata
   and the development re-validation), the `Replay` QML singleton that
   exposes load state, resolved colours and the key light, the
   `balsam`-generated `RowPlay.ReplayAssets` module built by `build.rs`, and
   `qml/RowPlay/Replay/` (scene, materials).

Dependency direction stays app → viewmodel → platform → core (ADR 0006). No
new crate and no new workspace dependency: the GLB reader is a small parser
in `rowplay_viewmodel::replay::glb` with no dependencies beyond `serde_json`
and `thiserror` (both already in the crate; here `serde_json` parses the glTF
JSON chunk only). The app's `build.rs` gains a build-dependency on the
workspace's own `rowplay-viewmodel` so the same validator runs at build time.

## D1 — Vendoring and hash pin

`tools/vendor-replay-assets.py` (Python, stdlib only, mirroring
`tools/vendor-fixtures.py`) copies the selected files from
`reference/rowplay/static/replay-assets/` into `assets/replay/`, computes
SHA-256 for each, and rewrites the vendored table in `ASSET_PROVENANCE.md`.
It refuses to run when `reference/` is absent and fails on any hash mismatch
against its embedded expectation table, so a refresh is an explicit act.
`--emit-rust` prints the table as Rust literals for the test below, so a hash
is never transcribed by hand.

`crates/rowplay-app/tests/asset_hashes.rs` holds the same expectation table as
Rust literals and hashes the committed bytes; a walk over `assets/replay/`
fails on any file the table does not know. It is Qt-free (pure file IO +
`rowplay_fixtures::sha256_hex`) so it runs in the default `cargo test`.

## D2 — The glTF JSON-chunk reader (`rowplay-viewmodel/src/replay/glb.rs`)

A minimal, bounded reader in the view-model crate — Qt-free, so it runs in
the default `cargo test` and inside the app's `build.rs`: parse the 12-byte
header, locate the JSON chunk, `serde_json` it into a `serde_json::Value`,
then walk `nodes` / `meshes` / `accessors` / `extras`. `validate_v3(bytes)`
validates the V3 contract (R2.3) over the node tree *as the contract defines
it* — `replayAssetKind`, `replayAssetTemplateSlot`, `replayAssetVersion`,
`replayAssetPartCount`, `replayMaterialRoles`, `replayAssetSlot`,
`replayMaterialRole` — and returns either a `V3Library { byte_length,
manifest: TemplateManifest { version, templates: [ { template, part_count,
material_roles } ] }, leaves, mesh_roles }` or an `AssetError` whose six
variants (`Container`, `Json`, `Hierarchy`, `Template { template, reason }`,
`Leaf { slot, reason }`, `Geometry { node, reason }`) name the offending
slot, template or node path. `mesh_roles` (glTF node name → role, template,
leaf slot) is what the runtime material walker indexes by `objectName` (D5);
node names are unique across the pack. Finiteness of accessor min/max for
every mesh attribute is checked from the accessors (cheaper and
context-free).

The reader never loads buffers or images: it is a metadata validator. Bounds:
files above 64 MiB (`MAX_GLB_BYTES`) are rejected before anything is parsed
(the athlete GLB is 4.6 MiB), the header's total length must equal the byte
slice, and every chunk length is checked against the container before the
chunk is read (privacy-invariant style bounded scanning).

Tests are unit tests in the file's `tests` module, not a separate
`tests/asset_contract.rs`: `a_valid_pack_validates` wraps a synthetic pack
(the seven roots with one part each plus the eighteen leaves) in a minimal
glTF binary; `the_vendored_rig_pack_validates` reads
`assets/replay/rowplay-rigs-v3.glb` and pins its byte length, seven
templates and eighteen leaves; `failures_name_the_offending_slot` mutates
the synthetic pack one defect at a time (missing root, part-count mismatch,
role mismatch, non-identity root, bad version, a part assigned to another
template, missing leaf, non-finite accessor bounds) and asserts each message
names the slot or template; `containers_are_bounded_and_typed` asserts a
non-glTF buffer is a `Container` error and an empty node list a `Hierarchy`
error.

The Qt-side pass does not walk a loaded node tree — a `RuntimeLoader` scene
is not addressable from QML and balsam drops the extras (D3) — and does not
need to: `build.rs` validates exactly the bytes it hands to `balsam`, so the
shipped components can only come from a pack that passed (a drift fails the
build with the named slot), and in development (`ROWPLAY_REPLAY_ASSETS`, or
the repository's `assets/replay/` in debug builds) `src/replay/assets.rs`
re-validates the on-disk pack at startup so an edited asset fails loudly with
its slot name. The `Replay` singleton reports which happened
(`validationMode` = "startup" or "build"); a startup failure sets
`loadState` = "error" with the message in `errorText`.

## D3 — Loader strategy (R2.2): balsam components in every build (ADR 0008)

`build.rs` finds `balsam` exactly like `rcc` (`ROWPLAY_BALSAM` override, then
`qmake -query QT_INSTALL_LIBEXECS/BINS/HOST_*`), and it is required: there is
no runtime fallback and no `replay_asset_mode.txt`. `build_replay_balsam`
runs `balsam --removeComponentAnimations -o OUT_DIR/replay-balsam/<pack>
assets/replay/<pack>.glb` for both packs, writes the `qmldir` of a generated
`RowPlay.ReplayAssets` module (`Rigs 1.0 rigs/Rowplay_rigs_v3.qml`,
`Athlete 1.0 athlete/Rowplay_athlete_v4.qml`) and bundles the components,
their `meshes/*.mesh` and the `qmldir` as a third `rcc --binary` blob
(`rowplay_replay.rcc`, every entry aliased under
`/qt/qml/RowPlay/ReplayAssets/` so the import URI resolves; registered in
`main.rs` next to the shell and i18n bundles). The GLBs themselves are not in
any rcc: `qml/rowplay.qrc` lists only the replay QML.

`build_replay_asset_meta` then validates the same bytes with `validate_v3`
and writes `OUT_DIR/replay_assets_meta.json` (`assetMode: "balsam"`,
`byteLength`, `templates`, `leaves`, `meshRoles`), read by
`src/replay/assets.rs` via `include_str!` and exposed to QML as
`Replay.assetMode`, `Replay.meshRoles` and `Replay.sceneNames` (the last two
are constants; only `meshRoles` is read by QML in 5a).

Why not `RuntimeLoader` (the plan in ADR 0003 and the first draft of this
design): `RuntimeLoader` exposes only `source`, `status`, `errorString`,
`bounds`, `instancing` and the supported-format lists; the imported nodes
carry no `objectName` and are not reachable through `children` from QML, so
neither the material-role walk of D4/D5 nor the 5b joint posing could
address anything in the loaded scene. balsam's components name every node
with its glTF name (`objectName`) and turn the V4 skin into
`Skin { joints: [...] }` whose 51 joints are named `Node`s (56 named `v4*` nodes in all), which is exactly what both
need. The costs are all handled at build time: balsam collapses each pack
onto one `PrincipledMaterial` placeholder (roles are re-applied from
`meshRoles` at runtime), drops the glTF `extras` (which is why the contract
is enforced on the bytes, not on the components), and would turn the V4
pack's three authored cycle clips into auto-running `QtQuick.Timeline`s
whose `.qad` keyframe files are not bundled — `--removeComponentAnimations`
strips them, because 5b evaluates the clips in Rust and Qt's animation
system must never drive the joints (the skin and the 56 joints survive; the
mesh bytes are identical). The repro, balsam's output shape and the
silent-hang failure mode of a broken QML import met while wiring the module
are in `docs/qt-bridges-notes.md`.

## D4 — Materials (R3)

`rowplay_viewmodel::replay::materials`:

```rust
pub enum MaterialRole { AthleteSkin, AthleteFabric, AthleteHair, AthleteFootwear,
    EquipmentPainted, EquipmentDark, EquipmentLight, EquipmentMetal,
    EquipmentRubber, EquipmentGrip, EquipmentTrim,
    AthleteShorts, AthleteTrim, AthleteEye, AthleteFaceDetail }
```

with `ALL_ROLES: [MaterialRole; 15]` in web order (the 11 V3 roles first,
also available as `V3_ROLES`), `from_id(&str) -> Option<Self>`,
`as_id() -> &'static str` and `is_v3()`.

`MaterialRole::spec() -> RoleSpec { theme_key: Option<&'static str>, venue:
bool, metalness: f32, roughness: f32 }` plus `RoleSpec::ghost_opacity(role)`.
The view-model owns the *theme key* indirection (e.g. `EquipmentMetal →
Theme.replayEquipmentMetal`) so `Theme.qml` stays the single colour source:
QML binds `Theme.<themeKey>` for the base colour and the view-model supplies
only the PBR scalars plus the opacity (1.0 live; `GHOST_EQUIPMENT_OPACITY =
0.45` for ghost equipment; the athlete ghost stays opaque per the V4 depth
contract). `equipment-painted` is the one `venue: true` role: its colour is
the lane paint from `replay::theme` (`Replay.livePaint` / `Replay.ghostPaint`,
D5) — documented divergence: the venue palette is data, not shell chrome.

The `Replay` singleton exports the table as `Replay.materialSpecs`
(`[{id, themeKey, venue, metalness, roughness, ghostOpacity}]`).
`ReplayScene.qml` declares the 15 `PrincipledMaterial`s as static children
inside the scene — a dynamically created material that nothing owns is
garbage-collected and its model drops out of the render, so the static form
needs no ownership discipline (docs/qt-bridges-notes.md) — with `baseColor`
the *binding* `Theme.<themeKey>` or `Replay.livePaint`, so a scheme switch
recolours the scene without rebuilding anything; `byRole[id]` is what
`ReplayScene.walkRigs` assigns to each mesh by `objectName` (D5).
`validateMaterials()` cross-checks every declared material's metalness and
roughness against `Replay.materialSpecs` at startup and takes the error path
on drift, keeping the Rust table the single source. 5a exports
`ghostOpacity` but instantiates no ghost material (5b). Tests in the
view-model: the ids round-trip in V3 order, every role has exactly one of a
theme key or the venue flag, every theme key exists in `Theme.qml` (scanning
the QML), and ghosts keep the athlete opaque.

## D5 — Scene (R4)

`ReplayScene.qml` is a `View3D` in metres (the web scene's unit) with:

- `ExtendedSceneEnvironment { backgroundMode: SkyBox; antialiasingMode: MSAA
  (High); tonemapMode: Filmic; exposure: 1.0 }` whose `lightProbe` is a bare
  `Texture`. Its `ProceduralSkyTextureData` is *rebuilt* rather than edited:
  a `Component` factory creates a new one — parented to the probe `Texture`
  so it lands in the 3D scene graph — whenever `skyKey` (the concatenation
  of `Replay.skyZenith/skyHorizon/skyGround/skySun/sunElevation/sunAzimuth`)
  changes, assigns it to `textureData` and destroys the previous one.
  Editing the colour properties in place leaves the uploaded texture, and so
  the sky box and the IBL, on the first sport's palette (Qt 6.11: the
  setters regenerate the RGBA16F data without marking the texture-data node
  dirty; only assigning a different `textureData` object re-uploads). The
  four colours are strings the `Replay` singleton resolves through
  `palette::sky_palette(sport, dark)` from `replay::theme::venues_{light,
  dark}` (zenith = `sky_top`, horizon = `sky_horizon`, ground = `ground_mid`,
  sun = `sun`); the ground plane tint is `ground_top`.
- A `PerspectiveCamera` framed statically per sport (rower at (3.4, 2.6, 9.6)
  with euler (-14, 20, 0)°, skierg (3.0, 2.0, 6.6) / (-12, 22, 0)°, bike
  (2.8, 1.8, 6.2) / (-10, 24, 0)°) with `clipNear: 0.1`. Qt's default near
  plane is 10 units, which in a metre scene clipped the SkiErg and BikeErg
  rigs (7–7.5 m from their cameras) and cut the ground off 10 m out; the web
  camera is `PerspectiveCamera(fov, 1, 0.1, 500)`. `clipFar` stays at Qt's
  10000 because the placeholder ground is 6 km across (divergence). The
  chase camera is 5b.
- The key light is the web's, as data: `palette::sun_offset(sport)` is
  `renderer3d.ts`'s `SUN_OFFSETS` (rower (-22, 18, 14), skierg (16, 28, 10),
  bike (12, 20, -10)) and `SHADOW_TARGET_HEIGHT = 0.55`. The scene puts a
  `LookAtNode` at `Replay.sunOffset` aimed at a `Node { y:
  Replay.shadowTargetHeight }`, with a child `DirectionalLight` (`color:
  Replay.skySun`, brightness 1.2, `castsShadow`, `ShadowMapQualityHigh`,
  `shadowFactor` 80). `LookAtNode` turns its forward (-Z) axis — the
  direction a `DirectionalLight` shines — at the target, so neither side
  does angle maths, and the shadow direction in the captures matches the
  web (rower sun at -X/+Z, shadows to +X/-Z). The procedural sky's
  `sunLatitude` / `sunLongitude` are derived from the same offset
  (`sun_elevation_degrees`, `sun_azimuth_degrees`, pinned by tests); the
  azimuth follows Qt's generator convention (`atan2(-x, -z)`) and is
  unverified in frame until 5b's chase camera shows the sun disc.
- Shadow parameters in metres: `shadowBias: 0.02`, `pcfFactor: 0.03`,
  `shadowMapFar: 60`, `csmNumSplits: 2`, and the ground `castsShadows:
  false`. Qt's defaults (bias 10, PCF 2.0, far 5000, and without cascades one
  map over the bounding box of every caster and receiver) assume a scene
  about a hundred times larger; untouched they mean a 10 m depth offset, a
  2 m blur and a map stretched over the 6 km ground, and no shadow
  survived. Two cascades follow the camera frustum out to 60 m and shadows
  render for boat, oars, skis, bike and athlete. This replaces the web's
  per-sport orthographic `SHADOW_FRAMES` envelopes (±5 to ±7 m) —
  divergence — and only the key light is ported: the procedural-sky IBL
  stands in for the hemisphere light, and the web's fill and rim lights are
  not ported.
- Ground: `Model { source: "#Rectangle" }` at y = 0, scaled 60 (6 km),
  `receivesShadows: true`, `baseColor: Replay.groundColor`, roughness 0.9.
- The V3 pack as the balsam `Rigs` component and the V4 athlete as `Athlete`
  (D3). `applySceneRules` → `walkRigs(rigs)` walks the component by
  `objectName`: every mesh in `Replay.meshRoles` gets
  `byRole[role]` (the scene's statically declared materials, D4); the 18 leaf shells are hidden (5b anchors
  them); each `equipment:<sport>:` template root is shown only for the
  current sport and the primary clone is placed statically at its README
  anchor (`Replay.anchors`: position plus yaw, radians from Rust turned into
  degrees at the boundary). The athlete is unposed (T-pose) and parked at
  (0, 0, -2.4) behind the equipment so the placeholder reads clearly; the 5b
  pose seats it on the rig. The sport comes from `Replay.sportIndex` (gate:
  `Replay.setSport(i)`; 5b: the selected workout) and the scheme from
  `Replay.setSchemeDark(Theme.dark)` in `Main.qml`.
- Load state: the components instantiate synchronously, so
  `Component.onCompleted` rebuilds the sky, applies the rules and calls
  `Replay.reportReady()` (`Replay.reportError(message)` is the failure
  path); `Replay.loadState` ("loading" |
  "ready" | "error") and `Replay.errorText` drive a 2D overlay
  (`replay.view3dLoading` / `replay.view3dError`) next to the `replay.back`
  button.

## D6 — Gate and screenshots (R6)

Gate steps 52–58 in `Main.qml`: step 52 pushes the route
(`Library.requestReplay(false)`), calls `Replay.setSport(0)` and holds the
walk until `Replay.loadState` is "ready" (an "error" is logged and the walk
continues; the wait is bounded at 60 ticks so a broken pack fails the pixel
assertion instead of hanging the gate); 53 grabs `replay-row`; 54 and 55
`Replay.setSport(1)` and grab `replay-ski`; 56 and 57 `Replay.setSport(2)`
and grab `replay-bike`; 58 `Library.closeReplay()` and clears the selection.
`grabScreen` saves each capture as `<ROWPLAY_SMOKE_SCREENSHOT_DIR>/<name>.png`
plus a PPM twin from the same `grabToImage` result, because uncompressed P6
parses without an image crate. `Replay` is in the member-probe registry
(`checkGateMembers`), so every `Replay.<member>` referenced from QML is
probed for `undefined`.

`crates/rowplay-app/tests/common/mod.rs` holds the pixel helpers shared by
`qml_runtime_gate.rs` and `smoke_screenshot.rs`: `parse_ppm` (moved out of
the smoke test, which still applies its own 256-colour and sky-blue checks),
`colour_diversity(pixels, step)` (distinct RGB colours and the share of the
most common one, sampled every `step`-th pixel) and `assert_rendered(width,
height, pixels, what)` (at least 320×200, at least 64 distinct colours, top
colour under 90 %). `replay_screenshots_render_per_sport` in the gate test
reads `replay-{row,ski,bike}.ppm` from `ROWPLAY_SMOKE_SCREENSHOT_DIR` and
runs `assert_rendered` on each; it skips, and says so, when the variable is
unset, since `offscreen` produces no frames. `Replay` joins the gate's
`SINGLETONS` list, the CI Linux leg (Xvfb + Mesa) uploads the three PNGs, and
the forbidden-pattern list gains "was not placed in the graphics scene" — the
message a Quick 3D object created with `createObject` under a 2D parent logs
(the rebuilt sky data of D5), which would otherwise pass silently.

The broken-asset defect tests are the `glb.rs` unit tests of D2
(`failures_name_the_offending_slot`, `containers_are_bounded_and_typed`), not
a separate integration fixture; `the_vendored_rig_pack_validates` exercises
every template root and leaf slot over the real pack (R6.3).

## D7 — What 5a deliberately does not do

- No playback, no posing, no contacts, no camera follow (5b).
- No quality tiers, no PerfGovernor wiring, no textures on the ground (5c).
- No venue geometry (Phase 6, ADR 0005).
- No new replay maths: anything numeric lives in `rowplay-core::replay`
  already or is authored data (anchors, palette keys) in the view-model.
