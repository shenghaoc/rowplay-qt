# Phase 0 — Bootstrap: tasks

- [x] Download rowplay and rowplay-studio at `main` into git-ignored `reference/`; record SHAs in `docs/source-map.md`.
- [x] Read the qtbridge crate sources and `qt/qtbridge-rust-examples`; pin `qtbridge = "=0.2.0"`.
- [x] Verify the toolchain (Qt 6.11.2, Quick 3D + Helpers, rustc 1.94.1).
- [x] Create the Cargo workspace, licences, SPDX headers, agent shims, `.gitignore`.
- [x] Stack smoke test: Rust backend singleton, QML window, `View3D` with procedural-sky IBL, filmic tonemapping, shadow-casting sun.
- [x] Headless screenshot test under Xvfb + Mesa; document why `offscreen` cannot render Quick 3D.
- [x] CI: Qt-free lint/test job, MSRV check, three-OS app matrix with `jurplel/install-qt-action`, Linux screenshot artifact.
- [x] Docs: README, ADRs 0001–0006, roadmap, source map, Qt Bridges notes, asset provenance, PR template, `AGENTS.md`.
- [x] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`, `git diff --check`.
