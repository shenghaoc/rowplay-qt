// SPDX-License-Identifier: GPL-3.0-or-later
// Pop-up button (ADR 0013): a ComboBox on the push-button bezel with a
// trailing chevron, opening the design system's own list — the popup fill,
// a hairline outline, rows with a hover wash, the keyboard-highlighted row
// in the accent and a checkmark on the current value. Every string arrives
// translated (or is an untranslated name) from the caller's model.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ComboBox {
    id: control

    implicitHeight: Theme.controlHeight
    leftPadding: Theme.spacingMedium + Theme.px(2)
    rightPadding: Theme.controlHeight
    topPadding: 0
    bottomPadding: 0
    font: Theme.body
    hoverEnabled: true

    contentItem: Label {
        text: control.displayText
        font: control.font
        color: control.enabled ? Theme.textPrimary : Theme.textDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        Accessible.ignored: true
    }

    indicator: Icon {
        x: control.width - width - Theme.spacingMedium
        y: Math.round((control.height - height) / 2)
        name: "chevron.down"
        size: Theme.iconSize
        color: control.enabled ? Theme.textSecondary : Theme.textDisabled
    }

    background: Rectangle {
        implicitWidth: Theme.px(160)
        radius: Theme.radiusSmall
        color: control.enabled ? Theme.controlBackground : Theme.groupBackground
        border.width: Theme.hairline
        border.color: control.enabled ? Theme.controlBorder : Theme.separator

        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: control.down ? Theme.pressedFill
                                : (control.hovered && control.enabled
                                   ? Theme.hoverFill : "transparent")
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: Theme.radiusSmall
        }
    }

    delegate: ItemDelegate {
        id: row

        required property var model
        required property int index
        readonly property string rowText: control.textRole.length > 0
                                          ? String(model[control.textRole])
                                          : String(model.modelData)
        readonly property bool current: index === control.currentIndex

        width: ListView.view ? ListView.view.width : implicitWidth
        implicitHeight: Theme.controlHeight
        leftPadding: Theme.spacingMedium
        rightPadding: Theme.spacingMedium
        highlighted: control.highlightedIndex === index
        hoverEnabled: true
        Accessible.name: rowText

        contentItem: RowLayout {
            spacing: Theme.spacingSmall

            // Laid out in every row, so the labels line up; only the
            // current row shows it (an Icon with no name hides itself, and
            // a layout skips hidden items).
            Icon {
                Layout.preferredWidth: Theme.iconSize
                name: "checkmark"
                opacity: row.current ? 1 : 0
                size: Theme.iconSize
                color: row.highlighted ? Theme.selectionText : Theme.textPrimary
            }
            Label {
                Layout.fillWidth: true
                text: row.rowText
                font: Theme.body
                color: row.highlighted ? Theme.selectionText : Theme.textPrimary
                elide: Text.ElideRight
                Accessible.ignored: true
            }
        }

        background: Rectangle {
            radius: Theme.radiusSmall - 2
            color: row.highlighted ? Theme.selectionFill
                                   : (row.hovered ? Theme.hoverFill : "transparent")
        }
    }

    popup: Popup {
        y: control.height + Theme.spacingXSmall
        width: Math.max(control.width, Theme.px(200))
        implicitHeight: Math.min(contentItem.implicitHeight + topPadding + bottomPadding,
                                 Theme.px(360))
        padding: Theme.spacingXSmall

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: control.popup.visible ? control.delegateModel : null
            currentIndex: control.highlightedIndex
            boundsBehavior: Flickable.StopAtBounds
            ScrollBar.vertical: AppScrollBar {}
        }

        background: Rectangle {
            color: Theme.popupBackground
            radius: Theme.radiusMedium
            border.width: Theme.outlineWidth
            border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator
        }
    }
}
