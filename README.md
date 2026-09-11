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

Phase 0 (bootstrap) and Phase 1 (core parity foundation) are done: the
workspace, the stack smoke test (Rust backend → QML → Qt Quick 3D scene lit by
a procedural sky), CI, and the pure-Rust ports of the web app's models,
formatting, datetime, pace input, privacy redaction, analytics, personal bests,
performance predictor, workout query, tags and deterministic demo library, all
with parity tests. The app itself only shows the smoke scene so far. See
[`docs/roadmap.md`](docs/roadmap.md) for every phase.

![Phase 0 smoke scene](docs/screenshots/phase-00-smoke.png)

## Requirements

- Rust ≥ 1.87 (stable toolchain with `rustfmt` and `clippy`).
- For the app: Qt 6.11 with the Quick 3D, Shader Tools, Quick Timeline and
  Graphs modules, a C++ toolchain, and `qmake` on `PATH` (or `QMAKE` set).
  Qt-free crates build without any of that.

## Build and run

```bash
cargo test                          # core + platform + fixtures, no Qt needed
cargo build -p rowplay-app          # needs Qt
cargo run -p rowplay-app            # Phase 0 smoke window
```

Headless screenshot test on Linux (Xvfb + Mesa):

```bash
QT_QPA_PLATFORM=xcb QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
  ROWPLAY_QT_SMOKE=1 ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts \
  xvfb-run -a cargo test -p rowplay-app
```

Demo mode is first-class: everything is explorable with deterministic seeded
data and no Concept2 token.

## Layout

```
crates/rowplay-core       pure domain logic (no Qt, no I/O)
crates/rowplay-platform   services behind traits with mocks (no Qt)
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
