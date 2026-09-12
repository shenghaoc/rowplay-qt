# Phase 5a — Assets and scene: tasks

- [x] T1 `tools/vendor-replay-assets.py`: copy the R1.1 file set from
  `reference/` into `assets/replay/`, verify byte counts and SHA-256 against
  the embedded expectation table, rewrite the vendored table in
  `ASSET_PROVENANCE.md` (R1.1–R1.3).
- [x] T2 `crates/rowplay-app/tests/asset_hashes.rs`: the Qt-free hash pin
  (R1.4).
- [x] T3 `crates/rowplay-app/src/replay/glb.rs` + `tests/asset_contract.rs`:
  the bounded JSON-chunk reader and the V3 validator with named-slot errors;
  the synthesised-defect fixtures (R2.3–R2.5, R6.3).
- [x] T4 `crates/rowplay-app/src/replay/assets.rs`: the Qt loader, env-var
  development path, rcc path, balsam attempt in `build.rs` with the
  `replay_asset_mode.txt` decision and the qtbridge note (R2.1, R2.2, R7.3).
- [x] T5 `rowplay_viewmodel::replay::{materials, palette, anchors}`: the role
  enum, the theme-key palette resolver with light/dark/lane/ghost, the sky
  palette bridge, the anchor table, unit tests incl. the Theme.qml key scan
  (R3.1–R3.4, R4.1, R5.1).
- [x] T6 `qml/RowPlay/Replay/`: `ReplayMaterials.qml`, `ReplayScene.qml`,
  `qmldir` entries, `rowplay.qrc` entries; the `Replay` backend singleton
  (load state, colours, gate hooks) registered under the `RowPlay` URI with
  the note #1 workaround; `Main.qml` replay route now hosts the scene
  (R4.1–R4.4).
- [x] T7 Gate: per-sport load + screenshot steps, pixel-diversity assertions,
  CI artifact upload in `.github/workflows/ci.yml` (R6.1, R6.2).
- [x] T8 Docs: `docs/source-map.md` Phase 5a rows + divergences,
  `docs/roadmap.md` status, `docs/qt-bridges-notes.md` entries, README notes
  (R7.3).
- [x] T9 Validation: fmt, clippy (workspace, Qt present), `cargo test
  --workspace`, `git diff --check`, manual `cargo run -p rowplay-app` on
  Wayland with a screenshot of each sport scene attached to the PR; gate
  member check green (R7.1, R7.2).
