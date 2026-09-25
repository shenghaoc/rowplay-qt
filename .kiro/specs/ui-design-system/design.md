# UI design system — design

## Layers

```
Theme.qml (tokens: palette, ramps, scale, accent, contrast)
   │
   ├── shared controls (qml/RowPlay/*.qml, Basic templates)
   │       Icon, FocusRing, ToolbarButton, PushButton, SegmentedControl,
   │       ToggleSwitch, InputField, PopupButton, FormSection / FormRow,
   │       ChartTheme, AppSlider (6/7), App{ToolTip, Menu, MenuItem, MenuSeparator,
   │       ScrollBar, ProgressBar, BusyIndicator, Dialog, DialogButtonBox,
   │       Drawer (round 2)}
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
same for type, never below `Theme.textFloor` (round 2): 12 px on Windows and
Linux, where the system font is 12 px and Windows' minimum for body text is
12 px, 11 px (11 pt) on macOS, and 12 px in Chinese and Japanese on every
platform. On a 12 px system font the captions, chart labels and the
segmented control's text stop at the floor, the body's size, so a caption
the floor lifts (`Theme.floored`) drops from medium to regular weight and
keeps the secondary colour. Every spacing, radius, control metric and font
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

## Width classes (round 2)

`Theme.widthClass(width)` maps the window's width to compact (below
`Theme.breakpointMedium`, 600 px at the reference), medium (below
`Theme.breakpointLarge`, 840 px) or large. The breakpoints are HarmonyOS's,
as design-reference lengths, so larger text reaches the narrower layouts in
a wider window (at 150 % text, 900 and 1260 px).

- **Large** is the full layout: the sidebar in the `SplitView` beside the
  content and the segmented sport filter.
- **Medium.** `Main.placeSidebar()` takes the one `SidebarPanel` out of the
  split view (`takeItem`) into `AppDrawer`'s content item, so its search
  text, date range and scroll position go with it, and puts it back
  (`insertItem`) at large. The drawer is modal, as wide as the sidebar's
  preferred width but at most the window's less 56 px, beside a strip of
  the dimmed content. A click on the strip, Escape or the sidebar toggle
  closes it. `AppDrawer` stays interactive, because Qt 6.11 closes a
  non-interactive popup neither on Escape nor on a click outside; it sets
  `dragMargin: 0` instead, so no drag at the window's edge opens it.
- **Compact.** The same drawer is the list's page: under the toolbar, the
  window's full width, with no dim and no edge rule. It is neither modal
  nor `CloseOnEscape`, because Qt 6.11 blocks every window shortcut outside
  a popup that is either (`docs/qt-bridges-notes.md`). The toolbar and the
  shell's shortcuts therefore stay live, and the drawer button, the sidebar
  toggle, Escape (the shell's shortcut closes the drawer first), settings
  and the menu's commands close the page. The window shows one column at a
  time.
- A leading `ToolbarButton` opens the drawer (the `sidebar.left` glyph,
  named with the web's `dashboard.sectionWorkoutsEyebrow`, checked while
  the list shows), and so do the sidebar toggle (F9 / Ctrl+Cmd+S) and Find.
  The modal drawer blocks the shell's shortcuts, so the toggle and Find
  have their own `Shortcut`s inside it, enabled only while it is modal and
  open: two enabled shortcuts on one key are ambiguous, and neither fires.
- The list takes the keyboard when the drawer opens from the keyboard (the
  toggle, or the button with visual focus). A click leaves the focus with
  the drawer, so it paints no ring on the list. When the drawer closes, Qt
  gives the focus back to the item that had it.
- `SidebarPanel.workoutChosen` (a click, Enter or Space, but not the arrows,
  whose selection follows focus) closes the drawer and shows the workout,
  over settings too. In the drawer, Escape in the list closes the drawer
  instead of clearing the selection.
- Below large the sport filter takes its compact form, and
  `toolbarMinimumWidth` counts the drawer button. An open drawer closes
  when the class changes between medium and compact, and before the
  sidebar goes back into the split view.
- Grids drop columns with the width: `balancedColumns` allows one column
  where two do not fit (a sidebar dragged wide beside a narrow window). The
  detail's rate and heart-rate charts stand side by side while each keeps
  240 px, and the replay HUD's speed and chips stack where they do not fit
  side by side (see Replay).
- The minimum window is 480 × 480, the width scaled with the text
  (`Theme.px(480)`), or wider where the toolbar needs it.

## Colour

- **Scheme.** `Theme.dark` follows `Qt.styleHints.colorScheme` (Unknown
  resolves to light) unless `Settings.colorSchemeOverride` pins it.
- **Contrast.** `Theme.highContrast` follows
  `Qt.styleHints.accessibility.contrastPreference === Qt.HighContrast`
  unless `Settings.contrastOverride` (`ROWPLAY_FORCE_CONTRAST`) pins it.
  Under high contrast every role comes from the system palette (round 2;
  the tonal ramps are not used):
  - `SystemPalette` (Active, and Disabled for grey text), each role
    composited to an opaque colour (`Theme.over`): macOS reports its text
    roles with alpha, window text at 85 %;
  - surfaces (window, toolbar, sidebar, groups, cards, popups, the HUD)
    are `window`; controls, segment tracks and off switches `button`;
  - primary and secondary text, `separator` and chart axes `windowText`;
    control outlines and the slider or off-switch knob `buttonText`;
  - content on a control's own fill (a button's or a field's text, an
    unselected segment, the pop-up button's value and chevron, the sport
    capsule, the palette's `text` and `buttonText`) `buttonText`
    (`Theme.controlText`), because a contrast theme may set ButtonText
    apart from WindowText; a destructive label falls back to it too;
  - the accent's roles (selection, the selected segment, switch-on, the
    prominent button, the slider fill) `highlight` with `highlightedText`;
    a prominent button and an on switch keep their outline, because the
    highlight can sit close to the window (1.64:1 in macOS's light palette);
  - an unfocused sidebar selection keeps the window fill inside a 2 px
    highlight outline;
  - placeholders `placeholderText` where it reaches 4.5:1 in a field,
    else the button text; disabled text the Disabled group's window text;
  - the PM5 metric and status colours stay where they reach 4.5:1 on the
    window (`Theme.hcFit`), else the window text;
  - hover and pressed washes are the window text at 12 % and 24 %, chart
    grid lines at 40 %;
  - 2 px outlines where two surfaces became the same colour
    (`cardBorderWidth`, `outlineWidth`, `ruleWidth`): the sidebar's edge
    and the toolbar's rule, cards and chart panels, grouped forms,
    popups, dialogs, tooltips and the HUD;
  - `Theme.dark` follows the palette's window colour, so the scheme always
    matches the colours drawn; `ROWPLAY_FORCE_COLOR_SCHEME` sets
    `Qt.styleHints.colorScheme` at startup, which the macOS palette
    follows;
  - scroll bars stay visible.
- **Focus ring** (round 2). `FocusRing` draws two bands outside its
  control, following its radius: `focusRingInnerWidth` (1 px) in
  `focusRingInner` (the window colour) against the control, then
  `focusRingWidth` (2 px) in `focusRing` (the accent; the text colour under
  high contrast or below 3:1). `focusRingExtent` (3 px) is how far it
  reaches, which a layout that clips reserves.
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
  key from it. Its `shortcutText` (round 2), a `Shortcut`'s `nativeText`,
  follows the label in the tooltip: "Reload (⌘R)" on macOS, "Reload (F5)"
  on Windows, the drawer's "Workouts (⌃⌘S)", the replay's "Close (⎋)" and
  "Play (Space)". The accessible name stays the label.
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
  opacity), so the labels line up. A `filterable` one (round 2, the
  timezone picker) opens with an `InputField` above its list, which holds
  the keyboard: Up / Down / Page Up / Page Down move the highlighted row,
  Enter chooses it, Escape closes, and typing on the closed button opens
  the list with that text (Space still opens it empty). Screen readers
  follow the focus, which stays in the field, so the row the keys or the
  filter highlight is announced (`Accessible.announce`, Qt 6.8). The
  caller maps
  `filterText` to `filterIndices` in the view-model
  (`settings::timezone_matches`: label or zone name, case-insensitive,
  `-` for the labels' `−`, the query bounded at 64 characters), and the
  list draws those rows like the plain ones. The ComboBox keeps its own
  keyboard behaviour while closed, and gets the focus back on close.
- `InputField.invalid` (round 2) draws the outline 2 px in the alert
  colour: the field itself says it was refused, and the caller puts the
  reason under it and in its `Accessible.description`.
- `AppScrollBar` has no track. Basic's own shows only under the OS contrast
  preference, in a colour the high-contrast thumb vanishes into (1.00–1.06:1).
  `maximumThickness` (the widened thumb and its padding) is the room a
  layout reserves for the bar, so hovering it moves nothing.
- `AppProgressBar`'s indeterminate segment returns to the centre when its
  sweep stops (reduce motion): a stopped animation leaves `x` where it was.
- Parts centred inside a control round to whole pixels: an antialiased
  rounded shape at a half pixel smears on a 1x display.
- `AppSlider` (6/7, the replay scrubber) draws a thin track with the accent
  fill and shows its knob on hover, press and keyboard focus, with the
  focus ring around it.

## Errors and status (round 2)

- A failed sync's status row is labelled "Sync failed" (`sync.failed`)
  above the error it met (`sync.errorHint`, which is the error alone) in
  the alert colour, with "Retry sync" (`sync.retry`) in the row:
  `Sync.retry()` starts the last sync again in its mode. Demo mode shows
  its own line instead, so neither the red nor the retry appears there.
- The date range validates each field on its own (`Library.isDayKey`, the
  view-model's `normalize_day_key`). A refused one takes the `invalid`
  outline, and under it sits "Please try again." (`common.tryAgain`) with
  the form it takes, today's date in the home timezone
  (`Library.dayKeyExample`). The web has no string for an invalid date:
  its inputs are native date pickers.
- No success opens a dialog: syncs, token changes and live-mode checks
  report on their own rows. The two dialogs are the logout confirmation
  and About.

## Replay

- The scene fills the route. The HUD is one opaque panel on the grouped
  surface (`Theme.overlayBackground`), centred at the bottom.
- `hudShown` drives the HUD's opacity: a 200 ms fade, none under reduce
  motion. The HUD may hide only while `hudMayHide` holds: the route is
  shown, the replay is playing, and keyboard focus is not inside the HUD
  (`hudHasFocus` walks up from `Window.activeFocusItem`). `wakeHud()`
  shows the HUD and restarts the 3 s `Timer` while `hudMayHide` holds, and
  stops it otherwise. The timer has no `running` binding, because
  `Timer.restart()` starts a stopped timer whatever the binding says. It
  checks `hudMayHide` again when it fires. `wakeHud()` runs on every change
  of `hudMayHide`, pointer moves, taps, the replay shortcuts and focus
  changes. The pointer hides with the HUD (`HoverHandler.cursorShape`).
- The metric chips' `Repeater` has a constant model (caption id and metric
  role), and each chip reads its value by index, so frames update the
  labels in place instead of rebuilding the delegates.
- The speed and the chips share a `GridLayout` that drops to one column
  where the two do not fit side by side (a compact window). It returns to
  two only with 24 px (scaled) to spare, so a value that widens during
  playback cannot flip it back and forth. In the transport row the times
  and the distance keep their width, and the scrubber gives way down to
  64 px.
- Pointer moves are compared by position (at least 1 px from the last one
  acted on), because Qt Quick sends a synthetic hover after every animated
  frame (`docs/qt-bridges-notes.md`).
- The toolbar shows the way back (named "Close") and the workout's title,
  and hides the sport filter and the trailing buttons. `SplitView` keeps the
  hidden sidebar's width.
- No camera framing inset (R7.4): auto-hide keeps the scene clear, and the
  pose, the chase camera and their tests stay untouched.

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
