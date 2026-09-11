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
  `Unable to assign` / `is not defined`, plus per-screen screenshot artifacts.

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

### Phase 5 — 3D replay

- Load `rowplay-rigs-v3.glb` and `rowplay-athlete-v4.glb`: `RuntimeLoader` in
  development, `balsam` output for release.
- Map the rigs' material-role metadata to `PrincipledMaterial`, following
  `renderer3dAssets.ts`.
- Procedural-sky IBL and shadows, chase camera, and the low / medium / high /
  ultra tiers from `ReplayRenderQuality`.

### Phase 6 — Venues

- A `tools/` Node script that runs rowplay's environment builder per sport and
  exports `.glb` via `GLTFExporter`. Check for DOM / canvas dependencies first;
  if they block Node, write an ADR.
- The author does a Blender clean-up pass; results are vendored with provenance.

### Phase 7 — Motion

- Drive the V4 athlete from the motion graph (Studio's production path ignores
  it), plus per-stroke variation. Enables the grip / equipment parity fixtures.

### Phase 8 — Live and hardware

- PM5 over BLE via `btleplug`, and live mode (including the demo live workout
  generator, `generateMockWorkout`, from the web app).

### Phase 9 — Packaging

- macOS `.app`, Windows installer, Linux AppImage or Flatpak.
