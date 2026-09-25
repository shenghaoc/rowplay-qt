# UI native styles — tasks

A stack of pull requests (`design.md`, "Stack"). Each task is ticked in the
pull request that delivers it.

- [x] T1 (`ui/native-style-switch`) The style switch.
  - [x] T1.1 `qml/qtquickcontrols2.conf` and its qrc entry removed; Qt
    6.11.2's defaults confirmed from `qquickstyle.cpp` (macOS, Windows,
    Fusion; FluentWinUI3 opt-in only).
  - [x] T1.2 `Main.qml` sets no palette role.
  - [x] T1.3 The shared controls import `QtQuick.Controls.Basic` until
    their area is replaced (ADR 0015, decision 7); no customisation warning
    in the native walks.
  - [x] T1.4 `Theme`: `dark` from the palette in use; surfaces, text and
    lines from the system palette with contrast fitting; the metric and
    status colours through `paletteColour` (R4, R7.3).
  - [x] T1.5 The screens use the style's scroll bars. The Basic
    `AppScrollBar` is never transient, so the macOS `ScrollView` reserved
    room for it and `contentWidth: availableWidth` looped.
  - [x] T1.6 CI: informational Windows walks in a real window (D3D11), the
    Windows style and FluentWinUI3, light and dark, uploaded as
    `screenshots-windows`.
  - [x] T1.7 Packaging inspected from the release workflow's logs; the
    AppImage's missing pieces proposed in the pull request (Qt Svg, the GTK
    3 platform theme). Not pushed.
  - [x] T1.8 ADR 0015 (0013's control layer superseded in part), this spec.
- [x] T2 (`ui/native-shell`) Shell and toolbar (R2.1, R5.1, R6.1).
  - [x] T2.1 `Glyphs.qml`: the glyph paths (moved from `Icon.qml`), platform
    icon names on macOS/Windows, and the SVG source on Linux. Qt asks the
    platform icon engine only while an icon has no `source`; the AppImage
    carries the SVG decoder. The configured icon colour follows enabled or
    disabled theme text so the black source paths remain visible on dark
    Fusion surfaces.
  - [x] T2.2 The toolbar is the style's `ToolBar` with tool buttons (an
    inline `CommandButton`: the platform icon, the label as accessible name
    and tooltip with the shortcut, hover tracking enabled regardless of
    platform hints, a press dismisses the tooltip, Space
    presses the focused button); the rule under it stays, as layout.
  - [x] T2.3 The sport filter is the style's `ComboBox`, as wide as its
    widest choice (design.md).
  - [x] T2.4 The Windows / Linux application menu is the style's `Menu`;
    its items show no shortcut (the tooltips name them).
  - [x] T2.5 The sidebar's drawer is the style's `Drawer`, keeping `shown`,
    no edge drag, the reduce-motion transitions and the close policy.
  - [x] T2.6 About is the style's `Dialog`, its standard button named by
    the web's `common.dismiss`; the landing's buttons are the style's
    `Button`s, the prominent one `highlighted`.
  - [x] T2.7 The Linux AppImage stages Qt Svg's image-format plugin and
    library for its SVG command icons. The local package launch check
    passes, and Qt's plugin log confirms `libqsvg.so` loads from the
    extracted AppImage.
- [x] T3 (`ui/native-sidebar`) Sidebar (R2.1).
  - [x] T3.1 The search and date fields are the style's `TextField`s (Qt
    6.11's `SearchField` draws no placeholder); a refused date keeps its
    message and accessible description under the field.
  - [x] T3.2 The sort button is a tool button with the platform's icon and
    the sort menu the style's `Menu`, the active field checked with its
    direction after the label. Its icon uses a theme text colour for dark
    Fusion and its tooltip explicitly tracks hover.
  - [x] T3.3 The rows are the style's `ItemDelegate`s with the workout's
    data as content; the day headers are the list's own sections with a
    populated grouping role on every workout. The selection uses the
    style's highlight while the list has keyboard focus and a neutral
    wash in the row content otherwise, without palette overrides.
- [ ] T4 Settings (R2.1).
- [ ] T5 Detail and dashboard (R2.1).
- [ ] T6 Replay HUD (R2.1, R7.4).
- [ ] T7 Theme cleanup and motion tokens (R2.3, R7.2).
- [ ] T8 Large and extra-large layouts (R7.1).
- [ ] T9 Documentation and the full verification matrix (R8, R9).
