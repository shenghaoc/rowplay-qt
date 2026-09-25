# UI native styles — design

## Base

The stack starts from the round-2 stack's top with every review fix
composed (`stack/round2-composed`): the tree `main` has once #100–#106 are
merged. The round-2 branches on GitHub carry their review fixes as added
commits, so the top branch's own tree lacks the lower fixes; building on it
would conflict with them at merge time.

## Stack

1. `ui/native-style-switch`: no forced style; `Main.qml` sets no palette
   role; the shared controls are pinned to Basic by their import until
   replaced; `Theme`'s surfaces, text and lines from the system palette
   with contrast fitting (R4); the screens' scroll bars are the style's;
   CI captures the Windows styles; ADR 0015; the packaging proposal.
2. Shell and toolbar: ToolBar and ToolButtons with icons, the app menu,
   tooltips, the sport filter, the drawer and the dialogs.
3. Sidebar: ItemDelegate rows, the SearchField, the date fields, the sort
   menu.
4. Settings: GroupBox and GridLayout, Switch, ComboBox (quality,
   distance unit, timezone with its filter), buttons, progress.
5. Detail and dashboard: buttons, the chart controls.
6. Replay HUD: the transport's buttons and slider, the speed buttons, the
   target sizes (R7.4).
7. Theme cleanup: the shared controls and their tokens deleted; content
   tokens only; motion tokens for content (R7.2).
8. Large and extra-large layouts (R7.1).
9. Documentation (R9).

## SegmentedControl, per use

- **The sport filter and the render quality: ComboBox.** Each is a filter
  or a setting changed now and then. A ComboBox is the platform's own
  pop-up button (NSPopUpButton on macOS), one tab stop, and read as a combo
  box with its value. A TabBar would claim a page change, and the macOS
  and Windows styles do not draw one (it falls back to Fusion).
- **The replay speed: checkable tool buttons in an exclusive ButtonGroup.**
  It is changed during playback, and one click with every speed in view
  matters there, as the web's row of speed buttons does.

## Icons

`Glyphs` (a singleton) holds the glyph paths `Icon.qml` draws. macOS and
Windows use platform icon names. Linux uses a `data:image/svg+xml` source
built from the same paths. The nonempty source takes precedence over the
freedesktop name, so the SVG is the icon rather than a theme fallback. The
AppImage deploys Qt Svg's image-format plugin for it (ADR 0015).

## Colours

`Theme.sysWindow` and `Theme.sysText` are the system palette's window and
window-text colours, composited opaque (macOS reports text with alpha).
Surfaces are the window and tonal steps toward the text (sidebar 4 %, groups
5 %); text levels are steps back toward the window, fitted by
`fitContrast` until they reach their floor on every text surface; lines and
chart axes are fitted to 3:1. The metric colours go through
`paletteColour`: fitted to 4.5:1 normally, `hcFit` under high contrast.

## Measured on macOS 27 (Qt 6.11.2, cocoa)

The system palette: light window `#FFFFFF`, window text `#D8000000`, base
`#FFFFFF`, mid `#B6B6B6`, accent `#0A60FF`; dark window `#1E1E1E`, window
text `#D8FFFFFF`, base `#171717`, alternate base `#8E8E8E` (not usable as a
surface). The PM5 blue on the dark window's cards measured 4.1:1 before
fitting.
