# Phase 5a — Assets and scene: tasks

- [x] T1 `tools/vendor-replay-assets.py`: copy the R1.1 file set from
  `reference/` into `assets/replay/`, verify byte counts and SHA-256 against
  the embedded expectation table, rewrite the vendored table in
  `ASSET_PROVENANCE.md`; `--emit-rust` regenerates T2's table (R1.1–R1.3).
- [x] T2 `crates/rowplay-app/tests/asset_hashes.rs`: the Qt-free hash pin,
  failing on any file under `assets/replay/` the table does not know (R1.4).
- [x] T3 `crates/rowplay-viewmodel/src/replay/glb.rs`: the bounded
  JSON-chunk reader and the V3 validator with named-slot errors; the
  synthesised-defect tests are unit tests in the same file
  (`a_valid_pack_validates`, `the_vendored_rig_pack_validates`,
  `failures_name_the_offending_slot`, `containers_are_bounded_and_typed`),
  not a separate `tests/asset_contract.rs` (R2.3–R2.5, R6.3).
- [x] T4 `crates/rowplay-app/build.rs` + `src/replay/assets.rs`: the balsam
  pipeline in every build (`balsam --removeComponentAnimations` on both
  packs, the generated `RowPlay.ReplayAssets` module bundled as
  `rowplay_replay.rcc` and registered in `main.rs`), build-time validation
  of the converted bytes into `replay_assets_meta.json`, and the startup
  re-validation of the on-disk pack in development (`ROWPLAY_REPLAY_ASSETS`,
  or `assets/replay/` in debug builds). No `replay_asset_mode.txt`, no
  `RuntimeLoader` and no GLB in the rcc (ADR 0008; the balsam outcome and
  the `RuntimeLoader` repro in `docs/qt-bridges-notes.md`) (R2.1, R2.2,
  R7.3).
- [x] T5 `rowplay_viewmodel::replay::{materials, palette, anchors}`: the role
  enum with its theme-key specs (light/dark via `Theme.qml`, lane paint via
  the venue palette, ghost opacity), the sky/ground/lane palette bridge, the
  key-light data (`sun_offset`, `SHADOW_TARGET_HEIGHT`, derived sun
  elevation/azimuth), the anchor table, unit tests incl. the Theme.qml key
  scan (R3.1–R3.4, R4.1, R5.1).
- [x] T6 `qml/RowPlay/Replay/`: `ReplayScene.qml` (static role materials
  cross-checked against the Rust spec table; a dynamic material needs a
  parent or a retained reference or it is garbage-collected — bridge notes),
  `qmldir`, `rowplay.qrc` entries; the `Replay` backend singleton
  (`crates/rowplay-app/src/backend/replay.rs`: load state, colours, key
  light, sport and scheme slots) registered under the `RowPlay` URI with the
  note #1 workaround; `Main.qml` replay route now hosts the scene
  (R4.1–R4.4).
- [x] T7 Gate: `Main.qml` steps 52–58 (route push, per-sport
  `Replay.setSport`, `replay-{row,ski,bike}` PNG + PPM twins, `Replay` in the
  member-probe registry); `crates/rowplay-app/tests/common/mod.rs`
  (`parse_ppm`, `colour_diversity`, `assert_rendered`) shared by
  `qml_runtime_gate.rs` (`replay_screenshots_render_per_sport`, `Replay` in
  `SINGLETONS`, the "was not placed in the graphics scene" pattern) and
  `smoke_screenshot.rs`; CI artifact upload of the three PNGs in
  `.github/workflows/ci.yml` (R6.1, R6.2).
- [x] T8 Docs, delivered by this PR: `docs/source-map.md` (Phase 5a section
  and divergence rows), `docs/roadmap.md` (5a delivered, 5b/5c documented),
  `docs/decisions/0008-balsam-components-not-runtimeloader.md` with its row
  in `docs/decisions/README.md`, `docs/qt-bridges-notes.md` entries,
  `README.md` (status, Phase 5a screenshots, `balsam` build requirement) and
  this spec's design/tasks/requirements outcome (R7.3).
- [x] T9 Validation (run by the main session before the PR opens):
  `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --
  -D warnings`; `cargo test --workspace`; `git diff --check`; the Wayland
  gate and screenshot run (`QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl
  LIBGL_ALWAYS_SOFTWARE=1 ROWPLAY_QT_SMOKE=1
  ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts
  ROWPLAY_SMOKE_SCREENSHOT_DIR=$PWD/artifacts cargo test -p rowplay-app`);
  manual `cargo run -p rowplay-app` on Wayland, looked at per sport, with
  the three captures (`docs/screenshots/phase-05a-replay-{row,ski,bike}.png`)
  attached to the PR; gate member check green (R7.1, R7.2).
