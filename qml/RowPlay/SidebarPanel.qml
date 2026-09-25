// SPDX-License-Identifier: GPL-3.0-or-later
// Library sidebar — a port of Studio's SidebarView: the search field with the
// sort menu, the date-range filter, the match count and the day-sectioned
// workout list backed by the Library QListModel. Keyboard: arrows navigate
// and select (selection follows focus, as in Studio), Enter/Space select,
// Escape clears.
//
// Design system (ADR 0013): the sidebar surface runs the full window height
// and its search row shares the content toolbar's band; day headers are
// small, bold and in sentence case; each row carries its sport glyph; the
// selection takes the accent with onAccent content while the list has
// keyboard focus and a neutral wash otherwise. Below the large width class
// the shell shows the panel in a drawer, which a chosen workout closes.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: panel

    /// Set while the panel is the shell's drawer (below the large width
    /// class): Escape in the list then closes the drawer instead of
    /// clearing the selection.
    property bool inDrawer: false

    /// A workout was chosen by a click, Enter or Space. Moving through the
    /// list with the arrows selects too (Studio's selection follows focus)
    /// but is not a choice: the drawer closes on this signal only.
    signal workoutChosen()

    /// StandardKey.Find from the shell: focus the search field, text selected.
    function focusSearch() {
        searchField.forceActiveFocus(Qt.ShortcutFocusReason)
        searchField.selectAll()
    }

    /// The drawer opened: the list takes the keyboard, so the arrows work
    /// at once.
    function focusList() {
        listView.forceActiveFocus(Qt.OtherFocusReason)
    }

    function choose(workoutId) {
        Library.selectWorkout(workoutId)
        workoutChosen()
    }

    /// The runtime-error gate opens the sort menu once.
    function showSortMenu(open) {
        if (open) {
            sortMenu.open()
        } else {
            sortMenu.close()
        }
    }

    // The search row sits in the same band as the content toolbar.
    topPadding: Math.round((Theme.toolbarHeight - Theme.controlHeight) / 2)
    leftPadding: Theme.spacingMedium + Theme.px(2)
    rightPadding: Theme.spacingMedium + Theme.px(2)
    bottomPadding: 0

    background: Rectangle {
        color: Theme.sidebarBackground
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingMedium

        // Search + sort menu (Studio's header row).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingXSmall

            InputField {
                id: searchField
                Layout.fillWidth: true
                leadingIcon: "magnifyingglass"
                placeholderText: Tr.t("workoutList.searchComments")
                Accessible.name: Tr.t("workoutList.search")
                // Debounced: a 5k-workout rebuild takes ~65 ms, so keystrokes
                // batch instead of re-filtering per character.
                onTextChanged: searchDebounce.restart()
            }

            ToolbarButton {
                id: sortButton
                iconName: "arrow.up.arrow.down"
                label: Tr.t("workoutList.sortGroup")
                onClicked: sortMenu.open()

                AppMenu {
                    id: sortMenu
                    y: sortButton.height + Theme.spacingXSmall

                    Repeater {
                        model: Library.sortFieldIds

                        AppMenuItem {
                            required property string modelData
                            required property int index
                            readonly property bool active: Library.sortFieldIndex === index

                            // Studio marks the active field and its direction:
                            // the checkmark in front, the sort arrow behind.
                            text: Tr.t(modelData)
                            checkable: true
                            checked: active
                            trailingIcon: active ? (Library.sortAscending ? "arrow.up"
                                                                          : "arrow.down")
                                                 : ""
                            // The arrow icon is decorative, so the name carries
                            // the direction, with the glyph main's label showed.
                            Accessible.name: active ? text + (Library.sortAscending ? " ↑" : " ↓")
                                                    : text
                            // A click re-sorts (and flips the direction of the
                            // active field); the checkmark follows the store.
                            onTriggered: {
                                Library.toggleSort(index)
                                checked = Qt.binding(function() { return active })
                            }
                        }
                    }
                }
            }
        }

        Timer {
            id: searchDebounce
            interval: 250
            onTriggered: Library.setSearchText(searchField.text)
        }

        // Date-range filter (web workoutList.dateFrom/dateTo). A date the
        // range refuses marks its own field, and the message sits right
        // under that field: "Please try again." and the form the field
        // takes, today's date as an example. The web has no string for an
        // invalid date (its inputs are native date pickers), so none says
        // it in words (docs/source-map.md).
        GridLayout {
            Layout.fillWidth: true
            columns: 2
            columnSpacing: Theme.spacingSmall
            rowSpacing: Theme.spacingXSmall

            InputField {
                id: dateFromField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                leadingIcon: "calendar"
                placeholderText: Tr.t("workoutList.dateFrom")
                invalid: panel.dateFromInvalid
                Accessible.name: Tr.t("workoutList.dateFrom")
                Accessible.description: invalid ? panel.dateErrorText : ""
                onEditingFinished: panel.applyDateRange()
            }

            InputField {
                id: dateToField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                leadingIcon: "calendar"
                placeholderText: Tr.t("workoutList.dateTo")
                invalid: panel.dateToInvalid
                Accessible.name: Tr.t("workoutList.dateTo")
                Accessible.description: invalid ? panel.dateErrorText : ""
                onEditingFinished: panel.applyDateRange()
            }

            Repeater {
                model: 2

                ColumnLayout {
                    required property int index
                    Layout.row: 1
                    Layout.column: index
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    visible: index === 0 ? panel.dateFromInvalid : panel.dateToInvalid
                    spacing: 0
                    // The field carries the message for screen readers.
                    Accessible.ignored: true

                    Label {
                        Layout.fillWidth: true
                        text: Tr.t("common.tryAgain")
                        font: Theme.subheadline
                        color: Theme.alertRed
                        wrapMode: Text.WordWrap
                    }
                    Label {
                        Layout.fillWidth: true
                        text: Library.dayKeyExample
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        elide: Text.ElideRight
                    }
                }
            }
        }

        // Match count: small secondary text in sentence case (not a header).
        Label {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.spacingXSmall
            Layout.topMargin: Theme.spacingXxSmall
            text: Tr.t("workoutList.matching", { n: Library.filteredCount })
            font: Theme.subheadline
            color: Theme.textSecondary
            elide: Text.ElideRight
            Accessible.name: text
        }

        Label {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: Library.filteredCount === 0
            text: Library.cacheError.length > 0 ? Library.cacheError
                                                : Tr.t("workoutList.empty")
            color: Library.cacheError.length > 0 ? Theme.alertRed
                                                 : Theme.textSecondary
            font: Theme.body
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            wrapMode: Text.WordWrap
            Accessible.name: text
        }

        // The list sits in a plain container so its focus ring can draw
        // around it: children of a ListView scroll with (and are clipped to)
        // its content.
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: Library.filteredCount > 0

            ListView {
                id: listView
                anchors.fill: parent
                clip: true
                model: Library
                spacing: Theme.spacingXxSmall
                bottomMargin: Theme.spacingMedium
                // The model is rebuilt wholesale (reset), so track the selection
                // by workout id instead of a stale index.
                currentIndex: -1
                // Whether the selected row is on screen, where its accent shows
                // that the list holds keyboard focus. A search, a filter or a
                // scroll can take it away while a workout stays selected.
                readonly property bool selectionInView: {
                    const top = contentY
                    const bottom = contentY + height
                    const rows = contentItem.children
                    for (let i = 0; i < rows.length; ++i) {
                        const row = rows[i]
                        if (row.selected === true && row.visible
                                && row.y + row.height > top && row.y < bottom) {
                            return true
                        }
                    }
                    return false
                }
                ScrollBar.vertical: AppScrollBar {}

                Keys.onPressed: function(event) {
                    if (event.key === Qt.Key_Escape && !panel.inDrawer) {
                        Library.clearSelection()
                        event.accepted = true
                    }
                }

                delegate: Item {
                    id: rowItem

                    // Role names are the QModelItem field names verbatim.
                    required property int workout_id
                    required property string title
                    required property string date_text
                    required property string time_text
                    required property string distance_text
                    required property string pace_text
                    required property string sport_key
                    required property string sport_name
                    required property bool is_pb
                    required property string section_text
                    required property bool is_section_start
                    required property string accessible_text

                    width: ListView.view.width
                    implicitHeight: rowContent.height
                              + (dayHeader.visible ? dayHeader.height : 0)
                    Accessible.name: accessible_text
                    Accessible.role: Accessible.ListItem

                    readonly property bool selected: Library.selectedWorkoutId === workout_id
                    // Focused selection: the accent with onAccent content.
                    readonly property bool emphasized: selected && listView.activeFocus
                    readonly property color primaryText: emphasized ? Theme.selectionText
                                                                    : Theme.textPrimary
                    readonly property color secondaryText: emphasized ? Theme.selectionText
                                                                      : Theme.textSecondary

                    Column {
                        anchors.left: parent.left
                        anchors.right: parent.right

                        // Day section header: small, bold, sentence case.
                        Label {
                            id: dayHeader
                            visible: rowItem.is_section_start
                            width: parent.width
                            height: implicitHeight + Theme.spacingMedium
                            topPadding: Theme.spacingMedium
                            bottomPadding: 0
                            leftPadding: Theme.spacingSmall
                            verticalAlignment: Text.AlignBottom
                            text: rowItem.section_text
                            font: Theme.sidebarSection
                            color: Theme.textSecondary
                            elide: Text.ElideRight
                            Accessible.ignored: true
                        }

                        Rectangle {
                            id: rowContent
                            width: parent.width
                            height: Math.max(Theme.sidebarRowHeight,
                                             rowLayout.implicitHeight + 2 * Theme.spacingSmall)
                            radius: Theme.radiusSmall
                            color: rowItem.emphasized ? Theme.selectionFill
                                   : (rowItem.selected ? Theme.selectionFillInactive
                                      : (rowHover.containsMouse ? Theme.hoverFill
                                                                : "transparent"))
                            // Under high contrast the unfocused selection
                            // keeps the window fill inside a highlight outline.
                            border.width: rowItem.selected && !rowItem.emphasized
                                          && Theme.highContrast ? 2 : 0
                            border.color: Theme.selectionOutline

                            RowLayout {
                                id: rowLayout
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spacingSmall
                                anchors.rightMargin: Theme.spacingMedium
                                spacing: Theme.spacingMedium

                                // Sport badge: the sport glyph on a neutral
                                // rounded square (sports carry no metric colour).
                                Rectangle {
                                    Layout.preferredWidth: Theme.px(28)
                                    Layout.preferredHeight: Theme.px(28)
                                    radius: Theme.radiusSmall + 1
                                    color: Qt.alpha(rowItem.primaryText, 0.08)
                                    Accessible.ignored: true

                                    Icon {
                                        anchors.centerIn: parent
                                        name: "sport." + rowItem.sport_key
                                        size: Theme.iconSize
                                        color: rowItem.secondaryText
                                    }
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.px(1)

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacingSmall

                                        Label {
                                            Layout.fillWidth: true
                                            text: rowItem.title
                                            font: Theme.bodyEmphasized
                                            color: rowItem.primaryText
                                            elide: Text.ElideRight
                                            Accessible.ignored: true
                                        }

                                        // PB capsule: the comparison orange
                                        // with text picked for AA on it.
                                        Rectangle {
                                            visible: rowItem.is_pb
                                            Layout.preferredHeight: pbLabel.implicitHeight
                                                                    + Theme.px(2)
                                            Layout.preferredWidth: pbLabel.implicitWidth
                                                                   + 2 * Theme.spacingSmall
                                            radius: height / 2
                                            color: Theme.comparisonOrange
                                            Accessible.ignored: true

                                            Label {
                                                id: pbLabel
                                                anchors.centerIn: parent
                                                text: Tr.t("dashboard.pbTag")
                                                font: Theme.compactLabel
                                                color: Theme.textOn(Theme.comparisonOrange)
                                            }
                                        }
                                    }

                                    RowLayout {
                                        Layout.fillWidth: true
                                        spacing: Theme.spacingSmall

                                        // The date, then the distance, which
                                        // moves under the date when the two do
                                        // not fit beside the pace: Chinese and
                                        // Japanese dates at the 12 px floor
                                        // elided it. Whole-pixel widths, so
                                        // the Flow places the second on the
                                        // pixel grid.
                                        Flow {
                                            id: metaFlow
                                            Layout.fillWidth: true
                                            spacing: Theme.spacingSmall

                                            Label {
                                                width: Math.min(Math.ceil(implicitWidth),
                                                                metaFlow.width)
                                                text: rowItem.date_text
                                                font: Theme.metricLabel
                                                color: rowItem.secondaryText
                                                elide: Text.ElideRight
                                                Accessible.ignored: true
                                            }
                                            Label {
                                                width: Math.min(Math.ceil(implicitWidth),
                                                                metaFlow.width)
                                                text: rowItem.distance_text
                                                font: Theme.metricLabel
                                                color: rowItem.secondaryText
                                                elide: Text.ElideRight
                                                Accessible.ignored: true
                                            }
                                        }

                                        Label {
                                            text: rowItem.pace_text
                                            font: Theme.tabularBody
                                            color: rowItem.primaryText
                                            Layout.alignment: Qt.AlignRight | Qt.AlignTop
                                            Accessible.ignored: true
                                        }
                                    }
                                }
                            }

                            MouseArea {
                                id: rowHover
                                anchors.fill: parent
                                hoverEnabled: true
                                onClicked: {
                                    // A click focuses the list, so the
                                    // selection shows in the accent.
                                    listView.forceActiveFocus(Qt.MouseFocusReason)
                                    panel.choose(rowItem.workout_id)
                                }
                            }
                        }
                    }

                    // Keyboard activation: ListView's arrow keys move
                    // currentIndex; Enter/Space select the focused row.
                    Keys.onReturnPressed: panel.choose(rowItem.workout_id)
                    Keys.onEnterPressed: panel.choose(rowItem.workout_id)
                    Keys.onSpacePressed: panel.choose(rowItem.workout_id)
                }

                // Arrow navigation selects, matching Studio's sidebar list.
                onCurrentIndexChanged: {
                    if (currentIndex < 0 || currentIndex >= count) {
                        return
                    }
                    var item = itemAtIndex(currentIndex)
                    if (item !== null) {
                        Library.selectWorkout(item.workout_id)
                    }
                }

                // The list itself takes focus, so the arrows work without
                // clicking a row first.
                activeFocusOnTab: true
            }

            // With no selected row on screen there is no accent row to show
            // that the list holds keyboard focus, so ring the list itself. Its
            // bottom edge stays inside the window.
            FocusRing {
                visible: listView.activeFocus && !listView.selectionInView
                controlRadius: Theme.radiusSmall
                anchors.bottomMargin: Theme.px(2)
            }
        }
    }

    // Each date field on its own: which one the range refused.
    property bool dateFromInvalid: false
    property bool dateToInvalid: false
    readonly property string dateErrorText: Tr.t("common.tryAgain") + " "
                                            + Library.dayKeyExample

    function applyDateRange() {
        dateFromInvalid = !Library.isDayKey(dateFromField.text)
        dateToInvalid = !Library.isDayKey(dateToField.text)
        if (!dateFromInvalid && !dateToInvalid) {
            Library.setDateRange(dateFromField.text, dateToField.text)
        }
    }
}
