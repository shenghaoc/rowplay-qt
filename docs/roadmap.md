# rowplay-qt roadmap

rowplay-qt ports rowplay (SvelteKit + three.js, canonical) to a cross-platform
desktop app: Rust for all logic, Qt Bridges for Rust to QML, Qt Quick 3D for the
replay, Qt Graphs for charts (see `docs/decisions/`). rowplay-studio (Swift) is
the second reference for layering and golden parity fixtures.

Each phase lands as its own pull request from its own session. Phases 0 and 1
were delivered together as the bootstrap PR.

## Phase 0 — Bootstrap

Status: delivered (this PR).

- Toolchain verified and reported in the PR: `qmake -query QT_VERSION`,
  Quick 3D and `QtQuick3D.Helpers` imports resolve, `rustc` ≥ 1.87.
- Workspace skeleton: `crates/rowplay-core`, `crates/rowplay-platform`,
  `crates/rowplay-app`, `crates/rowplay-fixtures` (dev only), `qml/`,
  `assets/`, `i18n/`, `tools/`, `tests/fixtures/`, `docs/`, `.kiro/specs/`.
- Stack smoke test: a qtbridge app with a Rust backend object (`RowPlay.Smoke`,
  one `tick(dt)` slot per frame from `FrameAnimation`), a QML window and a
  `View3D` with one PBR sphere and one cube lit by the procedural-sky
  `lightProbe`, `ExtendedSceneEnvironment` filmic tonemapping and a
  shadow-casting `DirectionalLight`. Headless screenshot test
  (`crates/rowplay-app/tests/smoke_screenshot.rs`) runs under Xvfb with
  Mesa's OpenGL backend and checks the rendered pixels.
- CI (GitHub Actions): Linux fmt / clippy / test for the Qt-free crates; app
  build matrix on Ubuntu, macOS and Windows with `jurplel/install-qt-action`
  (Qt 6.11 with `qtquick3d qtshadertools qtquicktimeline qtgraphs`); the
  Linux app job also runs the screenshot test. No blocking docs-check job —
  docs are enforced in review (AGENTS.md P1 rule and the PR template).
  (2026-09-24: a docs-only pull request skips the app build and test steps
  and the step-529 job, while the app jobs still report their required
  checks, and a newer push cancels the run it supersedes; AGENTS.md,
  "Continuous integration".)
- Docs: README, LICENSE and `LICENSES/`, `ASSET_PROVENANCE.md`, ADRs
  0001–0006, `docs/source-map.md` (web → Swift → Rust),
  `docs/qt-bridges-notes.md`, `.kiro/specs/phase-00-bootstrap/` and
  `phase-01-core-parity/`, `AGENTS.md` (canonical; `CLAUDE.md` and
  `GEMINI.md` are `@AGENTS.md` shims).

Exit criteria: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test --workspace`, `git diff --check` pass; the app builds wherever Qt is
available and the smoke screenshot shows the lit scene.

## Phase 1 — Core parity foundation

Status: delivered (this PR).

Ported into `rowplay-core` with parity tests (web tests and Studio tests
re-expressed in Rust, plus the golden fixtures):

- Domain models and the `Sport` mapping (`models`).
- Formatting: time, pace, distance, BikeErg watts divisor, challenge distance,
  imperial units (`formatting`).
- Datetime and day keys (`datetime`), pace input (`pace_input`), privacy
  redaction and the share-link guard (`privacy`).
- Analytics (trend fit, distance and duration bands, per-sport summaries,
  dashboard derivations), personal bests (±2% tolerance), the performance
  predictor (Paul's Law), workout query (parse / serialise / filter / sort /
  chips) and workout tags (`analytics`, `personal_bests`,
  `performance_predictor`, `workout_query`, `workout_tag`).
- Deterministic demo library (`demo`), byte-for-byte the web app's
  `mockData.ts` generator.
- Studio's fixtures vendored into `tests/fixtures/` with a SHA-256 manifest and
  a Rust loader crate; replay and Concept2-mapper fixtures land now with
  `#[ignore]` tests naming the phase that enables them.

Exit criteria: every ported helper has tests; divergences from the web app or
Studio are documented in `docs/source-map.md`; no UI beyond the smoke window.

## Later phases (document only)

### Phase 2 — Replay core

Status: delivered (this PR).

Ported into `rowplay-core` as `rowplay_core::replay::*` with unit tests
re-expressed from the web and Studio suites:

- `engine` (`sample_at`, `sample_index_at`, tick-driven `ReplayState`),
  `motion` (`clamp_dt`, `damp_factor`, `warp_stroke_phase`, `stroke_surge`,
  `catch_events`, `ParticlePool`, `PerfGovernor`),
  `stroke_model` (timeline + pose, web pipeline plus Studio's frame-based
  path), `motion_graph` (full channel set, evaluation order identical to the
  web module), `sport_kinematics` + 2D palette `theme`,
  `comparability` + ghost pick, `race_gap` / `race_result` (interpolated
  finish crossing), rival CSV / TCX / FIT parsers with Studio's bounds and
  normalisation, and the quality budgets + degradation ladder.
- `warpStrokePhase` is **C1-continuous and periodic** at the drive/recovery
  seam and the cycle boundary via a per-segment cubic Hermite ramp with unit
  slope at every knot (`dw/du = 1` at the catch, the finish and both sides of
  the seam) — the web/Studio piecewise-linear map is only C0 and caused
  visible speed jumps on SkiErg. It is monotonic for every sanitised drive
  fraction, degenerates to the identity at `f = 0.5` like the web version,
  and `warp_stroke_phase_rate` exposes the analytic derivative with
  derivative-continuity tests guarding the seam.
- The six `#[ignore]`d parity tests are enabled and pass:
  `stroke-pose-parity.json`, `replay-race-gap-parity.json`,
  `replay-race-result-parity.json`, `replay-rival-sources-parity.json`,
  `replay-current-main-motion.json` (1e-10 across 387 samples × 60–78
  channels), `replay-current-main-2d.json` (1e-10 + exact palettes).
- Divergences are recorded in `docs/source-map.md`.

Exit criteria: the Qt-free workspace passes fmt / clippy / test; no UI
beyond the smoke window; every divergence documented.

### Phase 3 — Platform

Status: delivered (this PR).

- `rowplay-core::concept2`: the Logbook raw payload types and the mapper
  (`map_workout`, `map_strokes`, `map_splits`, heart rate, targets, metadata,
  split synthesis, detail assembly) behind a bounded byte-slice API — tenths of
  a second, decimetres, the BikeErg pace divisor and the interval `t`/`d`
  offset accumulation.
- `rowplay-platform::concept2`: the `ureq` (rustls) Concept2 client — HTTPS
  only (loopback excepted), bring-your-own-token, no cookies and no HTTP cache,
  30 s per request / 300 s overall, a 25 MiB body cap, at most three redirects
  followed by hand and only same-origin ones; cross-host redirects and
  HTTPS→HTTP downgrades are typed failures and the token never leaves its host.
- `rowplay-platform::token_store`: the `keyring` store with an explicit native
  backend per target (macOS Keychain, Windows Credential Manager, Linux Secret
  Service), a default-run test that fails if keyring falls back to its mock
  store, and an opt-in live round trip (`ROWPLAY_KEYRING_TESTS=1`).
- `rowplay-platform::workout_cache`: `SqliteWorkoutCache` on `rusqlite`
  (`bundled`) with Studio's schema, `PRAGMA user_version` migrations and
  `0700`/`0600` files, plus `paths` on the `directories` crate.
- `rowplay-platform::preferences`: the atomic JSON file store (temp file +
  rename), tolerant reads and never-fatal corrupt-file fallback.
- `rowplay-platform::sync`: the synchronous, cancellable
  `WorkoutSyncCoordinator` (page all summaries, then fetch and save details;
  per-item failures counted, 401/403/429 abort) and `SyncStateTracker`.
- The `#[ignore]`d Concept2 mapper parity test is enabled for all four fixtures
  in `tests/fixtures/Concept2/`; ADR 0007 records the library choices.

Exit criteria: the Qt-free workspace passes fmt / clippy / test; the mapper
parity fixtures pass; no default-run test touches the network or a credential
store; every divergence documented in `docs/source-map.md`.

### Phase 4 — QML shell

Status: delivered (PRs 4a foundation + 4b screens).

Foundation (4a):

- `rowplay-viewmodel`: the Qt-free UI-logic crate (navigation state port,
  six-language locale date display golden-tested against the web's `Intl`,
  settings options). Dependency direction becomes app → viewmodel → platform
  → core; the qtbridge objects in `rowplay-app` are thin adapters.
- The `Library`, `Detail`, `Settings` and `Sync` QML singletons under the
  `RowPlay` URI, the Fusion-styled application shell (split view, toolbar,
  empty state, shortcuts) and the settings screen: keyring token handling
  (QML sees only `hasToken`), units / home timezone / language preferences,
  demo-mode toggle, and sync on a `std::thread` worker (`mpsc` events,
  `QmlMethodInvoker` pokes, `AtomicBool` cancel, mock-client CI coverage).
- `Theme.qml`: Studio's `DesignTokens` / `DESIGN.md` ported with light and
  dark palettes following `Qt.styleHints.colorScheme`.
