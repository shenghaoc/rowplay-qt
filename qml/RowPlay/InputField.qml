// SPDX-License-Identifier: GPL-3.0-or-later
// Text field (HIG Text fields): control fill, hairline border, 6 px corners,
// and the 3 px focus ring while the field has keyboard focus (macOS draws it
// on every focused field, clicked or tabbed). An optional leading glyph
// (`leadingIcon`, e.g. the search magnifier) sits inside the bezel.
import QtQuick
import QtQuick.Controls
import RowPlay

TextField {
    id: control

    /// Optional leading Icon.qml glyph.
    property string leadingIcon: ""

    implicitHeight: Theme.controlHeight
    leftPadding: leadingIcon.length > 0 ? 28 : 8
    rightPadding: 8
    topPadding: 0
    bottomPadding: 0
    verticalAlignment: TextInput.AlignVCenter
    font.pixelSize: 13
    color: Theme.textPrimary
    placeholderTextColor: Theme.textTertiary
    selectionColor: Theme.accentColor
    selectedTextColor: "#ffffff"
    hoverEnabled: true

    background: Rectangle {
        implicitWidth: 120
        radius: 6
        color: Theme.controlBackground
        border.width: 1
        border.color: Theme.controlBorder
        opacity: control.enabled ? 1.0 : 0.6

        Icon {
            x: 8
            anchors.verticalCenter: parent.verticalCenter
            name: control.leadingIcon
            size: 14
            color: Theme.textSecondary
        }

        FocusRing {
            visible: control.activeFocus
            controlRadius: 6
        }
    }
}
