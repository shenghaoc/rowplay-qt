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

## Post-bootstrap maintenance — 2026-09-26

- [x] Add weekly Dependabot checks for the complete root Cargo workspace and
  shared lockfile, and for GitHub Actions across `.github/workflows/`.
  Exclude only `dtolnay/rust-toolchain` from Actions updates because its refs
  select the manually synchronized Rust toolchains, including MSRV. This
  automation was added after bootstrap; the completed bootstrap tasks above
  retain their original history.
