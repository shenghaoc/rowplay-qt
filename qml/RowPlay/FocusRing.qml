// SPDX-License-Identifier: GPL-3.0-or-later
// The keyboard focus ring (ADR 0013), two-tone like Windows' focus visual:
// a 1 px inner band in the window colour right outside the control, and a
// 2 px outer band in Theme.focusRing (the accent, or the text colour under
// high contrast and for an accent too faint on the window). One of the two
// bands contrasts with whatever the ring surrounds or crosses: an accent
// fill, a metric colour, a card, the replay scene. Both follow the
// control's corner radius. Controls show it on `visualFocus` (focus that
// arrived by Tab, Backtab or a shortcut), so a mouse click never paints it;
// text fields show it while they are focused. Every control draws its ring
// through this one component.
import QtQuick
import RowPlay

Item {
    id: ring

    /// The corner radius of the control the ring surrounds.
    property real controlRadius: Theme.radiusSmall

    anchors.fill: parent
    anchors.margins: -Theme.focusRingExtent
    Accessible.ignored: true

    // The outer band.
    Rectangle {
        anchors.fill: parent
        radius: ring.controlRadius + Theme.focusRingExtent
        color: "transparent"
        border.width: Theme.focusRingWidth
        border.color: Theme.focusRing
    }

    // The inner band, between the outer one and the control.
    Rectangle {
        anchors.fill: parent
        anchors.margins: Theme.focusRingWidth
        radius: ring.controlRadius + Theme.focusRingInnerWidth
        color: "transparent"
        border.width: Theme.focusRingInnerWidth
        border.color: Theme.focusRingInner
    }
}
