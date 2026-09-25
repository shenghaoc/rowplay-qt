# The design system

rowplay-qt looks like each platform's app: its standard controls are Qt Quick
Controls in the platform's own style. It looks like rowplay in its content:
the charts, the metric colours, the tiles and personal-best cards, the splits
table, and the 3D replay with its HUD.

The decisions behind this page:
- [ADR 0015](decisions/0015-native-qt-styles.md): native Qt styles for
  standard controls.
- [ADR 0013](decisions/0013-cross-platform-design-system.md): kept for the
  type scale, the lengths, the width classes, the focus ring on our own
  focusables and the contrast floors.

The rules for writing QML are in [AGENTS.md](../AGENTS.md) ("Coding style"),
and the history is in the spec `.kiro/specs/ui-native-styles/`.

## Six principles

1. **Accessible by construction.** Accessibility is built into every
   control, not added after.
   - Every control and tile has an `Accessible.name`, and the styles give
     the controls their platform roles.
   - Text meets 4.5:1 and our marks 3:1 on the surfaces they are drawn on,
     and no text is set under `Theme.textFloor`.
   - Focus is always visible: the style draws it on its controls and
     `FocusRing` on ours.
   - Nothing is conveyed by colour alone and nothing is reachable only by
     hover.
   - Motion stills under reduce motion.
   - The replay HUD's controls answer within 40 px.
2. **Native by default.**
   - A standard control is the platform style's, used as it is: no
     `background` or `contentItem` of ours.
   - Icons are the platform's, named. A dialog's buttons follow the
     platform's order. macOS gets its menu bar.
3. **Our identity lives in the content.**
   - The PM5 palette and the Metric Mapping Rule colour the metrics.
   - Tiles, cards, the splits table, the charts and the replay are drawn
     by us.
   - That is where rowplay looks like rowplay.
4. **Qt before custom code.** Where Qt has an API for a platform behaviour
   (a shortcut, its notation, a dialog's buttons, an icon, the palette, a
   responsive layout, a hit area), it is used before anything is written.
   Custom components are for content. The few exceptions are bounded and
   listed in ADR 0015.
5. **Relative to the system.**
   - Lengths and type scale from the system font.
   - Surfaces, text and lines derive from the system palette, fitted to the
     contrast floors.
   - The accent is the system's. Light or dark follows the palette in use,
     and high contrast follows the system's contrast preference.
6. **Same product, native manners.** Every platform has the same screens,
   strings, content and behaviour. The manners are each platform's own:
   the controls, the icons, the shortcuts and their notation, the dialog
   order and the menus.

## Platform behaviour → Qt API

| Platform behaviour | Qt API | Where |
|---|---|---|
| The platform's control look | No style forced, so Qt picks macOS, Windows or Fusion (`QQuickStyleSpec::resolve`) | no `qtquickcontrols2.conf` (ADR 0015) |
| A control the style lacks (tool button, tool tip, drawer, pane, popup) | Qt's run-time fallback, Basic; the styles declare Fusion, but Basic wins | ADR 0015, "Context" |
| Scroll bars (transient or not, as the system says) | the style's `ScrollBar`, through `ScrollView` | the screens and the sidebar |
| Keyboard shortcuts | `Shortcut { sequences: [StandardKey.Find] }`, `StandardKey.Refresh`, `.Preferences`, `.Back`, `.Quit`, `.Close` | `Main.qml` |
| A shortcut in the platform's notation | `Shortcut.nativeText` in the tooltip | `CommandButton` |
| Icons | `icon.name`: SF Symbols through `QAppleIconEngine`, Segoe glyphs through `QWindowsIconEngine`, the freedesktop theme on Linux; `icon.source` (an SVG of our glyph) on Linux only, for a theme without the icon | `Glyphs.qml`, `CommandButton` |
| A dialog's buttons and their order | `Dialog.standardButtons`, renamed with web strings as the dialog opens | About, the logout confirmation |
| The macOS menu bar | `Qt.labs.platform` `MenuBar` with `AboutRole`, `PreferencesRole`, `QuitRole` | `Main.qml` (a `ToolButton` and `Menu` elsewhere) |
| The system palette | `SystemPalette` (active, inactive and disabled groups), composited opaque | `Theme.qml`, the sidebar's selection |
| Light or dark | the palette in use (`Theme.dark` from its window colour); `Qt.styleHints.colorScheme` only to pin it in tests | `Theme.qml`, `Main.qml` |
| The accent | `SystemPalette.accent`, with the brand blue where a platform reports none | `Theme.accentColor` |
| High contrast | `Qt.styleHints.accessibility.contrastPreference` (Qt 6.10) | `Theme.highContrast` |
| Text size | `Qt.application.font`, as a ratio of a 13 px reference | `Theme.px`, `Theme.fontPx` |
| Reduce motion | none in Qt 6.11; the app's own preference drives the motion tokens | `Theme.durationShort/Medium/Long` |
| Screen readers | `Accessible.name`, `.description`, `.role` | every control and tile |
| Responsive layouts | `Theme.widthClass` (M3's five classes, scaled with the text) and `LayoutItemProxy` | `Main.qml`, the detail, the dashboard |
| Touch targets | `containmentMask` reaching past a control's edges | the replay HUD (`HitArea`) |
| The busy spinner | `BusyIndicator`, except on macOS, where the style's needs the WebP plugin | the live-mode panel (`AppBusyIndicator` until then) |

## Per platform

| | macOS | Windows | Linux |
|---|---|---|---|
| Style | macOS (NSView-drawn) | Windows; FluentWinUI3 is opt-in, and CI captures both | Fusion |
| Controls the style lacks | Basic | Basic | (Fusion has them) |
| Icons | SF Symbols | Segoe Fluent Icons (Windows 11), Segoe MDL2 Assets | the desktop's freedesktop theme, then our SVG |
| Palette | the system's | the system's | the platform theme's: the desktop portal's in the AppImage, the GTK or KDE theme's with a distribution's Qt |
| How it is checked | natively, with the gate walk and scratch probes | CI captures in a real window (#81) | CI under Xvfb; dark through Fusion on macOS (Xvfb has no theme to ask) |
| Known issues | the style's spinner needs the WebP plugin; the slider's track beyond the knob all but vanishes on a grey surface | the Windows style draws its controls light under the dark scheme, so their dark-scheme text is unreadable (the rows, the pop-up buttons, the fields); FluentWinUI3 draws both schemes | the AppImage lacks Qt Svg and the GTK 3 platform theme (proposed) |

## What stays ours

Content, drawn by us:
- the charts (`ChartTheme`, `StrokeChart`) and `MetricTile`;
- the personal-best cards and the splits table;
- the replay scene and its HUD panel, with `AppSlider` as the scrubber;
- `FocusRing` on our own focusables;
- `Icon`, the glyphs drawn in content.

Bounded exceptions to "native by default", each in ADR 0015:
- a list delegate's content (the sidebar's rows);
- the sidebar's split handle;
- the live-mode spinner, until the WebP plugin ships.
