# Phase 9 — Packaging: tasks

- [x] T1 Vendor the web icon (`assets/icon/`), generate `.icns` / `.ico`
  with `tools/package/gen-icons.py`, pin all four in `asset_hashes.rs`
  (`ICON_EXPECTED` + inventory walk) and `ASSET_PROVENANCE.md` (R2.1, R2.2).
  Bite proofs: a stray file and a one-byte change both fail the test.
- [x] T2 Apple rpaths from `build.rs` (`emit_apple_rpaths`); ci.yml's macOS
  test step and the README drop `DYLD_FALLBACK_FRAMEWORK_PATH`; `.envrc`
  learns the macOS install (R2.4, R4.4). Measured: both `LC_RPATH` entries
  present; release binary and `cargo test -p rowplay-app` run with no
  `DYLD_*` variable.
- [x] T3 `ROWPLAY_EXIT_AFTER_FRAMES` launch probe (`Settings.exitAfterFrames`
  + `Main.qml` `FrameAnimation`) and `tools/package/launch-check.py` (R3).
  Bite proofs on macOS: platform plugin removed → exit −6 with the
  signature; `QtQuick3D.framework` removed → QML load failure with no exit,
  caught by the timeout.
- [x] T4 macOS pipeline: `packaging/macos/Info.plist`,
  `tools/package/macos.sh` (skeleton → macdeployqt → sqldrivers prune →
  otool scan → launch check → dmg) (R1.1). Measured on this Mac (arm64, Qt
  6.11.2): 215 MB bundle, 92 MB dmg, launch check ok in 2.4 s under cocoa,
  otool scan clean.
- [ ] T5 Linux pipeline: desktop entry, AppStream metainfo,
  `tools/package/linux.sh` with pinned + hashed linuxdeploy tools, Wayland
  plugins, Xvfb launch check (R1.3, R2.3). Written; ticks when the Linux
  leg of `release.yml` is green on the Phase 9 PR (this Mac cannot run it).
- [ ] T6 Windows pipeline: `packaging/windows/rowplay-qt.iss`,
  `tools/package/windows.ps1` (windeployqt → launch check → ISCC + zip)
  (R1.2). Written; ticks when the Windows leg of `release.yml` is green.
- [x] T7 `.github/workflows/release.yml`: package matrix on PR / dispatch /
  tag, draft release with `SHA256SUMS` on tags (R4.1–R4.3).
- [x] T8 Documentation: ADR 0012 + completed ADR index, README (install,
  build, corrected Status), AGENTS.md (commands, hook, release rule),
  qt-bridges-notes #10 / #5, tools/README, roadmap (R5).
- [ ] T9 Human eyes on each packaged app before the first public release:
  open the dmg / installer / AppImage on a real machine per OS, look at the
  dashboard, detail and all three replay scenes (R6.1). The launch check
  cannot see pixels, and the gate's visual assertions run on Linux only.
  Partial, macOS (2026-09-19, measured): the deployed bundle's live window
  was screenshotted on the build Mac — dark scheme, day-sectioned sidebar
  with the demo library, detail chrome and the Replay control all drawn.
  The 3D viewport was **not** exercised (the screenshot tooling could not
  drive the Qt Quick controls in the background); the replay scenes on
  macOS and everything on Windows / Linux remain for a human.
- [ ] T10 Follow-ups, each its own PR: Developer ID signing + notarytool
  submission once an identity exists (R4.3); prune the macdeployqt QML tree
  with a launch check per removed module (R6.3); Flatpak from the same
  AppDir (R6.4); macOS x86_64 / Linux aarch64 (R6.2); in-app version
  (R6.5, blocked on note #5 or a backend property).
