// SPDX-License-Identifier: GPL-3.0-or-later
// Pop-up button (ADR 0013): a ComboBox on the push-button bezel with a
// trailing chevron, opening the design system's own list — the popup fill,
// a hairline outline, rows with a hover wash, the keyboard-highlighted row
// in the accent and a checkmark on the current value. Every string arrives
// translated (or is an untranslated name) from the caller's model.
//
// A long list can filter as you type (`filterable`, the timezone picker):
// the pop-up opens with a search field above the list, and typing on the
// closed button opens it with that text. The field keeps the keyboard:
// Up / Down / Page Up / Page Down move the highlighted row, Enter chooses
// it, Escape closes, and the highlighted row is announced to screen
// readers. What matches is the caller's (view-model) decision: it maps
// `filterText` to `filterIndices`.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ComboBox {
    id: control

    /// Filter as you type (a long list).
    property bool filterable: false
    /// The search field's placeholder and accessible name.
    property string filterLabel: ""
    /// What the search field holds.
    readonly property string filterText: filterField.text
    /// The model indices `filterText` keeps, in order (the caller's).
    property var filterIndices: []

    /// Types `text` into the open list's search field (the runtime-error
    /// gate; typing on the closed button does the same).
    function typeFilter(text) {
        filterField.text = text
    }

    /// Chooses model row `index` from the filtered list, as a click on a
    /// row of the plain list does.
    function choose(index) {
        if (index >= 0 && index < count) {
            if (index !== currentIndex) {
                currentIndex = index
                activated(index)
            }
            popup.close()
        }
    }

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
        color: control.enabled ? Theme.controlText : Theme.textDisabled
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        Accessible.ignored: true
    }

    indicator: Icon {
        x: control.width - width - Theme.spacingMedium
        y: Math.round((control.height - height) / 2)
        name: "chevron.down"
        size: Theme.iconSize
        color: control.enabled ? Theme.controlTextSecondary : Theme.textDisabled
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

    // Typing on the closed, filterable button opens the list with that text
    // (Space still opens it empty, as on every pop-up button).
    Keys.onPressed: function(event) {
        if (!control.filterable || control.popup.visible || event.text.length !== 1
                || event.text === " " || event.text.charCodeAt(0) < 32
                || (event.modifiers & (Qt.ControlModifier | Qt.AltModifier | Qt.MetaModifier))) {
            return
        }
        control.popup.open()
        control.typeFilter(event.text)
        event.accepted = true
    }

    popup: Popup {
        y: control.height + Theme.spacingXSmall
        width: Math.max(control.width, Theme.px(200))
        implicitHeight: Math.min(contentItem.implicitHeight + topPadding + bottomPadding,
                                 Theme.px(360))
        padding: Theme.spacingXSmall

        // The search field takes the keyboard while the filterable list is
        // open; closed, the button has it back.
        onAboutToShow: {
            if (control.filterable) {
                filterField.text = ""
            }
        }
        onOpened: {
            if (control.filterable) {
                filterField.forceActiveFocus(Qt.PopupFocusReason)
                filterList.currentIndex = Math.max(0, control.filterIndices.indexOf(control.currentIndex))
                filterList.positionViewAtIndex(filterList.currentIndex, ListView.Contain)
            }
        }

        contentItem: ColumnLayout {
            spacing: Theme.spacingXSmall

            InputField {
                id: filterField
                Layout.fillWidth: true
                Layout.margins: Theme.spacingXxSmall
                visible: control.filterable
                leadingIcon: "magnifyingglass"
                placeholderText: control.filterLabel
                Accessible.name: control.filterLabel
                // A new filter highlights its first match.
                onTextChanged: filterList.currentIndex = 0

                Keys.onUpPressed: filterList.decrementCurrentIndex()
                Keys.onDownPressed: filterList.incrementCurrentIndex()
                Keys.onPressed: function(event) {
                    var page = Math.max(1, Math.floor(filterList.height / Theme.controlHeight) - 1)
                    if (event.key === Qt.Key_PageDown) {
                        filterList.currentIndex = Math.min(filterList.count - 1,
                                                           filterList.currentIndex + page)
                        event.accepted = true
                    } else if (event.key === Qt.Key_PageUp) {
                        filterList.currentIndex = Math.max(0, filterList.currentIndex - page)
                        event.accepted = true
                    }
                }
                Keys.onReturnPressed: control.choose(filterList.currentIndex >= 0
                                                     ? control.filterIndices[filterList.currentIndex]
                                                     : -1)
                Keys.onEnterPressed: control.choose(filterList.currentIndex >= 0
                                                    ? control.filterIndices[filterList.currentIndex]
                                                    : -1)
            }

            // The plain list: the ComboBox's own rows and highlight.
            ListView {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: contentHeight
                visible: !control.filterable
                clip: true
                implicitHeight: contentHeight
                model: !control.filterable && control.popup.visible ? control.delegateModel : null
                currentIndex: control.highlightedIndex
                boundsBehavior: Flickable.StopAtBounds
                ScrollBar.vertical: AppScrollBar {}
            }

            // The filtered list: the model indices the filter keeps, drawn
            // like the plain rows; the highlight moves from the field.
            ListView {
                id: filterList
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredHeight: contentHeight
                visible: control.filterable
                clip: true
                implicitHeight: contentHeight
                model: control.filterable && control.popup.visible ? control.filterIndices : null
                boundsBehavior: Flickable.StopAtBounds
                highlightMoveDuration: 0
                ScrollBar.vertical: AppScrollBar {}

                // Screen readers follow the focus, which stays in the field,
                // so the row the keys or the filter highlight is announced.
                onCurrentIndexChanged: {
                    if (control.popup.opened && filterField.activeFocus && currentIndex >= 0
                            && currentIndex < control.filterIndices.length) {
                        filterField.Accessible.announce(
                            control.textAt(control.filterIndices[currentIndex]))
                    }
                }

                delegate: ItemDelegate {
                    id: filterRow

                    required property int modelData
                    required property int index
                    readonly property string rowText: control.textAt(modelData)
                    readonly property bool current: modelData === control.currentIndex

                    width: ListView.view.width
                    implicitHeight: Theme.controlHeight
                    leftPadding: Theme.spacingMedium
                    rightPadding: Theme.spacingMedium
                    highlighted: ListView.isCurrentItem
                    hoverEnabled: true
                    focusPolicy: Qt.NoFocus
                    Accessible.name: rowText
                    onClicked: control.choose(modelData)

                    contentItem: RowLayout {
                        spacing: Theme.spacingSmall

                        Icon {
                            Layout.preferredWidth: Theme.iconSize
                            name: "checkmark"
                            opacity: filterRow.current ? 1 : 0
                            size: Theme.iconSize
                            color: filterRow.highlighted ? Theme.selectionText : Theme.textPrimary
                        }
                        Label {
                            Layout.fillWidth: true
                            text: filterRow.rowText
                            font: Theme.body
                            color: filterRow.highlighted ? Theme.selectionText : Theme.textPrimary
                            elide: Text.ElideRight
                            Accessible.ignored: true
                        }
                    }

                    background: Rectangle {
                        radius: Theme.radiusSmall - 2
                        color: filterRow.highlighted ? Theme.selectionFill
                                                     : (filterRow.hovered ? Theme.hoverFill
                                                                          : "transparent")
                    }
                }
            }
        }

        background: Rectangle {
            color: Theme.popupBackground
            radius: Theme.radiusMedium
            border.width: Theme.outlineWidth
            border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator
        }
    }
}
