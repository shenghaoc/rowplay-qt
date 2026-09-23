// SPDX-License-Identifier: GPL-3.0-or-later
// Pop-up button (HIG Pop-up buttons): a ComboBox on the PushButton bezel with
// the macOS up/down chevron badge in the accent colour at the trailing edge.
// The list itself stays Fusion's popup, coloured by the window palette.
import QtQuick
import QtQuick.Controls
import RowPlay

ComboBox {
    id: control

    implicitHeight: Theme.controlHeight
    leftPadding: 10
    rightPadding: 30
    topPadding: 0
    bottomPadding: 0
    font.pixelSize: 13
    hoverEnabled: true

    contentItem: Label {
        text: control.displayText
        font: control.font
        color: Theme.textPrimary
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        opacity: control.enabled ? 1.0 : 0.45
        Accessible.ignored: true
    }

    indicator: Rectangle {
        x: control.width - width - 6
        y: (control.height - height) / 2
        width: 16
        height: 16
        radius: 4
        color: Theme.accentColor
        opacity: control.enabled ? 1.0 : 0.45

        Icon {
            anchors.centerIn: parent
            name: "chevron.updown"
            size: 12
            color: "#ffffff"
        }
    }

    background: Rectangle {
        implicitWidth: 160
        radius: 6
        color: Theme.controlBackground
        border.width: 1
        border.color: Theme.controlBorder

        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: control.down ? Theme.pressedFill
                                : (control.hovered && control.enabled
                                   ? Theme.hoverFill : "transparent")
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: 6
        }
    }
}
