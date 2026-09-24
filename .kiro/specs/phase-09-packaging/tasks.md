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
- [x] T9 Human eyes on the distributed artifact before the first public
  release: open the AppImage on a real Linux machine (X11 and Wayland),
  look at the dashboard, detail and all three replay scenes (R6.1). The
  launch check cannot see pixels. macOS and Windows are not distributed, so
  their human checks are closed as **won't-do** with the distribution
  policy as the reason *(Superseded on 2026-09-24 by ADR 0014: macOS was checked natively,
  #66; Windows has no person to check it, #81)* — the macOS partial look below stays on record but
  blocks nothing.
  Author inspection (2026-09-20, recorded from the author's report): the
  author (Shenghao Chen) opened the packaged AppImage on this machine —
  RHEL 10.2, GNOME on Wayland — the release-run 35457858565 artifact
  (`rowplay-qt-0.1.0-linux-x86_64.AppImage`, sha256 `7b9b803a…3298e7`,
  sidecar-verified). What he reports seeing, and all that is recorded:
  the default **light** first launch showing the styled empty state — a
  Replay button, the heading "No stroke data" and the subtext "No
  per-stroke sample at this time." He then opened the replays of **all
  three sports — rower, SkiErg and bike — in the packaged AppImage**
  (his word as the evidence; the review holds no pixel capture of
  those scenes on the packaged binary). His inspection observations —
  not everything is ported, loading feels slow, the animation feels
  jerky at times — are recorded with the jerkiness analysis in
  `docs/roadmap.md` UI follow-ups. His screenshot also corrects an
  earlier review claim recorded here: the first-launch main area is
  **not blank** — it is the styled empty state above; the earlier
  "completely empty main area" reading came from coarse luminance
  sampling, which reads sparse text on a plain background as flat. A
  fresh light-theme capture of the same first-launch state is retained
  as `artifacts/t9/packaged-first-launch.png` (the review's earlier
  captures forced dark via `ROWPLAY_FORCE_COLOR_SCHEME`).
  The review's own pixel captures independently verify the packaged
  dashboard and the detail screen (title, date, metric tiles,
  split-breakdown charts, an enabled Replay button) against a fresh
  debug build of the same source (`artifacts/t9/packaged-detail.png`);
  that evidence is the review agent's, not the author's, and nothing
  beyond the report above is attributed to him.
  Synthetic-input record (method limits, claim narrowed to what was
  demonstrated): the X11 routes are refused on this machine — GNOME
  denies XTEST pointer faking, and `XSendEvent` **button** events are
  ignored by Qt xcb under XWayland while `XSendEvent` **key** events do
  arrive (measured) — `/dev/uinput` is root-gated and no nested X server
  is installed. The conclusion "synthetic input is impossible" was first
  recorded before the Wayland-native and accessibility routes had been
  tried; of those, AT-SPI was subsequently exercised (below) and the
  RemoteDesktop portal (`org.freedesktop.portal.RemoteDesktop`, libei
  back-end) remains **untried** — it requires one author consent dialog.
  Accessibility (measured, screen-reader audit): Qt 6.11 ships **no**
  `plugins/accessiblebridge/` directory at all — that is the Qt 5
  layout; the AT-SPI support is compiled into `libQt6Gui`, and the
  bundle's copy carries exactly the same AT-SPI string count as the
  installed Qt (74 each), so the distributed artifact has **no**
  accessibility-plugin gap of the missing-bridge kind. Running the
  packaged binary on the GNOME session with no environment flag, the app
  exposes a usable AT-SPI tree: named list items carrying the full
  workout summary ("RowErg 2000m test; May 27, 2026; 2.00 km;
  1:44.7/500m"), named combo boxes and check boxes, and the Replay
  button with role `button`, name `Replay` and a `Press` action — a
  screen reader can see and act on the distributed app.
  AT-SPI drive attempt (recorded as a follow-up seed): invoking `Press`
  on the Replay button over AT-SPI returned success; the app then
  re-created its X window and the main pane rendered blank white while
  the sidebar stayed intact — the 3D scene was not captured and the
  state was not diagnosed within budget. Building an
  accessibility-driven gate walk against the packaged artifact (instead
  of a debug build) would close the Quick 3D deployment boundary
  permanently rather than once per release by hand; left as a follow-up.
  Captures retained under `artifacts/t9/` (gitignored).
  Done, Linux (2026-09-19, measured on RHEL 10.2, GNOME 49.4 Wayland,
  Intel UHD 630 / Mesa 25.2.7, 1920x1080@144): the PR AppImage
  (`rowplay-qt-0.1.0-linux-x86_64.AppImage`, sha256 verified, no
  `QT_QPA_PLATFORM` forcing in `AppRun`) renders 300 frames and exits 0
  under native Wayland with `libqwayland.so` plus all three `wayland-*`
  plugin directories loaded from the bundle; the xdg_toplevel maps at
  1202x840 and presents frames with compositor frame callbacks delivered.
  Under X11 (`QT_QPA_PLATFORM=xcb`) it renders 60 frames and exits 0, and
  the window is a normal managed window on the current desktop. The debug
  build's gate walk under the same native Wayland session captures all
  screens with zero QML errors, and the row/ski/bike replay viewports show
  complete scenes (athlete, equipment, venue, shadows) — no black
  viewports; the review's gate re-walk measured the three venues'
  viewports physically (water ≈69, snow ≈196, road ≈124 mean-luma, σ
  10–45 — no flat or black viewport). Native-Wayland frame medians are re-measured in the same
  session (see the Task 2 report).
  Partial, macOS (2026-09-19, measured): the deployed bundle's live window
  was screenshotted on the build Mac — dark scheme, day-sectioned sidebar
  with the demo library, detail chrome and the Replay control all drawn.
  The 3D viewport was **not** exercised (the screenshot tooling could not
  drive the Qt Quick controls in the background); superseded by the
  won't-do above.
- [ ] T10 Follow-ups, each its own PR: prune the macdeployqt QML tree
  with a launch check per removed module (R6.3); Flatpak from the same
  AppDir (R6.4); macOS x86_64 / Linux aarch64 (R6.2). Developer ID signing +
  notarytool submission (R4.3) is closed as **won't-do** with the rest of
  the macOS distribution scope *(Superseded on 2026-09-24 by ADR 0014: an open decision for the
  author, with Windows code signing)*. In-app version (R6.5) is done in this PR:
  `Settings.appVersion` on the Settings screen through the
  `settings.appVersion` supplement key.
