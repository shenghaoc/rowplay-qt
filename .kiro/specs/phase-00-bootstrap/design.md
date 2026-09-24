# Phase 0 — Bootstrap: design

## Workspace

```
Cargo.toml                 workspace (resolver 3, edition 2024, MSRV 1.87, shared lints)
crates/rowplay-core        pure domain (no Qt, no I/O)
crates/rowplay-platform    traits + mocks (no Qt)
crates/rowplay-app         qtbridge binary; build.rs compiles qml/rowplay.qrc with rcc
crates/rowplay-fixtures    dev-only fixture loader (tests/fixtures)
qml/RowPlay                QML module `RowPlay` (qmldir, Main.qml, SmokeScene.qml)
qml/rowplay.qrc            resource collection, mounted at qrc:/qt/qml/RowPlay
```

`default-members` excludes the app so the root `cargo test` needs no Qt.

## Qt resources without C++

qtbridge's `include_bytes_qml!` embeds one file per call as a byte-literal
token stream. The app instead runs `rcc --binary --no-compress` from
`build.rs` (found via `qmake -query QT_INSTALL_LIBEXECS`, override with
`ROWPLAY_RCC`) and registers the output with
`qtbridge::qresource::register_bytes` from a `static` slice. The engine gets
`add_import_path("qrc:/qt/qml")` and loads `qrc:/qt/qml/RowPlay/Main.qml`.

## Backend object

`crates/rowplay-app/src/smoke.rs`: `SmokeBackend` with `#[qobject(NoQmlElement)]`
and a manual `QmlRegister` (URI `RowPlay`, element `Smoke`, singleton) so the
QML-facing module name is not derived from the Cargo package name. Properties:
`elapsed`, `frames`, `sphereY`, `cubeAngle` (notify `frameChanged`), and the
constant `status`, `screenshotPath`, `exitAfterFrames`. Slot `tick(dt)` does the
per-frame math. `status` proves the core dependency by describing the demo library.

## Scene

`qml/RowPlay/SmokeScene.qml`: `View3D` with `ExtendedSceneEnvironment`
(sky-box background, MSAA, filmic tonemapping) whose `lightProbe` is a
`Texture { textureData: ProceduralSkyTextureData { … } }`, a `PerspectiveCamera`,
a shadow-casting `DirectionalLight`, a PBR sphere (`#Sphere`), a metallic cube
(`#Cube`) and a ground plane.

## Headless screenshot

`Main.qml` runs a `FrameAnimation`; after `exitAfterFrames` frames it calls
`View3D.grabToImage`, saves PNG (for humans) and PPM (for the test), then
`Qt.exit(0)`. `crates/rowplay-app/tests/smoke_screenshot.rs` launches the binary
with `ROWPLAY_SMOKE_SCREENSHOT` / `ROWPLAY_SMOKE_EXIT_AFTER_FRAMES`, parses the
PPM and asserts ≥ 256 distinct colours and a blue sky pixel. Qt Quick 3D needs a
real RHI backend, so the test opts in with `ROWPLAY_QT_SMOKE=1` and expects
`QT_QPA_PLATFORM=xcb` under `xvfb-run` with Mesa (`QSG_RHI_BACKEND=opengl`,
`LIBGL_ALWAYS_SOFTWARE=1`). The `offscreen` platform falls back to the
software scene graph, where Quick 3D cannot render.

## CI

`.github/workflows/ci.yml`: `core` (fmt, clippy, test, diff --check, MSRV
check), `app` matrix (`ubuntu-24.04`, `macos-26`, `windows-2025`) with
`jurplel/install-qt-action@v4`, cargo build + clippy of the app; the Ubuntu
leg installs the xcb runtime libraries and Mesa, runs the smoke test under
`xvfb-run` and uploads the screenshot as an artifact.

*(2026-09-24: a `changes` job now classifies each pull request. For a
docs-only one, the `app` matrix skips its build and test steps but still
reports its checks, and the step-529 job is skipped. A concurrency group
per pull request cancels superseded runs. See AGENTS.md, "Continuous
integration".)*
