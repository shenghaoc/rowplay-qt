// SPDX-License-Identifier: GPL-3.0-or-later
// The keyboard focus ring (HIG Accessibility / Keyboard): a 3 px accent band
// drawn just outside a control, following its corner radius. Controls show it
// on `visualFocus` (focus that arrived by Tab, Backtab or a shortcut), so a
// mouse click never paints it; text fields show it while they have active
// focus, like macOS.
import QtQuick
import RowPlay

Rectangle {
    /// The corner radius of the control the ring surrounds.
    property real controlRadius: 6

    anchors.fill: parent
    anchors.margins: -3
    radius: controlRadius + 3
    color: "transparent"
    border.width: 3
    border.color: Theme.focusRing
    Accessible.ignored: true
}
