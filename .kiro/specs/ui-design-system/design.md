# UI design system — design

## Layers

```
Theme.qml (tokens: palette, ramps, scale, accent, contrast)
   │
   ├── shared controls (qml/RowPlay/*.qml, Basic templates)
   │       Icon, FocusRing, ToolbarButton, PushButton, SegmentedControl,
   │       ToggleSwitch, InputField, PopupButton, FormSection / FormRow,
   │       ChartTheme, App{ToolTip, Menu, MenuItem, MenuSeparator,
   │       ScrollBar, ProgressBar, BusyIndicator, Dialog, DialogButtonBox}
   │
   ├── ApplicationWindow palette (Main.qml): the safety net for any stock
   │       Basic control, each role mapped by Basic's use of it
   │
   └── screens (Main, SidebarPanel, DashboardScreen, DetailScreen,
           SettingsScreen, LiveModePanel, Replay/*) use only the above
```

The platform layer lives in `Theme.qml` for the visual inputs (scale,
accent, contrast, scheme) and in the shell for behaviour (shortcuts, menus,
the sidebar toggle). No screen branches on the OS.

## Scale

`Theme.basePx` is the system font's pixel size: `Qt.application.font`'s
`pixelSize`, or its `pointSize` converted through the primary screen's
logical DPI. `Theme.scale = basePx / 13`. `Theme.px(ref)` rounds a
design-reference length to whole pixels, and `Theme.fontPx(ref)` does the
same for type (minimum 8 px). Every spacing, radius, control metric and font
token goes through them. At the reference, controls are 32 px, toolbars
52 px, sidebar rows 48 px and form rows 44 px.

Two different things scale a Qt UI, and the design handles both:

- **A larger system font** (the Windows text-size setting, a larger desktop
  font) changes `Qt.application.font`, so `Theme.scale` grows and the layout
  reflows around bigger controls. Reproduced on Linux with
  `QT_ENABLE_HIGHDPI_SCALING=0 QT_FONT_DPI=120` / `144` (base 15 / 18 px).
- **Display scaling** (`QT_SCALE_FACTOR`, a HiDPI screen, or `QT_FONT_DPI`
  with high-DPI scaling on) changes the device pixel ratio. Logical sizes
  stay the same and everything is drawn larger uniformly.

## Colour

- **Scheme.** `Theme.dark` follows `Qt.styleHints.colorScheme` (Unknown
  resolves to light) unless `Settings.colorSchemeOverride` pins it.
- **Contrast.** `Theme.highContrast` follows
  `Qt.styleHints.accessibility.contrastPreference === Qt.HighContrast`
  unless `Settings.contrastOverride` (`ROWPLAY_FORCE_CONTRAST`) pins it.
  Under high contrast:
  - surfaces stay opaque;
  - `separator` and `controlBorder` become ≥ 4.4:1;
  - `textSecondary` equals `textPrimary`, and `textTertiary` takes the
    normal secondary value, so placeholders stay distinct;
  - the focus ring is 3 px;
  - scroll bars stay visible.
- **Accent.** `Theme.accentColor` is `SystemPalette.accent`, unless that
  reads as Qt's built-in default `#308cc6` (the platform supplied none), in
  which case it is the brand blue `#0066CC` / `#0A84FF`. `Theme.onAccent` is
  white or `#0e1014`, whichever contrasts more with the accent: at least
  4.5:1 on the brand blues and the macOS system blue, but under it for an
  accent of luminance about 0.183–0.198 (#76). `focusRing` is
  the accent, or `textPrimary` if the accent falls below 3:1 on the window.
- **Ramps.** The values and their measured ratios are in
  `docs/source-map.md` ("Surface and text colours").

## Controls

- Built on the Basic style's templates. `FormRow.toggleTarget` is typed
  `T.AbstractButton` (`QtQuick.Templates`), because under Basic a plain
  `AbstractButton` names Basic's own composite type, which a `Switch` is not.
- `ToolbarButton` has `focusPolicy: Qt.TabFocus`, so a click keeps the focus
  where it was, while Tab reaches the button and shows its ring. It keeps
  the template's accessible role (Button, or CheckBox while checkable) and
  claims Space while focused, so a window shortcut on Space cannot take the
  key from it.
- `SegmentedControl` emits `activated(index)` and never writes
  `currentIndex`, so its binding to the store stays intact. Capped below
  its implicit width, it shows the same choices as a `PopupButton` (the
  compact form), with focus and the accessible role on the pop-up button.
  `compactWidth` is the narrowest width that still shows every choice
  whole (the pop-up button around its widest title, in its own font): a
  caller that caps the control keeps it at least that wide, and the pop-up
  button is never narrower.
- `FormRow` stacks its controls under the label when they and a 140 px
  (scaled) label column do not fit side by side.
- A binding that measures text with `FontMetrics.advanceWidth()` also reads
  the metrics' `font`, so the width follows a font that lands late or
  changes with the system font (`docs/qt-bridges-notes.md`).
- `FormRow` with a `toggleTarget`: a `TapHandler` over the row toggles the
  switch and emits `toggled()`. Focus stays on the switch.
- `AppDialogButtonBox` keeps `DialogButtonBox`'s default `buttonLayout`
  (platform order), right-aligns natural-width PushButtons, and has no
  background of its own: Basic paints `palette.window`, which differs from
  the popup surface in dark mode.
- `PopupButton` rows keep the checkmark's slot in every row (it shows by
  opacity), so the labels line up.
- `AppScrollBar` has no track. Basic's own shows only under the OS contrast
  preference, in a colour the high-contrast thumb vanishes into (1.00–1.06:1).
  `maximumThickness` (the widened thumb and its padding) is the room a
  layout reserves for the bar, so hovering it moves nothing.
- `AppProgressBar`'s indeterminate segment returns to the centre when its
  sweep stops (reduce motion): a stopped animation leaves `x` where it was.
- Parts centred inside a control round to whole pixels: an antialiased
  rounded shape at a half pixel smears on a 1x display.

## Verification harness

A scratch gallery (not committed) instantiates every control from
`qml/RowPlay/` under the `qml` runtime with a stub `Settings` singleton. It
renders light, dark and high contrast at 100 %, 125 % and 150 % text, forces
keyboard focus on each control in turn, and opens the menu, the pop-up
list, the tooltip and the dialog. Captures are taken from the X screen
(`import -window root`) so popups are included. The contrast table is
printed by the QML runtime from the real tokens (`Theme.contrastRatio`).

## Stack

| PR | Branch | Content |
| --- | --- | --- |
| 1/7 | `ui/01-fixes` | pre-existing defects, one commit and issue each (#48–#61) |
| 2/7 | `ui/02-foundation` | Basic style, tokens, scale, accent, contrast, shared controls, ADR 0013, this spec |
| 3/7 | `ui/03-shell` | sidebar, toolbar, platform layer (shortcuts, menus, sidebar toggle), empty state |
| 4/7 | `ui/04-dashboard-detail` | tile grid, charts, metric grid, splits table, no all caps |
| 5/7 | `ui/05-settings` | grouped page, logout dialog in platform order, visible notes |
| 6/7 | `ui/06-replay` | floating HUD with auto-hide, sidebar hidden during replay, keyboard |
| 7/7 | `ui/07-docs` | README screenshots, AGENTS.md, roadmap and source-map consolidation |

Each PR builds and passes the full suite on its own. Screens not yet migrated
in a given PR keep their stock controls, coloured by the palette safety net.
