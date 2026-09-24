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
// keyboard focus and a neutral wash otherwise.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: panel

    /// StandardKey.Find from the shell: focus the search field, text selected.
    function focusSearch() {
        searchField.forceActiveFocus(Qt.ShortcutFocusReason)
        searchField.selectAll()
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

        // Date-range filter (web workoutList.dateFrom/dateTo).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingSmall

            InputField {
                id: dateFromField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                leadingIcon: "calendar"
                placeholderText: Tr.t("workoutList.dateFrom")
                Accessible.name: Tr.t("workoutList.dateFrom")
                onEditingFinished: panel.applyDateRange()
            }

            InputField {
                id: dateToField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                leadingIcon: "calendar"
                placeholderText: Tr.t("workoutList.dateTo")
                Accessible.name: Tr.t("workoutList.dateTo")
                onEditingFinished: panel.applyDateRange()
            }
        }

        Label {
            id: dateRangeError
            Layout.fillWidth: true
            visible: false
            text: Tr.t("common.tryAgain")
            font: Theme.subheadline
            color: Theme.alertRed
            wrapMode: Text.WordWrap
            Accessible.name: text
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
                    if (event.key === Qt.Key_Escape) {
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

                                        Label {
                                            Layout.fillWidth: true
                                            text: rowItem.date_text + "  "
                                                  + rowItem.distance_text
                                            font: Theme.metricLabel
                                            color: rowItem.secondaryText
                                            elide: Text.ElideRight
                                            Accessible.ignored: true
                                        }

                                        Label {
                                            text: rowItem.pace_text
                                            font: Theme.tabularBody
                                            color: rowItem.primaryText
                                            Layout.alignment: Qt.AlignRight
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
                                    Library.selectWorkout(rowItem.workout_id)
                                }
                            }
                        }
                    }

                    // Keyboard activation: ListView's arrow keys move
                    // currentIndex; Enter/Space select the focused row.
                    Keys.onReturnPressed: Library.selectWorkout(rowItem.workout_id)
                    Keys.onEnterPressed: Library.selectWorkout(rowItem.workout_id)
                    Keys.onSpacePressed: Library.selectWorkout(rowItem.workout_id)
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

    function applyDateRange() {
        var accepted = Library.setDateRange(dateFromField.text, dateToField.text)
        dateRangeError.visible = !accepted
    }
}
