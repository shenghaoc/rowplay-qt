// SPDX-License-Identifier: GPL-3.0-or-later
// The keyboard focus ring (ADR 0013): a band in Theme.focusRing — the accent,
// or the primary text colour when an unusual system accent would be too
// faint — drawn just outside a control and following its corner radius;
// thicker under the high-contrast preference. Controls show it on
// `visualFocus` (focus that arrived by Tab, Backtab or a shortcut), so a
// mouse click never paints it; text fields show it while they are focused.
import QtQuick
import RowPlay

Rectangle {
    /// The corner radius of the control the ring surrounds.
    property real controlRadius: Theme.radiusSmall

    readonly property int gap: Theme.px(2)

    anchors.fill: parent
    anchors.margins: -(gap + Theme.focusRingWidth)
    radius: controlRadius + gap + Theme.focusRingWidth
    color: "transparent"
    border.width: Theme.focusRingWidth
    border.color: Theme.focusRing
    Accessible.ignored: true
}
