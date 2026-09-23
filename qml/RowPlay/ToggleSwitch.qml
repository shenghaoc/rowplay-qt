// SPDX-License-Identifier: GPL-3.0-or-later
// Switch (HIG Toggles): a 34×20 pill, accent when on, a white knob, and no
// text of its own — in a grouped form the label sits at the row's leading
// edge (FormRow), so the caller sets `Accessible.name` to that label.
import QtQuick
import QtQuick.Controls
import RowPlay

Switch {
    id: control

    /// Knob slide; off under Settings' reduce-motion preference.
    property bool animated: !Settings.reduceReplayMotion

    text: ""
    padding: 0
    spacing: 0
    implicitWidth: indicator.implicitWidth
    implicitHeight: Math.max(indicator.implicitHeight, 22)
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    indicator: Rectangle {
        implicitWidth: 34
        implicitHeight: 20
        x: control.leftPadding
        y: (control.height - height) / 2
        radius: height / 2
        color: control.checked ? Theme.accentColor
                               : (Theme.dark ? Qt.rgba(1, 1, 1, 0.18)
                                             : Qt.rgba(0, 0, 0, 0.14))
        border.width: control.checked ? 0 : 1
        border.color: Theme.dark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.06)
        opacity: control.enabled ? 1.0 : 0.45

        Rectangle {
            width: 16
            height: 16
            radius: 8
            y: 2
            x: control.checked ? parent.width - width - 2 : 2
            color: "#ffffff"
            border.width: 1
            border.color: Qt.rgba(0, 0, 0, control.down ? 0.22 : 0.14)

            Behavior on x {
                enabled: control.animated
                NumberAnimation { duration: 140; easing.type: Easing.OutCubic }
            }
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: 10
        }
    }

    contentItem: Item {}
}
