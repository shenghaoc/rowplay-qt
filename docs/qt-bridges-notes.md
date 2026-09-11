# Qt Bridges for Rust — feedback log

Friction points, bugs and missing features met while building rowplay-qt with
`qtbridge` 0.2.0 (Qt 6.11.2, Rust 1.94.1, Linux x86_64). Each entry has a
minimal repro so it can be sent upstream
(`https://codereview.qt-project.org/q/project:qt/qtbridge-rust`,
bug tracker `https://qt-project.atlassian.net/browse/QTBRIDGES`).

What worked well is listed at the end so the report is balanced.

## 1. QML module URI is derived from the Cargo package name and cannot be overridden

`#[qobject]` implements `QmlRegister` with `URI` = `CARGO_PKG_NAME` with every
non-alphanumeric character replaced by `_` (`qtbridge-gen/src/qt_gen_impl/qml_element.rs`).
A package named `rowplay-app` therefore registers into `import rowplay_app`, and
the module version is the Cargo `major.minor`.

Repro: `cargo new rowplay-app`, add `#[qobject(Singleton)] impl Backend {}`,
`QApp::new().register::<Backend>()`; QML must `import rowplay_app`.

Workaround used: `#[qobject(NoQmlElement)]` plus a hand-written
`impl QmlRegister for SmokeBackend { const URI: &str = "RowPlay"; … }`
(`crates/rowplay-app/src/smoke.rs`).

Suggestion: `#[qobject(Uri = "RowPlay", Version = "1.0", ElementName = "Smoke")]`.

## 2. `Singleton` and `NoQmlElement` are mutually exclusive, but a manual singleton works

`qobject_macro_params.rs` rejects `NoQmlElement, Singleton`, yet a manual
`QmlRegister` with `IS_SINGLETON = true` registers fine. The restriction is
therefore only cosmetic and blocks the obvious "singleton with a custom URI".

## 3. `include_bytes_qml!` does not scale to real assets and leaks path components

`qtbridge-gen/src/qt_resource/include_bytes.rs` builds an rcc tree in the
proc macro and emits it as `&[#(#data),*]`, one token per byte. A 700 KB
`.glb` becomes ~700 k tokens; a QML module needs one call per file; and the
qrc path is the include path verbatim, so `include_bytes_qml!("../qml/Main.qml")`
registers `:/../qml/Main.qml`.

Repro: `include_bytes_qml!("../../qml/RowPlay/Main.qml", "qt/qml")` from
`crates/rowplay-app/src/main.rs`, then `load_qml_from_file("qrc:/qt/qml/RowPlay/Main.qml")`
fails to find the file.

Workaround used: `build.rs` runs `rcc --binary --no-compress qml/rowplay.qrc`
(found via `qmake -query QT_INSTALL_LIBEXECS`) and the app calls
`qtbridge::qresource::register_bytes(include_bytes!(concat!(env!("OUT_DIR"), "/rowplay.rcc")))`.

Suggestion: an `include_qrc!("../../qml/rowplay.qrc")` macro or a
`qtbridge-build-utils` helper that runs `rcc --binary` and exposes the path.

## 4. Supported property and argument types stop at scalars, `String`, `Vec<scalar>` and QObjects

`QPropertyMember` / `QMetaCallArg` (`qtbridge-runtime/src/qpropertymember.rs`,
`qmetacallarg.rs`) cover primitives, `String`, `Vec<primitive | String>`,
`Rc<RefCell<T: QObjectHolder>>`, `Vec<Rc<RefCell<T>>>` and, behind the
`serde_json` feature, `serde_json::Value`. No `QVector3D`, `QColor`,
`QPointF`, `QVariantMap`, tuples or plain structs.

Consequence for a 3D app: a per-frame pose has to be flattened into many
scalar properties (`sphereY`, `cubeAngle`, …) or serialised as JSON, which is
either chatty or allocation-heavy. `crates/rowplay-app/src/smoke.rs` uses
scalars with one `frameChanged` notify.

Suggestion: `QVector3D`, `QQuaternion`, `QColor`, `QPointF/QSizeF/QRectF` as
`QPropertyMember` / `QMetaCallArg` and a `#[derive(QGadget)]` for POD structs.

## 5. No organisation name / domain / version setters on `QApp`

`QApp` exposes `application_name` only. `QSettings` and `QStandardPaths`
(Phase 3 preferences and cache locations) need the organisation name and
domain, and about-dialogs need `applicationVersion`. Process arguments *are*
forwarded: `QGuiApplication::new()` builds `argc`/`argv` from `std::env::args_os()`
(`qtbridge-type-lib/src/generated/gui/qguiapplication/qguiapplication.rs`), so
`rowplay-app -platform offscreen` works.

Suggestion: `organization_name`, `organization_domain`, `application_version`
builder methods on `QApp`.

