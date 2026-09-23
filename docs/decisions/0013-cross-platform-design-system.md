# ADR 0013 — One cross-platform design system on Qt Quick Controls Basic, plus a thin platform-behaviour layer

Status: accepted (2026-09-23)

## Context

Phase 4 delivered the shell on Qt Quick Controls' Fusion style, coloured from
`Theme.qml` (a port of Studio's `DesignTokens`), with stock controls: a
`ComboBox` as the sport filter, text buttons in a full-width toolbar, Fusion
switches and fields loose on the settings page, letter badges for the sports
and an opaque transport strip under the replay. The author's first inspection
of the packaged app banked the UI work for later (`docs/roadmap.md`, "UI
follow-ups").

A first redesign (PR #47, never merged) made every screen follow Apple's
Human Interface Guidelines, with macOS system colours, on all three
platforms. The author redirected it. rowplay-qt is a cross-platform product
(Linux, macOS and Windows build, test and launch-check in CI; ADR 0012). It
should not imitate the Mac, and it should not look like a Linux-only tool.

The constraints that shape the answer:

- **No hand-written C++** (ADR 0001).
- **qtbridge has no `QQuickStyle` binding**, so the Quick Controls style is
  chosen in `qml/qtquickcontrols2.conf`.
- **The platform styles are one platform's look.** The macOS and Windows
  styles exist only on their own OS and are not built for customisation.
  Fusion is its own desktop look.
- **Nothing may be added under `assets/`**, whose bytes are pinned by
  `asset_hashes.rs`. SF Symbols are licensed for Apple platforms only.
- **Every user-visible string must be an existing web key.**
- **Qt 6.11 exposes what the OS prefers:** the colour scheme
  (`Qt.styleHints.colorScheme`), the accent (`palette.accent`), the contrast
  preference (`Qt.styleHints.accessibility`, Qt 6.10) and the system font
  (`Qt.application.font`).

## Decision

**1. One visual design system of our own, the same on every OS.** Apple's
HIG, Microsoft's Fluent guidance and the GNOME HIG are sources of principles
(hierarchy, grouping, spacing, focus and restraint), not of looks.

- **Layout.** A sidebar and a content area with a toolbar. The sidebar is
  hidden while the immersive replay is shown.
- **Toolbar.** Icon-only toolbar buttons, each with a tooltip and an
  accessible name.
- **Choices.** A segmented control for exclusive choices that apply
  immediately.
- **Settings.** Grouped rows: the label on the leading edge and one control
  on the trailing edge (two at most). Clicking anywhere on a row toggles its
  switch, but the control takes focus rather than the row. Footers carry the
  notes.
- **Buttons.** At most one prominent or destructive button per view. Outside
  the toolbar a button shows an icon or a label, never both.
- **Icons.** Monochrome symbols drawn from original path data in `Icon.qml`.
- **Accessibility.** Nothing is conveyed by colour alone: deltas and the race
  verdict carry a sign, word or glyph. Nothing is reachable only by hover.
  Focus rings are visible. Text is in sentence case, never all caps.
- **Replay.** The playback controls float over the replay scene.
- **Colour.** The PM5 "Erg Display" palette and the Metric Mapping Rule from
  Studio's `DESIGN.md` stay. Surfaces and text use this repository's own
  neutral light and dark ramps, which pass WCAG AA (`docs/source-map.md`).
  Radii, spacing and chart styling are also this repository's own.

**2. It is built on Qt Quick Controls' Basic style.** Basic is the style Qt
designs for full customisation, and `qtquickcontrols2.conf` forces it. The
shared controls in `qml/RowPlay/` replace every stock visual:

- `Icon`, `FocusRing`, `ToolbarButton`, `PushButton`, `SegmentedControl`,
  `ToggleSwitch`, `InputField`, `PopupButton`, `FormSection` / `FormRow`
  and `ChartTheme`;
- Basic's own popups and indicators: `AppToolTip`, `AppMenu` /
  `AppMenuItem` / `AppMenuSeparator`, `AppScrollBar`, `AppProgressBar`,
  `AppBusyIndicator`, `AppDialog` and `AppDialogButtonBox`.

Screens use these, never raw `Button`, `ComboBox`, `Switch`, `TextField`,
`Menu`, `ToolTip` or `Dialog`. `Main.qml` sets every palette role from
`Theme.qml`, mapped by how Basic uses each role, as the safety net for any
stock control.

A custom style module (a `RowPlayStyle` in `qtquickcontrols2.conf` with Basic
as its fallback) worked under the `qml` runtime and was rejected: a style
that fails to resolve in a deployed bundle falls back to Basic silently.
Explicit wrappers fail loudly at load, where the runtime gate catches them.

**3. A thin platform-behaviour layer holds the only per-OS differences.**

- **Typography and scale.** Every font size and every length is a ratio of
  the system font (`Qt.application.font`) to a 13 px design reference, with
  no hardcoded pixel sizes. Controls are about 32 px tall at the reference,
  and a larger system font scales the whole UI instead of clipping it.
- **Colour scheme** follows `Qt.styleHints.colorScheme`.
- **Accent.** The system accent (`palette.accent`) colours selection, focus
  rings, switch-on and prominent buttons, with the brand blue as the
  fallback. The metric colours never follow it. Text on an accent fill is
  white or near-black, whichever passes AA.
- **Contrast.** The OS contrast preference (`QAccessibilityHints`) switches
  to a high-contrast variant: opaque surfaces, stronger separators and
  outlines, full-contrast secondary text and a solid replay HUD.
- **Shortcuts.** `QKeySequence.StandardKey` wherever one exists
  (Preferences, Quit, Close, Find, Refresh, Back). The replay keys stay. A
  sidebar toggle uses F9 on Linux and Windows and Ctrl+Cmd+S on macOS, and
  is disabled during the replay. No custom shortcut shadows the platform's
  Quit or Close.
- **Menus.** macOS gets the native menu bar, whose items carry roles
  (Preferences, Quit, About) so the OS supplies their titles. Windows and
  Linux get a menu button at the toolbar's trailing edge. No new strings.
- **Dialog button order.** `DialogButtonBox` keeps its default layout, so the
  platform theme orders the buttons.
- **Window chrome.** Native title bars everywhere.
- **Settings** stay an in-app page with no dismiss button. On macOS the
  native Settings… item and the Preferences shortcut open it. Back
  navigation, Escape and the toolbar toggle close it.

## Consequences

- One look on Linux, macOS and Windows, with keyboard focus visible
  everywhere and no new asset, dependency or C++. Users get their platform's
  behaviour: shortcuts, menus, dialog order, text size, accent, contrast and
  title bars. They do not get Aqua, Fluent, Adwaita or Breeze visuals. That
  is deliberate.
- The shared controls are this repository's to maintain, including the
  accessibility roles a stock style would supply (the segmented control
  declares `PageTabList` / `PageTab` itself). Basic's templates
  (`QtQuick.Templates`) are the stable base. A Qt upgrade that changes a
  Basic default can change only what the palette still colours.
- `QtQuick.Shapes` is a runtime QML module (the icons and the busy
  indicator). The packaging scripts deploy what `qml/` imports (linuxdeploy's
  `QML_SOURCES_PATHS`, `macdeployqt` and `windeployqt` `-qmldir`), so it
  ships without a script change.
- **What is verified where.** Linux rendering is verified locally and by the
  runtime gate's visual assertions in CI. macOS and Windows run the gate
  offscreen in CI; their rendering, the native menu bar, the accent and the
  contrast preference need a human on those systems (tracked in #66). The
  per-OS behaviour found on the way is recorded in `docs/qt-bridges-notes.md`,
  and every divergence from the web and Studio is recorded in
  `docs/source-map.md`.