- i18n: `tools/convert-locales.mjs` converts the six web locales into
  ID-based `.ts` catalogues (message id = the web's dotted key, empty
  `<source>` so `qsTrId` resolves, English fill for the web's per-key
  fallback); `build.rs` runs `lrelease` and bundles `qml_<lang>.qm` into the
  rcc; `Tr.t(id, vars)` interpolates `{name}` like the web; `Qt.uiLanguage`
  drives live retranslation. Qt-free parity tests guard the key set, the
  placeholders and every `Tr.t` id used in QML. (No numerus forms: the web
  has no plural rules — superseding the original "plurals use Qt numerus
  forms" note.)
- The QML runtime-error gate: a smoke-mode walk over every screen and all six
  languages that fails CI on `TypeError` / `ReferenceError` / `Binding loop` /
  `Unable to assign` / `is not defined`, plus per-screen screenshot artifacts
  (the CI copies only exist since Phase 5a: until then the walk's grabs failed
  silently because the artifacts directory did not exist on a fresh checkout;
  the README's Phase 4 screenshots were always local captures).

Screens (4b):

- Library sidebar: the `Library` singleton as a qtbridge `QListModel`
  (`SidebarRowItem`, 13 roles, bulk `reset()`), day-sectioned rows with PB
  badges, sport / text / date-range filters, the Studio sort menu, and
  keyboard navigation (arrows select, Enter/Space activate, Escape clears).
- Dashboard: metric tiles, personal-best cards, and Qt Graphs panels
  (distance-by-sport bars, recent-pace line with Rust-rendered pace tick
  labels), all series loaded in bulk with `replace(list<point>)`.
- Workout detail: header, metric strip with semantic colours, the
  splits/intervals table (web `replay.th*` headers), targets read-out and
  comments.
- Stroke analysis: pace (negated, average rule, split boundaries), power
  (average-watts rule), stroke rate and heart rate over distance, with
  Studio's 500-point downsample and the stroke-less empty state.
- Performance: 5,000-workout synthetic library — filter + sort 2.5–7.7 ms
  (budget 50 ms), full display-string rebuild ≈ 66 ms, search debounced
  250 ms (`crates/rowplay-viewmodel/tests/perf_5k.rs`, runs in CI).
- Deferred and listed in the PRs: replay (Phase 5), live mode (Phase 8),
  HR import, annotations, comparison panel, file actions/export, rival
  controls; the reduce-motion toggle waits for the replay UI.

Exit criteria: every screen ports its Studio view with web-canonical values;
all six languages switch live; the runtime-error gate walks every screen in
CI on all three OSes; no metric is formatted in QML.

### Phase 5a — Replay assets and scene

Status: delivered (this PR).

- Vendored assets: `assets/replay/` holds the V3 rig pack
  (`rowplay-rigs-v3.glb`), the V4 athlete (`rowplay-athlete-v4.glb` plus its
  contract JSON) and the 13 Poly Haven environment texture families (three
  maps each) from rowplay commit `173c6fa…` — 42 hash-pinned files plus the
  environments `README.md` — with their `ASSET_PROVENANCE.md` rows,
  `tools/vendor-replay-assets.py` (`--emit-rust` regenerates the hash table)
  and `crates/rowplay-app/tests/asset_hashes.rs`, which pins path, byte
  count and SHA-256 and fails on any unpinned file.
- `rowplay_viewmodel::replay::glb`: a bounded glTF JSON-chunk reader and
  `validate_v3`, the port of `collectReplayAssetTemplateLibrary` (7 template
  roots, 18 leaf slots, 11 V3 material roles, identity transforms, part
  counts, declared-vs-actual roles, finite accessor bounds, size bound);
  every `AssetError` names the slot, template or node path, and
  `V3Library.mesh_roles` feeds the runtime material walker. Synthesised
  defects are unit-tested in the module.
- `rowplay_viewmodel::replay::{materials, palette, anchors}`: the 15
  `MaterialRole`s (11 V3 plus the four V4-only athlete roles), each with its
  `Theme.qml` token or the venue lane-paint flag, metalness / roughness and
  the 0.45 ghost opacity for equipment (a test scans `Theme.qml` for every
  key); the sky, ground, lane, marker and safety palette from
  `rowplay_core::replay::theme` plus the web's `SUN_OFFSETS` and
  `SHADOW_TARGET_HEIGHT` and the derived procedural-sky sun angles; the
  README anchor table as data.
- `build.rs` runs Qt's `balsam --removeComponentAnimations` on both packs in
  every build (ADR 0008), bundles the generated `RowPlay.ReplayAssets` module
  (`Rigs`, `Athlete`, meshes) as a third `rcc --binary` blob, validates the
  exact bytes converted with `validate_v3` (a drift fails the build with the
  named slot) and embeds the name → role map as JSON. The GLBs are not
  shipped; debug builds (or `ROWPLAY_REPLAY_ASSETS`) re-validate the on-disk
  pack at startup.
- The `Replay` QML singleton (constant asset / validation mode, material
  specs, mesh roles and anchors; notify load state, sport, colour scheme,
  sky and ground colours, lane paint, sun offset and angles) and
  `qml/RowPlay/Replay/`: `ReplayScene` declares one `PrincipledMaterial`
  per role as static children inside the scene, bound to `Theme` tokens and
  cross-checked against the Rust spec table at startup (a dynamic material
  needs a parent or a retained reference or it is garbage-collected — bridge
  notes; no colour literal in replay QML); the scene is a `View3D` with
  `ExtendedSceneEnvironment` (sky box, MSAA
  high, filmic), a procedural-sky light probe rebuilt on every palette
  change, a per-sport camera with `clipNear: 0.1`, the web's per-sport key
  light aimed through `LookAtNode` and casting two-cascade shadows tuned for
  a metre scene, a palette-tinted ground plane, the `Rigs` component
  re-materialled by `objectName` from the role map with the current sport's
  templates placed at their anchors, and the `Athlete` component unposed
  (T-pose) behind them on balsam's placeholder material over the pack's
  vertex colours (the V4 surface roles are applied when 5b poses it).
- Gate: steps 52–58 push the replay route, switch through the three sports
  and grab `replay-row` / `replay-ski` / `replay-bike`;
  `replay_screenshots_render_per_sport` asserts each capture is a real
  render (≥ 320×200, ≥ 64 distinct colours, top colour < 90 %); "was not
  placed in the graphics scene" is a forbidden pattern; CI uploads the three
  PNGs.

Exit criteria: fmt / clippy / test pass across the workspace; the gate
captures each sport's scene; the scene was run and looked at by hand under
Wayland (`cargo run -p rowplay-app`).

### Phase 5b — Replay playback ✓

- Transport (play / pause / seek / speed) over `replay::engine::ReplayState`
  in the `Replay` singleton, driven by one `FrameAnimation` → `tick(dt)` per
  frame; the whole frame crosses the bridge once as a flat `Vec<f32>` (225
  floats), with a 600-tick test that asserts exactly one emission per tick.
- The V4 athlete posed via `PoseSolver`: clip-time warping, translation-only
  pelvis alignment, two-bone IK contact pass on each limb with the V4
  contract's bone offsets, 19 semantic joints packed into the frame bundle.
  The ADR 0008 decision gate resolved: balsam components expose the skin
  joints by objectName with no C++ needed.
- Equipment from two `Rigs` balsam components (primary + mirror) for the
  README anchors: 2 oars + 2 blades, 2 skis + 6 pole parts, 2 wheels +
  drivetrain + frame + hull + seat. All leaf transforms (blade positions,
  pole-leaf positions) computed in Rust and packed into the frame.
- The web's chase camera (per-sport framing, speed FOV gain, damped smoothing)
  computed in Rust inside `tick`; HUD formatted in Rust only; reduce-motion
  wired from Settings.
- Gate: fixed equipment inventory per sport (6/8/4 from the assets README);
  shadow luminance margin (≥5%); colour diversity. Sun-disc azimuth
  confirmed correct for all three sports.
  - **The shadow check, replaced 2026-09-25.** From Phase 5c on, the
    captures it read were Medium, which casts no shadows, and once the
    design system hid the sidebar during the replay its centre sample
    landed on the athlete and hull. So it compared their albedo with the
    water's: it passed in light, and failed the dark scheme on Metal at
    4.9 % (AGENTS.md had pinned light for native runs). The gate now grabs
    the rower at High twice, as the tier sets the key light and with its
    shadow off, and the check measures the darkening against the same
    pixels unshadowed: at least 1 % of the capture must lose more than the
    noise delta, and those pixels more than 5 % of their luminance (Apple
    M5: 9.24 % by 10.99 % in light, 8.99 % by 19.08 % in dark). The dark
    scene's shadow is the stronger one. The same diff showed that at High
    the shadow darkens most of the SkiErg and BikeErg ground and none shows
    on the rowing water (#96).
  - **High and Ultra shadows, FIXED 2026-09-25 (#96).** Every venue mesh
    cast and received the key light's shadow: a Qt Quick 3D Model does both
    by default, three.js meshes neither, and the GLB bake carries no flags.
    So the BikeErg roof shell shadowed the whole track, the snow field
    shadowed itself into stripes, and the tower and pontoons shaded
    themselves while the rowing water showed no boat. The venue now takes
    the web's flags by mesh name (`venue_runtime::venue_shadow_flags`,
    checked against `tests/fixtures/replay-venue-shadow-parity.json`, which
    records the web's own objects), the ghost casts and receives nothing,
    and the live equipment only casts. Ultra's map is 2048 as the web's
    (both tiers drew 1024), and a 0.05 m bias with 32-bit depth and 16-sample
    PCF clears the speckle left on the athlete, the one object that still
    casts and receives. Share of the frame the shadow darkens, Apple M5,
    light, High, each sport's demo workout at 0:00: RowErg 8.91 → 1.34 %,
    SkiErg 51.84 → 0.62 %, BikeErg 61.74 → 1.26 %. The gate's rower check
    reads 1.62 % in light and 1.49 % in dark, over its 1 % floor; a 0.1 m
    bias also cleared the speckle but took the rower to 0.96 %. Low and
    Medium cast no shadow and did not change.

### Phase 5c — Quality tiers and polish ✓

- `TierSettings` resolver maps `RenderQuality` + sport to shadow enable/size,
  MSAA samples, environment texture sets (Low/Medium bind none; High
  diffuse + roughness; Ultra adds normals and the SkiErg timber terrace) and
  `QualityBudgets`. User-settable in Settings, persisted, per-tier texture
  gate assertion from the environments README.
- `PerfGovernor` fed `renderStats.frameTime` with outlier clamping (3×
  budget) and a payoff check: if a step-down doesn't materially improve the
  EMA (≥10% drop), the governor rolls back and locks. This prevents
  bottoming out on structural spikes that don't respond to tier changes.
- Ghost athlete + equipment on the ghost loop (26 m) with the ghost material
  variant (45% equipment opacity, athlete opaque). Race gap from
  `race_gap_metres`/`race_gap_seconds` and verdict from `race_result`, both
  worded with the web's locale ids (`replay.ahead`/`replay.behind`,
  `replay.raceVerdictWinSession`/`LoseSession`). *(The gap only since
  2026-09-25: until then it was English built in Rust, in every language;
  see the source map's "Race gap" divergence.)* Compact windows put the localized
  gap on its own wrapping row below two columns of metric chips, so longer
  translations and platform fonts cannot crowd them out; the native gate
  checks all six locales at 480 px.
- Reduce-motion toggle in Settings via a desktop-supplement locale key
  (`settings.reduceMotion`); the pipeline zeroes accents, snaps the camera
  and returns neutral poses.
- Frame times measured on Intel UHD 630 (Mesa 25.2.7, Wayland, 144 Hz,
  `QSG_NO_VSYNC=1`) via `renderStats.frameTime` with wall-clock cross-check.
  The UHD 630 renders the default tier (Medium) close to budget with a ghost
  present: p95 20.0–20.6 ms against the 22 ms governor budget. Ultra with a
  ghost exceeds budget at p95 (22.4 ms rower). The governor steps down once,
  sees no improvement (structural: ~6% of frames spike from doubled balsam
  geometry, confirmed by wall-clock at 42–54 per 720-frame run), rolls back
  and locks. The user stays at Ultra with occasional stalls rather than being
  dropped to Low. Reducing ghost draw calls or instancing the geometry is the
  path to fixing the stalls. *(Re-attributed 2026-09-24: the rate matches
  the diagnostics-driven scene walk, see UI follow-ups.)*
- Deferred: compare control UI (default ghost pick already selects
  automatically), rival file import (core parsers tested, the file dialog is
  UI plumbing for Phase 6).

### Phase 6 — Venues

**6a — the baking pipeline** (delivered, this PR):

- Feasibility checked first (ADR 0010): the web builder runs under plain Node
  — its only DOM references are the two guarded lines in
  `loadEnvironmentTexture`, and the web repo's own vitest suite already builds
  all three worlds under `environment: "node"`. No headless browser needed.
  Two Node mechanics were required: `--experimental-transform-types` (the
  builder class uses a constructor parameter property) and a resolve hook for
  its extensionless relative imports.
- Determinism: the builder's randomness is a module-scope unseeded
  `SimplexNoise`; pinning `Math.random` (mulberry32, seed `20260913`) before
  the module is imported fixes the permutation. A double bake is
  byte-identical, asserted on every run.
- `tools/bake-venues/`: one GLB per sport per tier (12 total; the geometry
  differs at every `environmentDetail` level) plus a contract JSON, all from
  one command (`tools/bake-venues/bake.sh`). Textures are not baked in —
  bindings are recorded in the contract, `repeat` is multiplied into the GLB
  UVs, and the procedural water/snow maps are encoded to PNG by the baker's
  own deterministic `node:zlib` encoder. `InstancedMesh` is split into an
  archetype mesh plus contract instance transforms, because balsam silently
  drops `EXT_mesh_gpu_instancing` and Blender expands it.
- Blender pass (`cleanup.py`, scripted, `--threads 1`, `PYTHONHASHSEED=0`):
  drop loose/degenerate geometry, consolidate the material copies Blender
  splits out (a shared material used by meshes that disagree about vertex
  colours), verify every name/material/UV/colour layer survives, re-export.
  No merge-by-material, no AO baking (ADR 0010).
- Vendored into `assets/replay/venues/` (28 files, 9.66 MiB) with a reviewed
  `MANIFEST.json`, SHA-256 pins in `tests/asset_hashes.rs`, rows in
  `ASSET_PROVENANCE.md` and a `README.md` naming convention. The whole
  `assets/replay/` tree measures 18.29 MiB, so ADR 0011 keeps plain Git and
  raises the ADR 0009 tripwire to 100 MB on the measured numbers.
- `rowplay_viewmodel::replay::venue::validate_venue` plus a `build.rs` drift
  gate: bounded read, all nodes/materials named, `environment:<sport>:`
  prefix, no embedded images, finite POSITION bounds, plausible world extent,
  and a contract inventory cross-check. Defect classes are unit-tested.

**6b — runtime** (delivered, this PR):

- All 12 venue variants convert with balsam into `RowPlay.ReplayAssets` and
  instantiate **statically** inside the `View3D`; exactly the (sport,
  effective tier) match is visible. Dynamically created 3D components
  (`Qt.createComponent` + `createObject` under a scene Node) build a complete,
  walkable object tree but never reach the rendered frame — found in 6b,
  recorded in qt-bridges-notes. Static instantiation is the rigs' own pattern
  and makes governor step-downs instant visibility flips.
- Each variant is walked once on first show: contract materials (light/dark
  base colour with live scheme re-tint, roughness/metalness, blend, unlit,
  clearcoat from balsam's placeholder, vertex colours, double-sided), the
  contract's instance groups as bucketed `InstanceList`s (4-shade quantisation
  of the web's scatterTint, no custom shaders; bucketing unit-tested against
  the vendored contracts), and tier-gated textures from a fourth rcc
  (`Environments`): none at Low, procedural-only at Medium, the sport's sets
  at High, plus normals at Ultra — exactly the 5c resolver.
- Gate: per-sport `replay venue` inventory lines asserted **exactly** against
  the vendored contracts; `replay venue FAILED` anywhere fails the walk; the
  5c per-tier texture assertion extended with per-tier venue binding counts;
  screenshots + shadow margin re-verified on GL.
- Measurements (Intel UHD 630, Mesa 25.2.7, Wayland, 144 Hz, `QSG_NO_VSYNC=1`,
  ghost present, debug binary — the 5c methodology and build, for
  comparability; `renderStats.frameTime` with wall-clock cross-check),
  median / p95 in ms and wall-clock >50 ms stalls per 720 frames:

  | Sport | Low | Medium | High | Ultra |
  | --- | --- | --- | --- | --- |
  | RowErg | 10.8 / 20.7 / 46 | 9.7 / 20.4 / 48 | 11.4 / 28.3 / 55 | 11.6 / 23.2 / 52 |
  | SkiErg | 11.6 / 20.5 / 46 | 11.4 / 21.1 / 47 | 12.1 / 28.2 / 53 | 12.2 / 26.9 / 52 |
  | BikeErg | 10.9 / 21.2 / 47 | 10.5 / 21.0 / 49 | 11.4 / 22.7 / 50 | 11.7 / 23.5 / 54 |

  All measurements in this project are taken on a **debug binary** — the
  gate and bench hooks (`ROWPLAY_SMOKE_GATE`, `ROWPLAY_REPLAY_BENCH`) are
  compiled out of release builds by design — which keeps the 6b table
  comparable with 5c's. The corollary: debug Rust is slower on the CPU side
  of `tick`, so the ~10–12 ms medians are partly an artifact and a release
  build should floor lower; a one-off release spot check (test hooks
  temporarily enabled locally) is recorded in `docs/qt-bridges-notes.md`.
  The finding: medians are identical across builds (the ~10–12 ms floor is
  real render cost, not a debug artifact).
  A p95 repeatability experiment (three additional identical debug bench
  runs, same binary/env/data dir, compared against the PR run and the
  release spot check — five runs total) pins down how much p95 can be
  trusted at 600 frames:
  - **Low and Medium: tight and uniformly under budget.** p95 across all
    five runs spans 17.9–21.3 ms per cell (identical-run spread ≤ 7 %, one
    low outlier aside); the worst sample anywhere is 21.3 ms. The default
    tier's **22 ms budget holds in every one of five independent runs, two
    builds** (medians ~9.7–11.9 ms — 5c measured 20.0–20.6 ms p95 without
    venues: the venue geometry is static and largely hidden behind the
    camera, and the ghost still dominates the frame cost).
  - **High and Ultra: p95 straddles the 22 ms line and is not resolvable at
    600 frames** — identical-run spreads of 23–38 % (e.g. bike-high p95
    21.0/21.0/22.8/29.4). Honest statement: High/Ultra p95 **typically lands
    at or above 22 ms** (ski-high and bike-ultra exceeded it in all runs;
    row-high, row-ultra, ski-ultra flip per run) with medians ~11–12 ms.
    Tail claims at these tiers need repeats or more frames; single-run p95
    there is noise.
  - **Debug vs release: no difference beyond this run variance** — every
    release p95 sits inside the debug repeats' range for its cell.
  Wall-clock stalls are 46–55 per 720 frames (6.4–7.6 %), the same
  structural band 5c measured (~6 %) — the venues did not multiply the stall
  rate. *(Re-attributed 2026-09-24: 720 / 15 = 48 is the diagnostics-driven
  scene walk's cadence, see UI follow-ups.)* Governor: at venue-era
  medians the sustained-over window never
  triggers, so no step-down (and hence no payoff-rollback) fires during the
  Ultra bench — verified by the absence of venue reloads (an effective-tier
  change would re-walk and re-log the venue); the governor's threshold unit
  tests are unchanged and green.
- **Re-measured post-Phase 7.5** (2026-09-20, this machine: Intel UHD 630,
  Mesa 25.2.7, native Wayland / GNOME 49.4, 144 Hz, `QSG_NO_VSYNC=1`, ghost
  present, debug binary at `origin/main` `05d6629`; two isolated 600-frame
  runs, medians in ms): Low/Medium still match the 6b table (9.7–12.2).
  **High/Ultra medians now run ~3–5 ms above the table**: row-high
  14.7/16.5, row-ultra 15.1/18.7, ski-high 15.2/16.4, ski-ultra 14.4/16.3,
  bike-high 14.9/16.6, bike-ultra 15.3/16.8 (table: 11.4–12.2). Both runs
  agree in direction and the between-run spread (~2–4 ms) is smaller than
  the gap; cause not diagnosed — the plausible candidate is the Phase 7/7.5
  rig work (dual rigs, hand layer, ski plant anchor), which landed without
  a re-bench (inferred). p95 stays unresolvable at 600 frames (row-ultra
  24.9/40.1 across the two runs); wall stalls 48–55 per 720, unchanged
  band. The 6b table above stands as the historical record for its build;
  current-main High/Ultra medians are ~15–17 ms, still under the 22 ms
  budget on medians, with p95 straddling it as recorded.

### Phase 7 — Motion

Status: in progress (spec + slices 1–2 landed).

The exploration finding (spec R0): the web's V4 is "clip-contact-constrained"
— the motion graph drives a procedural rig that provides contact targets, and
the V4 skin constrains onto them. The Rust port already matches that split
(`solve_rig_pose` consumes the parity-pinned graph and derives targets;
`PoseSolver` constrains the clip onto them), so Phase 7 completes the **hand
layer** the port leaves at clip identity rather than rewiring the athlete.

- Slice 1 (landed): `rowplay_core::replay::hand_grip` — the port of the web's
  `handGrip.ts` geometry-closure grip: fitted channel constants, hand-local
  digit chains, and `solve_hand_grip_closure` (staged bounded flexion with
  first-contact bisection, mid-range-touch bracketing, the thicker-than-span
  emerge search, thumb end-press with its own pad allowance, the `v4*Fingers`
  carrying cup, and the opt-in final-enclosure wrap search) with signed
  contact reports. `grip_closure_parity` is enabled and matches the vendored
  fixture exactly (3 sports × 2 hands, every pose and contact at 1e-9), with
  9 module invariant tests alongside.
- Slice 2 (landed): the equipment contracts — `rowplay_core::replay::{row,
  ski, bike}_equipment` plus `bike_saddle` (ports of the web's `rowRig.ts`,
  `skiEquipment.ts`, `bikeRig.js`, `bikeSaddle.js`: shell landmarks, scull /
  pole / hood contact geometry, the fitted bike derivation chain, the saddle
  station table and the oar-yaw / elbow / knee / saddle-drop / skier-elbow
  solves) — with the equipment parity half enabled at Studio's own 1e-12
  (253 samples green on the first run, including the skier kinematics through
  the already-pinned motion graph). Alongside it the wrist layer
  (`replay::wrist`: `orientHandToGripChannel`, spin/tilt relief, the
  swing–twist `constrainWristFrame` with the 75°/150° budgets and the SkiErg
  30° keep + 0.5 shoulder-share rules, 12 unit tests re-expressing the web
  orientation suite). Enabling the equipment fixture first caught one slice-1
  defect: `hand_palm_normal_out` returned the negated construction ray rather
  than the shipped rig's measured outward normal — corrected and pinned.
- Slice 3 (landed): the runtime hand layer. `PoseSolver::pose` takes the
  per-sport grip frames (`rowplay_viewmodel::replay::grip`, mirroring the
  three avatar layers: oar/pole/hood shaft, roll reference and base in
  rig-root space, plus the rower flat-wrist window from the stroke phase)
  and replaces the clip's authored wrist orientation per frame — channel
  alignment, sport refinements, swing–twist budgets with the forearm share —
  then re-closes the hands onto their targets; the two-bone solves and the
  residual/usability gates are unchanged and hold (worst 1.6 cm row,
  3.7 cm ski at reach extremes, inside the 9 cm budget). Ghosts ride the
  same path, so their hands match the player's. The backend solves the
  install-time finger table per sport (`Replay.gripPoses` + `gripContacts`),
  the scene applies it to the finger helper joints on the sport walk
  (player and ghost), and the gate requires `replay grip <sport>: 10/10
  digit contacts` per sport. *(2026-09-24: that held for SkiErg only because
  every sport logged the rower's table, #85. The web's closure gives the
  rower and the bike 10/10 and SkiErg 8/10, and the gate now requires each
  sport's count from the grip fixture.)* Probing the frames anatomically (shafts must
  run midline-ward) caught one sign bug the residuals could not see: the
  left scull shaft pointed outboard — the re-solve had hidden it by moving
  the elbow. Residuals cannot validate orientation; the shaft-direction
  test pins it instead.
- Slice 3 follow-up: the SkiErg elbow-seam excess now splits 50/50 into
  shoulder internal rotation like production (`distributeSkiElbowTwist`,
  forearm world bit-exact), and the wrist metrics carry the unclamped
  `requested_twist` beside the clamped value so T8 can tell a saturating
  budget from a frame bug. Measured verdict: no flips or wraps anywhere.
  The row finish→recovery swing whip first traced to the feather-window
  weights turned out to be those weights interacting with the inverted
  rower phase (below) — resolved by the phase fix, not by the wrist layer;
  what remains on it is the full-speed feather read.
- Rower phase fix (T8 finding): the first T8 capture review caught the
  rower rig stroking backwards against the equipment — Studio's phase
  calibration had been ported op-for-op while Studio itself is inverted
  against the web (seat farthest from the feet at the catch, catch grips
  behind the torso; the web source comments predict the exact symptom).
  Fixed fixture-first with a new web-generated fixture
  (`replay-row-phase-parity.json`, `tools/gen-row-phase-parity.mjs`) that
  pins the web avatar's seat/sweep/dip-roll mapping per cycle — the corpus
  had no phase coverage, which is how the inversion slipped through.
  Post-fix the row wrist demand sweeps −41°…+178° continuously (previously
  all-negative and clamped most of the cycle) and the feather-window whip
  halved. Capture tooling gained close-up twins
  (`ROWPLAY_PHASE_CLOSEUPS=1`, torso-and-hands framing) because the wide
  shots cannot resolve wrist detail.
- Remaining: per-stroke verification (T8, including the catch/finish
  visual on all three sports — now against the corrected phase); docs
  (T9).
- Post-audit upstream bump (reference `011e8303` → `173c6fa`, rowplay
  PRs #199/#200/#202): regenerated all three web-generated fixtures and
  adopted the two web fixes with recorded divergences (fr `emptyTrend`
  `{n}` placeholder, `distance_band` lowest-band nominal). PR #199's
  `v4HandTargets` surface closed the skierg contact-window coverage gap
  the audit had scoped — and its first consumption found the **seventh
  fixture-found defect**: the oar composition order (`qz ⊗ qy` vs the
  web's Euler-XYZ `qy ⊗ qz`), 0.181 m of grip error at the catch,
  invisible to residuals. Fixed with the pin bump in one change; the
  skierg contact/approach windows are measured (0.977 m / 0.033 m) and
  bounded, attributed to the documented `plant_basket_z` divergence's
  scoped follow-up — **promoted ahead of Phase 8** (author decision,
  Sept 2026): the rewrite is now Phase 7.5 in the queue order, the last
  known rendered defect in the shipped scene, double-confirmed by the
  plant position and the exposed hand target.

**Phase 7.5 — DONE (Sept 2026, PR #31).** The skierg pole-plant rewrite
landed: contact **0.977 m → 0.02 m** against the fixture's exposed
`v4HandTargets`, pole-tip oracle exact at the catch, recovery still
machine epsilon. The plant is fixed at catch in course space; the
carried tip is authored from the technique-phase attitude and rotated in
direction space to converge at contact (position and velocity
continuous at both boundaries). Continuity under the fixed plant needed
three companion port-side laws (the web's full `setArmBendHint`, measured-
once segments, the web's 6+4 orient/solve pass structure) — without them
the two-bone elbow branch flipped and the wrist frame oscillated. Two
web-inherited ±π atan2 wraps (tilt cyc 0.2635, spin cyc 0.7080) are
recorded, not fixed: the web snaps too, and the port's V4-style chain
amplifies the orientation snap into a ~0.11 m position jump
(`docs/parity-coverage.md` ranking 11 has the mechanism and the failed
bounded unwrap attempt). The skierg wrist's pre-clamp twist demand now
*saturates* at the 30° keep budget for the whole cycle (measured max
`|requested_twist|` = π/6 + ~1.4e-15 at step 222 — at the cap, not
inside with headroom) — the pre-rewrite "clamp engaged" assertion was
measuring the branch-flip defect, not coverage, and was converted to pin
the saturation invariant.

### Phase 8 — Live mode

**Status: in progress.** View-model cadence (`rowplay_viewmodel::live`),
platform page-1 poll (`rowplay_platform::live::poll_recent`), preferences
(`live_mode_enabled` / `live_interval_sec` / `live_sound_enabled`), the
`Live` QML singleton (`rowplay-app` backend, Sync-shaped worker + mpsc +
`QmlMethodInvoker`), and `LiveModePanel.qml` in Settings have landed. Qt
runtime / smoke verification is still owed on a machine with Qt 6.11.

Live mode is logbook polling, not hardware: rowplay reads the Concept2 Logbook
after upload and never connects to a PM5 (web README, repeated in its
limitations; Studio's `Connectivity/` is a mock-only boundary whose spec
excludes real Bluetooth, FTMS and PM protocols). No new subsystem — a timer
runs the Phase 3 pieces (Concept2 client, cache) against a short window: the
newest results page only, never the full history. It must **not** set or clear
`fully_synced` (that checkpoint means a full history walk). The poll is
`Concept2Client::list_results(1, 25)` (web `listRecentWorkouts`) and dedupes
by result **id**. The Concept2 list API has no documented ETag / conditional
GET surface, so polls are unconditional.

- Interval presets 30 / 60 / 120 / 300 s, default **60** (web `LIVE_INTERVALS`),
  minimum 30 per Concept2 rate guidance. Failed polls back off
  30 s → 60 s → 120 s → 300 s cap and retry automatically; three consecutive
  failures raise `liveMode.warning`. 401 stops live mode (`liveMode.reauth`);
  429 surfaces `liveMode.rateLimit` and honours `Retry-After` when present.
- The panel mirrors the web `LiveModePanel`: enable toggle, last-poll time
  (formatted in Rust), polling indicator, manual refresh, warning line.
- When a fresh result lands, `libraryRefreshRequested` reloads the library.
- Demo mode: HTTP is skipped for now (empty poll success) so the cadence/UI
  path stays exercisable; the web's `generateMockWorkout` (random 30 s–3 min
  delay + injected RNG) is deferred.

**Decision memo — visibility-aware polling (post-landing parity check).**
The web's live mode at the pinned commit (`173c6fa`) keys its cadence on tab
visibility: `effectiveIntervalSec` floors at 5 minutes while hidden, the
poller re-polls immediately (`scheduleNext(0)`) when the tab becomes
visible, and aborts the in-flight request when it hides. The port has
`effective_interval_sec` and a `tab_visible` field, but nothing ever sets
them from the window — polls run at the configured interval whether the
window is focused, visible or minimised, which is more aggressive than the
web whenever nobody is looking. Three options:

1. **Match the web.** Drive `tab_visible` from the window state and add
   focus/visibility reactions. The plumbing exists on the QML side
   (`ApplicationWindow` already exposes `active` and `visibility`; a
   `#[qslot]` like the existing `Live.tick()` would carry the change into
   the session — one new slot, no new bridge) and the view-model already
   applies the hidden floor and reschedules from it. Work: a window-state
   watcher in `Main.qml`, one slot on the `Live` backend, and a
   "reschedule now on becoming visible" path; plus tests that drive both
   transitions through `LiveSession`. Cost is small and contained to
   `rowplay-app` + `rowplay-viewmodel`.
2. **Reduce to refresh-on-focus.** Drop the interval timer entirely and
   poll once when the window gains focus. This deletes the 1-second
   cadence `Timer` in `Main.qml`, `LiveSession::on_tick`'s due-poll
   scheduling, `success_delay_ms`/`failure_delay_ms`'s interval math and
   the interval preference surface from PR #33 — the panel keeps only the
   enable toggle and a manual refresh. Cost: the backoff ladder and
   `Retry-After` handling lose their automatic retry (nothing retries
   while unfocused), and the interval UI the PR just built is deleted.
3. **Close PR #33** and record the divergence in `docs/source-map.md`.
   Cost: live mode does not exist on desktop; the parity gap moves from
   "diverges unattended" to "absent", which is an honest place to stand
   but removes the polling groundwork (page-1 poll, id dedupe, the
   `fully_synced` guard) that any future option 1 or 2 would rebuild.

Options 1 and 2 **largely converge**: the web's design is already mostly
focus-driven — an immediate poll when you look at it, and a 5-minute floor
when you don't — so matching canonical (option 1) already delivers most of
what dropping the timer (option 2) would deliver. Also note plainly: the
web ships live mode **off by default**, so no fixture covers it, no replay
maths depends on it, and it is the weakest kind of parity obligation.
Shenghao decides.

Related dead surface (recorded so green tests are not misread as shipped
behaviour): `effective_interval_sec` is only ever reached with
`tab_visible` pinned `true` (above); `failure_delay_ms` has no production
caller — `LiveSession::on_poll_err` inlines the same
`success_delay_ms + backoff` math, so the *behaviour* ships but the
wrapper does not; and `staleness_threshold_ms` has no production caller at
all — the app ships no staleness indicator. All three carry unit tests
pinning behaviour the app does not yet exercise; wiring or removing them
belongs to whichever option above is chosen.

What that dead surface means for the decision (post-review, 2026-09-20):
as shipped, this PR delivers a fixed-interval poller with an enable
toggle and a manual refresh — of the web feature's parts, visibility
awareness, the staleness indicator and (as a named function) the failure
delay are already absent. That strengthens option 2 rather than weakening
it: reducing to refresh-on-focus would delete a timer whose
interval-aware companions are partly dead already, so the trade is
smaller than the original memo implied. It is not a recommendation —
option 1's plumbing still exists and is still small — just the fact that
the "deletes working behaviour" cost of option 2 is overstated above.

### Phase 9 — Packaging

Status: delivered in the Phase 9 PR (ADR 0012, spec
`.kiro/specs/phase-09-packaging/`), ahead of the remaining parity-audit
rankings by the handover's priority call: a public port with a rigorous
audit and no installer ships nothing.

**Distribution policy (ADR 0014, 2026-09-24): Linux, macOS and Windows.**
When Phase 9 landed, the Linux AppImage was the only distributed
artifact, and macOS and Windows were built and launch-checked in CI only.
ADR 0014 supersedes that:
- all three platforms are distributed;
- macOS is checked on a Mac by a person;
- Windows ships verified by CI only (#81);
- signing and notarisation are open decisions for the author.

- One script per platform under `tools/package/` — `macos.sh`
  (`rowplay-qt.app` + `.dmg` via `macdeployqt`), `windows.ps1` (Inno Setup
  installer + portable zip via `windeployqt`), `linux.sh` (AppImage via
  pinned, SHA-256-verified `linuxdeploy` + Qt plugin, X11 and Wayland) —
  and one workflow, `release.yml`, that runs all three on pull requests
  touching a packaging input, on dispatch, and on `v*` tags, where it drafts
  a GitHub release. The release job still attaches the Linux AppImage and
  `SHA256SUMS` only, and doesn't check that the tagged commit passed CI.
  Attaching the macOS and Windows packages and gating the draft on the
  tagged commit's `CI` run, as ADR 0014 decides, are a workflow change
  awaiting the author's approval. Until then, releases are tagged only on
  commits whose `CI` run passed.
- Every package is launch-checked on its own runner: the deployed binary
  starts from a clean environment (no `PATH`, `DYLD_*`, `LD_LIBRARY_PATH`,
  `QT_*`), renders 30 frames under the new release-safe
  `ROWPLAY_EXIT_AFTER_FRAMES` probe and exits 0, or the build fails
  (`tools/package/launch-check.py`; proven to bite on two broken bundles).
- macOS binaries now carry their own `LC_RPATH`s from `build.rs`
  (qt-bridges-notes #10 resolved on our side); the `DYLD_FALLBACK_FRAMEWORK_PATH`
  workaround is gone from CI and the README.
- The icon is the web app's own, vendored with provenance and pinned;
  `.icns` / `.ico` are generated and pinned too.
- Recorded gaps (spec R6): the gate's pixel assertions run on the Linux leg
  only. The Linux-only policy had closed the macOS and Windows gaps as
  won't-do. Under ADR 0014 they read:
  - **macOS rendering:** verified natively on 2026-09-24 (#66).
  - **Windows rendering:** CI only, with no person to look at it (#81).
  - **Code signing and notarisation:** open decisions for the author (ADR
    0014 lists what each needs).
  - **The macOS `grabToImage` "black viewport"** (bridge note #17): it was
    the gate test's `offscreen` default. In a real window the same host
    captures the replay.

  Still open: Flatpak (deferred, ADR 0012), pruning the 215 MB macOS
  bundle's `QtQuick` QML tree, and macOS x86_64 / Linux aarch64 (qtbridge's
  support statement, note #8).

## UI follow-ups

Recorded 2026-09-20 from the author's first packaged-artifact inspection and
the review's keyboard walk; the author has said the UI work comes later. **No
code changes here — these are banked so they are not rediscovered.**

1. A workout with no per-stroke data loses its entire detail pane — no title,
   no date, no summary tiles, no charts — even though total time, distance and
   pace are workout-level facts that do not depend on strokes. The empty state
   should replace only the stroke-dependent charts.
2. The empty state's subtext, "No per-stroke sample at this time", suggests
   the condition is a current *time index* rather than the workout lacking
   data. Checked in source (ten-minute check): the state is gated on
   `!Detail.hasStrokes` (`qml/RowPlay/StrokeAnalysisPanel.qml`), a
   workout-level fact — the "at this time" wording is the web locale string's
   phrasing (`inspector.noStrokeData`), not a time-index mechanism. It is
   nonetheless the first thing seen on first launch, because nothing is
   selected yet and an empty `Detail` has no strokes.
3. Replay disables itself silently when a workout has no stroke data
   (`enabled: Detail.hasStrokeData && !Sync.isRunning`,
   `qml/RowPlay/DetailScreen.qml`): it greys out, drops out of tab order, and
   explains nothing. Same fact from the keyboard-walk side: no reachable
   Replay, no reason given.

**Outcome (2026-09-23, the UI stack's `ui/01-fixes`).** Items 1 and 2 had
one cause: the demo library selects its default workout while the
`Library` singleton is constructed, before `Main.qml` connects
`selectionChanged`, so `Detail` was never told and the first launch showed
an empty detail pane (see #55; a genuinely stroke-less workout keeps its
header and metric strip). The "at this time" subtext stays the web string.
Item 3 needs a web locale key first and is tracked in #64. The same pass
found and filed the shell's other standing defects, each with its own
issue: the stroke pace axis printed negated seconds (see #48) over crowded
ticks (#49), wide Y labels overflowed Qt Graphs' fixed 40 px column (#50),
the trend chart's date list was never filled (#51), split boundaries never
drew (#52), the PB cards never showed (#53), tabular figures were never
applied (#54), the dashboard tiles orphaned a tile (#56), scrolled content
stopped short of the pane (#57), the stroke section repeated the splits
heading (#58), the splits table numbered from 0 (#59), the gate's
`detail.png` was the settings screen (#60) and its grabs stored
translucent pixels (#61).

Recorded same day from the author's inspection of the packaged AppImage
(all three sports' replays opened, his report): not everything is ported
(he did not note what was missing at the time); loading feels slow; the
animation feels jerky at times. The jerkiness is not filed as polish —
two candidate causes, separated by a render-cadence measurement
(60 fps, 41 spm, rendered hand position, all three sports, 2026-09-20):

- **The issue #40 wrap discontinuities are user-visible on SkiErg —
  FIXED 2026-09-21.** The frame-to-frame hand snaps this bullet filed
  (0.19 m at cyc ≈ 0.26, 0.13 m at cyc ≈ 0.70) are gone: with the arm
  chain closing on the sport's grip channel by the web's own
  origin-aim law, those windows now measure 0.21 and **0.04** m/frame,
  against the web's own 0.28 and 0.04 at the same cadence. The dense
  guard's carve-out windows are deleted and replaced by a per-step
  comparison against a recorded oracle. The device profiles below (frame
  pacing, and the press ramp's legitimate 0.26 m/frame) are unchanged and
  still the co-cause of the reported jerkiness. The *rendered hand* no
  longer snaps; a ~11x-smaller residue still reaches the elbow at the
  snap (issues #44, #43).
- **Large per-frame motion also exists legitimately**, and is bigger
  than the snaps: the SkiErg press ramp peaks at 0.23 m/frame at 60 fps
  (sustained over ~8 frames — fast, not snapped), and the rower drive
  reaches 0.11 m/frame. Bike hands are static (0.0001 m/frame), so any
  bike jerkiness can only be frame pacing or camera.
- **Frame pacing remains a co-cause for every sport**: the re-measured
  High/Ultra medians (15.1/18.7 ms) sit below 60 Hz, so frames drop and
  double the apparent per-frame motion. The measurement cannot attribute
  the author's perception between the snaps (SkiErg-only) and pacing
  (all sports); both are real.
- **A periodic GUI-thread stall: FIXED 2026-09-24.** The governor's
  diagnostics string refreshed on every 15th rendered frame through the
  scene-wide `replayChanged` signal, whose QML handler re-ran the whole scene
  rule walk. Nothing displays that string (the 5c diagnostics strip was
  never built), so the walk was the refresh's only effect. The walk re-read
  the `Replay.meshRoles` JSON map for every node (qt-bridges-notes #18), so
  each refresh cost ~115 ms of GUI thread in a debug build. Measured on a
  4-core Linux VM (Xvfb + llvmpipe, rower 5000 m demo at a pinned Low tier,
  10 s windows, median of 3 runs):
  - playback before: 12 inter-frame gaps over 120 ms (12–14), p95 136 ms,
    18.9 fps;
  - playback after: none over 80 ms, p95 53 ms, 22.4 fps;
  - paused before: the scene redrew itself at 66 frames/s for 161 % CPU;
  - paused after: 0 frames, 0.7 % CPU.
  - on macOS (Apple M5, cocoa on Metal, 120 Hz display, debug build, 10 × 1 s
    with `top`), paused went from 44 % to 11 % CPU. The remaining 11 % was
    120 frames/s from the replay's tick animation, which ran while paused
    (#93, fixed below).

  Two side effects went with it. A paused replay no longer feeds the
  governor, so an idle scene cannot be stepped down any more (it could
  before, and did at 32 s on llvmpipe). That held under llvmpipe only until
  #93's fix: on macOS the paused scene kept rendering and feeding the
  governor. The gate's hold had relied on that
  redraw loop for its frames (qt-bridges-notes, the `grabToImage` entry):
  without it every settle ran out its tick bound and the walk took
  947.7 s; with the hold requesting its own frames it takes 327.8 s
  (310.5 s before; one run each). App startup also sheds the hidden
  replay scene's two walks: launch → first frame → exit went 1527 →
  1228 ms (median of 5, debug). The 5c/6b stall band (42–55 per 720
  frames) matches the old cadence (720 / 15 = 48): re-measure it on the
  UHD 630 before attributing what remains to the ghost geometry.
- **A paused replay rendered at the display rate on macOS: FIXED
  2026-09-25 (#93).** The replay's tick `FrameAnimation` ran whenever a
  workout was loaded, and on cocoa with Metal a running `FrameAnimation`
  keeps the window rendering at the display rate. Closing the replay
  neither paused nor unloaded it, so after the first replay every screen
  kept rendering too. The animation now runs only while the route is shown
  and playing, plus a six-frame settle after a change made while paused;
  each settle frame asks the View3D for a render (qt-bridges-notes, the
  `grabToImage` entry). Leaving the route pauses the replay, which reloads
  from the start anyway. Measured on an Apple M5 (macOS 27, 120 Hz display,
  debug build, rower 1001, 10 s windows; CPU from the process's CPU time
  under `top`, frames from `frameSwapped`):
  - paused: 120 frames/s and 12.1–14.4 % CPU before (n = 3), 0 frames and
    0.2–1.1 % after (n = 3);
  - the detail screen after closing the replay: 120 frames/s and
    18.6–19.0 % before (n = 2), 0–1 frames and 0.1–1.1 % after (n = 3);
  - a seek while paused: 7 frames (the change and the settle), then none;
  - playback: 118.9–120.0 frames/s, p95 inter-frame gap 9 ms and no gap
    over 80 ms, before (n = 3) and after (n = 6). The longest single gap was
    15–26 ms before and 14–55 ms after; the two over 26 ms (35 and 55 ms)
    did not recur in the three runs that logged where each gap fell, whose
    longest were 14–25 ms (the 25 ms one at the third frame after play).

  The fix also made three paused-state changes push their own frame, as a
  seek does: a ghost loaded or dismissed, and a new viewport aspect. The
  last one mends the first replay entry, which drew the frame computed
  while the scene still had the sidebar's width (879 px, the narrow
  framing) and never re-applied it after the scene widened: on macOS that
  first view showed the hull filling the screen until the first seek or
  play. Why a frame applied before that first resize draws wrong was not
  isolated. Playback that reaches the end now tells the transport, which
  kept showing pause. A paused scene no longer feeds the governor on any
  platform. The Codex review found two paused framing gaps, both fixed
  with a test each. A ghost loaded while paused was framed without the
  ghost: the camera read the ghost's position before the pipeline wrote
  it, so the load ran two passes (one since #97, whose camera places the
  ghost itself). A small aspect change stayed under the camera's 3 m snap
  distance and left a ghost pair too close, so a paused resize now places
  the camera afresh.
- **The ghost never moved: FIXED 2026-09-25 (#97).** The ghost's
  `ReplayState` was built at load and never played, so the ghost stayed on
  its start line and the race gap counted the player's whole distance
  (after 10 s of rower 1001 against 1002 the ghost read 9.6 m where its
  strokes say 48.4 m). Each pipeline pass now seeks it to the player's
  absolute stroke time, as the web samples it (`sampleAt(ghostStrokes,
  f.t)`); Studio's elapsed-time sampling, from the ghost's own first
  stroke, reads 49.7 m there, because the pair's first strokes differ.
  The chase camera places the ghost at that same instant rather than
  reading the previous pass's position, which a paused seek left stale.
  The verdict waits for the finish (the web's `raceFinished`); it had
  shown whenever playback was paused, on the start line too
  ("win|49.5|214" at 0:00). The native gate
  (Apple M5, light) shows the change where a ghost is loaded and nowhere
  else: `replay-ghost` loses the start-line verdict, the phase shots'
  close-ups their mid-race verdict, and the twelve chase-view phase shots
  now frame the pair at its real gap (the rower 96–97 m ahead, the skier
  9 m behind, the bike's shorter rival finished and holding its last
  stroke) instead of around a ghost on its start line; Studio's
  elapsed-time sampling had read 95 m and 12 m there.

Open question, connection not chased: the review's AT-SPI drive saw the
app **re-create its X window** on the Replay press and paint only after
a long delay, and the author reports slow loading — possibly one
phenomenon (the 3D scene rebuilt from scratch on entry). Worth one
look when the UI work starts. Tracked in #67 with the UI pass's own
observation (more than 12 s from the Replay press to the first frame under
Xvfb + llvmpipe); measured below.

**Replay entry has a budget (2026-09-24, issue #67).** The runtime-error
gate times its step 52 (`Library.requestReplay`, the rower) to the first
presented frame after it, prints the result with the walk's phase
durations, and warns in CI (a `::warning`, never a failure) above
**30 s under llvmpipe**. Measured on a 4-core Linux VM, Xvfb + llvmpipe,
one run per row:

| Case | Entry |
| --- | --- |
| First entry, debug, warm shader caches | 13.7 s |
| First entry, release (test hooks kept) | 13.4 s |
| First entry, debug, empty shader caches (every CI run) | 15.6 s |
| Sport switch, debug / release | 13.3–13.9 s / 12.2–13.0 s |

About 12 s of each is Quick 3D prefiltering the procedural sky into the
light probe's environment map, on the CPU rasteriser: the first frame's
swap takes 11955 ms with every llvmpipe thread in JIT-compiled shader code.
With the probe disabled (an experiment, not a change) first entry takes
1.9 s. Every sport switch rebuilds the sky and bakes again. Pre-baking the
probes would remove it and is ADR 0004 territory; the budget only makes a
regression visible. On a GPU the bake is a GPU job, so the llvmpipe
number says nothing about hardware. The hardware budget is still to be set
from a measurement on the author's machines: run the same gate with a real
window (`QT_QPA_PLATFORM=cocoa` on macOS, `wayland` on Linux, as in the
test commands in AGENTS.md) and `-- --nocapture`, and read its
"replay entry" line.

## UI design system (ADR 0013)

One visual design system of our own, the same on Linux, macOS and Windows,
plus a thin platform-behaviour layer. It supersedes the unmerged Apple-HIG
pass (#47). The spec is `.kiro/specs/ui-design-system/`, delivered as a
stack of seven PRs, each green on its own:

1. `ui/01-fixes`: the pre-existing defects above, one issue each.
2. `ui/02-foundation`: the Basic style; the tokens (neutral ramps that pass
   WCAG AA, every length and font a ratio of the system font, the system
   accent with a brand-blue fallback, a high-contrast variant); the shared
   controls and themed popups. Screens keep their stock controls,
   coloured by the palette, until their own PR.
   - Delivered in #69 and validated there: the full suite under CI's Linux
     recipe, the controls rendered light, dark and high contrast at 100 %,
     125 % and 150 % text, and the app's captures before and after.
3. `ui/03-shell`: sidebar, toolbar, the platform layer (shortcuts, menus,
   sidebar toggle) and the empty state.
   - Delivered in #70 and validated there: the full suite, the captures
     before and after, the platform layer driven with xdotool, and the
     AppImage's launch check.
   - Codex review of #70: a keyboard press hid the next hover's tooltip,
     settings opened over the replay left it presented, and very large text
     could squeeze the sport filter to 0 px. Each is fixed in its layer
     (the filter's floor is a 2/7 control property) and recorded in T3.7.
   - Second-reviewer pass: half-pixel centring, the sort direction for
     screen readers and the list's focus cue off screen (T3.8).
4. `ui/04-dashboard-detail`: tiles, charts, metric grid and splits table.
   - Delivered in #71 and validated there: the full suite and the captures
     before and after, scrolled views included.
   - Second-reviewer pass: Replay's clipped focus ring and scroll-bar
     overlap, and the splits card's jump on hovering its scroll bar (T4.6).
5. `ui/05-settings`: the grouped settings page.
   - Delivered in #72 and validated there: the full suite, the captures
     before and after, and the whole-row and direct switch clicks driven
     with xdotool.
6. `ui/06-replay`: the floating HUD with auto-hide, and the sidebar hidden
   during the replay.
   - Delivered in #73 and validated there: the full suite, the captures
     before and after, and the auto-hide cycle and the sidebar's return
     driven with xdotool in both schemes.
   - Second-reviewer pass: the scrubber's half-pixel centring, and Space on
     the focused Close button (fixed in 2/7's `ToolbarButton`) (T6.7).
7. `ui/07-docs`: the README screenshots and the final documentation pass.

**Outcome (2026-09-23, the stack as opened).** PRs #68–#74, in that order;
each builds and passes the full suite on its own, and each carries before /
after captures in both schemes.

- **Delivered.** The Basic style drawn from our own tokens: neutral ramps,
  the PM5 metric colours kept, and every figure measured by the QML runtime
  and truncated (text ≥ 4.78:1, metric colours ≥ 4.53:1 on the grouped
  surface). Lengths and type follow the system font, verified at 125 % and
  150 %. The system accent is used where the platform has one, with a
  high-contrast variant. The shared controls replace every stock one with a
  look of its own. The shell has a full-height sidebar, an icon-only
  toolbar, StandardKey shortcuts, the native macOS menu bar (a toolbar menu
  on Windows and Linux) and an About dialog built from existing strings.
  The dashboard and detail sit in balanced card grids in sentence case, and
  settings is a grouped page. The replay's controls float over the scene
  and hide while it plays, with the sidebar hidden.
- **Found on the way.**
  - The PM5 duration colour measured 4.496:1 on the first light grouped
    surface, printed as 4.50 (2/7 moved the surface one step).
  - Qt Quick sends a synthetic hover after every animated frame, which kept
    the first auto-hide build from ever hiding the HUD.
  - `Timer.restart()` overrides a `running` binding, which let the HUD
    hide while paused or idle (found in review, confirmed by driving it).
  - Large text in a narrow window broke layouts that the default size
    never showed, found at 150 % text in the 1000 px minimum window. The
    toolbar filter ran under its neighbours, the splits table elided
    values, settings rows covered their labels, and the sync buttons ran
    out of their group. Each is fixed in its own layer: the segmented
    control's compact form and the stacking row in 2/7, the toolbar in
    3/7, the table in 4/7 and the settings page in 5/7.
  - `FontMetrics.advanceWidth()` registers no binding dependency, so a
    measure taken before its font landed kept the default font. A
    default-size capture comparison caught it in 5/7's settings (the
    quality control drew 308 px wide instead of 292), and the fix also
    moved 4/7's dashboard pace chart 19 px left: its label reserve had
    been measured at 12 px while the labels draw at 9. Every such binding
    now reads its metrics' font (2/7, and the table in 4/7), so the widths
    also follow a live change of the system font.
  - Qt Quick Layouts round each item's width up to a whole pixel, so the
    splits table's fractional column shares clipped its last column
    wherever the table fits, the default size included; the columns are
    whole pixels now (4/7). A `Flow` does not snap at all, which put a
    sync button off the pixel grid (5/7 uses a grid).
  - The Codex reviews of #68 and #70 (2026-09-24, after the stack was
    opened) raised five findings, all confirmed, the code ones reproduced
    before their fixes:
    - the PB time's inline font still named `Font.TabularNumbers`, and
      the Phase 4 design still prescribed it (1/7);
    - a keyboard press hid the next hover's tooltip (3/7);
    - settings opened over the replay left it presented. 6/7 had fixed
      its own paths only; the fix moved down to 3/7;
    - very large text could squeeze the toolbar's sport filter to 0 px,
      where the segmented control shows nothing (3/7). Its floor
      (`compactWidth`, 2/7) showed that `ComboBox.WidestText` measures
      only a `TextInput`, so the compact form had been sized by its
      background alone.

  - A second-reviewer pass (2026-09-24) found ten more defects, each
    reproduced before its fix, in the layer that owns it:
    - 2/7: a checkable toolbar button lost its toggle role; a window
      shortcut took Space from a focused button; the pop-up list indented
      its current row; scroll bars vanished into Basic's track under the OS
      contrast preference; the progress segment froze off-centre under
      reduce motion;
    - 3/7: the sort direction was invisible to screen readers; the focused
      list showed nothing with its selection off screen;
    - 4/7: Replay's ring was clipped and its edge under the scroll bar; the
      splits card jumped when its scroll bar was hovered;
    - 2/7, 3/7 and 6/7: centred parts sat on half pixels.

    It also filed what lies outside the stack (#76–#80). The macOS menu's
    role titles turned out to be Qt's English "Preferences…" (#80).

  The Qt findings are in `docs/qt-bridges-notes.md`.
- **Verified where.**
  - **Linux**, locally, when the stack was opened: Xvfb + Mesa llvmpipe,
    light, dark and high contrast, 125 % and 150 % text in the minimum
    window with the longest labels (Spanish), driven with xdotool, with
    pixel statistics read before any visual reading. Since then CI only.
  - **macOS**, natively on 2026-09-24 (Apple M5, macOS 27, Qt 6.11.2; #66):
    - the gate walk in a real window, light, dark, forced high contrast
      and Spanish, the 3D replay included;
    - the chords, accent, contrast preference, dialog order and system font
      read from Qt;
    - the package built and launch-checked with `tools/package/macos.sh`.

    #66 lists what still needs a person there: pointer and keyboard
    driving, the OS accent and contrast switches, and VoiceOver.
  - **Windows** ran the gate walk offscreen in CI; nobody has looked at it
    (#81).
- **Open.** The replay's disabled state explains nothing until the web has
  a string for it (#64). Two labels inherit web wording that suits the web
  better (#65). Replay entry is slow and not diagnosed (#67).

### Round 2 — Windows, KDE and HarmonyOS guidance

Rules the Apple and GNOME guidelines did not cover, from Microsoft's
Windows and Fluent guidance (WinUI as reference), the KDE HIG and
HarmonyOS's width breakpoints, each checked against the code first. A
stack of five PRs merged bottom-up (spec, "Round 2"):

1. `ui/focus-ring-high-contrast`: the focus ring is two-tone like Windows'
   focus visual (a 1 px band in the window colour right outside the
   control, a 2 px band in the accent outside that), through the one
   `FocusRing`. Under the OS contrast preference every colour role comes
   from the system palette's pairs, as Windows' contrast themes require,
   instead of our strengthened ramps; metric colours stay only where they
   reach 4.5:1, and surfaces that turn the same colour get 2 px outlines.
   - Checked natively on macOS (Apple M5): twelve real controls given Tab
     focus by a scratch probe, in light, dark and forced high contrast in
     both; the ring's pixel runs read the two bands. macOS's palette has
     alpha text roles and a selection highlight at 1.64:1 on the window,
     so the prominent button and an on switch keep their outline there.
   - Not measured: macOS under "Increase contrast" (a system setting, for
     the owner; #66) and Windows' contrast themes (#81).
2. `ui/min-text-cjk`: a text floor under the whole type scale, 12 px on
   Windows and Linux (Windows' minimum for body text), 11 px on macOS and
   12 px in Chinese and Japanese; a caption the floor lifts to the body's
   size keeps its place by regular weight and the secondary colour.
   - A scratch probe listed every elided or overflowing text on four
     screens in English, Chinese, Japanese and Spanish, natively and at
     12 px and KDE's 14 pt test font (18 px) on the offscreen platform.
     The floor made Chinese and Japanese sidebar rows elide their
     distance, which now moves under the date where the two do not fit.
   - Found on the way, older than the floor: the trend chart labelled
     every point, and its dates overlapped in Japanese at the default size
     and in English at 18 px. It labels every n-th point now.
3. `ui/breakpoints-min-window`: HarmonyOS's width breakpoints, scaled with
   the text: compact below 600 px, medium below 840 px, large from there.
   Large keeps the layout as it was. Below it the one sidebar moves into
   a drawer, modal over the content at medium, the list's own page under
   the toolbar at compact. A leading toolbar button named "Workouts" (an
   existing web key) opens it, as do the sidebar toggle and Find, and the
   sport filter takes its compact form. Grids, the detail's rate and
   heart-rate charts and the replay HUD's controls stack where they do
   not fit. The minimum window drops from 1000 × 680 to 480 × 480.
   - A scratch probe walked five screens at the three widths in English,
     Spanish and Japanese, light and dark, natively and at 150 % text:
     nothing elided or past the window's edge beyond T9's known chart
     labels. A second one pressed real keys and clicks through QtTest's
     `TestEvent`. The gate now walks medium and compact.
   - Two Qt 6.11 behaviours shaped the drawer (docs/qt-bridges-notes.md):
     every window shortcut outside a modal popup, or one that Escape
     closes, is blocked, so the compact page is neither and the toolbar
     and the shortcuts stay live there; and a non-interactive popup
     ignores Escape and the click outside, so the drawer stays
     interactive and turns edge drags off with `dragMargin: 0`.
4. `ui/shortcut-tooltips-timezone-errors`: toolbar tooltips name their
   shortcut in the platform's notation ("Reload (⌘R)"); the timezone
   picker filters as you type, in the view-model; a failed sync is
   labelled "Sync failed" with "Retry sync" in its row; a refused date
   marks its own field, with the form it takes under it; screen-reader
   names lead with what tells an item apart (a sidebar row's title, a
   split's number), and the detail header's name lost an English
   "intervals". The errors and statuses were audited path by path, and
   the application menu holds no Quit or window items (it never did).
   Access keys are assessed in #104, not implemented.
5. `docs/design-system-round-2`: ADR 0013's round-2 notes (what was not
   adopted, and why: Mica and acrylic, Kirigami and qqc2-desktop-style,
   HarmonyOS Sans and the Huawei visual language), the AGENTS.md style
   lines, and the documentation pass.
   - Plasma, from the sources: the accent follows the colour scheme,
     because KDE's platform theme sets `QPalette::Accent`, but no KDE
     theme reports a contrast preference, so a high-contrast Plasma
     scheme does not engage the app's high-contrast variant. Not tried on
     a Plasma desktop.
   - Still open: macOS under "Increase contrast" (the owner's switch),
     and Windows' contrast themes, focus ring, Snap layouts and text
     sizes (#81, where the round's checklist additions are posted).

## Native Qt styles (ADR 0015)

The owner redirected the design system on 2026-09-25: standard controls
become Qt's own, per platform, and the product's identity lives in its
content (the charts, metric colours, tiles and personal-best cards, the
splits table, the replay and its HUD). ADR 0015 supersedes ADR 0013's
control layer; the spec is `.kiro/specs/ui-native-styles/`. A stack of pull
requests on the round-2 stack's composed top:

1. `ui/native-style-switch`: no style is forced any more, so Qt 6.11.2
   picks macOS, Windows or Fusion (confirmed in `qquickstyle.cpp`;
   FluentWinUI3 is opt-in only), and `Main.qml` sets no palette role. The
   shared controls are pinned to Basic until their area is replaced.
   `Theme`'s surfaces, text and lines now come from the system palette,
   fitted to the contrast floors, and the metric colours are fitted on
   their surfaces: the PM5 blue measured 4.1:1 on the macOS dark window's
   cards. The macOS `ScrollView` reserves room for a scroll bar that is not
   transient, so the replaced Basic scroll bars looped `contentWidth`; the
   screens use the style's own. CI gains informational Windows walks in a
   real window for the Windows style and FluentWinUI3.
   - Packaging, read from the release workflow's logs: the macOS bundle
     and the Windows package carry their native styles; the AppImage
     carried Fusion but no Qt Svg plugin and only the desktop portal's
     platform theme. The Qt Svg image plugin is added in layer 2.
2. `ui/native-shell`: the shell's controls are the style's. The toolbar's
   buttons show SF Symbols through Qt's Apple icon engine on macOS, Segoe
   glyphs through its Windows engine, and our original SVG glyphs on Linux
   (decoded by the AppImage's Qt Svg plugin); the sport filter is a
   `ComboBox`, the Windows / Linux menu a
   `Menu`, the sidebar's drawer a `Drawer` and About a `Dialog`. Qt asks
   the platform icon engine only while an icon has no source, so a first
   version with both showed our SVGs on macOS. Linux uses its SVG source
   directly because a nonempty source bypasses the freedesktop name.
3. `ui/native-sidebar`: the sidebar's fields are the style's text fields,
   the sort menu its menu, and the rows its item delegates carrying the
   workout's data, with the list's own day sections. Every workout carries
   its day's grouping label, so a second workout cannot create a blank
   section. The selection uses the style's highlight while the list has
   the keyboard and a neutral wash in the row content otherwise, without
   overriding palette roles. The date fields lose round 2's alert outline:
   the style's field has no error state, and the message under it marks
   it.
4. `ui/native-settings`: the settings page is the style's group boxes,
   switches, pop-up buttons, fields, buttons, progress bar and dialog. A
   switch carries no text of its own: the styles elide it (Spanish at the
   minimum width lost the end of the live-mode switch's), so the label
   wraps beside the switch and toggles it. The timezone picker keeps its
   filter, as a field above the pop-up of matches. The live-mode spinner
   stays ours: the macOS style's is a WebP animation, and no Qt install of
   the project carries the WebP plugin (proposed to the owner).
5. `ui/native-detail`: the detail's Replay is the style's highlighted
   button, macOS's default button (the accent only in an active window, as
   every default button there). The detail and the dashboard keep their
   margins inside their scroll views, like settings, so the style's scroll
   bar sits at the pane's edge. Neither page has another control.
6. `ui/native-replay-hud`: the HUD's play button and speed buttons are
   the style's (the toolbar's command button, and checkable tool buttons in
   an exclusive group with one tab stop). The scrubber stays the HUD's own:
   the macOS style's slider hides its track beyond the knob on the HUD's
   grey. Every HUD control answers within at least 40 px without moving
   (round 3's 2f), with no two hit areas overlapping under the macOS style
   or Fusion. The speed and chips now stack against the HUD column's
   width: Spanish at 480 px overflowed once the speed could no longer
   shrink. ADR 0015 is corrected: controls the native styles lack come
   from Basic, not Fusion (measured with Qt's own `qml` runner).
7. `ui/native-theme-cleanup`: the seventeen shared controls nothing uses
   any more are deleted, and `Theme` keeps content tokens only (27 members
   gone). Round 3's motion tokens (2e) replace the last ad hoc durations:
   three durations and M3's two curves, all instant under reduce motion.
   Two Basic imports stay by decision: the HUD's scrubber (content) and
   the live-mode spinner (until the WebP plugin ships). AGENTS.md's QML
   rules now say to use native Qt Quick Controls as-is.
8. `ui/native-large-layouts`: round 3's 2d. `Theme.widthClass` has M3's
   five classes, adding large (≥ 1200) and extra-large (≥ 1600). In a
   large window whose own column fits two panes, the detail opens M3's
   supporting pane (the stroke charts beside the summary and splits) and
   the dashboard its feed (the charts side by side). Tiles stop at a
   maximum width once they fit one row, the content stops at 1440 px
   centred, and running text at 640 px. Checked at 1200, 1440, 1600 and
   1920 px in English and Japanese, light and dark.
