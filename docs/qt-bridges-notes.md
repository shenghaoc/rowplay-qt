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

## 14. `QListModel::reset()` panics before the QObject is attached

`#[qobject(Base = QListModel)]` works well for the 5,000-row sidebar (bulk
swap the `Vec` in `rebuild`, then `reset()`; role names are the
`#[derive(QModelItem)]` field idents verbatim — snake_case in QML — and the
15-role cap is enough at 13). But the generated `QListModelBase` methods
(`reset`, `push`, …) call `try_get_rust_proxy_ptr().expect("No proxy")`, so
calling `reset()` from `Default::default()` — i.e. while the singleton is
being constructed during QML type resolution, before attachment — panics.

Repro: `#[qobject(Singleton, Base = QListModel)]`, call `self.reset()` inside
`Default::default()`.

Workaround used: `if self.try_get_rust_proxy_ptr().is_some() { self.reset(); }`
(`crates/rowplay-app/src/backend/library.rs`). Data-only rebuilds before
attachment need no notification anyway (no view is bound yet).

Suggestion: make the `QListModelBase` notification helpers no-ops (or return
`bool`) when unattached, and document the construction-order constraint.

## 11. `qproperty!` rejects doc-comment attributes

`#[qobject]` fails with "Attributes for qproperty! macro are not supported"
when a `///` doc comment sits directly above a `qproperty!(…)` invocation
inside the impl block (`qtbridge-gen` parses statement attributes there and
bails). `///` on `#[qslot]`/`#[qsignal]` functions is fine.

Repro: add `/// doc` above any `qproperty!(…)` in the smoke backend.

Workaround used: plain `//` comments above property macros
(`crates/rowplay-app/src/backend/*.rs`).

Suggestion: accept (and drop or forward) doc comments on `qproperty!`.

## 12. No `QTranslator` binding and no engine access on `QApp` — but i18n works without them

`QApp` keeps its `QQmlApplicationEngine` private and `qtbridge-type-lib` has
no `QTranslator`, so a Rust side cannot install translators. That turned out
not to matter: `QQmlApplicationEngine` itself loads
`<main-qml-dir>/i18n/qml_<lang>.qm` from the resource system and reloads on
`Qt.uiLanguage` changes (Qt 6.11 documented behaviour). Setting
`Qt.uiLanguage` from QML (it is a JS global — a `Binding` element cannot
target it, assign imperatively) gives live retranslation of every `qsTrId`
binding with zero Rust involvement.

Caveat found while wiring this up (Qt, not qtbridge): ID-based `.ts` files
must have an **empty `<source>`** for `lrelease` to emit the id lookup —
with a non-empty source, `qsTrId` silently misses even though
`qsTranslate(context, source)` hits. `tools/convert-locales.mjs` generates
the canonical shape (`<source></source>`, English kept in `<oldsource>`).

Suggestion: still worth exposing the engine (or a `set_ui_language` helper)
on `QApp` so the startup language can be applied before the first QML
evaluation instead of in `Component.onCompleted`.

## 13. `QmlMethodInvoker` cross-thread pattern works, with a borrow caveat

The worker-thread pattern from the docs holds up: create the singleton on the
Qt thread, call `get_qml_method_invoker()` inside a slot (the object is
attached by then), move the invoker into a `std::thread`, and poke a
`#[qslot]` (`pumpEvents`) through the queued connection after each
`mpsc::Sender::send`. Verified with a full Concept2-mock sync
(`crates/rowplay-app/src/backend/sync.rs`): progress events, completion and
cancellation all arrive on the Qt thread.

Caveat: the queued call panics if the object is mutably borrowed when it
runs, and a poke can in principle be lost if the object dies mid-call — the
app keeps a 50 ms QML `Timer` polling the same slot while a sync runs as a
safety net.

Suggestion: document the recommended "channel + invoker poke + timer
fallback" recipe in the qtbridge docs; it is the only safe cross-thread
story and currently has to be assembled from three API corners.

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
- `#[qobject(NoQmlElement, ConvertToCamelCase)]` plus a manual `QmlRegister`
  registers four singletons (`Library`, `Detail`, `Settings`, `Sync`) under
  one custom `RowPlay` URI next to a `qmldir` module of the same name — the
  note #1 workaround scales.
- `Vec<String>` / `Vec<i32>` `Constant` properties, `String`/`bool`/`i32`/
  `f64` `Member` properties with a shared `Notify` signal, and `&mut self`
  slots called from QML at UI rates with no visible overhead.
- `get_qml_method_invoker()` inside a slot + `invoke_method("pumpEvents")`
  from a worker thread (note 13).
- Second `rcc --binary` blob for the `lrelease` output registered alongside
  the QML blob through `qresource::register_bytes` — multiple blobs coexist.
- `#[qobject(Base = QListModel)]` + `#[derive(QModelItem)]` for a 5,000-row
  `ListView`: bulk `Vec` swap + `reset()` (guarded, note 14), `required
  property` role access in delegates, smooth scrolling with lazily created
  delegates. Filtering 5k rows takes 2.5–7.7 ms in the view-model.

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
- Qt's QML JavaScript engine has no `String.prototype.replaceAll` (ES2021)
  in 6.11; use `split(…).join(…)`.
- `ApplicationWindow.contentItem` is C++-created and refuses `grabToImage`
  ("item has no QML engine"); wrap the shell in a QML `Item` and grab that.
- `grabToImage` callbacks never fire on the `offscreen` QPA platform (no
  frames are produced); screenshot CI steps need Xvfb/`xcb` (or a real
  Wayland session with software GL, which also works:
  `QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1`).
- ID-based `.ts` catalogues need an empty `<source>` (see note 12); `lupdate`
  writes that shape itself, hand-written files with a non-empty source
  silently break `qsTrId` after `lrelease`.
- Qt Graphs in Qt 6.11 uses the post-6.9 type names (`LineSeries`,
  `BarSeries`, `BarSet`, `ValueAxis`, `BarCategoryAxis`, `GraphsView`,
  `GraphsTheme`); most sample code on the web still shows the 6.8 names
  (`Lines`, `Bars`, `ValuesAxis`) or the 6.8 camera-based `GraphsView`.
  Bulk loading is `XYSeries.replace([{x,y}, …])` — one call, object literals
  or `Qt.point` (flat arrays and pair arrays silently produce (0,0)
  points); `BarSet.values = [...]` for bars. Custom tick strings go through
  `AbstractAxis.labelDelegate` (an `Item` with `property string text`).
  Per-series `axisY` works for multi-scale charts, but the extra axis must
  NOT be declared as a `GraphsView` child (its default property is
  `seriesList`, so the axis silently lands there and the series renders
  nothing). `GraphsTheme.colorScheme` has `Automatic` following the system.
