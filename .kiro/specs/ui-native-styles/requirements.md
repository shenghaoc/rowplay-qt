# UI native styles — requirements

The owner's follow-up to design-system round 3 (2026-09-25): standard
controls become Qt's own, per platform, and the product's identity lives in
its content. The decision record is ADR 0015, which supersedes ADR 0013's
control layer. It is delivered as a stack of pull requests (`tasks.md`).

## R0 — Scope and invariants

- R0.1 No new strings: every `Tr.t` id is an existing web key
  (`i18n_parity`), `i18n/*.ts` is never edited, and where Qt supplies a
  button's text its label is set from a web key.
- R0.2 Nothing is added under `assets/` (`asset_hashes`). No C++. No new
  Rust dependency.
- R0.3 `tools/package/**` and `release.yml` change only with the owner's
  go-ahead: packaging changes are proposed in a pull request's description.
- R0.4 The QML runtime-error gate stays green, and so do the tests, fmt,
  clippy and `i18n_parity`.
- R0.5 Numbers and dates are formatted in Rust.
- R0.6 Every control keeps its `Accessible.name`; an icon-only button
  carries its label as its accessible name and its tooltip.

## R1 — Styles

- R1.1 Qt 6.11 picks its default macOS and Fusion styles on macOS and Linux.
  A file-selected `qtquickcontrols2.conf` selects FluentWinUI3 on Windows
  (T10), because the default Windows style was unreadable under the dark
  scheme in CI's captures.
- R1.2 Linux: Fusion with the system palette, from the platform theme.
- R1.3 FluentWinUI3 is evaluated on Windows from CI captures of both
  styles, light and dark, before any per-version choice.
- R1.4 Only the two bounded content imports of `QtQuick.Controls.Basic`
  survive: `AppSlider` for the replay scrubber and `AppBusyIndicator` until
  the native macOS spinner's WebP dependency is packaged (ADR 0015).

## R2 — Standard controls

- R2.1 The shared controls give way to stock ones: `PushButton` → Button
  (`highlighted` for the prominent action); `ToolbarButton` → ToolButton in
  a ToolBar, icon and a tooltip with the shortcut's native text;
  `SegmentedControl` → ComboBox or checkable buttons in a ButtonGroup
  (design.md gives the choice per use); `ToggleSwitch` → Switch;
  `InputField` → TextField (SearchField for the search); `PopupButton` →
  ComboBox, the timezone filter kept; `FormSection`/`FormRow` → GroupBox and
  GridLayout; sidebar rows → ItemDelegate; `AppDrawer` → Drawer; dialogs →
  Dialog with `standardButtons`; menus → the native MenuBar on macOS, a
  ToolButton and Menu elsewhere.
- R2.2 No `background` or `contentItem` is replaced on a platform-drawn
  standard control, except a list delegate's row data. The Basic-fallback
  Drawer uses a structural `contentItem` host to reparent the sidebar and
  hold its in-drawer shortcuts (ADR 0015).
- R2.3 The shared controls are deleted once unused, with their `qmldir` and
  qrc entries, and `Theme.qml` keeps only content tokens.

## R3 — Content stays ours

- R3.1 The charts, metric colours, tiles and personal-best cards, the
  splits table and the 3D replay with its HUD keep their own drawing.
- R3.2 Round 2's floors hold for them: the text floor, 4.5:1 for text, 3:1
  for non-text marks, the two-tone focus ring on our own focusables, and
  Reduce Motion.

## R4 — Colours relative to the system

- R4.1 Surfaces, text and lines derive from the system palette, fitted to
  4.5:1 (text) and 3:1 (non-text) on the surfaces they are drawn on.
- R4.2 The PM5 metric colours stay the product's, fitted to 4.5:1 on their
  surfaces; they never follow the accent.
- R4.3 `Theme.dark` follows the palette in use.

## R5 — Behaviours kept

- R5.1 StandardKey shortcuts, tooltips naming them, the width classes and
  the drawer, the sync failure's Retry, the date fields' errors, the
  screen-reader names, the auto-hiding HUD and the immersive replay.

## R6 — Icons

- R6.1 A tool button uses a platform icon name on macOS (SF Symbol) and
  Windows (Segoe glyph). On Linux it uses an SVG built from `Glyphs.qml`'s
  original path data, with Qt Svg deployed in the AppImage. A nonempty
  `icon.source` takes precedence over `icon.name`; no theme lookup occurs
  for these commands. No image file is added.

## R7 — Round 3, re-scoped

- R7.1 Large and extra-large layouts for the content (2d).
- R7.2 Motion tokens for content components, under Reduce Motion (2e).
- R7.3 Content colour roles against the system palette (2a, narrowed) and
  3:1 for content marks (2c, narrowed): R4.
- R7.4 Target sizes for the replay HUD's controls (2f, narrowed).

## R8 — Verification

- R8.1 macOS natively: every screen, light and dark, 100 % and 150 % text,
  English, Spanish and Japanese, each width class, with real input; a
  non-blue accent and Increase Contrast by the owner; the native menu's
  roles.
- R8.2 Windows: CI captures of the native style in a real window
  (informational), both styles, light and dark.
- R8.3 Linux: CI captures under Xvfb with Fusion.

## R9 — Documentation

- R9.1 ADR 0015; `docs/design-system.md` with the six principles and a
  table of platform behaviour → Qt API; the AGENTS.md QML style lines; the
  source map, the roadmap and this spec; the README screenshots.
