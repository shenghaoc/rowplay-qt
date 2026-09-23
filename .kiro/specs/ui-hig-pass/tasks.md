# UI pass — Apple HIG: tasks

One PR, six commits: theme and controls; shell and sidebar; dashboard, detail
and charts (with the Rust pace-axis fix); settings; replay HUD; gate fix and
docs.

- [x] T0 Baseline: the gate walk under Xvfb + Mesa with
  `ROWPLAY_FORCE_COLOR_SCHEME=light` and `dark`, before any change; captures
  kept as `docs/screenshots/hig-pass/before-*.png`. Pixel statistics first:
  `detail.png` matched `settings.png` (mean absolute difference 0.30 light,
  0.23 dark) — the R8.6 gate quirk, established from the bytes before any
  visual reading.
- [x] T1 Theme tokens (R1) and the shared controls (R2), registered in
  `qmldir` and the qrc; every control rendered in a scratch harness in both
  schemes with each focus ring forced on (R2.3) before any screen used it.
- [x] T2 Shell (R3): full-height sidebar, toolbar over the content column,
  hairline divider with a 9 px drag mask, segmented sport filter bound to
  `Library.sportFilterIndex`, icon buttons, Ctrl/Cmd+, and Ctrl/Cmd+F; the
  startup selection handed to `Detail` (R8.1).
- [x] T3 Sidebar (R4): search + sort menu, date fields, count line, glyph
  badges, sentence-case headers, focused / unfocused selection; keyboard
  walk: search → sort → From → To → list → sport filter → reload → settings,
  each with a ring; Down selects with the accent fill, Tab away turns it
  neutral.
- [x] T4 Dashboard, detail and charts (R5): `Detail.paceAxisValues` /
  `paceAxisLabels` (R5.2, with a view-model test on the stroke domain);
  equal-width tile and PB grids; chart cards on `ChartTheme`; the Y-label
  overflow handling for Qt Graphs' fixed 40 px column; the metric-strip card;
  the splits table; R8.2 (`pace_date_texts`), R8.3 (split boundaries), R8.4
  (PB section) and R8.5 (`tnum`).
- [x] T5 Settings (R6): the grouped form, `LiveModePanel` as a section, no
  Dismiss button, the logout dialog on PushButtons in the HIG order (rendered
  in the harness in both schemes: the gate never opens it and demo mode has
  no token). Keyboard walk: demo switch → quality → reduce motion → token
  field → live-mode switch, disabled buttons skipped.
- [x] T6 Replay HUD (R7): the floating capsule, the thin scrubber, speed as a
  segmented control that claims Left/Right while focused, metric chips, the
  verdict line; the toolbar's back chevron and title; Space toggles playback
  (checked by driving the app with xdotool).
- [x] T7 Gate case 12 sets `screenIndex = 1` with the selection so `detail.png`
  is the detail screen (R8.6): `detail.png` against `settings.png` went from
  a mean absolute difference of 0.30 to 8.88 (light). R8.7, found in the
  after captures: `shellRoot` painted in the window colour (the hairlines
  had been stored translucent — bare black / white in the PPM; an `xwd`
  capture of the screen showed 229 grey), and the toolbar title guarded on
  `Replay.workoutId`.
- [x] T8 Docs: ADR 0013, `docs/source-map.md` rows and divergences,
  `docs/roadmap.md`, `docs/qt-bridges-notes.md` (Qt and Qt Graphs findings),
  AGENTS.md (shared controls), this spec, README screenshots.
- [x] T9 Validation (R9): after captures in both schemes, looked at (pixel
  statistics first: no near-identical screen pairs, every pixel opaque, the
  hairlines 229 / 53 grey); `cargo test --workspace` with CI's recipe (Xvfb
  + Mesa, `ROWPLAY_QT_SMOKE=1`, phase shots and close-ups): exit 0, 582
  passed, 0 failed, 2 ignored — gate 426 s, smoke, asset hashes, the
  viewport self-test and `i18n_parity` (with `reference/`) green; clippy
  `--workspace --all-targets -D warnings`, `cargo fmt --check` and
  `git diff --check` clean. `qmlimportscanner` over `qml/` resolves
  `QtQuick.Shapes` (`qmlshapesplugin`), the import scan all three packaging
  scripts deploy from; `tools/package/linux.sh` put
  `usr/qml/QtQuick/Shapes/libqmlshapesplugin.so` into the AppImage, whose
  launch check rendered 30 frames and exited 0 (xcb, Xvfb + Mesa). macOS
  and Windows packaging was not run (`release.yml` does not trigger on
  these paths).
