# Phase 0 — Bootstrap: requirements

## R1: Toolchain verification

- **R1.1** The PR reports `qmake -query QT_VERSION`, that the `QtQuick3D` and
  `QtQuick3D.Helpers` imports resolve, and `rustc --version` (≥ 1.87).
- **R1.2** If Qt is missing, everything that only needs cargo is still
  delivered and the PR states exactly what failed.

## R2: Workspace skeleton

- **R2.1** Cargo workspace with `crates/rowplay-core`, `crates/rowplay-platform`,
  `crates/rowplay-app` and the dev-only `crates/rowplay-fixtures`; top-level
  `qml/`, `assets/`, `i18n/`, `tools/`, `tests/fixtures/`, `docs/`, `.kiro/specs/`.
- **R2.2** Dependency direction is app → platform → core; core and platform
  have no Qt dependency.
- **R2.3** `cargo build` / `cargo test` at the root default to the Qt-free
  crates so contributors without Qt can work on the core.

## R3: Stack smoke test

- **R3.1** A qtbridge application with a Rust backend object registered as a
  QML singleton, a QML window, and a `View3D` with one PBR sphere and one cube.
- **R3.2** Lighting uses `ProceduralSkyTextureData` as the
  `ExtendedSceneEnvironment.lightProbe`, filmic tonemapping, and a
  shadow-casting `DirectionalLight`.
- **R3.3** QML drives the backend with a `FrameAnimation` calling one `tick(dt)`
  slot per frame and binds the per-frame results (bridge stays thin).
- **R3.4** A headless screenshot test renders the scene (Xvfb + Mesa OpenGL on
  Linux) and checks the pixels; it self-skips without the opt-in variable.

## R4: Continuous integration

- **R4.1** Linux job: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
  and `cargo test` for the Qt-free crates, plus `git diff --check`.
- **R4.2** App build matrix on Ubuntu, macOS and Windows using
  `jurplel/install-qt-action` with Qt 6.11 and the modules
  `qtquick3d qtshadertools qtquicktimeline qtgraphs`.
- **R4.3** No blocking docs-check job; docs are a P1 review item enforced by
  `AGENTS.md` and the PR template.

## R5: Documentation and agent files

- **R5.1** `README.md`, `LICENSE` (GPL-3.0), `LICENSES/`, `ASSET_PROVENANCE.md`.
- **R5.2** ADRs 0001–0006 record the decisions in the bootstrap brief.
- **R5.3** `docs/source-map.md` maps web → Swift → Rust; `docs/qt-bridges-notes.md`
  logs every Qt Bridges friction point with a minimal repro; `docs/roadmap.md`
  lists every phase.
- **R5.4** `AGENTS.md` is the single canonical agent guide; `CLAUDE.md` and
  `GEMINI.md` contain only `@AGENTS.md`.
