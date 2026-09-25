// SPDX-License-Identifier: GPL-3.0-or-later
// Menu item (ADR 0013): a control-height row with a reserved checkmark column
// for checkable items, the label, and an optional trailing shortcut hint;
// the highlighted row (hover or keyboard) takes the accent with onAccent
// text. Disabled items use the disabled text colour.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import RowPlay

MenuItem {
    id: control

    /// Optional platform shortcut text shown at the trailing edge (the
    /// caller passes a Shortcut's nativeText, so it reads as the OS writes it).
    property string shortcutText: ""
    /// Optional glyph drawn after the label (e.g. a sort direction).
    property string trailingIcon: ""

    readonly property color textColor: !enabled ? Theme.textDisabled
                                       : highlighted ? Theme.selectionText : Theme.textPrimary

    implicitWidth: Math.max(Theme.px(200), leftPadding + row.implicitWidth + rightPadding)
    implicitHeight: Theme.controlHeight
    leftPadding: Theme.spacingMedium
    rightPadding: Theme.spacingMedium
    topPadding: 0
    bottomPadding: 0
    font: Theme.body
    hoverEnabled: true
    Accessible.name: text

    indicator: Item {}
    arrow: Item {}

    contentItem: RowLayout {
        id: row
        spacing: Theme.spacingSmall

        Icon {
            Layout.preferredWidth: Theme.iconSize
            visible: control.checkable
            name: control.checked ? "checkmark" : ""
            size: Theme.iconSize
            color: control.textColor
        }
        Label {
            Layout.fillWidth: true
            text: control.text
            font: control.font
            color: control.textColor
            elide: Text.ElideRight
            Accessible.ignored: true
        }
        Icon {
            visible: control.trailingIcon.length > 0
            name: control.trailingIcon
            size: Theme.iconSize
            color: control.textColor
        }
        Label {
            visible: control.shortcutText.length > 0
            text: control.shortcutText
            font: Theme.subheadline
            color: control.highlighted ? Theme.selectionText : Theme.textSecondary
            Accessible.ignored: true
        }
    }

    background: Rectangle {
        radius: Theme.radiusSmall - 2
        color: control.highlighted && control.enabled ? Theme.selectionFill : "transparent"
    }
}
