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
  hidden while the immersive replay is shown. Below HarmonyOS's large
  breakpoint (840 px, scaled with the text) the sidebar is a drawer: a
  modal one over the content down to 600 px, the list's own page below
  that, so a narrow window shows one column at a time (round 2).
- **Toolbar.** Icon-only toolbar buttons, each with a tooltip and an
  accessible name. The tooltip names the command's shortcut in the
  platform's notation (round 2).
- **Choices.** A segmented control for exclusive choices that apply
  immediately.
- **Settings.** Grouped rows: the label on the leading edge and one control
  on the trailing edge (two at most). Clicking anywhere on a row toggles its
  switch without moving keyboard focus: the switch is the row's only focus
  stop. Footers carry the notes.
- **Buttons.** At most one prominent or destructive button per view. Outside
  the toolbar a button shows an icon or a label, never both.
- **Icons.** Monochrome symbols drawn from original path data in `Icon.qml`.
- **Errors.** An error says what went wrong where it happened, with its
  retry right there when one exists; a success is a status line, never a
  dialog (round 2).
- **Accessibility.** Nothing is conveyed by colour alone: deltas and the race
  verdict carry a sign, word or glyph. Nothing is reachable only by hover.
  A screen-reader name leads with the part that tells the item apart
  (round 2).
  Focus rings are visible on any background: two-tone, like Windows' focus
  visual, with an outer band in the accent and an inner band in the window
  colour (round 2). Text is in sentence case, never all caps.
- **Replay.** The playback controls float over the replay scene, on an
  opaque surface, and hide while playing until the pointer moves.
- **Colour.** The PM5 "Erg Display" palette and the Metric Mapping Rule from
  Studio's `DESIGN.md` stay. Surfaces and text use this repository's own
  neutral light and dark ramps, which pass WCAG AA (`docs/source-map.md`).
  Radii, spacing and chart styling are also this repository's own.

**2. It is built on Qt Quick Controls' Basic style.** Basic is the style Qt
designs for full customisation, and `qtquickcontrols2.conf` forces it. The
shared controls in `qml/RowPlay/` replace every stock visual:

- `Icon`, `FocusRing`, `ToolbarButton`, `PushButton`, `SegmentedControl`,
  `ToggleSwitch`, `InputField`, `PopupButton`, `FormSection` / `FormRow`,
  `AppSlider` and `ChartTheme`;
- Basic's own popups and indicators: `AppToolTip`, `AppMenu` /
  `AppMenuItem` / `AppMenuSeparator`, `AppScrollBar`, `AppProgressBar`,
  `AppBusyIndicator`, `AppDialog` and `AppDialogButtonBox`.

