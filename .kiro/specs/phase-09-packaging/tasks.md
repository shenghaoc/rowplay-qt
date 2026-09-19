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
  otool scan clean. Measured again on the `macos-26` leg of `release.yml`
  (run 35449652423, first attempt): 219 MB bundle, 97 MB dmg, launch check
  ok in 3.1 s, otool scan clean.
- [x] T5 Linux pipeline: desktop entry, AppStream metainfo,
  `tools/package/linux.sh` with pinned + hashed linuxdeploy tools, Wayland
  plugins, Xvfb launch check (R1.3, R2.3). First CI attempt failed before
  linuxdeploy ran: the script exported a bare `QMAKE=qmake`, which
  qtbridge's build script treats as a file path ("could not detect Qt");
  fixed by resolving it with `command -v` first. Second attempt reached
  linuxdeploy (release build 1m 47s, core deployment fine) and died in the
  Qt plugin's platform deployer: `EXTRA_QT_PLUGINS` is a deprecated alias
  of `EXTRA_QT_MODULES` (module names), and the deployer then died on
  `libqwayland-egl.so`, which Qt 6.11 no longer has — it ships one
  `libqwayland.so` (the runner's platform inventory, printed by the script
  from then on: eglfs, linuxfb, minimal, minimalegl, offscreen,
  vkkhrdisplay, vnc, wayland, xcb). Third attempt (run 35450474901) green
  as xcb-only while the detection still looked for the old pair: 61 MB
  AppImage, launch check ok in 2.3 s under xcb on Xvfb; the Qt plugin's
  "Missing qml module: RowPlay…" lines are the compiled-in modules and are
  not fatal. Fourth attempt (run 35450835570), with the detection
  accepting `libqwayland.so`: green — `libqwayland.so` deployed alongside
  xcb, the hand-staged graphics-integration directory's dependencies
  pulled in (`libQt6WaylandClient`, `libwayland-cursor`, `libwayland-egl`),
  62 MB AppImage, launch check ok in 3.8 s under xcb — but the AppDir
  listing showed no `wayland-shell-integration` or
  `wayland-decoration-client`: the pinned deployer does not copy them, and
  without shell integration the Wayland platform plugin cannot open a
  window. All three directories are now staged by the script (R6.6).
  Fifth attempt (run 35451035388): green with all three present in the
  AppDir alongside `libqwayland.so`, 62 MB AppImage, launch check ok in
  3.7 s under xcb on Xvfb. Native-Wayland launch remains a human check
  (T9).
  Local-build findings on the author's RHEL 10.2 machine (measured
  2026-09-20, review of this PR): with the documented `.envrc`
  environment and `NO_STRIP=1`, `linux.sh` builds a 64 MB AppImage that
  passes the launch check in 3.0 s under xcb on the native session, and
  the CI-built AppImage (run 35457858565, sha256 verified against its
  sidecar) also passes it on that machine — the bundle is portable to
  EL10. Two local-only findings, both loud: (a) the strip bundled in the
  pinned linuxdeploy cannot parse the `.relr.dyn` sections of EL10-era
  system libraries it deploys (libssl, libsystemd, …), aborting the run —
  `NO_STRIP=1` is the local workaround, recorded in the script; ubuntu
  24.04 (the release runner) is unaffected; (b) appimagetool resolved the
  bare relative `OUTPUT` name against its own process cwd (the AppImage
  runtime did not preserve the caller's), landing the AppImage in `$HOME`
  — `OUTPUT`/`LDAI_OUTPUT` are now absolute paths in the script.
- [x] T6 Windows pipeline: `packaging/windows/rowplay-qt.iss`,
  `tools/package/windows.ps1` (windeployqt → launch check → ISCC + zip)
  (R1.2). Measured on the Windows leg of `release.yml` (run 35449652423,
  first attempt): launch check ok in 2.8 s under the native platform,
  `…-setup.exe` 35 MB, `.zip` 54 MB, both with `.sha256`.
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
