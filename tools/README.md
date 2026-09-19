# tools/

Pipeline scripts. Nothing here is linked into the app.

| Script | Purpose |
| --- | --- |
| `vendor-fixtures.py` | Copy rowplay-studio's golden parity fixtures into `tests/fixtures/` and rewrite `manifest.json` (locally generated fixtures are re-hashed in place; see its `LOCAL_FIXTURES`). |
| `gen-rig-phase-parity.mjs` (+ `gen-rig-phase-resolve.mjs`) | Regenerate `tests/fixtures/replay-rig-phase-parity.json`: evaluates the web avatar factories in `reference/rowplay` at the pinned commit under Node (`pnpm install` there first; Node ≥ 23.6 with `--experimental-transform-types`). Parity audit stage 2, `docs/parity-coverage.md`. |
| `gen-row-phase-parity.mjs` | Regenerate `tests/fixtures/replay-row-phase-parity.json`: pins the rower authored stroke-phase calibration from the web at its pinned commit (Node ≥ 23.6; needs the web checkout's `node_modules` for three.js). |
| `gen-stroke-model-parity.mjs` | Regenerate `tests/fixtures/replay-stroke-model-parity.json`: imports the web's real `strokeModel.ts` at the pinned commit and records `buildStrokeTimeline` / `strokePoseAt` / `fallbackStrokePose` outputs over nine timelines and a boundary-inclusive query sweep (Node ≥ 23.6; no node_modules needed). Parity audit ranking 5. |
| `package/macos.sh`, `package/linux.sh`, `package/windows.ps1` | Phase 9 packaging (ADR 0012): release build → platform bundle (`macdeployqt` / `linuxdeploy` + Qt plugin / `windeployqt` + Inno Setup) → launch check → `.dmg` / `.AppImage` / installer + zip under `dist/`, each with a `.sha256`. `.github/workflows/release.yml` runs all three. |
| `package/launch-check.py` | Starts a packaged binary from a clean environment (no `PATH`, `DYLD_*`, `LD_LIBRARY_PATH`, `QT_*`), with `ROWPLAY_EXIT_AFTER_FRAMES=30`, and fails on a non-zero exit, a timeout, or a loader / QML failure signature. Shared by the three package scripts. |
| `package/gen-icons.py` | Derives the committed `assets/icon/rowplay-qt.icns` / `.ico` from the vendored web icon with Pillow (`--check` compares bytes); both outputs are SHA-256-pinned by `asset_hashes.rs`. |
| `convert-locales.mjs` | Regenerate the ID-based Qt `.ts` catalogues in `i18n/` from the web locales in `reference/rowplay/src/lib/locales/` (`--check` verifies the committed files; needs Node ≥ 23.6 for type stripping). |

Later phases add asset vendoring and `balsam` pre-processing (Phase 5), and
the Node venue exporter plus Blender clean-up scripts (Phase 6).
