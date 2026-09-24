# UI design system — requirements

rowplay-qt gets one visual design system of its own, the same on Linux,
macOS and Windows, and a thin platform-behaviour layer that holds the only
per-OS differences. This is the UI work `docs/roadmap.md` banked under "UI
follow-ups". The decision record is ADR 0013, which replaces the unmerged
Apple-HIG pass (#47). It is delivered as a stack of seven PRs
(`tasks.md`).

## R0 — Scope and invariants

- R0.1 UI and presentation only. No change to the view-model logic, the 3D
  scene, `applyFrame`, the bench code or the gate hooks, except where a
  requirement below names a Rust change, and except 1/7's pre-existing
  defects that lived in Rust (the stroke pace axis, the trend chart's date
  list and the splits table rows: #48, #51 and #59).
- R0.2 No new strings. Every `Tr.t` id is an existing web key
  (`i18n_parity`), and `i18n/*.ts` is never edited. Where Qt supplies a
  title (the macOS menu-item roles), the app does not replace it (#80).
- R0.3 Nothing is added under `assets/` (`asset_hashes`). No C++. No new
  Rust dependency. Packaging is unchanged (ADR 0012).
- R0.4 Every new source file (QML, Rust, scripts) carries the SPDX header,
  as the repository's source files do. Every new QML file is listed
  in `qml/RowPlay/qmldir` and `qml/rowplay.qrc`.
- R0.5 The QML runtime-error gate stays green: no TypeError,
  ReferenceError, binding loop or "Unable to assign", and every
  `Singleton.member` resolves. The gate walk and its captures stay intact.
- R0.6 Numbers and dates are formatted in Rust. QML may do layout
  arithmetic but never formats a data value.
- R0.7 `Accessible.name` on every control and tile. An icon-only button
  carries its label as both its accessible name and its tooltip.

## R1 — Tokens (`Theme.qml`)

- R1.1 The PM5 palette and the Metric Mapping Rule from Studio's `DESIGN.md`
  are unchanged. The metric colours never follow the accent.
- R1.2 Surfaces and text use this repository's own neutral light and dark
  ramps. Every text level passes WCAG AA (≥ 4.5:1) on every surface it is
  used on, control outlines and the focus ring reach ≥ 3:1, and text on an
  accent fill passes AA (not met for every possible system accent: #76).
  Radii, spacing and chart styling are this repository's own.
- R1.3 Every font size and length derives from the system font
  (`Qt.application.font`) as a ratio of a 13 px reference, with no
  hardcoded pixel sizes beyond the window's own size and hairline or 2 px
  strokes. Controls are about 32 px tall at the reference.
  Nothing clips at 125 % and 150 % text.
- R1.4 The colour scheme follows `Qt.styleHints.colorScheme`
  (`ROWPLAY_FORCE_COLOR_SCHEME` pins it).
- R1.5 The accent follows the system accent (`palette.accent`) for
  selection, focus rings, switch-on and prominent buttons. The brand blue is
  the fallback when the platform reports none.
- R1.6 The OS contrast preference (`Qt.styleHints.accessibility`, Qt 6.10+)
  switches to a high-contrast variant: opaque surfaces, stronger separators
  and outlines, full-contrast secondary text and a solid replay HUD
  (`ROWPLAY_FORCE_CONTRAST` pins it).

## R2 — Controls on the Basic style

- R2.1 `qml/qtquickcontrols2.conf` selects Basic. The shared controls draw
  every visual from the tokens: `Icon`, `FocusRing`, `ToolbarButton`,
  `PushButton`, `SegmentedControl`, `ToggleSwitch`, `InputField`,
  `PopupButton`, `FormSection` / `FormRow` and `ChartTheme`.
- R2.2 Every stock popup and indicator the app uses has a themed
  replacement: `AppToolTip`, `AppMenu` / `AppMenuItem` / `AppMenuSeparator`,
  `AppScrollBar`, `AppProgressBar`, `AppBusyIndicator`, `AppDialog` and
  `AppDialogButtonBox`. The `PopupButton` list is its own. The replay
  scrubber's `Slider` is themed with the replay (6/7). No raw Basic or
  Fusion look remains once the stack is complete.
- R2.3 The palette on `ApplicationWindow` sets every role from the tokens,
  mapped by how Basic uses each role, as the safety net for any stock
  control.
- R2.4 Every focusable control draws a visible focus ring on keyboard focus,
  and text fields draw it while focused. A click on a toolbar button does not
  take focus.
- R2.5 Icons are monochrome original path data in `Icon.qml`. Outside the
  toolbar a button shows an icon or a label, never both. At most one
  prominent or destructive button per view.
- R2.6 Motion (thumb slides, indeterminate progress, the busy spinner) stops
  under the reduce-motion preference.

## R3 — Shell

- R3.1 A full-height sidebar and a content area with a toolbar. Toolbar
  buttons are icon-only, with tooltips and accessible names.
- R3.2 The sport filter is a segmented control (exclusive, instantly
  applied).
- R3.3 Sidebar rows have a focused (accent) and an unfocused (neutral)
  selection, sentence-case section headers and no all caps.
- R3.4 The empty state replaces only the content area.

## R4 — Platform layer

- R4.1 Shortcuts use `StandardKey` where one exists: Preferences, Quit,
  Close, Find, Refresh and Back. The replay keys (Space, arrows, `[` `]`,
  Home / 0) stay. A sidebar toggle uses F9 on Linux and Windows and
  Ctrl+Cmd+S on macOS, and is disabled during the replay. No custom shortcut
  shadows the platform's Quit or Close.
- R4.2 macOS gets the native menu bar, with menu-item roles (Preferences,
  Quit, About), so Qt supplies the titles and key equivalents (#80).
  Windows and Linux get a menu button at the toolbar's trailing edge.
  Existing keys only (`settings.title`, `pwa.reload`, …).
- R4.3 `DialogButtonBox` keeps its default layout, so the platform orders the
  dialog buttons. No `MacLayout`.
- R4.4 Native title bars everywhere.
- R4.5 Settings stay an in-app page with no dismiss button. On macOS the
  application menu's Preferences… item (⌘,) reaches it. Back navigation,
  Escape and the toolbar toggle close it.

## R5 — Dashboard and detail

- R5.1 Balanced tile and PB grids, charts on `ChartTheme`, the metric grid
  and the splits table on the tokens.
- R5.2 No `toUpperCase()` anywhere in `qml/`, and sentence case throughout.
- R5.3 Nothing is conveyed by colour alone. Deltas and the race verdict
  carry a sign, word or glyph.

## R6 — Settings

- R6.1 A grouped page: the label on the leading edge and at most two
  controls on the trailing edge. Clicking anywhere on a row toggles its
  switch, while the control keeps the focus. Footers carry the notes.
- R6.2 Nothing is reachable only by hover: the timezone note is visible
  (#63).
- R6.3 The logout dialog uses `AppDialog` with PushButtons in the platform
  order.

## R7 — Replay

- R7.1 Playback controls float over the scene (a solid surface under high
  contrast).
- R7.2 While playing, the HUD hides after about 3 s without pointer
  movement. It reappears on pointer movement, a tap, any keyboard input or
  pause, and appears instantly under reduce motion.
- R7.3 The sidebar is hidden while the replay is shown and its width is
  restored on exit (#62). Back navigation and the keyboard work.
- R7.4 A camera framing inset is added only if it touches neither the replay
  pose, the camera maths nor their tests; otherwise auto-hide is the answer.

## R8 — Documentation

- R8.1 ADR 0013; `docs/source-map.md` rows and divergences;
  `docs/roadmap.md`; `docs/qt-bridges-notes.md` with the per-OS findings;
  this spec's tasks ticked in the PR that delivers them.
- R8.2 The AGENTS.md coding-style line names the Basic style and the shared
  controls. The README shows a single current set of screenshots in place of
  the stale `docs/screenshots/phase-04-*`, keeping repository growth small
  (ADR 0009 / 0011).

## R9 — Validation (every PR in the stack)

- R9.1 `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
  -- -D warnings`, `cargo test --workspace` under `ci.yml`'s Linux recipe
  (Xvfb + Mesa) including the gate, `i18n_parity` and `asset_hashes`, and
  `git diff --check`, all with exit status 0.
- R9.2 Before and after captures in light and dark for the screens a PR
  touches, in its PR body, with pixel statistics read before any visual
  reading.
