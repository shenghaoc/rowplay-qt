# ADR 0015 — Native Qt styles for standard controls; our identity in the content

Status: accepted (2026-09-25). Supersedes ADR 0013's control layer (its
decisions on the Basic style and the shared controls). ADR 0013's type scale,
lengths from the system font, width classes, focus ring on our own
focusables, contrast floors and platform-behaviour layer stay.

## Context

ADR 0013 gave rowplay-qt one visual system of its own on Qt Quick Controls
Basic: a set of shared controls (`PushButton`, `SegmentedControl`,
`ToggleSwitch`, `InputField`, `PopupButton` and the `App*` popups) drew every
button, field and menu the same way on Linux, macOS and Windows. On
2026-09-25 the owner redirected it: standard controls should be the
platform's own, and the product's identity should live in its content — the
charts, the metric colours, the tiles and personal-best cards, the splits
table, the 3D replay and its HUD.

What Qt 6.11.2 does, read from its sources at the tag:

- **The default style** (`qquickstyle.cpp`, `QQuickStyleSpec::resolve`):
  with no style requested it is **macOS** on macOS, **Windows** on Windows
  and **Fusion** on Linux, Basic elsewhere. The style is resolved from, in
  order, `QQuickStyle::setStyle`, the `-style` argument,
  `QT_QUICK_CONTROLS_STYLE`, the `Style` key of `:/qtquickcontrols2.conf`,
  then that default. **FluentWinUI3 is never chosen automatically**, on
  Windows 11 either; it is opt-in.
- **The native styles are not customisable.** The macOS and Windows styles
  draw Button, CheckBox, ComboBox, TextField, SearchField, Slider, SpinBox,
  Switch, ScrollBar, ProgressBar, GroupBox, Frame and a few more through
  `QtQuick.NativeStyle`, and warn when a `background` or `contentItem` is
  replaced. The macOS `ItemDelegate` is plain QML over the template, drawn
  from the palette.
- **The controls they do not implement come from Basic, not Fusion.** That
  covers ToolBar, ToolButton, TabBar, ToolTip, Drawer, Pane, Popup, Label
  and SplitView.
  - Both styles declare Fusion as their fallback (`IMPORTS
    QtQuick.Controls.Fusion/auto`, "the required fallback style"), and
    Qt's documentation says a style's qmldir names its fallback.
  - With run-time selection, though, the Controls plugin registers Basic
    as the style's fallback (`QtQuickControls2Plugin::registerTypes`),
    and Basic wins.
  - Measured on macOS with Qt's own `qml` runner (2026-09-25; the split
    handle in the app):

    | Control               | Measured | Basic | Fusion |
    |-----------------------|----------|-------|--------|
    | Pane padding          | 12       | 12    | 9      |
    | Popup padding         | 12       | 12    | 6      |
    | ToolBar height        | 40       | 40    | 26     |
    | ToolButton background | 40       | 40    | 20     |
    | SplitView handle      | 6 px     | 6 px  | 2 px   |

    `QT_QUICK_CONTROLS_FALLBACK_STYLE=Fusion` gives Fusion's values.
  - Windows takes the same code path; it was not measured.
- **The platform icon engines** (Qt 6.7+): on macOS `QAppleIconEngine`
  maps freedesktop names to SF Symbols and passes any other name to
  `imageWithSystemSymbolName`, so an SF Symbol name works as it is; on
  Windows `QWindowsIconEngine` maps freedesktop names to Segoe Fluent Icons
  (Windows 11) or Segoe MDL2 Assets glyphs, and a one-character name is
  drawn as that glyph; on Linux the desktop's freedesktop icon theme
  answers.
