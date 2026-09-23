# UI pass — Apple HIG: design

How the requirements are met, and the mechanics that are not obvious from the
code. Decision record: ADR 0013.

## Control layer

The shared controls live in `qml/RowPlay/` next to the screens (one module,
one `qmldir`). Each keeps its Qt Quick Controls base so keyboard handling,
accessibility and signals stay stock, and replaces only the visuals:

| Control | Base | Replaces | Notes |
| --- | --- | --- | --- |
| `ToolbarButton` | `AbstractButton` | everything | `focusPolicy: Qt.TabFocus` — Tab reaches it, a click does not steal focus from the list |
| `PushButton` | `Button` | `background`, `contentItem` | a disabled `prominent` button falls back to the neutral bezel (macOS) |
| `SegmentedControl` | `Control` | — | emits `activated(index)` and never assigns `currentIndex`, so a binding to the store survives; claims Left/Right through `Keys.onShortcutOverride` while focused |
| `ToggleSwitch` | `Switch` | `indicator`, `contentItem` | label-less; the form row carries the label, the caller the accessible name |
| `InputField` | `TextField` | `background` | ring while `activeFocus` (macOS draws it on clicked fields too) |
| `PopupButton` | `ComboBox` | `background`, `contentItem`, `indicator` | the list stays Fusion's popup |
| `FormSection` / `FormRow` | `ColumnLayout` / `Item` | — | a row draws the inset hairline above itself unless it is the first *visible* row, found through the `isFormRow` marker, so rows can hide freely |
| `FocusRing` | `Rectangle` | — | 3 px `Theme.focusRing` band outside the control; on `visualFocus` |
| `Icon` | `Item` + `Shape` | — | one stroke `ShapePath` and one fill `ShapePath`, `PathSvg`, scaled from the 16-unit grid |
| `ChartTheme` | `GraphsTheme` | — | grid, axis lines and label fonts; per-axis switches stay on the `ValueAxis` |

The glyphs were drawn for this repository and iterated at 16 / 20 / 28 / 88 px
in light and dark before use. The web's sport icons are Lucide artwork from a
package (`SportIcon.svelte`); they were not copied.

## Shell

`shellRoot` → `SplitView` → [`SidebarPanel` | `ColumnLayout` { toolbar,
hairline, `StackLayout` }]. The `StackLayout` keeps its id and child order
(`benchRun` reaches the replay scene as `detailColumn.children[3]`).

The divider is a 1 px `Rectangle` handle whose `containmentMask` is a 9 px
`Item` centred on it — the SplitView documentation's own recipe for a thin
handle with a larger drag area.

The sport filter is centred in the toolbar with `anchors.centerIn`; the replay
back button and title get the space left of it
(`(toolbar.width − filter.width) / 2`) so a long title elides instead of
running under the filter.

The replay route's back navigation is the toolbar's leading chevron with the
workout title. A second floating chevron inside the scene was considered and
left out: it would sit directly beneath the toolbar's identical control. The
error / loading overlay keeps its labelled back button as the recovery action
next to its message. The title is `Detail.workoutType` shown only while
`Replay.workoutId` is the selected workout: the route always opens on the
selection, but the gate walk loads demo workouts straight into `Replay`, and
its captures showed the selected interval session's title over the RowErg
and SkiErg scenes.

`shellRoot`, the item the gate grabs, is a `Rectangle` in
`Theme.windowBackground` rather than a bare `Item`. The divider and the
toolbar hairline are translucent (`Theme.separator`) and sit over no painted
item of their own; on screen the window background shows through, but an
item grab has no window under it and stored them translucent — alpha 26 in
the PNG, the bare black (light) or white (dark) in the PPM the Rust
assertions read — while an `xwd` capture of the same frame showed the
blended grey (229, 229, 229). See `docs/qt-bridges-notes.md`.

## Charts

- Tick spacing: `ChartUtils.niceInterval(span, 4)` (1 / 2 / 2.5 / 5 × 10ᵏ) for
  plain numeric axes; for Rust-labelled axes `ChartUtils.spanInterval(low,
  high, n)` — the exact step shrunk by 1e-9, because
  `low + (n − 1) · step` can land a hair above `max` and Qt Graphs then drops
  the top tick (seen on the stroke pace chart).
- Y labels: Qt Graphs 6.11 lays the Y axis out in a fixed strip — a 40 px
  label column, 5 px, 15 px of ticker (`QGraphsViewPrivate`'s
  `m_defaultAxis*` constants, no API). Each label delegate is given the 40 px
  width and asked to right-align. The Rust pace labels ("2:54.3/500m") are
  wider, so the delegate right-anchors its text on the tick and the chart
  reserves `ChartUtils.yLabelOverflow(labels, metrics)` in `marginLeft`;
  stacked stroke charts share the pace chart's inset so their plots line up.
  The Y axis `color` is transparent (no axis line, no tick marks), which also
  keeps the tick marks from running into the labels.
- Split boundaries: an `Instantiator` creates one dashed `LineSeries` per
  boundary and `GraphsView.insertSeries(0, …)` puts it behind the data; a new
  Y range re-spans the existing series.
- Bars: `BarSeries.barDelegate` with a `Rectangle` rounding the top corners;
  the delegate declares the documented `barColor` … `barIndex` properties.

## Settings

The form column is `min(width, 640)` wide and centred. Status and result lines
are `FormRow.detail` text (red for failures); the sync status expression moved
into a named property with every branch intact. The logout confirmation keeps
`onAccepted: Settings.clearToken()` and gets PushButtons with web strings;
`DialogButtonBox.MacLayout` pins the HIG order (Cancel, then the action) on
every platform.

## Replay HUD

The `View3D` now fills the route (`anchors.fill`) and the HUD floats over it:
bottom-centred, 16 px margin, at most 760 px wide, `Theme.overlayBackground`,
`radiusLarge`, a hairline border. At the default window it spans roughly
y 0.86–0.98 and x 0.32–0.95 of the capture, clear of the band the gate's
shadow assertion samples (y 0.55–0.75). The verdict, the one long string, gets
its own wrapped line only when a ghost has finished.

`Replay.setViewport` already received the route's full size; with the strip
gone the `View3D` now has that size too, so the camera aspect and the viewport
agree.

## Reduce motion

`Settings.reduceReplayMotion` (the "Reduce motion" toggle) also switches off
the new UI animations — the segmented-control thumb and the switch knob — the
way the macOS setting stills interface motion.
