// SPDX-License-Identifier: GPL-3.0-or-later
// Borderless icon button for the toolbar and floating HUDs (HIG Toolbars):
// no fill at rest, a rounded wash on hover and press, the glyph tinted with
// the accent while `checked`. Icon-only, so `label` becomes both the
// accessible name and the tooltip. Tab reaches it (with a focus ring); a
// click does not take focus, so clicking Reload keeps the list focused.
import QtQuick
import QtQuick.Controls
import RowPlay

AbstractButton {
    id: control

    /// Icon.qml glyph key.
    property string iconName: ""
    /// Translated label: accessible name and tooltip.
    property string label: ""
    property real iconSize: 16
    property real cornerRadius: 6

    implicitWidth: 32
    implicitHeight: Theme.controlHeight
    padding: 0
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    Accessible.name: label
    Accessible.role: Accessible.Button
    ToolTip.visible: hovered && label.length > 0
    ToolTip.text: label
    ToolTip.delay: 600

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: control.iconName
            size: control.iconSize
            opacity: control.enabled ? 1.0 : 0.35
            color: control.checked ? Theme.accentColor
                                   : (control.hovered ? Theme.textPrimary
                                                      : Theme.textSecondary)
        }
    }

    background: Rectangle {
        radius: control.cornerRadius
        color: control.down ? Theme.pressedFill
                            : (control.hovered || control.checked
                               ? Theme.hoverFill : "transparent")

        FocusRing {
            visible: control.visualFocus
            controlRadius: control.cornerRadius
        }
    }
}
