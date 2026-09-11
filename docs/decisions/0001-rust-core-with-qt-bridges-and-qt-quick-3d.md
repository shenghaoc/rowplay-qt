# ADR 0001 — Rust for all logic, Qt Bridges for Rust to QML, Qt Quick 3D and Qt Graphs

Status: accepted (2026-09-11)

## Context

rowplay-qt is the cross-platform (Linux, macOS, Windows) port of rowplay. The
web app (SvelteKit + three.js) is the canonical behaviour; rowplay-studio
(Swift + RealityKit) proved that the domain logic ports cleanly to a strongly
typed core with golden parity fixtures. We want one native code base for three
desktop platforms, a real-time 3D replay, and charts, without a second
hand-written C++ code base to maintain.

## Decision

- **Rust** implements all application logic (domain models, analytics, replay
  engine, sync, storage, preferences). Rust style: `cargo fmt`,
  `cargo clippy --all-targets -- -D warnings`, `#![forbid(unsafe_code)]` in
  app code, `thiserror` for library errors.
- **Qt Bridges for Rust** (`qtbridge` crate, public beta since July 2026,
  pinned to an exact version) exposes Rust objects to **QML / Qt Quick**. The
  bridge is kept thin: QML drives per-frame work through one `tick(dt)` slot
  and reads back compact properties; per-frame math lives in Rust.
- **Qt Quick 3D** renders the replay; **Qt Graphs** renders charts; **Qt 6.11**
  is the baseline.
- **No hand-written C++ in this repository.** If a feature needs a C++-only Qt
  API (for example subclassing `QQuick3DGeometry` or `QQuick3DTextureData`),
  stop and write an ADR with options (asset baking, a QML-only alternative, a
  CXX-Qt island) instead of quietly adding C++.

## Consequences

- Everything testable runs under `cargo test` without Qt (ADR 0006); only the
  app crate needs a Qt installation (`qmake` on `PATH` or `QMAKE`).
- qtbridge is newer than any model's training data: its API is read from the
  crate sources and upstream examples, never written from memory, and every
  friction point is logged in `docs/qt-bridges-notes.md` for upstream reports.
- Qt Quick 3D and Qt Graphs are GPLv3-only in the open-source edition, which
  drives ADR 0002.
- Qt resources are produced with `rcc --binary` from `build.rs` and registered
  with `qtbridge::qresource::register_bytes`, which is Qt tooling rather than
  C++ (see `docs/qt-bridges-notes.md` for why `include_bytes_qml!` is not used
  for the QML module).
