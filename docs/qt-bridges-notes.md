# Qt Bridges for Rust — feedback log

Friction points, bugs and missing features met while building rowplay-qt with
`qtbridge` 0.2.0 (Qt 6.11.2, Rust 1.98.1, Linux x86_64). Each entry has a
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

Packaging consequence (Phase 9): the version reaches Info.plist, the Inno
Setup script and the AppStream metadata from `cargo metadata`, but nothing
inside the running app can read it (no About dialog until this lands or
the version is threaded through a backend property).

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

**Resolved on our side (Phase 9, 2026-09-19):** `crates/rowplay-app/build.rs`
now emits `cargo::rustc-link-arg-bins` / `-tests` of
`-Wl,-rpath,<QT_INSTALL_LIBS>` and `-Wl,-rpath,@executable_path/../Frameworks`
on Apple targets (`emit_apple_rpaths`). Measured: `otool -l` shows both
`LC_RPATH` entries, `cargo run` / `cargo test -p rowplay-app` work with no
`DYLD_*` variable set, and `macdeployqt` drops the absolute entry from the
bundled binary (only `@executable_path/../Frameworks` remains). The upstream
suggestion stands: the runtime crate should emit this itself, since every
qtbridge binary on macOS needs it. ci.yml's macOS test step runs without the
fallback variable so the fix stays exercised.

Same defect, second direction: `DYLD_FRAMEWORK_PATH=$QT_ROOT_DIR/lib`
also launches the binary. Qt Creator / aqt-style env scripts often export
that variable (not the FALLBACK form), so a machine that "just works" after
sourcing a Qt env can look like the link is correct when it is still
missing every `LC_RPATH`. Both variables only mask the absent rpath; neither
is a substitute for emitting `-Wl,-rpath,…` on Apple targets. Prefer the
FALLBACK form in scripts (narrower override, matches CI) and treat a working
`DYLD_FRAMEWORK_PATH` as the same note-10 symptom, not a different fix.

## 15. A missing `qproperty!` registration reads as `undefined` with no QML error

