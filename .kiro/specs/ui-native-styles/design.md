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
5. Detail and dashboard: the Replay button (the only control either page
   has; the charts have none) and the pages' margins inside their scroll
   views.
6. Replay HUD: the play button and the speed buttons (the scrubber stays
   the HUD's own, "The HUD" below), the target sizes (R7.4).
7. Theme cleanup: the shared controls and their tokens deleted; content
   tokens only; motion tokens for content (R7.2).
8. Large and extra-large layouts (R7.1).
9. Documentation (R9).

## SegmentedControl, per use

- **The sport filter and the render quality: ComboBox.** Each is a filter
  or a setting changed now and then. A ComboBox is the platform's own
  pop-up button (NSPopUpButton on macOS), one tab stop, and read as a combo
  box with its value. A TabBar would claim a page change, and the macOS
  and Windows styles do not draw one (it falls back to Basic's).
- **The replay speed: checkable tool buttons in an exclusive ButtonGroup.**
  It is changed during playback, and one click with every speed in view
  matters there, as the web's row of speed buttons does.

## Settings rows

- **A row is a label beside its control.** The page is the style's
  `GroupBox`es laid out by `GridLayout`s: a row's label and detail lead,
  its control trails, in two columns while the form is at least 460 px
  (scaled) wide and one below.
- **A switch sits beside its label, always.** The styles elide a switch's
  own text and never wrap it: in Spanish at the minimum width the live-mode
  switch's text was cut off. So a switch carries no text; its label and
  detail wrap beside it (`SwitchRow`) and name it for accessibility, and a
  click on them toggles it, as a click on its text would. That is also the
  layout of the platforms' own settings pages.
- **The page's margin is inside the scroll view.** Outside it, the style's
  scroll bar stood inset by the margin and the groups ran up to it.

## Fallback style

The macOS and Windows styles lack some controls: ToolBar, ToolButton,
TabBar, ToolTip, Drawer, Pane, Popup, Label and SplitView. Qt draws those
with Basic, not the Fusion the styles declare. ADR 0015 has the
measurement. So the toolbar's and the HUD's tool buttons are Basic's on
macOS: flat, with a 40 × 40 background in the palette's button colour.

## The HUD

- **Play:** a `CommandButton` (the toolbar's, now a file of its own) with
  the platform's play and pause symbols.
- **Speed:** the style's checkable tool buttons in an exclusive
  `ButtonGroup`, in a `RowLayout`.
  - The checked choice's label is bold. Basic's checked wash differs
    from the unchecked one by 1.39:1 in light and 1.05:1 in dark, too
    little to carry the choice, which would then be conveyed by colour
    alone.
  - Every choice is as wide as the widest label in bold, so a change of
    speed moves nothing. `uniformCellWidths` was tried and gives each cell
    the mean of the preferred widths, which elided "0.5×" and "1.5×" at
    150 % text.
  - One tab stop, the checked choice: the arrows move the choice, and
    neither they nor Space reach the window's seek and play shortcuts
    while a choice has focus.
  - Roles: each choice is a radio button in a group named "Playback
    speed".
- **Scrubber:** `AppSlider` stays, as content (ADR 0015, decision 3).
  The macOS style's slider draws its track beyond the knob in 243 on the
  HUD's 244 grey, so the rest of the workout vanished.
- **Hit areas (round 3's 2f):**
  - Every control answers within at least 40 px each way. A containment
    mask (`HitArea`) reaches past the control's edges, so its visual and
    its place stay as they are.
  - The two rows of controls are at least 40 px tall. Under Fusion the
    masks of the play button and the speed buttons otherwise overlapped
    by 2 px; under the macOS style the rows already were.
  - A probe sampled every control's `contains()` and found no two hit
    areas overlapping, under the macOS style and under Fusion.
- **Stacking:** the speed and the chips stack when the HUD's column is
  too narrow for both. Before, that was tested against the row's own
  width, which never drops below its two columns' minimum. The old
  segmented control could shrink, which hid that; the new speed buttons
  cannot, so Spanish at 480 px overflowed the window. The test now reads
  the column's width.

## The spinner

The macOS style's `BusyIndicator` is an animated WebP image, and Qt reads
WebP only through Qt Image Formats' plugin, which none of the project's Qt
installs carries (the local one, CI's or the release workflow's): the
style's spinner drew nothing and logged an image error. The live-mode
panel keeps its own (`AppBusyIndicator`) until the plugin is installed and
packaged, which touches the release workflow and is the owner's call. The
Windows style falls back to Basic's drawn spinner, and FluentWinUI3 draws
its own.

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
