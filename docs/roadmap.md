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

- Port `engine`, `motion`, `motionGraph`, ghost pick, race gap / result, rival
  CSV / TCX / FIT parsers, stroke pose, quality budgets and the perf governor
  into `rowplay-core`.
- `warpStrokePhase` must be **C1-continuous** at the drive / recovery seam.
  Add a derivative-continuity test (Studio's version is only C0 and causes
  visible speed jumps on SkiErg).
- Enable the `#[ignore]`d parity tests: `stroke-pose-parity.json`,
  `replay-race-gap-parity.json`, `replay-race-result-parity.json`,
  `replay-rival-sources-parity.json`, `replay-current-main-motion.json`,
  `replay-current-main-2d.json`.

### Phase 3 — Platform

- Keyring token store (`keyring`), rusqlite workout cache, Concept2 client
  (HTTPS only, same-host redirects only, ephemeral session, strict timeouts),
  preferences and sync, each behind the existing traits with mocks.
- Concept2 raw-payload mapper validated by `tests/fixtures/Concept2/*.fixture.json`
  (enables the `#[ignore]`d mapper test).

### Phase 4 — QML shell

- Dashboard, library sidebar, workout detail, splits and strokes, Qt Graphs charts.
- Settings and demo mode.
- Convert the six web locales (en, de, es, fr, ja, zh) into `.ts` files via
  `tools/`; strings use `qsTr`, plurals use Qt numerus forms. Add a key parity
  check against the web locales.

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