The most dangerous failure mode found so far. A property that QML reads but
that was never registered with `qproperty!` (here `Sync.progressText`) does
not raise a `TypeError`, a `ReferenceError` or a binding warning: the
expression evaluates to `undefined`, the `Label` renders empty or the string
"undefined", and neither `qmllint` (no `.qmltypes` for Rust types, note #9)
nor the runtime-error gate notices.

Repro: drop a `qproperty!` line from a `#[qobject(NoQmlElement)]` backend,
bind a QML label to that property; the app runs and logs nothing.

Workaround used: the gate scans `qml/` for every `Singleton.member` reference,
passes the list to the app through `ROWPLAY_GATE_MEMBER_CHECK`, and the shell
probes each with dynamic lookup (`typeof Sync["progressText"] === "undefined"`
→ failure). `crates/rowplay-app/tests/qml_runtime_gate.rs` fails on any miss;
verified by injecting a deliberate miss.

Suggestion: have `#[qobject]` emit a `qmldir`/`plugins.qmltypes` entry (the
note #9 ask) — `qmllint` would then catch this at compile time. Failing that,
a debug-build warning when QML reads an unknown member of a Rust-backed
QObject would make the class visible.

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

## 16. A QML load failure leaves `run()` blocking with no window and no error exit

`load_qml_from_file` returns `&mut Self` and only calls
`QQmlApplicationEngine::load` (`qtbridge-runtime/src/qapp.rs`); nothing
checks whether a root object was created, and `run()` is a bare
`QGuiApplication::exec()`. When the root file fails to load — a missing
import, a syntax error, an unknown type — Qt logs the error to stderr and the
event loop then idles with no window: the process never exits and never
returns a non-zero code. Met repeatedly while wiring the generated
`RowPlay.ReplayAssets` module (Phase 5a), each time as a silent hang. The
runtime gate (`crates/rowplay-app/tests/qml_runtime_gate.rs`) launches the app
through `Command::output()`, so a load failure blocks the test binary too,
and CI has no step timeout: it would surface as a job time-out, not as a red
test.

Repro: `QApp::new().load_qml_from_file("qrc:/qt/qml/RowPlay/Main.qml").run()`
with `import DoesNotExist` added to `Main.qml`; the import error is logged
and the process runs until it is killed.

Workaround used: gate walks are run by hand under `timeout`, with stdout and
stderr redirected to a log file and the exit code echoed after it, and the
log is read before any capture is trusted (no "gate screenshot saved" line
means the walk never ran).

Suggestion: make `load_qml_from_file` return a `Result` (Qt has
`QQmlApplicationEngine::objectCreationFailed` and `rootObjects()` to build it
from), or have `run()` return non-zero when no root object was created.

## 17. `grabToImage` returns a black Quick 3D viewport on **one** macOS/Metal host

Not qtbridge, but it silently corrupted a QA baseline, so it belongs here.
Scope of the bug (**narrow, not "capture is broken"**):

- **Affected**: one previous session's host, Apple M5, macOS + Metal,
  Qt 6.11.2. On that host `QQuickItem::grabToImage` composites the 2D
  scene graph correctly but the `View3D` region comes back **solid
  black** on the replay route, while the live window renders it. Phase
  0's sphere-and-cube smoke capture *does* render, so it is not "grab
  is broken for Quick 3D" — the cause is not yet identified
  (candidates: the replay scene's `ExtendedSceneEnvironment` /
  procedural sky probe, its post-processing pass, or the platform's
  Quick 3D texture path).
- **Unaffected**: Linux + Xvfb + Mesa (`QT_QPA_PLATFORM=xcb`,
  `QSG_RHI_BACKEND=opengl`, `LIBGL_ALWAYS_SOFTWARE=1`) — both the
  Linux CI legs and any local Linux session with those env vars.
  Verified 2026-09 in this session by running the gate walk locally
  (`xvfb-run -a cargo test -p rowplay-app --test qml_runtime_gate`);
  the resulting `replay-{row,ski,bike}.ppm` render the venue palettes
  (row teal water, ski near-white snow, bike cream terrace) with zero
  black samples across a 96-point grid over the `View3D` region.
- **Also unaffected**: the author's RHEL box (Wayland, Intel UHD 630),
  used for the Phase 7 T8 baseline.

**Do not read this note as "local visual verification is only
possible on the RHEL box"** — the earlier paragraph in
`docs/parity-coverage.md`'s Capture caveat section (and the
handover) both imply that. A Linux session with Xvfb + Mesa (this
one, and the Linux CI legs) does render the 3D captures. When you
need to look at ski / rower / bike phase output, running the gate walk
locally on Linux is a valid path.

The failure is silent on the affected host because the older
`assert_rendered` samples the **whole window**: the sidebar and chrome
supply well over 64 distinct colours and keep the top colour under
90 %, so an all-black viewport passed. That has been superseded:
`common::assert_viewport_rendered` samples the viewport region under
a real GL backend so a blank 3D area fails instead of riding on the
sidebar's colours.

Repro (macOS/Metal only): `ROWPLAY_SMOKE_GATE=1 ROWPLAY_PHASE_SHOTS=1
ROWPLAY_SMOKE_SCREENSHOT_DIR=$PWD/artifacts cargo test -p rowplay-app
--test qml_runtime_gate` on the affected macOS host produces
`artifacts/phase-*-*.ppm` with a black viewport region while the app's
own window shows the scene. If someone hits this on a different macOS
host, log a comment here — the "one macOS/Metal host" scope stands
until a second host reproduces.

**Release impact: none (won't-do, Phase 9 distribution policy).** Linux
AppImage is the only distributed artifact; macOS is built and
launch-checked in CI to keep the port cross-platform but is not
distributed, so this capture defect is no longer a release blocker and
will not be chased. It stays recorded because it silently corrupted a QA
baseline once and the failure mode is worth knowing on any macOS host.

Phase 7's T8 baseline was captured on the RHEL machine and its twelve
captures are valid — do not re-shoot it on account of this note. New
visual work can be done on the Linux CI leg, on the RHEL box, or on
any Linux session with the Xvfb + Mesa env vars above.

A related capture-path defect, found while checking the phase twins and fixed
in `bf8d77f`: the QA close-up camera (`closeup_camera_view`, only under
`ROWPLAY_PHASE_CLOSEUPS`) **measured ~29–31 m from the athlete on `bf8d77f`** —
one loop radius — because the offset was rotated into the rig frame but never
translated to the athlete's placement, so the twins framed venue geometry. Note
the measurement is of `bf8d77f` only: the function is unchanged since Phase 7
introduced it (`cf85cdb`) and Phase 7's own T8 notes record close-up judgement,
so how those earlier close-ups were framed is an open question
(`docs/parity-coverage.md`, ranking 4) — not a settled "it never worked".

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
- A `Vec<f64>` `Member` property with a `Notify` signal (`Replay.sunOffset`,
  `crates/rowplay-app/src/backend/replay.rs`) reads as a JS array in QML
  (`Qt.vector3d(Replay.sunOffset[0], …)`).
- `serde_json::Value` `Constant` properties (the `serde_json` feature) as
  read-only QML maps and lists: `Replay.meshRoles[node.objectName]` keyed
  lookup, `Replay.anchors` iterated with `.template` / `.position[0]`, and
  `Replay.materialSpecs` cross-checking the statically declared role
  materials in `ReplayScene.qml` against the Rust spec table at startup —
  structured data crosses the bridge without a `QGadget` (the note #4 ask
  still stands for typed or writeable values).
- A `[build-dependencies]` entry on the workspace's own `rowplay-viewmodel`
  lets `build.rs` validate the vendored V3 pack with the same `validate_v3`
  the app uses and embed the derived `replay_assets_meta.json` through
  `include_str!`; the balsam module is a third `rcc --binary` blob
  (`rowplay_replay.rcc`) registered next to the other two.

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
- Qt Quick 3D's `RuntimeLoader` (6.11.2) exposes only `source`, `status`,
  `errorString`, `bounds`, `instancing`, `supportedExtensions` and
  `supportedMimeTypes` (`qquick3druntimeloader_p.h`); the nodes it imports
  carry no `objectName` and are not reachable through `children` from QML,
  so neither a material-role walk nor joint posing can address them. Repro:
  `RuntimeLoader { source: "…/rowplay-rigs-v3.glb" }`, then walk `children`
  recursively looking for `objectName === "equipment:row:boat-assembly"` —
  never found. Phase 5a therefore converts the packs with `balsam` at build
  time (ADR 0008, `build_replay_balsam` in `crates/rowplay-app/build.rs`).
- `balsam` output (6.11.2): one `PrincipledMaterial` placeholder per pack
  that every `Model` shares, the glTF `extras` (`replayAsset*`,
  `replayMaterialRole`) dropped, every node's `objectName` set to its glTF
  name, the V4 skin as `Skin { joints: [...] }` (51 joints) over `Node`s
  named after the contract's bones (56 `v4*` names, the four `*Contact`
  helpers included) and — unless `--removeComponentAnimations` is passed —
  the three authored clips (`rowplay-v4-row-cycle`, `-ski-cycle`,
  `-bike-cycle`) as `QtQuick.Timeline` `Timeline`s that are `enabled: true`
  with a `TimelineAnimation { running: true; loops: Animation.Infinite }`
  and `KeyframeGroup`s whose keyframes are 58 `.qad` files in an
  `animations/` directory. Bundled without those `.qad` files, the
  auto-running timelines flooded one gate walk with 68,145 "Could not find
  any constructor for value type QQuickQuaternionValueType … Cannot set
  property "rotation"" and 3,717 "position" warnings; the athlete is posed
  from Rust in Phase 5b, so `build.rs` strips the animation component (the
  mesh bytes are identical either way). The generated files need a
  hand-written `qmldir` and rcc aliases to sit at
  `/qt/qml/RowPlay/ReplayAssets/` for `import RowPlay.ReplayAssets`.
- `ProceduralSkyTextureData` lives in `QtQuick3D.Helpers`, and the `Model`
  shadow property is `receivesShadows` (not `receiveShadows`).
- Editing a `ProceduralSkyTextureData` in place does not refresh a light
  probe that has already rendered: with the colour properties rebound per
  sport, the sky box and the IBL stayed on the first sport's palette (the
  sky-top pixel read #90acbc in all three captures). Qt 6.11 sources: every
  setter calls `scheduleTextureUpdate()` → `generateRGBA16FTexture()` →
  `QQuick3DTextureData::setSize/setFormat/setHasTransparency/setTextureData`
  (`proceduralskytexturedata.cpp`) with no `update()` of its own; and
  `QSSGBufferManager::setRhiTexture` (`qssgrenderbuffermanager.cpp`) runs
  `createEnvironmentMap` only while `texture.m_texture == nullptr`, its
  later-update path for `MipModeBsdf` (a light probe) re-uploading only a
  pre-baked `QT_IBL_BAKER_VERSION` file — a regenerated raw image never
  reaches the existing environment cube map. Assigning a *different*
  `textureData` object does refresh it (`QQuick3DTexture::setTextureData`
  sets `TextureDataDirty`, and the manager keys its cache on the data
  pointer). Workaround
  (`qml/RowPlay/Replay/ReplayScene.qml`, `rebuildSky`): a `Component`
  factory creates a new `ProceduralSkyTextureData` whenever the palette key
  changes, assigns it to the probe `Texture` and destroys the old one; the
  captures now read #90acbc (row), #6fa7c9 (ski) and #e0e2e3 (bike). The
  same generator starts the sun at (0, 0, -1) and rotates it about X by
  `sunLatitude`, then about Y by `sunLongitude`. Phase 5b confirmed the
  azimuth convention with the chase camera in frame: `atan2(-x, -z)` from the
  web's SUN_OFFSETS produces a Qt sun direction matching the web's normalised
  offset to three decimal places for all three sports.
- `createObject` of a Quick 3D object with a 2D parent (the `View3D`) logs
  "QML ProceduralSkyTextureData: Created graphical object was not placed in
  the graphics scene" and the object never reaches the scene graph; parent
  it to a `QQuick3DObject` (here the probe `Texture`). The gate now fails on
  that message.
- Qt Quick 3D's unit defaults assume a scene about a hundred times larger
  than a metre scene: `PerspectiveCamera.clipNear` 10 and `clipFar` 10000
  (`qquick3dperspectivecamera_p.h`), `Light.shadowBias` 10, `pcfFactor` 2.0
  and `shadowMapFar` 5000 (`qquick3dabstractlight_p.h`). `shadowBias` and
  `pcfFactor` are documented as world-space approximations ("needs to be
  tweaked depending on the size of your scene") and with `csmNumSplits` 0 a
  directional shadow map "covers the bounding box of all shadow casting and
  receiving objects". In the metre-scale replay scene the defaults clipped
  everything within 10 m of the camera (the SkiErg and BikeErg cameras sit
  7–7.5 m from the rigs, so their equipment and athlete vanished) and erased
  every shadow (a 10 m depth offset, a 2 m blur and one map stretched over
  the 6 km ground plane). `ReplayScene.qml` sets `clipNear: 0.1`,
  `shadowBias: 0.02`, `pcfFactor: 0.03`, `shadowMapFar: 60`,
  `csmNumSplits: 2` and `castsShadows: false` on the ground.
- `LookAtNode` (`QtQuick3D.Helpers`) points its forward (-Z) axis at
  `target`: `updateLookAt` (`lookatnode.cpp`) is the camera `lookAt` maths —
  yaw and pitch from `sourcePosition - targetPosition` — so a child
  `DirectionalLight` shines at the target. The captures agree: the rower's
  sun offset at -X/+Z casts shadows towards +X/-Z.
- The gate's member probe (`checkGateMembers` in `qml/RowPlay/Main.qml`,
  note 15) only probes singletons named in its registry object; a new
  Rust singleton (`Replay`) must be added there and to `SINGLETONS` in
  `crates/rowplay-app/tests/qml_runtime_gate.rs`, or its members are never
  checked and a missing `qproperty!` again reads as `undefined` in silence.
- CI (GitHub Actions, `ubuntu-24.04`) runs the app build with no display:
  `balsam` is a `QGuiApplication`, so it died initialising the default xcb
  platform plugin ("could not connect to display") before converting
  anything, failing `build.rs` with exit 101 while the same build passed on
  a developer Wayland desktop. Conversion needs no window, so `build.rs`
  passes `QT_QPA_PLATFORM=offscreen` to the balsam invocation; verified
  headless (`env -u DISPLAY -u WAYLAND_DISPLAY`) against both packs. The
  workflow's Toolchain report step prints `balsam --version` so a missing
  binary is visible at a glance.
- **Dynamic Quick 3D content does not reliably reach the rendered scene;
  static declarations are the pattern** (Phases 5a–6b, three independent
  findings). This is the consolidated rule for the codebase; the entries
  below keep the individual evidence. In this repository's setup
  (qtbridge-generated QML engine, Qt 6.11.2), Quick 3D content that must
  rasterise is declared statically in the scene, and what is built at
  runtime from JavaScript is limited to data, inspection, and objects whose
  rendering has been proven per case:
  - *Phase 5a*: a dynamically created `PrincipledMaterial` that nothing owns
    is garbage-collected and its `Model` drops out of the render (ownership
    is one failure mode — parent or retain fixes that case, and parented
    dynamic materials do render).
  - *Phase 5b*: assigning `model.instancing = myInstanceList` from
    JavaScript on a balsam-generated Model had no effect (see the 5b entry
    below); the dual-component pattern replaced it.
  - *Phase 6b*: a fully dynamic component subtree
    (`Qt.createComponent` + `createObject(sceneNode)`) under a `Node` inside
    a `View3D` built a complete, walkable object graph — children
    enumerable, `objectName`s intact, no warnings, no "was not placed in the
    graphics scene" — yet nothing in it ever rasterised. Proven by swapping
    the identical component to a static declaration, which rendered
    immediately. The venue runtime therefore instantiates all twelve
    variants statically and toggles `visible` (which also makes tier swaps
    instant).
  Rule of thumb going forward: any new dynamic Quick 3D creation must be
  render-tested on the first attempt, and the default plan is a static
  declaration with visibility/state driven from data. If upstream documents
  a supported injection path for runtime-created scene content, revisit.
- A dynamically created Quick 3D *material* that nothing owns is garbage
  collected, and the `Model` it was assigned to then drops out of the
  render entirely — no fallback to the default material, while the shadow
  pass still draws it (the row boat left only water and its shadow). This
  is ownership, not dynamic creation: a `PrincipledMaterial` from
  `Component.createObject(null)` (or `Qt.createQmlObject` with no QML
  parent) is JavaScript-owned, and a `Model.materials` list is not a
  JavaScript reference, so the next collection deletes it. Probe (Phase
  5a, one gate run, `gc()` forced right after assignment): the row hull
  with a parentless, unreferenced red material vanished (497 red pixels
  left, the shadow intact); the decks with a parentless green material
  retained in a `property var` rendered (2,939 pixels); the cockpit tub and
  gunwales with a blue material parented to a scene `Node` and not retained
  rendered (1,583 pixels). A second run restored the first
  implementation — 15 materials from `Qt.createQmlObject` parented to a
  singleton `QtObject` and kept in a `var` map — and rendered the full
  lane-painted boat. An earlier version of this note blamed dynamic
  creation as such; that does not reproduce — ownership is a separate,
  fixable failure mode (see the consolidated dynamic-content rule above for
  the cases dynamic creation cannot fix).
  `qml/RowPlay/Replay/ReplayScene.qml` keeps the 15 role materials as
  static children of the scene (like balsam's inline placeholder) and
  cross-checks each one's metalness/roughness against the Rust spec table
  (`Replay.materialSpecs`) at startup — a mismatch calls
  `Replay.reportError` and shows the error overlay — because static
  declarations need nothing kept alive, not because dynamic ones cannot
  work.
- `grabToImage` composites the last *rendered* frame, and an idle Quick 3D
  scene renders exactly one frame per change — so a gate that switches
  sport and grabs on the next timer tick races the renderer: under load or
  llvmpipe the captures showed the *previous* sport's palette, and mesh
  buffer uploads (which progress only on rendered frames) were caught
  half-done as a crumpled hull. Fix in `Main.qml`'s gate: count frames via
  `onAfterAnimating`, keep a `FrameAnimation` running while a replay hold
  is active (an idle scene would otherwise never render the second frame),
  and release the grab only after both N rendered frames and a minimum
  wall time; the release must `return` without advancing the gate step, or
  the next sport switch lands before the asynchronous grab callback.
- `grabToImage`'s `saveToFile` returns `false` with no Qt warning when the
  target directory does not exist. The gate logged `FAILED` and walked on,
  so every 2D capture in CI (dashboard, settings, detail) had silently never
  been written since Phase 4; the upload step looked healthy because the
  smoke test creates its own artifact directory and `if-no-files-found:
  error` only fires when nothing at all matches. The walk test now creates
  `ROWPLAY_SMOKE_SCREENSHOT_DIR` before launching the app and asserts on the
  files it expects. Rule for later phases: a capture that no test reads back
  is unverified, whatever the upload step says.
- A Windows checkout with `core.autocrlf` breaks byte-exact asset hash
  tests: the V4 contract JSON (835 lines) gained exactly 835 CR bytes and
  failed `asset_hashes.rs` on the Windows CI leg only (macOS/Linux check
  out LF). The repo has no `.gitattributes` by default, so one now pins
  `*.json text eol=lf` plus the binary extensions; `git add --renormalize`
  must stay a no-op. Hash tests pin bytes, so they cannot be lenient about
  line endings — the checkout must be.
- **Dynamically assigned properties on balsam Models** (Phase 5b). Setting a
  balsam-generated Model's `instancing` property from JavaScript
  (`model.instancing = myInstanceList`) appeared to have no effect: the
  instanced copies never rendered. The root cause was not conclusively
  isolated — it is one instance of the consolidated dynamic-content rule
  above. The workaround: a second `Rigs` balsam component for the
  mirror-side copies (see "dual-component pattern" below). Phase 6b's venue
  runtime later applied `instancing` from JavaScript successfully on
  statically declared components, so the failing ingredient is the dynamic
  creation path, not the property assignment.
- **Dual-component pattern for multi-instance templates** (Phase 5b). The V3
  pack has three templates with `instances > 1` (oar-rig, ski-assembly,
  wheel-assembly) and four equipment leaves (blade, three pole parts) that
  need multiple positioned copies. Rather than instancing, the scene uses
  two `Rigs` balsam components side by side: the primary is walked with the
  assets README's primary anchor table (`Replay.anchors`), the mirror with
  the clone-index-1 table (`Replay.mirrorAnchors`). `walkRigs(node,
  anchorMap, isMirror)` shows only multi-instance templates
  (`anchor.instances > 1`) in the mirror copy and hides everything else. For
  equipment leaves (blades, pole parts), both copies are shown and
  positioned per frame from the Rust-computed transforms in the frame bundle.
  The per-frame update caches node references by side ("right"/"left") and
  reads the flat frame data without any composition.
- **Frame layout contract** (Phase 5b). The per-tick frame bundle is 225
  `f32` values, starting at offset 0 and packed in this order: sequence (1),
  HUD scalars (8: distance, pace, rate, elapsed, speed, ghost gap, finish
  ETA, progress), camera (7: position xyz, aim xyz, fov), course (8: x, z,
  yaw quat, bob, surge, roll quat), 19 semantic joints (133: 7 per joint —
  translation xyz, rotation xyzw), equipment (64: seat z, oar-left/right
  quats, blade-roll degrees, pole-left/right pos+quat, crank quat, wheel
  quat, blade-left/right pos+quat, pole-leaves-left/right 3×xyz). QML reads
  the layout's named offsets from `Replay.frameLayout` (a JSON object) and
  indexes the flat `Replay.poseFrame` array by those offsets. Every transform
  is computed in Rust (`rowplay_viewmodel::replay::frame`,
  `equipment.rs`); QML assigns, never composes.
- **Three gate assertions and what each catches** (Phase 5b). (1) Colour
  diversity (≥64 distinct colours, no single colour >90%): catches blank or
  single-colour renders — a completely failed scene or a sky-only frame.
  (2) Fixed equipment inventory per sport (RowErg 6, SkiErg 8, BikeErg 4,
  from the assets README): catches missing geometry. This assertion was added
  because colour diversity alone passed with half the equipment absent — a
  one-oared boat with no blades still showed high diversity from the sky
  gradient and the ground plane. (3) Shadow luminance margin (≥5%, measured
  8–12% on llvmpipe): catches shadow-map failures. The centre band (55–75%
  of height, 40–60% of width) is compared against the far right edge, both
  sampling unshaded ground; the sample regions are tied to the chase camera's
  deterministic framing at t=0 for the demo workouts.
- **`renderStats.frameTime` vs wall-clock frame deltas** (Phase 5c). With
  `QSG_NO_VSYNC=1`, `renderStats.frameTime` reports the Qt Quick 3D render
  pass cost (sync + prepare + render), while `FrameAnimation.frameTime`
  measures the threaded render loop's output pacing (~6 ms regardless of
  tier). Without `QSG_NO_VSYNC`, both include the vsync wait and report the
  display refresh interval. For frame-time measurement, `renderStats` with
  vsync off is the correct source; for stutter detection,
  `FrameAnimation.frameTime` is the correct source because it measures what
  the user sees. The `drawCallCount` and `drawVertexCount` on
  `View3D.renderStats` read zero on Qt 6.11.2; the render pass cost is
  trustworthy despite this (cross-checked against wall-clock stutter counts:
  both agree at ~42–54 spikes per 720-frame run). An earlier figure of 20%
  spike rate was from excluding >100 ms samples from `renderStats` only; the
  wall-clock cross-check corrected this to ~6%.
- **Ghost geometry doubles the balsam scene** (Phase 5c). Two `Rigs` × 2
  (primary + mirror) plus two `Athlete` components = 4× the player's balsam
  geometry in the scene graph. On the Intel UHD 630, this produces ~6% of
  frames with >50 ms stalls from structural overhead (GC, buffer uploads)
  that no tier change reduces. The `PerfGovernor` was extended with outlier
  clamping (3× budget) and a payoff check (roll back if a step-down doesn't
  improve the EMA by ≥10%) so it settles rather than walking the sticky
  ladder to Low. The user stays at their chosen tier with occasional stalls.
  Reducing ghost draw calls — instanced geometry or shared scene-graph nodes
  — is the path to fixing the stalls in a later phase.
- **Desktop-supplement locale keys** (Phase 5c). The web has no UI toggle
  for reduce-motion, so there's no locale key for it. The converter
  (`tools/convert-locales.mjs`) gained a `DESKTOP_SUPPLEMENT` map that
  injects desktop-only keys with the English value into all six locales,
  keeping the pipeline as the single source. The i18n parity test pins at
  909 = 908 web + 1 supplement.
- **`balsam` silently drops `EXT_mesh_gpu_instancing`** (Phase 6a). Feeding
  Qt 6.11.2's `balsam` a GLB whose nodes carry three.js's
  `EXT_mesh_gpu_instancing` extension (marked `extensionsRequired`) produces
  a component with a single plain `Model` per node and no warning — every
  instance collapses onto the origin. The extension is not in the list of
  extensions balsam warns about, so this fails silently in a scene where the
  only visible symptom is missing scenery. Minimal repro: export any three.js
  `InstancedMesh` (three r184 `GLTFExporter`) and run
  `balsam --removeComponentAnimations -o out in.glb`; the generated QML has
  no `Instances`/`InstanceList` and the mesh appears once. Qt's own importer
  (`QQuick3DInstancing`) does support the extension in QML form, so the gap is
  the balsam conversion, not the runtime.
- **Blender's glTF importer does not preserve object names for mesh nodes it
  synthesises, and expands instances** (Phase 6a). For a glTF scene the
  exporter wrote from three.js, Blender's importer created extra unnamed mesh
  objects under the named empties where the source had instanced nodes, and
  its exporter then wrote those children as `Mesh_6`, `Mesh_6.001`, …,
  destroying the `objectName`-keyed runtime contract. Once the baker stopped
  emitting `EXT_mesh_gpu_instancing` and named every node, Blender's round-trip
  preserved all names 1:1 — but its importer also copies a shared material
  into `name.001` copies when the meshes using it disagree about vertex
  colours (Blender models vertex colour as a shader node, glTF as a per-vertex
  attribute), and its default `export_vertex_color="MATERIAL"` then drops the
  colour attribute for meshes whose shader no longer references it. The venue
  clean-up script consolidates the copies back onto the base name and exports
  with `export_vertex_color="ACTIVE"`.
- **Dynamically created Quick 3D components never reach the rendered frame**
  (Phase 6b) — the third and most absolute instance of the consolidated
  dynamic-content rule above: a full component subtree under a scene `Node`,
  complete and walkable, never rasterises and never warns. The venue runtime
  instantiates all twelve variants statically and toggles `visible`. If
  upstream knows a supported way to inject runtime-created components into a
  live scene graph, the dynamic loader is the preferred shape and this note
  can be retired.
- **Qt 6.11 `PrincipledMaterial` slot/property drift vs three.js and older Qt**
  (Phase 6b), found wiring the venue contracts onto Qt materials:
  three's `map` slot is `baseColorMap` in Qt; Qt 6.11 renamed
  `PrincipledMaterial.normalScale` (vector2d) to `normalStrength` (float);
  `Texture` needs explicit `tilingModeHorizontal/Vertical: Texture.Repeat` —
  UV transform (`texture.repeat`) does not exist, so the bake multiplies the
  repeat into the geometry UVs; and `Material.DisableCulling`/
  `Material.NoCulling` is shadowed by QtQuick.Controls' attached `Material`
  in files importing both — qualify through the concrete type
  (`PrincipledMaterial.NoCulling`).
- **Debug-binary measurements vs a release spot check** (Phase 6b
  follow-up). All frame-time measurements in this project are taken on a
  debug binary because the gate and bench hooks (`ROWPLAY_SMOKE_GATE`,
  `ROWPLAY_REPLAY_BENCH`) are compiled out of release via `test_env`; that
  keeps the 6b table comparable with 5c's. To size the artifact, one release
  run was made with the hooks temporarily re-enabled locally (not committed):
  same machine, scene, seed and bench settings. Result: **medians are
  build-independent** — 10.0–12.5 ms across all twelve sport×tier cells on
  both builds (debug 9.7–12.2 ms) — so `tick`'s CPU side is not a measurable
  share of the frame cost and the ~10–12 ms floor is real render cost, not a
  debug artifact. p95 moved by more than medians (row-high 28.3 debug →
  20.5 release, ski-ultra 26.9 → 20.9, but bike-ultra 23.5 → 31.3), i.e.
  within run-to-run variance for the GPU-bound tail; wall-clock stall counts
  were equal (46–56 per 720 frames on both). Conclusion: the debug-binary
  methodology overstates nothing that the 22 ms budget judges; spot-checking
  release again after future CPU-side changes is still worthwhile.
- **p95 at 600 frames resolves Low/Medium but not High/Ultra tails**
  (Phase 6b follow-up). Three additional identical debug bench runs (same
  binary, env, data dir) plus the PR run and the release spot check give
  five samples per cell. Low/Medium p95 is tight — 17.9–21.3 ms across all
  runs, identical-run spread ≤ 7 %, worst sample 21.3 — so "Medium holds its
  22 ms budget" is safe to state per-run. High/Ultra p95 straddles the 22 ms
  line and swings 23–38 % between identical runs (bike-high: 21.0, 21.0,
  22.8, 29.4 across four runs) because the tail has a second mode near the
  threshold and the 30th-worst of 600 samples flips across it. Protocol
  going forward: tail claims at High/Ultra are stated as ranges over ≥ 3
  runs ("typically at or above 22 ms"), never as a single-run p95; medians
  are stable everywhere (9.7–12.8 ms across all 58 samples) and need no such
  treatment.
