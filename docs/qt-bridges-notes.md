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

**Resolved 2026-09-24: not a Metal defect, but the gate test's `offscreen`
default.** Re-checked on an Apple M5 host (macOS 27, Qt 6.11.2), the
configuration this note names:

- `qml_runtime_gate.rs` sets `QT_QPA_PLATFORM=offscreen` whenever the
  caller leaves it unset, and the `offscreen` platform draws no Quick 3D.
  The replay's viewport is then the flat window colour: `#ffffff` in light
  and `#111317` in dark, one colour with zero variance. The dark one reads
  as "solid black". The repro below runs through `cargo test`, so it ran
  offscreen.
- The same gate walk run in a real window (the debug binary run directly
  with `ROWPLAY_SMOKE_GATE=1 ROWPLAY_SMOKE_SCREENSHOT_DIR=…` and
  `QT_QPA_PLATFORM` unset, so `cocoa` on Metal) captures the replay: in a
  200 × 100 downsample of the row viewport, 2602 distinct colours in light
  and 2248 in dark, with no near-black sample, at 2400 × 1600 on the Retina
  display.
- Through the test itself, `QT_QPA_PLATFORM=cocoa QSG_RHI_BACKEND=metal
  ROWPLAY_PHASE_SHOTS=1` passes in both schemes. Until 2026-09-25 the dark
  one failed `replay-row`'s shadow check at a 4.9 % margin against its 5 %
  threshold. That check read the Medium captures, which cast no shadows (the
  tier rules turn them on at High and Ultra), and once the replay hid the
  sidebar its centre sample landed on the athlete and hull: it compared
  their albedo with the water's, and dark water is about as dark as the
  athlete. The check now compares the rower at High with its twin grab
  with the key light's shadow off (`assert_shadows` in
  `crates/rowplay-app/tests/common/mod.rs`). The test
  keeps a caller-provided platform, and `QSG_RHI_BACKEND` turns on its
  viewport and shadow checks: all twelve phase shots clear
  `assert_viewport_rendered` (docs/parity-coverage.md, "Capture caveat").

The rest of this note is the original record.

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

**Release impact (updated 2026-09-24, ADR 0014):** macOS is distributed
now, and there is no capture defect to block it. Run the gate walk in a
real window to check macOS pixels (see the resolution above). The note
stays because it silently corrupted a QA baseline once: on any platform,
an offscreen capture shows no 3D, and a flat viewport in a capture means
"which platform ran?" before it means "the renderer is broken".

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

## 18. A `serde_json::Value` property is converted on every read, and one shared `Notify` fans out