Screens use these, never raw `Button`, `ComboBox`, `Switch`, `TextField`,
`Slider`, `Menu`, `ToolTip` or `Dialog`. `Main.qml` sets every palette role from
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
  The one fixed size is the text floor (round 2): no text below 12 px on
  Windows and Linux (Windows' minimum for body text), 11 px on macOS
  (11 pt), or 12 px in Chinese and Japanese.
- **Colour scheme** follows `Qt.styleHints.colorScheme`.
- **Accent.** The system accent (`palette.accent`) colours selection, focus
  rings, switch-on and prominent buttons, with the brand blue as the
  fallback. The metric colours never follow it. Text on an accent fill is
  white or near-black, whichever contrasts more. That passes AA on the brand
  blues and on the macOS system blue, but not on every accent: for an accent
  of luminance about 0.183–0.198 both stay under 4.5:1 (tracked in #76).
- **Contrast.** Under the OS contrast preference (`QAccessibilityHints`)
  every colour comes from the system palette's pairs, as Windows' contrast
  themes require: window and window text for surfaces and text, highlight
  and highlighted text for selection and the accent's roles, button and
  button text for controls, and the disabled group's text for disabled
  labels only. A metric colour stays only where it reaches 4.5:1 on the
  window. Surfaces that become the same colour get 2 px outlines. Round 2
  replaced the first version, which strengthened our own ramps instead.
- **Shortcuts.** `QKeySequence.StandardKey` wherever one exists
  (Preferences, Quit, Close, Find, Refresh, Back). The replay keys stay. A
  sidebar toggle uses F9 on Linux and Windows and Ctrl+Cmd+S on macOS, and
  is disabled during the replay. No custom shortcut shadows the platform's
  Quit or Close.
- **Menus.** macOS gets the native menu bar, whose items carry roles
  (Preferences, Quit, About), so Qt moves them into the application menu
  under its own titles and key equivalents: "About rowplay",
  "Preferences…" (⌘,) and "Quit rowplay" (⌘Q), in English until Qt's own
  catalogues ship (tracked in #80). Windows and Linux get a menu button at
  the toolbar's trailing edge. No new strings.
- **Dialog button order.** `DialogButtonBox` keeps its default layout, so the
  platform theme orders the buttons.
- **Window chrome.** Native title bars everywhere.
- **Settings** stay an in-app page with no dismiss button. On macOS the
  application menu's Preferences… item (⌘,) opens it. Back navigation,
  Escape and the toolbar toggle close it.

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
- `QtQuick.Shapes` (the icons and the busy indicator) and, from 3/7,
  `Qt.labs.platform` (the macOS menu bar) are runtime QML modules. The
  packaging scripts deploy what `qml/` imports (linuxdeploy's
  `QML_SOURCES_PATHS`, `macdeployqt` and `windeployqt` `-qmldir`), so they
  ship without a script change. That was checked for the AppImage (3/7) and
  for the macOS bundle (`tools/package/macos.sh` at 7/7's head,
  2026-09-24, launch check included); the Windows installer is built only in
  CI.
- **What is verified where.** Linux rendering was verified locally (Xvfb +
  Mesa). The runtime gate's visual assertions in CI read only the replay's
  3D captures, so CI checks none of the design system's 2D rendering. macOS
  and Windows run the gate offscreen in CI. macOS was checked natively on
  2026-09-24 (#66, which lists what is left for a human there); Windows has
  CI only. The
  per-OS behaviour found on the way is recorded in `docs/qt-bridges-notes.md`,
  and every divergence from the web and Studio is recorded in
  `docs/source-map.md`.

## Round 2 notes (2026-09-25)

A second pass took rules the Apple and GNOME guidelines had not covered
from Microsoft's Windows and Fluent guidance (WinUI as the reference), the
KDE HIG and HarmonyOS's width breakpoints, each checked against the code
first. The decisions above gained the round's rules where they belong;
the spec's "Round 2" tasks (T8–T12) record how each was checked.

**Adopted:** a two-tone focus ring; high contrast drawn in the system
palette's colour pairs; a text floor (12 px on Windows and Linux, 11 px on
macOS, 12 px in Chinese and Japanese); HarmonyOS's width breakpoints, with
the sidebar as a drawer below the large one and a 480 × 480 minimum
window; shortcuts in toolbar tooltips; a filterable timezone picker;
errors stated where they happen, with their retry; screen-reader names
that lead with what tells an item apart. Access keys (mnemonics) were
assessed but not implemented (#104).

**Not adopted:**

- **Mica and acrylic** (Windows 11's translucent, wallpaper-tinted
  backdrops). They are one platform's look, where this system is the same
  on every OS. Qt Quick has no API for them, so they would need native
  window attributes set from C++. And translucency is what dropped the
  replay HUD's text below AA before it became opaque.
- **Kirigami and qqc2-desktop-style.** Kirigami is KDE's component
  framework, with its own navigation and look; qqc2-desktop-style draws Qt
  Quick Controls as the KDE desktop does, a platform look and only on
  KDE. Either would be a new dependency and a second visual system. What
  the app takes from Plasma instead is its palette:
  - **The accent follows Plasma's.** KDE's platform theme builds the
    palette with `KColorScheme::createApplicationPalette`, which sets
    `QPalette::Accent` to the colour scheme's selection colour
    (kcolorscheme, master `b44cfeac`). `Theme` reads `palette.accent`, so
    the selection, the focus ring and the prominent button take the
    user's accent.
  - **A high-contrast Plasma scheme is not reported as one.** KDE's
    platform theme implements `colorScheme()` but not
    `contrastPreference()` (plasma-integration, master `276324f5`), and
    qtbase's own KDE theme always answers `NoPreference`. So under Plasma
    the app keeps its own ramps, and high contrast engages only when Qt
    reports the preference. Guessing it from the palette's colours would
    also catch ordinary black-on-white palettes, so the app does not try.
  - Not tried on a Plasma desktop: there is none here, and the Linux CI
    leg runs without a desktop. Both points come from the sources above.
- **HarmonyOS Sans and the Huawei visual language.** A brand typeface and
  a platform's look. The type stays the system font on every OS; of
  HarmonyOS only the width breakpoints were taken, because they are
  about layout, not appearance.

**What was checked where (round 2):**

- **macOS, natively** (Apple M5, macOS 27):
  - the focus ring on twelve controls;
  - forced high contrast in light and dark;
  - the text floor in four languages;
  - the width classes in three languages, light and dark;
  - real key and mouse events through QtTest's `TestEvent`;
  - the full gate walk, visual assertions included.
- **Offscreen:** 12 px and 18 px (150 %) text.
- **Linux:** each PR's CI captures, looked at (Xvfb + llvmpipe, a 12 px
  system font).
- **Still open:**
  - macOS under "Increase contrast", which only the owner can switch;
  - Windows' four contrast themes, the focus ring's visibility, Snap
    layouts and text sizes (#81).
