# rowplay-qt

Cross-platform (Linux, macOS, Windows) desktop port of
[rowplay](https://github.com/shenghaoc/rowplay), a Concept2 logbook analytics
and real-time workout replay app for RowErg, SkiErg and BikeErg athletes.

- **Rust** implements all application logic (`crates/rowplay-core`,
  `crates/rowplay-platform`).
- **Qt Bridges for Rust** (`qtbridge`, public beta) exposes Rust objects to
  **QML / Qt Quick**; **Qt Quick 3D** renders the replay and **Qt Graphs** the
  charts, on **Qt 6.11**. No hand-written C++.
- The web app is the canonical behaviour and
  [rowplay-studio](https://github.com/shenghaoc/rowplay-studio) (native macOS)
  supplies the layering and the golden parity fixtures that every port is
  tested against.

Not affiliated with Concept2. Concept2, RowErg, SkiErg and BikeErg are
Concept2 trademarks.

## Why

One native code base for three desktop platforms, with a real 3D replay and
charts, without a second C++ code base. The decisions behind the stack are
recorded as ADRs in [`docs/decisions/`](docs/decisions/README.md).

## Status

Phases 0–4 are done.

- **Phase 4 — QML shell:** the Qt-free `rowplay-viewmodel` crate, the design-token
  theme (light/dark), the application shell, the settings screen (keyring
  token, units, home timezone, language, demo mode, threaded sync), the
  six-language i18n pipeline generated from the web locales, and the screens:
  day-sectioned library sidebar, dashboard (metric tiles, personal bests,
  Qt Graphs trend charts), workout detail (metric strip, splits/intervals,
  targets) and stroke analysis (pace, power, rate and heart-rate charts).

- **Phase 1–2 — `rowplay-core`:** the pure ports of the web app's models,
  formatting, datetime, pace input, privacy redaction, analytics, personal
  bests, performance predictor, workout query, tags and deterministic demo
  library, plus the whole replay core (engine, motion, stroke model, motion
  graph, 2D kinematics, ghost pick, race gap / result, rival parsers and the
  quality budgets), all with parity tests.
- **Phase 3 — `rowplay-platform`:** the Concept2 raw-payload mapper, the
  blocking `ureq` HTTPS client (HTTPS-only, same-origin redirects only, strict
  timeouts, 25 MiB body cap), the OS-keychain token store, the `rusqlite`
  workout cache, the JSON preferences store and the synchronous cancellable
  sync coordinator.

- **Phase 4a — shell foundation:** `rowplay-viewmodel` (Qt-free UI logic),
  `Theme.qml` (Studio's design tokens, light/dark via the system colour
  scheme), the Fusion-styled application shell, the settings screen (keyring
  token, units, home timezone, language, demo mode, threaded sync) and the
  six-language i18n pipeline generated from the web locales.

See [`docs/roadmap.md`](docs/roadmap.md) for every phase.

![Phase 0 smoke scene](docs/screenshots/phase-00-smoke.png)

| Dashboard (demo data) | Workout detail (demo data) |
| --- | --- |
| ![Dashboard](docs/screenshots/phase-04-dashboard.png) | ![Workout detail](docs/screenshots/phase-04-detail.png) |

| Settings (light) | Settings (dark) |
| --- | --- |
| ![Settings, light](docs/screenshots/phase-04-settings-light.png) | ![Settings, dark](docs/screenshots/phase-04-settings-dark.png) |

## Requirements

- Rust ≥ 1.87 (stable toolchain with `rustfmt` and `clippy`).
- Linux only, for the keyring Secret Service backend, the `libdbus-1` headers
  at build time: `libdbus-1-dev` and `pkg-config` on Debian/Ubuntu, `dbus-devel`
  and `pkgconf` on RHEL/Fedora. macOS (Keychain) and Windows (Credential
  Manager) use system frameworks and need nothing extra.
- For the app: Qt 6.11 with the Quick 3D, Shader Tools, Quick Timeline and
  Graphs modules, a C++ toolchain, and `qmake` on `PATH` (or `QMAKE` set).
  Qt-free crates build without any of that.

## Build and run

```bash
cargo test                          # core + platform + fixtures, no Qt needed
cargo build -p rowplay-app          # needs Qt
cargo run -p rowplay-app            # Phase 0 smoke window
```

No test in the default run touches the network or a real credential store: the
HTTP rules are exercised against a local `TcpListener` server and every service
has an in-memory mock. The round trip against the real OS keychain is opt-in,
because CI has no Secret Service and macOS prompts:

```bash
ROWPLAY_KEYRING_TESTS=1 cargo test -p rowplay-platform
```

Headless screenshot test on Linux (Xvfb + Mesa):

```bash
QT_QPA_PLATFORM=xcb QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
  ROWPLAY_QT_SMOKE=1 ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts \
  xvfb-run -a cargo test -p rowplay-app
```

On macOS no Xvfb is needed — the app renders through the normal window server
and writes the same artifacts:

```bash
ROWPLAY_QT_SMOKE=1 ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts \
  cargo test -p rowplay-app
```

## Installing Qt locally

CI uses `jurplel/install-qt-action`; for a local build `aqtinstall` fetches the
same archives. Install the modules the app imports and leave the **default
archives** alone: on Linux the defaults are what bring `qtwayland`, so the app
runs natively under Wayland (a restricted `--archives` list would drop it).

```bash
python3 -m venv ~/.venvs/aqtinstall
~/.venvs/aqtinstall/bin/pip install "aqtinstall==3.3.*"
# Where python3-pip/venv are not installed (minimal RHEL 10), any pip works:
# `pipx install aqtinstall` or `uv tool install aqtinstall`.

# Linux — the desktop architecture is linux_gcc_64
~/.venvs/aqtinstall/bin/aqt install-qt linux desktop 6.11.2 linux_gcc_64 \
  -m qtquick3d qtshadertools qtquicktimeline qtgraphs --outputdir ~/Qt

# macOS
~/.venvs/aqtinstall/bin/aqt install-qt mac desktop 6.11.2 clang_64 \
  -m qtquick3d qtshadertools qtquicktimeline qtgraphs --outputdir ~/Qt
```

`qmake` must be on `PATH` (or `QMAKE` set) for `build.rs` to find `rcc`:

```bash
export PATH="$HOME/Qt/6.11.2/macos/bin:$PATH"        # .../gcc_64/bin on Linux
export QMAKE="$HOME/Qt/6.11.2/macos/bin/qmake"
cargo build -p rowplay-app
```

macOS binaries reference Qt through `@rpath` with no `LC_RPATH` emitted, so
running or testing the app also needs the frameworks on the fallback path
(see note 10 in [`docs/qt-bridges-notes.md`](docs/qt-bridges-notes.md)):

```bash
export DYLD_FALLBACK_FRAMEWORK_PATH="$HOME/Qt/6.11.2/macos/lib"
```

On a Wayland desktop the app runs natively (`QT_QPA_PLATFORM` unset — Qt picks
`libqwayland`) and the headless tests need no Xvfb:
`QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1
ROWPLAY_QT_SMOKE=1 cargo test -p rowplay-app` passes on RHEL 10.2 with Qt
6.11.2 (verified; Xvfb is not even installed there).

Demo mode is first-class: everything is explorable with deterministic seeded
data and no Concept2 token.

## Layout

```
crates/rowplay-core       pure domain logic (no Qt, no I/O)
crates/rowplay-platform   services behind traits with mocks (no Qt)
crates/rowplay-viewmodel  Qt-free UI logic: navigation, dates, settings, screens
crates/rowplay-app        qtbridge binary, QML shell, Qt Quick 3D
crates/rowplay-fixtures   dev-only loader for tests/fixtures
qml/                      QML modules            tests/fixtures/  golden parity JSON
assets/  i18n/  tools/    assets, locales, pipeline scripts
docs/                     roadmap, source map, Qt Bridges notes, ADRs
.kiro/specs/              per-phase requirements / design / tasks
```

`AGENTS.md` is the canonical guide for contributors and coding agents.

## Licence

GPL-3.0-or-later (Qt Quick 3D and Qt Graphs are GPLv3-only in the open-source
Qt edition). Vendored assets and fixtures keep their original MIT / CC0 terms;
see `LICENSES/`, `ASSET_PROVENANCE.md` and `tests/fixtures/PROVENANCE.md`.
