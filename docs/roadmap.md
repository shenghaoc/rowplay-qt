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
  `race_gap_metres`/`race_gap_seconds`, verdict from `race_result` using the
  web's locale ids (`replay.raceVerdictWinSession`/`LoseSession`).
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
  path to fixing the stalls.
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
  rate. Governor: at venue-era medians the sustained-over window never
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
  digit contacts` per sport. Probing the frames anatomically (shafts must
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

**Distribution policy: Linux AppImage is the distributed artifact.** macOS
and Windows are built and launch-checked in CI on every relevant push to
keep the port honestly cross-platform, but they are not distributed, not
rendering-verified and not signed/notarised.

- One script per platform under `tools/package/` — `macos.sh`
  (`rowplay-qt.app` + `.dmg` via `macdeployqt`), `windows.ps1` (Inno Setup
  installer + portable zip via `windeployqt`), `linux.sh` (AppImage via
  pinned, SHA-256-verified `linuxdeploy` + Qt plugin, X11 and Wayland) —
  and one workflow, `release.yml`, that runs all three on pull requests
  touching a packaging input, on dispatch, and on `v*` tags, where it drafts
  a GitHub release with the Linux AppImage and `SHA256SUMS` only.
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
  only. The gaps that follow from the Linux-only distribution policy are
  closed as **won't-do** (the author ships on Linux; macOS and Windows exist
  to keep the port cross-platform): macOS/Windows rendering verification,
  code signing, notarisation and the macOS `grabToImage` black viewport
  (bridge note #17). Still open: Flatpak (deferred, ADR 0012), pruning the
  215 MB macOS bundle's `QtQuick` QML tree, and macOS x86_64 / Linux aarch64
  (qtbridge's support statement, note #8).

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

Open question, connection not chased: the review's AT-SPI drive saw the
app **re-create its X window** on the Replay press and paint only after
a long delay, and the author reports slow loading — possibly one
phenomenon (the 3D scene rebuilt from scratch on entry). Worth one
look when the UI work starts. Tracked in #67 with the UI pass's own
observation (more than 12 s from the Replay press to the first frame under
Xvfb + llvmpipe); not diagnosed.

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
5. `ui/05-settings`: the grouped settings page.
6. `ui/06-replay`: the floating HUD with auto-hide, and the sidebar hidden
   during the replay.
7. `ui/07-docs`: the README screenshots and the final documentation pass.
