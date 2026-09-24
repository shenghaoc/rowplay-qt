// SPDX-License-Identifier: GPL-3.0-or-later
// Borderless icon button for the toolbar and the floating replay controls
// (ADR 0013): no fill at rest, a rounded wash on hover and press, the glyph in
// the accent while `checked`. Icon-only, so `label` is both the accessible
// name and the tooltip — and the tooltip is never the only way in: the
// command also has its menu entry or shortcut. Tab reaches it (with a focus
// ring); a click does not take focus, so clicking Reload keeps the list
// focused.
import QtQuick
import QtQuick.Controls
import RowPlay

AbstractButton {
    id: control

    /// Icon.qml glyph key.
    property string iconName: ""
    /// Translated label: accessible name and tooltip.
    property string label: ""
    property real iconSize: Theme.iconSize
    property real cornerRadius: Theme.radiusSmall

    implicitWidth: Theme.controlHeight
    implicitHeight: Theme.controlHeight
    padding: 0
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    // No explicit role: the template reports Button, or CheckBox with its
    // checked state while the button is checkable (the settings toggle).
    Accessible.name: label

    // While focused, Space presses the button: a window shortcut on Space
    // (the replay's play / pause) would otherwise take the key first.
    Keys.onShortcutOverride: function(event) {
        if (event.key === Qt.Key_Space && event.modifiers === Qt.NoModifier) {
            event.accepted = true
        }
    }

    // A press dismisses the tooltip for the rest of the hover: otherwise it
    // stays up over whatever the click opened (the application menu). Any
    // hover change clears it, so a Space press made while the pointer was
    // elsewhere cannot hide the tooltip from the next hover.
    property bool tipDismissed: false
    onPressed: tipDismissed = true
    onHoveredChanged: tipDismissed = false

    AppToolTip {
        visible: control.hovered && !control.tipDismissed && control.label.length > 0
        text: control.label
    }

    contentItem: Item {
        Icon {
            anchors.centerIn: parent
            name: control.iconName
            size: control.iconSize
            color: !control.enabled ? Theme.textDisabled
                 : control.checked ? Theme.accentColor
                 : (control.hovered ? Theme.textPrimary : Theme.textSecondary)
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
