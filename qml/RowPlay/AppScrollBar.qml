// SPDX-License-Identifier: GPL-3.0-or-later
// Scroll bar (ADR 0013): a thin rounded thumb that shows while scrolling or
// hovered and widens under the pointer; always visible under high contrast
// when there is something to scroll. Set explicitly on every ScrollView and
// ListView (`ScrollBar.vertical: AppScrollBar {}`), replacing Basic's.
import QtQuick
import QtQuick.Controls.Basic
import RowPlay

ScrollBar {
    id: control

    readonly property bool scrollable: size < 1.0
    /// The thumb's thickness at rest, and widened under the pointer.
    readonly property int restThickness: Theme.px(5)
    readonly property int hoverThickness: Theme.px(8)
    /// The bar's widest cross-section, padding included. A layout that
    /// reserves room for the bar reserves this, so hovering it moves nothing.
    readonly property real maximumThickness: hoverThickness + 2 * padding

    padding: Theme.px(2)
    minimumSize: 0.08
    hoverEnabled: true

    contentItem: Rectangle {
        implicitWidth: control.hovered || control.pressed ? control.hoverThickness
                                                          : control.restThickness
        implicitHeight: implicitWidth
        radius: width / 2
        color: control.pressed ? Theme.textSecondary : Theme.textTertiary
        opacity: !control.scrollable ? 0
               : (Theme.highContrast || control.policy === ScrollBar.AlwaysOn
                  || control.active || control.hovered) ? (Theme.highContrast ? 1.0 : 0.7) : 0

        Behavior on opacity {
            enabled: !Theme.reduceMotion
            NumberAnimation { duration: 180 }
        }
    }

    // No track. Basic's own shows only under the OS contrast preference,
    // in `palette.mid` (Theme.controlBorder), and the high-contrast thumb
    // measures 1.0–1.06:1 on it: the thumb would vanish into the track.
    background: Item {}
}