`qproperty!` over a `serde_json::Value` member (the `serde_json` feature, the
note #4 route for structured data) hands QML a fresh conversion on every
access: the whole value becomes a `QJsonValue`, then a `QVariant`, then a JS
object, each time the property is read. Nothing is cached on either side, not
even for a `Constant` property whose value can never change. A lookup inside a
loop therefore converts the entire value once per iteration.
`ReplayScene.qml`'s `walkRigs` read `Replay.meshRoles[node.objectName]` for
every node of four `Rigs` components, so one scene walk cost 115 ms (median,
n = 25; debug build, 4-core x86_64 VM, Xvfb + llvmpipe). Reading the map into
a QML property once brought it to 1 ms. The GUI thread's perf profile had the
time in `qtbridge_runtime::serde_tools::serde_to_qjsonvalue`,
`QJsonObject::insertAt` and `QCborValue::fromJsonValue`.

The second half is ours, but qtbridge's shape invites it. Every `Member`
property needs a `Notify` signal, and the `Replay` singleton shares
`replayChanged` across about twenty of them, so any emission re-evaluates every
binding on all of them and runs every `onReplayChanged` handler. The
governor's diagnostics string, refreshed on every 15th rendered frame, rode
that signal. The scene re-ran its full rule walk about four times a second
during playback: a ~115 ms GUI-thread stall every 15 frames. Because the walk
re-assigns materials, it also made a paused replay redraw itself continuously
(70 frames/s, 160 % CPU on llvmpipe). With `diagnosticsChanged` as its own
signal, a paused replay idles at 0.6 % CPU under llvmpipe (roadmap, UI
follow-ups). On macOS it kept rendering at the display rate for a separate
reason: the replay's tick animation ran while paused (#93). On an Apple M5
the paused replay cost 11 % CPU, down from 44 % before this change, and
0.2–1.1 % once #93 stopped the animation while nothing moves.

Repro: expose a ~70-entry `serde_json::Value` map as a `Constant` property and
read `Obj.map[key]` 1,000 times in a QML loop, against `var m = Obj.map` once
and `m[key]` in the loop.

Workaround used: read structured `Constant` properties into a QML `property
var` once (`ReplayScene.qml`'s `meshRoles`), pass plans down a walk as
arguments instead of re-reading them per item, and give values that change
at frame rate their own notify signal.

Suggestion: convert a `Constant` property once and hand out the cached
`QVariant`, and document that each read of a `serde_json` property costs
O(value size).

## 20. A pure slot must take a receiver and an owned `String`

(Numbered 20: the round-2 design-system stack adds 19.)

`Replay.venueShadowFlags(name)` (#96) is a lookup that needs no state. A
`#[qslot]` must still be a method, and qtbridge implements `QMetaCallArg`
for `String` but not `&str`:

```rust
#[qslot]
fn venue_shadow_flags(&self, name: &str) -> i64 { /* ... */ }
// error[E0277]: the trait bound `str: qtbridge::QMetaCallArg` is not satisfied
```

With `&self, name: String` it builds, and clippy pedantic then reports
`unused_self` and `needless_pass_by_value`, which the slot allows with the
reason in a comment.

Suggestion: accept `&str` for string arguments (the call already holds the
converted string), or allow an associated function as a slot.

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
  `shadowBias: 0.05` with a 32-bit map and `PCF16` over `pcfFactor: 0.05`
  (0.02 and 0.03 with four samples until #96), `shadowMapFar: 60`,
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
- **A QML `Timer` runs on the animation clock, not on wall time.** Under
  Qt 6.11.2's threaded render loop the animation driver advances one
  display interval (16.7 ms) per presented frame. When frames outrun the
  display rate (Xvfb has no vsync) a `Timer`'s 300 ms interval fires every
  18 frames: 288 ms at 62 fps, 237 ms at 76 fps, measured in both a
  standalone repro and the gate walk (270 frames per 15-tick settle in
  every run). When frames are slower than the display rate, the driver
  falls back to wall time: 315 ms per tick at ~10 fps in the same repro.
  So the gate's tick-counted waits are shorter than their nominal wall
  time on a fast headless renderer, and never longer. Repro: a `Window`
  with a `FrameAnimation` that rotates a `Rectangle` each trigger (with and
  without a 100 ms busy loop) next to a 300 ms repeating `Timer` that logs
  `Date.now()` deltas and the `frameSwapped` count between ticks; run it
  with `qml` under `xvfb-run`.
- **The quality governor made the gate's captures machine-dependent.** It
  samples every rendered frame, and under llvmpipe it steps the scene down
  mid-walk, so two identical walks captured `replay-row` at Medium and at
  Low (4.3 % of pixels, channel delta up to 198). The gate pins the
  requested tier (`Replay.setGovernorAuto(false)` at step 52); the
  governor's ladder stays covered by its unit tests in
  `rowplay_core::replay::motion`.
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
  **Under Xvfb + llvmpipe, a running `FrameAnimation` does not schedule
  frames on its own**: it fires on frames something else caused. The hold
  advanced only because the replay scene re-dirtied itself every few
  frames: the governor's diagnostics refresh re-ran the scene's rule walk
  through `replayChanged`. With that refresh gone, or with the governor
  pinned, a paused scene renders once and every settle runs out its tick
  bound. The hold now calls the window's `update()` on every trigger and on
  every gate tick while it lasts (2026-09-24). **macOS differs:** on cocoa
  with Metal, a running `FrameAnimation` keeps the window rendering at the
  display rate. The replay's tick animation ran while paused, so a paused
  replay rendered 120 frames/s on a 120 Hz display (Apple M5, measured with
  `QSG_RENDER_TIMING=1`; #93). The explicit `update()` calls are harmless
  there. Since #93 the tick animation runs only while playing, or for a
  few frames after a change made while paused, so both platforms render a
  still replay on demand. **A window `update()` does not re-render a
  `View3D` that is not dirty:** in the default `Offscreen` render mode the
  3D scene renders only when the item's `updatePaintNode` calls
  `scheduleRender()` on its framebuffer node (`qquick3dviewport.cpp` and
  `qquick3dscenerenderer.cpp`, Qt 6.11.2); otherwise the node keeps its
  last texture. So the replay's
  settle frames call the `View3D`'s own `update()`. The gate's hold counts
  window frames, which the window's `update()` is enough for.
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
  deterministic framing at t=0 for the demo workouts. **Superseded
  2026-09-25:** from Phase 5c the captures it read were Medium, which casts
  no shadows, and from the design system's replay (the sidebar hidden) its
  centre sampled the athlete and hull, so it compared their albedo with the
  ground's and failed the dark scheme on Metal at 4.9 %. It now compares the
  rower at High with its twin grab with the key light's shadow off: the
  shadow must darken at least 1 % of the capture by more than the noise
  delta, and take more than 5 % of the luminance where it falls, measured
  against the same pixels unshadowed (Apple M5: 9.24 % of the capture by
  10.99 % in light, 8.99 % by 19.08 % in dark; CI's llvmpipe: 9.85 % by
  10.51 %). Those figures counted the acne of a venue that shadowed itself
  (#96). With the web's shadow flags the rower's own shadow is what remains:
  Apple M5 1.62 % by 11.36 % in light, 1.49 % by 20.51 % in dark; CI's
  llvmpipe 1.67 % (16,074 px) by 11.38 %.
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
  — is the path to fixing the stalls in a later phase. *Re-attributed
  2026-09-24:* the stall rate matches the diagnostics refresh (720 frames /
  15 = 48), which re-ran the whole scene walk through the shared
  `replayChanged` signal (note #18). Re-measure on the UHD 630 before
  attributing whatever remains to the ghost geometry.
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
- **`Font.TabularNumbers` does not exist; a font object drops it silently**
  (UI fixes, refs #54). Phase 4's `Theme.qml` fonts were JS objects such as
  `({ pixelSize: 13, weight: Font.DemiBold, features: Font.TabularNumbers })`.
  The enum is `undefined` in Qt 6.11, and the object-to-`font` conversion
  ignores an undefined member without a word (`features {}`), so no text
  ever got tabular figures. Assigned directly (`font.features:
  Font.TabularNumbers`) the same mistake is loud — "Unable to assign
  [undefined] to QVariantMap", which the runtime gate fails on.
  `font.features` is a map of OpenType tags since Qt 6.6: `features: {
  "tnum": 1 }` works both ways (verified under the `qml` runtime). DejaVu
  Sans, the Xvfb default, has tabular digits anyway, so the loss only shows
  with proportional-digit fonts such as SF Pro. Measured on macOS under the
  `qml` runtime (the system font, 16 px semibold): "1:11.1" is 37.36 px wide
  and "0:00.0" 48.94 px with the enum or with no feature, and both are
  48.69 px with `"tnum": 1`. The first fix covered `Theme.qml` and missed
  one inline font object, the dashboard's PB time (found in review), so the
  gate test now fails on the name anywhere in `qml/`
  (`qml_names_no_font_tabular_numbers_enum`).
- **Qt Graphs' Y axis has a fixed 40 px label column** (UI fixes, refs #50).
  In 6.11.2 the Y axis strip is `m_defaultAxisLabelsWidth` 40 + 5 +
  `m_defaultAxisTickersWidth` 15 px (`qgraphsview_p.h`, marked "Add
  properties for these"); each label delegate is sized to the 40 px and
  asked to right-align, whatever its text. A wider custom label (a Rust pace
  string such as "2:54.3/500m") overflows into the ticks and the plot.
  Neither the delegate's `implicitWidth` nor `labelFormat` changes the
  column; what works is right-anchoring the delegate's text and reserving
  the overflow through `GraphsView.marginLeft` (`ChartUtils.yLabelOverflow`),
  with a transparent axis `color` so no tick marks sit under long labels.
  The automatic `tickInterval` also aims at about ten ticks regardless of
  the plot height (refs #49).
- **A `Repeater` cannot create chart series** (UI fixes, refs #52). The
  stroke charts' split-boundary `LineSeries` were a `Repeater` delegate
  inside the `GraphsView` since Phase 4: a series is not an `Item`, nothing
  was created, and nothing warned. An `Instantiator` creating the series
  plus `GraphsView.insertSeries(0, series)` / `removeSeries(series)` in
  `onObjectAdded` / `onObjectRemoved` works.
- **A `ScrollView` child sized with `width: parent.width` follows its own
  implicit width** (UI fixes, refs #57): its parent is the Flickable
  content item, whose width tracks the content. `contentWidth:
  availableWidth` on the view plus `width: scroll.availableWidth` on the
  child spans the view.
- **An item grab is not the screen where the item is translucent** (UI
  fixes, refs #61). `grabToImage` renders the item on a transparent
  background; the window background is not part of the item. Pixels that
  nothing the grab root paints covers are stored translucent: alpha in the
  PNG (a viewer composites them over its own page colour) and, in the
  alpha-less PPM the Rust visual assertions read, the bare colour. Repro
  under the `qml` runtime (offscreen): a 10 % black `Rectangle` inside a
  bare `Item` grabs as (0, 0, 0, 26) / PPM (0, 0, 0); inside a white
  `Rectangle` as (229, 229, 229) in both, matching an `xwd` capture of the
  window. The gate's `shellRoot` is therefore painted in the window colour.
- **Under the Basic style, `AbstractButton` in a property type names Basic's
  own composite type** (UI design system, ADR 0013). Basic ships an
  `AbstractButton.qml`, so in a file that imports `QtQuick.Controls`,
  `property AbstractButton target` is typed as that composite. A `Switch`
  (Basic's `Switch.qml` derives from `T.Switch`, not from it) is then
  rejected at load: "Unable to assign ToggleSwitch_QMLTYPE_54 to
  AbstractButton_QMLTYPE_17". Type such properties with the template:
  `import QtQuick.Templates as T` and `property T.AbstractButton target`
  (`FormRow.toggleTarget`).
- **Basic's `DialogButtonBox` stretches its buttons and paints
  `palette.window`** (`Basic/DialogButtonBox.qml`: `alignment` is undefined
  for more than one button, and the background is `control.palette.window`).
  In a dialog on a popup surface that differs from the window colour, as in
  dark mode, the box shows a band of the window colour under the buttons.
  `AppDialogButtonBox` sets `alignment: Qt.AlignRight`, no background and
  PushButton delegates, and keeps the default `buttonLayout` so the
  platform theme still orders the buttons.
- **`QT_FONT_DPI` scales the device pixel ratio, not the font, under
  high-DPI scaling** (the Qt 6 default). On `xcb`, `QT_FONT_DPI=144` drew
  the whole UI 1.5× larger while `Qt.application.font` stayed 12 px (9 pt at
  96 dpi), so it does not exercise a larger system font.
  `QT_ENABLE_HIGHDPI_SCALING=0 QT_FONT_DPI=144` does: the device pixel ratio
  stays 1, the 9 pt system font becomes 18 px, and `Theme.scale` reflows the
  layout (checked in a scratch gallery of every control at 120 and
  144 dpi).
- **`FontMetrics.advanceWidth()` registers no binding dependency** (UI
  design system, ADR 0013). A binding re-runs when a property it read
  changes, and a C++ method call reads nothing on the binding's behalf: a
  binding over `metrics.advanceWidth(text)` follows its text, never
  `metrics.font`. If it first runs before the `FontMetrics`' own font
  binding has landed, it keeps a width measured in the default font, and it
  never follows a live change of the system font, which `Theme` scales
  from. Found in 5/7: given `Layout.maximumWidth: implicitWidth`, the
  settings quality control measured its titles at the default 12 px
  instead of its own 11 px and drew 308 px wide instead of 292. Repro under
  the `qml` runtime (offscreen): that `SegmentedControl` in a `FormRow`
  measures its widest title at 53.78 px, and at 49.30 px without the
  `Layout` line; a `ChartUtils.yLabelOverflow` binding keeps its 25 px
  reserve after its metrics' font grows from 10 to 20 px, where 89 px is
  needed. Reading the font in the binding (`void metrics.font`) registers
  the dependency: 292 px in every case, and 89 px after the change.
- **`ComboBox.WidestText` measures only a `TextInput` content item** (UI
  design system, review of #70). `QQuickComboBoxPrivate::
  calculateWidestTextWidth()` (qtdeclarative `v6.11.2`) returns 0 unless
  `contentItem` is a `QQuickTextInput`, so with a `Label` content item the
  policy leaves `implicitContentWidth` at 0 and the implicit width is the
  background's. Probed under the `qml` runtime on macOS: the compact
  `SegmentedControl`'s `PopupButton` (a `Label`) reported 0 for the sport
  choices at every text size, so its width never depended on its text.
  The control now measures its widest choice with a `FontMetrics` in the
  button's font (`compactWidth`).
- **Qt Quick Layouts round each item's width up to a whole pixel** (UI
  design system, 4/7). `QQuickGridLayoutEngine` constructs its engine with
  `snapToPixelGrid` set (`qquickgridlayoutengine_p.h`, Qt 6.11.2), so a
  row of fractional preferred widths computed to fill a width exactly
  lays out wider than it, by up to a pixel per item. Repro under the `qml`
  runtime (offscreen): seven `Rectangle`s in a `RowLayout` (`spacing:
  11`) whose preferred widths sum with the gaps to 832.0 give the row an
  `implicitWidth` of 835, and the last one ends at 834.9. The splits
  table's spare-width shares were fractional, so its scroller clipped the
  last column wherever the table fits; whole-pixel shares end it flush. A
  `Flow` does not snap: it places an item at a fractional x.
- **Basic's `ScrollBar` draws a track under the OS contrast preference**
  (UI design system, second-reviewer pass). Its `background` is a
  `Rectangle` in `palette.mid` at the thumb's opacity, visible only while
  `Qt.styleHints.accessibility.contrastPreference === Qt.HighContrast`
  (`Controls/Basic/ScrollBar.qml`, Qt 6.11.2). A scroll bar that replaces
  only `contentItem` keeps it, and the app's `ROWPLAY_FORCE_CONTRAST=high`
  never shows it, because Basic reads the OS preference. `AppScrollBar`'s
  high-contrast thumb measured 1.00–1.06:1 on that track, so it sets
  `background: Item {}`.
- **A checkable `AbstractButton` reports the CheckBox role**
  (`QQuickAbstractButton::accessibleRole()`, qtdeclarative v6.11.2), with
  its checked state. An explicit `Accessible.role: Accessible.Button`
  overrides that and hides the toggle from a screen reader.
- **A window `Shortcut` takes Space from a focused button.** An
  `AbstractButton` does not accept Space on `ShortcutOverride`, so a window
  shortcut on the same key wins and the focused button is never pressed
  (probed with a scratch Qt Quick Test). A control that owns a key while
  focused accepts it in `Keys.onShortcutOverride`, as `SegmentedControl`
  does for the arrows and `ToolbarButton` now does for Space.
- **An `Animation on x` that stops leaves `x` where it was.** The
  property's binding stays but does not re-run until a dependency changes,
  so a centred segment stopped mid-sweep stays off-centre (x 113.3 against
  70.0 in a probe of `AppProgressBar`). Setting the binding again when the
  animation stops puts it back.
- **Open popups are not part of any item grab.** A `Popup`, `Menu`,
  `ToolTip` or `Dialog` renders in the window's overlay, a sibling of the
  content item. `grabToImage` on content never includes it, and the
  window's root item refuses a QML grab ("item has no QML engine", like
  `ApplicationWindow.contentItem` above). Captures of open popups come from
  the screen (`import -window root` on the Xvfb display).
- **The system accent and the contrast preference, per platform** (UI
  design system, ADR 0013). Read from the qtbase `v6.11.2` sources
  (`ef55f427`), and probed on Linux.
  - **Accent on macOS:** `palette.accent` is `NSColor.controlAccentColor`
    (`qcocoatheme.mm`, `qt_mac_createSystemPalette`).
  - **Accent on Windows:** the UISettings accent, or the DWM `AccentColor`
    registry value (`qwindowstheme.cpp`, `qt_accentColor`): AccentDark1 in
    light mode, AccentLight2 in dark mode.
  - **Accent on Linux: none.** No Linux platform theme sets
    `QPalette::Accent`: not the generic theme, the desktop portal,
    GNOME / gtk3 or KDE. The portal's `accent-color` is not read up to
    qtbase `dev` of 2026-09-23. The palette therefore keeps
    `qt_fusionPalette()`'s `#308cc6` (`qplatformtheme.cpp`), and so does
    `offscreen`. `Theme.qml` reads `#308cc6` as "no system accent" and uses
    the brand blue.
  - **Contrast:** `Qt.styleHints.accessibility.contrastPreference` (Qt 6.10)
    is `HighContrast`:
    - on macOS under "Increase contrast"
      (`accessibilityDisplayShouldIncreaseContrast`; the source does not
      show whether toggling it notifies a running app);
    - on Windows under a contrast theme (`SPI_GETHIGHCONTRAST`);
    - on Linux from the portal's `contrast` key or GNOME's
      `org.gnome.desktop.a11y.interface high-contrast`.

    The generic, KDE and `offscreen` themes always report `NoPreference`.
  - **Probed here:** under `xcb` on Xvfb with no desktop, and under
    `offscreen`, the probe reads accent `#308cc6`, `NoPreference`, colour
    scheme `Unknown` and a 9 pt (12 px) "Sans Serif" system font. The macOS
    and Windows rows come from the sources only (tracked in #66).
    `ROWPLAY_FORCE_CONTRAST=high` exercises the high-contrast variant on any
    platform.
- **`QKeySequence.StandardKey` per platform** (UI design system, shortcuts;
  qtbase `v6.11.2` `qplatformtheme.cpp` key-binding table). Priority rows
  come first, and the first entry is what `QKeySequence(StandardKey)` and a
  singular `Shortcut.sequence` use.

  | Key | Generic Unix (X11 scheme) | macOS (Ctrl = Command) | Windows |
  | --- | --- | --- | --- |
  | Preferences | none (KDE: Ctrl+Shift+,) | Cmd+, | none |
  | Quit | Ctrl+Q | Cmd+Q | none |
  | Close | Ctrl+W | Cmd+W, Cmd+F4 | Ctrl+F4, Ctrl+W |
  | Find | Ctrl+F | Cmd+F | Ctrl+F |
  | Refresh | F5 (GNOME: Ctrl+R first) | Cmd+R | F5 |
  | Back | Alt+Left | Cmd+[, Cmd+Left | Alt+Left, Backspace |

  The dedicated keys (Settings, Exit, Close, Find, Refresh, Back) are left
  out of the table. The `offscreen` platform sets no keyboard scheme and
  falls back to the Windows column: a probe read Close as Ctrl+F4 and Quit
  as none there, against Ctrl+W and Ctrl+Q under `xcb`.
- **Bind `StandardKey.Back` to its primary chord only.**
  `QQuickTextInput::event` accepts `ShortcutOverride` for Backspace and a
  list of editing sequences, but not for `MoveToStartOfLine`. On macOS that
  is Cmd+Left, the second `Back` chord, so a window shortcut with
  `sequences: [StandardKey.Back]` would take "move to line start" away from
  every text field. `sequence: StandardKey.Back` binds only the primary
  chord (Alt+Left; Cmd+[ on macOS). It logs "Only binding to one of
  multiple key bindings associated with 13" once at load, which is
  expected.
- **`Qt.labs.platform`'s `MenuBar` is native on macOS only.** Elsewhere,
  without `QApplication`, it prints "ERROR: No native Menu implementation
  available. Qt Labs Platform requires Qt Widgets on this setup." (probed
  under `xcb`). The shell therefore creates it through an `Instantiator`
  that is active only on macOS.
  - **Role items.** On macOS, items with a role (About, Preferences, Quit)
    move into the application menu. Their titles come from Qt's own
    `MAC_APPLICATION_MENU` strings ("About %1", "Preferences...", "Quit %1",
    `qcocoamenuitem.mm`) through `QCoreApplication::translate`. The app
    loads only its own `qml_<lang>.qm` (note 12), so those titles stay in
    English until Qt's catalogues are shipped.
  - **Empty menus.** A menu left with no visible items after the move is
    hidden (`qcocoamenubar.mm`).
  - **Not verified on a Mac** (tracked in #66).
- **While anything animates, Qt Quick sends a synthetic hover every
  frame** (UI design system, the replay HUD). After each frame that changed
  an item, `QQuickDeliveryAgentPrivate::flushFrameSynchronousEvents`
  delivers a hover event at the last known pointer position, unless an item
  holds the mouse grab (`frameSynchronousHoverEnabled`, a private flag that
  defaults to on; qtdeclarative `v6.11.2`, `qquickdeliveryagent.cpp` and
  `qquickdeliveryagent_p_p.h`). It keeps hover states right under moving
  items, but a `HoverHandler` sees `pointChanged` on every animated frame
  while the pointer rests. The first build of the HUD's "hide after 3 s
  without pointer movement" therefore never hid while the replay played
  (driven under Xvfb). The HUD now wakes only when the position differs by
  at least a pixel from the last one it acted on.
- **`Timer.restart()` starts a timer whatever its `running` binding says**
  (UI design system, the replay HUD). `restart()` stops and starts the
  timer from C++; the `running` binding survives but is re-evaluated only
  when one of its inputs changes. The HUD called `restart()` from handlers
  on the same changes the binding depended on (play / pause, focus), so
  whether a paused or focused HUD hid three seconds later depended on the
  order the handler and the binding ran. Driven under Xvfb, it hid 4.5 s
  after a pause and 4.5 s after opening idle. The countdown is now
  imperative only: no `running` binding, `restart()` only while the HUD
  may hide, `stop()` otherwise, and the condition checked again on
  `triggered`.
