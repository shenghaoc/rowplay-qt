// SPDX-License-Identifier: GPL-3.0-or-later
// Scroll bar (ADR 0013): a thin rounded thumb that shows while scrolling or
// hovered and widens under the pointer; always visible under high contrast
// when there is something to scroll. Set explicitly on every ScrollView and
// ListView (`ScrollBar.vertical: AppScrollBar {}`), replacing Basic's.
import QtQuick
import QtQuick.Controls
import RowPlay

ScrollBar {
    id: control

    readonly property bool scrollable: size < 1.0

    padding: Theme.px(2)
    minimumSize: 0.08
    hoverEnabled: true

    contentItem: Rectangle {
        implicitWidth: control.hovered || control.pressed ? Theme.px(8) : Theme.px(5)
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
}
