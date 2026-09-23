# UI pass — Apple HIG: requirements

Bring the shell's look and interaction up to the Apple Human Interface
Guidelines for macOS (Sidebars, Toolbars, Segmented controls, Toggles,
Buttons, Pop-up buttons, Text fields, grouped Settings forms, Charts, Color,
Typography, Accessibility, Keyboard) on every platform, on Fusion and
`Theme.qml`. This is the UI work `docs/roadmap.md` banked under "UI
follow-ups". Decision record: ADR 0013.

## R0 — Scope and invariants

- R0.1 UI and presentation only. No behaviour change, no view-model logic
  change, no change to the 3D scene, `applyFrame`, the bench code or the gate
  hooks. The Rust changes are the Detail pace-axis labels (R5.2) and the
  defects the pass surfaced in its own screens (R8).
- R0.2 No new strings: every `Tr.t` id is an existing web key
  (`i18n_parity`); `i18n/*.ts` is not edited. Nothing is added under
  `assets/` (`asset_hashes`). No C++. Every new file carries the SPDX header
  and every new QML file is listed in `qml/RowPlay/qmldir` and
  `qml/rowplay.qrc`.
- R0.3 The QML runtime-error gate stays green (no TypeError, ReferenceError,
  binding loop, "Unable to assign"; every `Singleton.member` resolves) and the
  gate walk (`gateTimer`, `grabScreen`, the replay holds) is kept intact.
- R0.4 Numbers and dates stay formatted in Rust. QML may do layout
  arithmetic (tick spacing, column counts) but never formats a data value.
- R0.5 `Accessible.name` on every control and tile; an icon-only button
  carries its label as the accessible name and a tooltip.

## R1 — Theme

- R1.1 Surfaces and text take the macOS system colours (light / dark):
  window `#ffffff` / `#1e1e1e`, sidebar `#f4f4f6` / `#262628`, toolbar
  `#fbfbfc` / `#242426`, labels `#1d1d1f` / `#f5f5f7`, secondary `#6e6e73` /
  `#a1a1a6`, tertiary `#86868b` / `#8a8a8f`; new tokens for the focused and
  unfocused selection, separator, hover and pressed washes, control fill and
  border, segmented track and thumb, focus ring, grouped-form box, chart grid
  and axis. The PM5 palette and the Metric Mapping Rule do not change.
- R1.2 Secondary text keeps WCAG AA (≥ 4.5:1) on the surfaces it is used on;
  tertiary text is used only for placeholders and decoration.
- R1.3 Typography stays on the system font: `stripMetric` becomes 20 px
  semibold with tabular figures and no monospace family; new
  `bodyEmphasized`, `tabularBody`, `sidebarSection`, `groupTitle`, and the
  control metrics `controlHeight` 28, `toolbarHeight` 52,
  `sidebarRowHeight` 48.

## R2 — Shared controls

- R2.1 `Icon`: SF Symbols-style glyphs as SVG path data on a 16-unit grid
  (`QtQuick.Shapes`, curve renderer, one 1.5-unit round stroke, caller
  colour): transport, chevrons, reload, settings, search, sort, arrows,
  calendar, checkmark and three sport glyphs. Original path data only.
- R2.2 `ToolbarButton` (borderless icon button, hover / press wash, accent
  when checked), `PushButton` (bezel; `prominent`, `destructive`, leading
  glyph), `SegmentedControl` (equal segments, raised thumb, Left/Right keys,
  `PageTabList` / `PageTab` roles, `activated(index)` without breaking the
  caller's binding), `ToggleSwitch` (label-less), `InputField` (leading
  glyph), `PopupButton` (accent chevron badge), `FormSection` / `FormRow`
  (grouped form with inset separators, `stacked` rows) and `ChartTheme`.
- R2.3 Every focusable control draws a visible 3 px focus ring on keyboard
  focus (text fields: while focused). A click on a toolbar-style control does
  not take focus.

## R3 — Shell

- R3.1 The sidebar runs the full window height; the toolbar sits over the
  content column only; the split-view divider is a hairline with a usable drag
  area; `shellRoot` stays the grabbable item.
- R3.2 Toolbar: the sport filter as a segmented control that follows
  `Library.sportFilterIndex`; reload (disabled while syncing, Ctrl/Cmd+R) and
  a checkable settings button; a back chevron and the workout title while the
  replay route is shown.