- **What the packages carry** (the release workflow's logs, 2026-09-24):
  `macdeployqt` copies the whole QtQuick QML tree, so the macOS style and
  `QtQuick.NativeStyle` are in the bundle; `windeployqt` deploys the
  Windows style, `QtQuick.NativeStyle`, FluentWinUI3, Fusion and Qt Svg
  with its image-format and icon-engine plugins; the AppImage carries every
  Controls style Linux can use, Fusion included, and one platform theme
  (the desktop portal's), but no image-format or icon-engine plugin.

## Decision

1. **No style is forced.** `qml/qtquickcontrols2.conf` is removed, so each
   OS gets Qt's default: macOS, Windows, Fusion. FluentWinUI3 is not
   selected: Qt does not default to it, choosing it per OS version would
   need a runtime `QQuickStyle` call that qtbridge does not expose or an
   environment variable set before the application starts (unsafe in Rust
   2024, and the crates forbid `unsafe`), and nobody on the project can
   look at Windows (#81). CI captures both styles on Windows so the choice
   can be revisited on evidence.
2. **Standard controls are Qt Quick Controls used as they are.** No
   `background` or `contentItem` is replaced on a control the platform
   draws. Four bounded exceptions: a list delegate (an `ItemDelegate`
   carries its row's data as its `contentItem`, and keeps the style's
   background, selection and hover); the sidebar's split handle, a
   hairline with a 9 px hit area where the default handle is Basic's 6 px
   bar (Fusion's, on Linux, is 2 px); the Basic-fallback Drawer's
   `contentItem`, a structural host for the sidebar's reparenting and
   in-drawer shortcuts without custom paint;
   and, until Qt Image Formats' WebP plugin is installed and packaged, the
   live-mode panel's spinner: the macOS style's is a WebP animation and
   draws nothing without the plugin.
3. **Our identity lives in the content.** The charts, the metric colours,
   the tiles and personal-best cards, the splits table and the 3D replay
   with its HUD keep their own drawing. In the HUD, the panel, the times,
   the metric chips, the verdict and the scrubber stay ours. The play
   button and the speed buttons are the style's.
   - The macOS style's slider draws its track beyond the knob in 243 on
     the HUD's 244 grey, about 1.0:1, so the part of the workout still
     to come vanished.
   - Every HUD control answers within at least 40 px (round 3's 2f).

4. **Relative to the system.** Surfaces, text and lines derive from the
   system palette (the window and window-text colours, the base colour, and
   tonal steps between them), fitted to the floors ADR 0013 set: 4.5:1 for
   text, 3:1 for non-text marks. The PM5 metric colours stay the product's
   and are fitted to 4.5:1 on the surfaces they are drawn on. The accent is
   the system's, with the brand blue where a platform reports none.
   `Theme.dark` follows the palette in use, so the tokens always match the
   colours the native controls draw with.
5. **Linux: Fusion with the system palette.** The platform theme supplies
   the palette: in the AppImage the desktop portal's (colour scheme and
   contrast), with a distribution's own Qt the GTK or KDE theme's.
6. **Platform symbols on macOS and Windows, original SVGs on Linux.** Tool
   buttons name an SF Symbol on macOS and a Segoe glyph on Windows
   (`Glyphs.qml`). On Linux they use SVG data URLs built from the same
   original paths. A nonempty `icon.source` wins over `icon.name`, so the
   freedesktop theme is not consulted for those buttons. The AppImage
   deploys Qt Svg's image plugin and library; no image asset is added.
7. **One step at a time.** The switch comes first, with the shared
   controls pinned to Basic by an explicit import so nothing is drawn half
   native; each area's pull request then replaces its controls with stock
   ones, and the last one deletes the shared controls and the tokens only
   they used. Two `import QtQuick.Controls.Basic` survive the stack: the
   spinner's, until the WebP plugin ships, and the HUD scrubber's, which is
   content (decision 3).

## Consequences

- The app looks like each platform's app: macOS controls on macOS, the
  Windows style on Windows, Fusion on Linux. Screens differ per OS by
  design; the content does not.
- Where the macOS and Windows styles have no control of their own (the
  toolbar and its tool buttons, tool tips, the drawer, the replay's speed
  buttons), Qt's Basic style draws it, flat, in the system palette. A
  `FallbackStyle` in a configuration file would give Fusion's bevelled
  ones instead, as the styles declare; that is a style choice left to the
  owner, and none is made.
- Screen readers get the platform's own control roles from the styles.
- The AppImage carries Qt Svg to draw its command icons. Its desktop
  portal theme reports the colour scheme and contrast preference, but not
  GNOME's palette; GTK platform-theme support remains separate work.
- Windows is verified by CI captures only (#81).
- The app's reduce-motion toggle no longer stills the controls the style
  draws, where the style animates them (Fusion slides its switch's
  handle): Qt 6.11 surfaces no OS motion preference for a style to read.
  Our own motion runs on `Theme`'s motion tokens (round 3's 2e), which
  the toggle makes instant: the HUD's fade, the drawer's slide and the
  scrubber's knob. It also stills the replay and our spinner.
- The native macOS `ScrollView` reserves room for a scroll bar that is not
  transient, so a replaced (Basic) scroll bar made `contentWidth:
  availableWidth` a binding loop; the style's own scroll bars are used.

## What was checked where (2026-09-25)

The page that applies this ADR is
[`docs/design-system.md`](../design-system.md): the six principles, platform
behaviour mapped to Qt API, and what stays ours.

- **macOS, natively, before the #124 restack** (Apple M5, macOS 27, cocoa and Metal):
  - **Gate walks:** the full walk in light, phase shots and close-ups
    included, at the then-current stack tip. In dark, the quick walk captures every
    screen; its old shadow check then trips at 4.9 % on this base, as
    AGENTS.md records.
  - **Text fit:** five screens × English, Spanish and Japanese × three
    widths, native light and dark, and 150 % text light and dark (180 runs).
    - Nothing is flagged natively.
    - At 150 % only Qt Graphs' own axis labels are flagged, as on the base.
  - **Large windows:** 1200, 1440, 1600 and 1920 px, English and Japanese,
    light and dark (32 runs), none flagged.
  - **Real input** through QtTest's `TestEvent`:
    - a switch row's label toggles its switch, and Replay opens;
    - the replay HUD's hit areas answer beyond their controls;
    - the speed choices take the arrows without seeking;
    - Space on a speed choice leaves play alone.
  - **Not checked here:** a non-blue accent, Increase Contrast and the
    native menu bar's roles. They need the owner at the Mac: the agent
    changes no System Settings, and a background process cannot bring the
    app's window forward.
  - These checks do not cover every R8.1 combination on the current rebased
    head; T9.3 stays open for the full width/language/text-scale matrix and
    current light and dark native gate walks.
- **Windows:** CI's captures in a real window (D3D11), both styles, light
  and dark.
  - The Windows style draws its controls light under the dark scheme, so the
    dark scheme's light text on them is unreadable: the sidebar's rows, the
    pop-up buttons, the fields and the buttons.
  - FluentWinUI3 draws both schemes.
- **Linux:** CI under Xvfb (Fusion, light). Dark was checked through Fusion
  on macOS, because Xvfb has no platform theme to report dark.
