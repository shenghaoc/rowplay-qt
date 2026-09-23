# ADR 0013 — The UI follows the Apple Human Interface Guidelines on every platform, on Fusion-based custom controls

Status: accepted (2026-09-23)

## Context

Phase 4 delivered the shell on Qt Quick Controls' Fusion style, coloured from
`Theme.qml` (Studio's `DesignTokens` port) with stock controls: a `ComboBox`
as the sport filter, text buttons in a full-width toolbar, Fusion switches,
text fields and combo boxes loose on the settings page, a KDE Breeze-like
dark palette (`#232629`), letter badges for the sports and an opaque transport
strip under the replay. The author's first packaged-artifact inspection banked
the UI work for later (`docs/roadmap.md`, "UI follow-ups"); read against its
sibling rowplay-studio (SwiftUI), the port looked like a generic Qt tool rather
than a well-made Mac-class app.

The constraints that shape the answer:

- Linux is the distributed platform (AppImage); macOS and Windows are built and
  launch-checked (ADR 0012). A look has to hold on all three.
- Qt's macOS style exists only on macOS; the iOS style is a mobile idiom;
  nothing in Qt draws a Mac-class desktop look cross-platform. The style is
  forced to Fusion through `qtquickcontrols2.conf` because qtbridge exposes no
  `QQuickStyle` binding.
- No hand-written C++ (ADR 0001); nothing may be added under `assets/`, whose
  bytes are pinned by `asset_hashes.rs`; SF Symbols are licensed for Apple
  platforms only; every string must be an existing web key.

## Decision

- The shell follows the Apple Human Interface Guidelines for macOS on all three
  platforms: a full-height sidebar with sentence-case sections and a focused
  versus unfocused selection; a unified toolbar over the content column with
  icon buttons and a segmented control; grouped settings forms; push, pop-up
  and toggle controls with visible keyboard focus rings; quiet charts; a
  floating media HUD over the replay; the macOS system colours for surfaces,
  text, selection, separators and control fills. The PM5 palette and the
  Metric Mapping Rule stay as they are, and the system font stays the system
  font (SF Pro on macOS).
- It stays on Fusion and `Theme.qml`. A small set of shared QML controls —
  `Icon`, `FocusRing`, `ToolbarButton`, `PushButton`, `SegmentedControl`,
  `ToggleSwitch`, `InputField`, `PopupButton`, `FormSection`, `FormRow` and
  `ChartTheme` — replace Fusion's `background` / `contentItem` / `indicator`;
  screens use them instead of raw Fusion controls. Stock Fusion remains only
  where the palette colours it adequately: menus, pop-up lists, tooltips,
  scroll bars, the progress bar, the busy indicator and dialog frames.
- Symbols are original SVG path data on a 16-unit grid, drawn by
  `QtQuick.Shapes` with the curve renderer and coloured by their control — no
  image files, no SF Symbols, no third-party icon set.

## Consequences

- One Mac-class look on Linux, Windows and macOS, keyboard focus visible
  everywhere, no new asset, dependency or C++. Glyphs scale crisply with the
  control that draws them.
- The app deliberately does not adopt the GNOME or KDE look on Linux: the
  author's target is the Mac-class look of the sibling app, on every
  platform.
- The custom controls are this repository's to maintain, including the
  accessibility roles that Fusion used to supply (the segmented control
  declares `PageTabList` / `PageTab` itself). A Qt upgrade that changes
  Fusion's templates can change them.
- `QtQuick.Shapes` becomes a runtime QML module. The packaging scripts deploy
  what `qml/` imports (linuxdeploy's `QML_SOURCES_PATHS`, `macdeployqt` and
  `windeployqt` `-qmldir`), so it ships without a script change.
- Qt Graphs friction met on the way (a fixed 40 px Y-label column, series that
  a `Repeater` cannot create) is recorded in `docs/qt-bridges-notes.md`; the
  divergences from the web and Studio (symbols, palette, layout) are recorded
  in `docs/source-map.md`.