## 6. Signal handler names default to `onFoo_bar`

A `#[qsignal] fn frame_changed(&mut self)` is `onFrame_changed` in QML unless
`qml_name = "frameChanged"` is given; `ConvertToCamelCase` on the `qobject`
covers slots. Documented, but easy to trip over. Suggestion: camelCase by
default for both slots and signals, snake_case opt-in.

## 7. `qtbridge-runtime` links `QtQuickTest` into every application

`qtbridge-runtime/build.rs` lists `["Core", "Gui", "Qml", "QuickTest"]`. Every
binary depends on `libQt6QuickTest`, which distribution packages split into a
`-dev`/test package and which is pointless at runtime. Suggestion: move the
test dependency behind a feature.

## 8. Windows/macOS support statement

The README lists Linux x86_64, Windows x64 and macOS arm64 (experimental).
Our CI matrix builds on all three; Linux aarch64 and macOS x86_64 are not
listed. Recorded so that CI failures on those legs are attributed correctly.

## 9. No `.qmltypes` for Rust-registered types, so `qmllint` / `qmlls` cannot see them

`qmllint -I qml qml/RowPlay/Main.qml` reports `Unqualified access` for every
use of the Rust singleton `Smoke` and marks `import RowPlay` as unused in
`SmokeScene.qml`, because nothing describes the Rust-registered types to the
tooling (the README lists QML language-server support as a future plan).
Every Rust-backed property therefore loses completion, type checking and the
`unqualified` lint, which is the main guard against typos in QML bindings.

Suggestion: have `#[qobject]` (or a `qtbridge-build-utils` helper) emit a
`plugins.qmltypes` / `qmldir` `typeinfo` entry per registered type so
`qmllint` and `qmlls` can resolve them.

## 10. macOS binaries reference Qt frameworks through `@rpath` but no `LC_RPATH` is emitted

On macOS (arm64, Qt 6.11.2 frameworks from aqt), any binary linking
`qtbridge-runtime` fails at launch with:

```
dyld[…]: Library not loaded: @rpath/QtCore.framework/Versions/A/QtCore
  Referenced from: …/target/debug/deps/rowplay_app-…
  Reason: no LC_RPATH's found
```

Repro: `cargo test -p rowplay-app` on macOS with `qmake` from a framework
install on `PATH`; the build succeeds, the test binary aborts before `main`.
`otool -l <binary>` shows the `@rpath/…` load commands but no `LC_RPATH`.

Workaround used: run with `DYLD_FALLBACK_FRAMEWORK_PATH=$QT_ROOT_DIR/lib`
(CI sets it for the macOS test step). Suggestion: have the runtime build
script emit `-Wl,-rpath,<qt_lib_dir>` (or `@loader_path`-relative rpaths)
on Apple targets, as it effectively does on Linux.

## What worked

- `QApp::new().register::<T>().add_import_path("qrc:/qt/qml").load_qml_from_file(...)`
  mixes Rust-registered types and a `qmldir`-based module under the same URI.
- `qresource::register_bytes` with an `rcc --binary` blob from `build.rs`.
- `Member`-based properties with a shared `Notify` signal, `Constant` members,
  `u64` / `f64` / `String` / `i32` members, and a `&mut self` slot called from
  `FrameAnimation` every frame with no visible overhead in the smoke scene.
- `Qt.exit(code)` propagates through `QApp::run()` to the process exit code.
- Build: `qmake` on `PATH` (or `QMAKE`) is enough; the C++ bridge crates compile
  in about a minute on the first build and are cached afterwards.

## Not qtbridge, but worth knowing

- The `offscreen` QPA platform falls back to the software scene graph, where
  Qt Quick 3D refuses to render (`QSGRendererInterface::isApiRhiBased`). Headless
  rendering needs Xvfb with `xcb` (`libxcb-cursor0 libxcb-icccm4 libxcb-keysyms1
  libxcb-shape0 …` installed) or `eglfs` with `QT_QPA_EGLFS_INTEGRATION=eglfs_x11`,
  plus Mesa (`LIBGL_ALWAYS_SOFTWARE=1`, `QSG_RHI_BACKEND=opengl`).
- Qt warns about the `C` locale; set `LANG=C.UTF-8` in CI.
- aqtinstall 3.3.0 (latest release) cannot install Qt 6.11.x on Windows: the
  Windows repository moved to per-arch folders
  (`qt6_6112/qt6_6112_msvc2022_64/Updates.xml`) and aqt still looks for
  `qt6_6112/qt6_6112/Updates.xml` (miurahr/aqtinstall#1007, fixed by
  miurahr/aqtinstall#1000, unreleased). CI installs aqt from the pinned merge
  commit via `jurplel/install-qt-action`'s `aqtsource` on Windows only; Linux
  and macOS work with the released 3.3.0.
