// SPDX-License-Identifier: GPL-3.0-or-later
// Text field (ADR 0013): control fill, an outline at 3:1 against its
// surface, the focus ring while the field has keyboard focus (clicked or
// tabbed — the caret is where typing goes), selection in the accent. An
// optional leading glyph (`leadingIcon`, e.g. the search magnifier) sits
// inside the bezel. A refused entry (`invalid`) draws a 2 px outline in the
// alert colour, so the field itself shows which one is wrong; the caller
// puts the message beside it. Sizes scale with the system font.
import QtQuick
import QtQuick.Controls
import RowPlay

TextField {
    id: control

    /// Optional leading Icon.qml glyph.
    property string leadingIcon: ""
    /// The entry was refused (the caller shows why, next to the field).
    property bool invalid: false

    implicitHeight: Theme.controlHeight
    leftPadding: leadingIcon.length > 0 ? Theme.iconSize + 2 * Theme.spacingMedium
                                        : Theme.spacingMedium + Theme.px(2)
    rightPadding: Theme.spacingMedium + Theme.px(2)
    topPadding: 0
    bottomPadding: 0
    verticalAlignment: TextInput.AlignVCenter
    font: Theme.body
    color: enabled ? Theme.controlText : Theme.textDisabled
    placeholderTextColor: enabled ? Theme.textTertiary : Theme.textDisabled
    selectionColor: Theme.accentColor
    selectedTextColor: Theme.onAccent
    hoverEnabled: true

    background: Rectangle {
        implicitWidth: Theme.px(120)
        radius: Theme.radiusSmall
        color: control.enabled ? Theme.controlBackground : Theme.groupBackground
        border.width: control.invalid ? 2 : Theme.hairline
        border.color: control.invalid ? Theme.alertRed
                    : (control.enabled ? Theme.controlBorder : Theme.separator)

        Icon {
            x: Theme.spacingMedium
            anchors.verticalCenter: parent.verticalCenter
            name: control.leadingIcon
            size: Theme.iconSize
            color: control.enabled ? Theme.controlTextSecondary : Theme.textDisabled
        }

        FocusRing {
            visible: control.activeFocus
            controlRadius: Theme.radiusSmall
        }
    }
}
