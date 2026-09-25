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
- [ ] T2 Shell and toolbar (R2.1, R5.1, R6.1).
- [ ] T3 Sidebar (R2.1).
- [ ] T4 Settings (R2.1).
- [ ] T5 Detail and dashboard (R2.1).
- [ ] T6 Replay HUD (R2.1, R7.4).
- [ ] T7 Theme cleanup and motion tokens (R2.3, R7.2).
- [ ] T8 Large and extra-large layouts (R7.1).
- [ ] T9 Documentation and the full verification matrix (R8, R9).
