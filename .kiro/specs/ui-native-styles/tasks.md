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
- [x] T4 (`ui/native-settings`) Settings (R2.1).
  - [x] T4.1 The page's groups are the style's `GroupBox`es laid out by
    `GridLayout`s, a row's label and detail leading and its control
    trailing, two columns while they fit; the page's margin is inside the
    scroll view, so the style's scroll bar sits at the pane's edge.
  - [x] T4.2 The switches are the style's `Switch`es beside their labels
    (`SwitchRow`): the labels wrap where a switch's own text would be
    elided (Spanish at the minimum width), and a click on them toggles
    the switch.
  - [x] T4.3 The quality, distance-unit and language pickers are the
    style's `ComboBox`es; the timezone picker is a filter `TextField` above
    a `ComboBox` of the matches, which always shows the zone in use.
  - [x] T4.4 The token field, the buttons and the sync progress are the
    style's `TextField`, `Button`s (Connect `highlighted`) and
    `ProgressBar`; the logout confirmation is the style's `Dialog`, its
    standard buttons renamed with web strings when it opens.
  - [x] T4.5 The live-mode panel is a `GroupBox` of the same rows. Its
    spinner stays ours: the macOS style's needs the WebP plugin, which no
    Qt install of the project carries (design.md, "The spinner").
- [x] T5 (`ui/native-detail`) Detail and dashboard (R2.1).
  - [x] T5.1 The detail's Replay is the style's `Button`, `highlighted`:
    on macOS the default button, drawn in the accent only while the window
    is active (`CE_PushButtonBevel` in Qt's macOS style), like every macOS
    default button.
  - [x] T5.2 The detail's and the dashboard's margins are inside their
    scroll views, as the settings page's are; the Replay button needs no
    room for our focus ring or the scroll bar any more.
  - [x] T5.3 The dashboard has no control, and the charts none of their
    own: nothing else to replace (design.md's "chart controls" did not
    exist).
- [x] T6 (`ui/native-replay-hud`) Replay HUD (R2.1, R7.4).
  - [x] T6.1 Play / pause is a `CommandButton`, moved from `Main.qml` to
    a file of its own and shared with the toolbar.
  - [x] T6.2 The speed is the style's checkable tool buttons in an
    exclusive `ButtonGroup`: one tab stop, the arrows move the choice, the
    checked choice's label bold (the style's wash alone measured 1.05:1 in
    dark), every choice as wide as the widest (design.md, "The HUD"). A
    speed change through a global shortcut transfers focus to the newly
    checked button when focus was already inside the speed group.
  - [x] T6.3 The scrubber stays `AppSlider`, as content (ADR 0015,
    decision 3): the macOS style's slider hides its track beyond the knob
    on the HUD.
  - [x] T6.4 Every HUD control answers within at least 40 px (`HitArea`
    masks, rows at least 40 px tall); no two hit areas overlap under the
    macOS style or Fusion (R7.4).
  - [x] T6.5 The speed and the chips stack against the HUD column's
    width, not the row's own.
  - [x] T6.6 The replay's Close button is the style's `Button`.
  - [x] T6.7 ADR 0015 corrected: the native styles' missing controls come
    from Basic, not Fusion (measured).
- [x] T7 (`ui/native-theme-cleanup`) Theme cleanup and motion tokens (R2.3,
  R7.2).
  - [x] T7.1 The seventeen shared controls nothing uses any more are
    deleted with their `qmldir` and qrc entries: `PushButton`,
    `ToolbarButton`, `SegmentedControl`, `ToggleSwitch`, `InputField`,
    `PopupButton`, `FormSection`, `FormRow` and the `App*` tool tip, menu,
    menu item, menu separator, scroll bar, progress bar, dialog, drawer and
    dialog button box.
  - [x] T7.2 `Theme` keeps content tokens only: the 27 members only those
    controls read are gone (hover, press, selection, segment and switch
    washes, the control and destructive text, the popup and toolbar
    surfaces, unused sizes and fonts, and the palette pairs nothing reads
    any more), and `motionDuration` gave way to the motion tokens.
    `deltaColor` stays, a documented port of Studio's.
  - [x] T7.3 Motion tokens (round 3's 2e): `durationShort` / `Medium` /
    `Long` (150 / 250 / 400 ms, 0 under reduce motion) and M3's standard
    and emphasized-decelerate curves. The HUD's fade, the drawer's slide
    and the scrubber's knob use them; no literal duration is left but the
    spinner's period.
  - [x] T7.4 Two Basic imports stay: `AppSlider` (the HUD's scrubber,
    content) and `AppBusyIndicator` (the WebP exception). AGENTS.md's QML
    rules now say "use native Qt Quick Controls as-is".
- [x] T8 (`ui/native-large-layouts`) Large and extra-large layouts (R7.1).
  - [x] T8.1 `Theme.widthClass` has M3's five classes, scaled with the
    text: compact, medium, expanded (the old "large", ≥ 840), large
    (≥ 1200) and extra-large (≥ 1600). The sidebar's drawer rule reads
    "below expanded"; the gate's full-width step sets `Theme.px(1200)` and
    expects class 3 on every platform.
  - [x] T8.2 The detail's supporting pane: in a large window whose detail
    column fits two panes of `paneMinWidth`, the stroke charts sit beside
    the summary, the splits and the targets. One set of sections, two
    layouts of `LayoutItemProxy`; a sometimes-absent section's visibility
    is on its proxies, because a proxy that takes control shows its
    target.
  - [x] T8.3 The dashboard's feed: the charts side by side under the same
    rule; a tile or card grid whose items all fit one row stops at
    `tileMaxWidth` per item.
  - [x] T8.4 The content stops at `contentMaxWidth`, centred; the
    comments and the settings page at `readableWidth`.
  - [x] T8.5 Checked at 1200, 1440, 1600 and 1920 px, light and dark,
    English and Japanese: every text item fits (32 runs, none flagged).
- [ ] T9 Documentation and the full verification matrix (R8, R9).