- R3.3 Keyboard: Ctrl/Cmd+1 and Escape keep working; Ctrl/Cmd+, opens
  settings; Ctrl/Cmd+F focuses the sidebar search.
- R3.4 The empty state uses push buttons, "Explore demo" prominent.

## R4 — Sidebar

- R4.1 Search field with the magnifier (250 ms debounce kept) and a sort
  button whose menu marks the active field and direction; From/To fields; the
  match count in small secondary text, not uppercased.
- R4.2 Day headers in small bold sentence case; 48 px rows with a sport-glyph
  badge (neutral, no metric colour), the title, a date and distance subtitle
  and the tabular pace with the PB capsule; a hover wash.
- R4.3 Selection: the accent fill with white content while the list has
  keyboard focus, the neutral fill otherwise; a click focuses the list; the
  arrow / Enter / Space / Escape behaviour is unchanged.

## R5 — Dashboard, detail and charts

- R5.1 Dashboard tiles form equal-width columns (one row of four at the
  default 1200×800 window, no orphan); PB cards use the same rule; the charts
  sit in cards on `ChartTheme` with about four Y ticks, no X gridlines and
  rounded bars.
- R5.2 The stroke pace chart shows pace labels, never the negated seconds:
  `Detail.paceAxisValues` / `paceAxisLabels` from
  `rowplay_viewmodel::dashboard::pace_axis_labels(domain, 4)`, like the
  dashboard's pace chart.
- R5.3 Detail: the title with the sport in a capsule and Replay as the
  prominent button (same enabled rule); the metric strip as an equal-column
  card that never overlaps; the splits table with a header row, hairline row
  rules, right-aligned tabular numbers and receding rest rows; the stroke
  charts in a card.

## R6 — Settings

- R6.1 A grouped form at most 640 px wide in the order: title with version;
  data (demo mode); quality (segmented) and reduce motion; token (entry or
  log out, with the status line); sync (buttons, progress, result); live mode;
  unit / timezone / language pop-ups. Every `Settings` / `Sync` / `Live` call
  and every branch of the sync status text is kept.
- R6.2 No Dismiss button; `SettingsScreen.closed()` stays for `Main.qml`.

## R7 — Replay HUD

- R7.1 The opaque transport strip becomes a floating translucent HUD over the
  scene: play/pause glyph, elapsed time, a thin scrubber, total time and
  distance; the speed as a segmented control; metric chips; the race gap and
  verdict with their colour logic.
- R7.2 Every transport shortcut stays (Space, arrows, Shift+arrows, `[` `]`,
  Home/0); the error / loading overlay keeps its own way back.
- R7.3 Under the reduce-motion preference the HUD's animations (the
  segmented thumb) are off.

## R8 — Defects the pass surfaced

- R8.1 First launch showed an empty detail pane although the demo default
  workout is selected: `Detail` never received the startup selection.
- R8.2 The recent-pace chart spanned one point with no dates:
  `pace_date_texts` was never filled.
- R8.3 Split boundaries never drew: a `Repeater` cannot create a
  `LineSeries`.
- R8.4 The personal-best cards never showed: their visibility read a `count`
  that `GridLayout` does not have.
- R8.5 Tabular figures were never applied: `Font.TabularNumbers` does not
  exist in Qt 6.11.
- R8.6 The gate's "detail" capture showed the settings screen (the walk
  selected a workout while settings was still open).
- R8.7 The gate captures must show what the screen shows: translucent
  hairlines over the unpainted grab root were stored translucent (bare black
  / white in the PPM), and the replay captures carried another workout's
  title (the walk loads demo workouts into `Replay` without selecting them).

## R9 — Verification

- R9.1 Before and after captures of dashboard, detail-full, detail-nostrokes,
  settings and replay-row in light and dark, each looked at; numbers first
  (pixel statistics), eyes second (AGENTS.md).
- R9.2 A keyboard-only walk reaches the toolbar, search, list, form controls
  and HUD, each with a visible focus ring; Space plays and pauses.
- R9.3 `cargo test --workspace` (gate, smoke screenshot, asset hashes),
  `i18n_parity`, clippy `--workspace`, `cargo fmt --check` and
  `git diff --check` pass.
