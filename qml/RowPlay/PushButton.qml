// SPDX-License-Identifier: GPL-3.0-or-later
// Bezel push button (HIG Buttons): 28 px tall, 6 px corners, a hairline
// border on the control fill. `prominent` is the view's default action —
// accent fill, white label, at most one per view; `destructive` tints the
// label red. An optional leading glyph (`iconName`) sits before the label.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Button {
    id: control

    property bool prominent: false
    property bool destructive: false
    /// Optional leading Icon.qml glyph.
    property string iconName: ""

    readonly property color labelColor: prominent ? "#ffffff"
                                        : (destructive ? Theme.alertRed
                                                       : Theme.textPrimary)

    implicitHeight: Theme.controlHeight
    leftPadding: 12
    rightPadding: 12
    topPadding: 0
    bottomPadding: 0
    spacing: 6
    hoverEnabled: true
    font.pixelSize: 13
    Accessible.name: text

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight
        opacity: control.enabled ? 1.0 : 0.45

        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: control.spacing

            Icon {
                visible: control.iconName.length > 0
                name: control.iconName
                size: 14
                color: control.labelColor
            }
            Label {
                text: control.text
                font: control.font
                color: control.labelColor
                Accessible.ignored: true
            }
        }
    }

    background: Rectangle {
        implicitWidth: 64
        radius: 6
        color: control.prominent ? Theme.accentColor : Theme.controlBackground
        border.width: control.prominent ? 0 : 1
        border.color: Theme.controlBorder
        opacity: control.enabled ? 1.0 : 0.6

        // Hover / press wash over either fill.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: control.down ? (control.prominent ? Qt.rgba(0, 0, 0, 0.18)
                                                     : Theme.pressedFill)
                                : (control.hovered && control.enabled
                                   ? (control.prominent ? Qt.rgba(1, 1, 1, 0.08)
                                                        : Theme.hoverFill)
                                   : "transparent")
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: 6
        }
    }
}
