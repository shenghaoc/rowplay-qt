// SPDX-License-Identifier: GPL-3.0-or-later
// Library sidebar — a port of Studio's SidebarView: count header with the
// sort menu, search field, date-range filter, and the day-sectioned workout
// list backed by the Library QListModel. Keyboard: arrows navigate and
// select (macOS sidebar behaviour), Enter/Space select, Escape clears.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: panel

    padding: Theme.spacingLarge

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingMedium

        // Count header + sort menu (Studio's section header row).
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingXSmall

            Label {
                Layout.fillWidth: true
                // Studio text-cases the header; uppercasing is a
                // presentation concern, not number formatting.
                text: Tr.t("workoutList.matching",
                           { n: Library.filteredCount }).toUpperCase()
                font: Theme.compactLabel
                color: Theme.textTertiary
                Accessible.name: text
            }

            Button {
                id: sortButton
                flat: true
                text: Library.sortAscending ? "↑" : "↓"
                Accessible.name: Tr.t("workoutList.sortGroup")
                ToolTip.visible: hovered
                ToolTip.text: Tr.t("workoutList.sortGroup")

                onClicked: sortMenu.open()

                Menu {
                    id: sortMenu
                    y: sortButton.height

                    Repeater {
                        model: Library.sortFieldIds

                        MenuItem {
                            required property string modelData
                            required property int index
                            text: Tr.t(modelData)
                            // Studio marks the active field with an arrow.
                            icon.source: ""
                            onTriggered: Library.toggleSort(index)

                            contentItem: Label {
                                text: parent.text
                                  + (Library.sortFieldIndex === parent.index
                                     ? (Library.sortAscending ? "  ↑" : "  ↓") : "")
                                font: Theme.body
                                color: Theme.textPrimary
                            }
                        }
                    }
                }
            }
        }

        TextField {
            id: searchField
            Layout.fillWidth: true
            placeholderText: Tr.t("workoutList.searchComments")
            Accessible.name: Tr.t("workoutList.search")
            // Debounced: a 5k-workout rebuild takes ~65 ms, so keystrokes
            // batch instead of re-filtering per character.
            onTextChanged: searchDebounce.restart()
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

            TextField {
                id: dateFromField
                Layout.fillWidth: true
                placeholderText: Tr.t("workoutList.dateFrom")
                Accessible.name: Tr.t("workoutList.dateFrom")
                onEditingFinished: applyDateRange()
            }

            TextField {
                id: dateToField
                Layout.fillWidth: true
                placeholderText: Tr.t("workoutList.dateTo")
                Accessible.name: Tr.t("workoutList.dateTo")
                onEditingFinished: applyDateRange()
            }
        }

        Label {
            id: dateRangeError
            Layout.fillWidth: true
            visible: false
            text: Tr.t("common.tryAgain")
            font: Theme.metricLabel
            color: Theme.alertRed
            Accessible.name: text
        }

        Label {
            Layout.fillWidth: true
            visible: Library.filteredCount === 0
            text: Library.cacheError.length > 0 ? Library.cacheError
                                                : Tr.t("workoutList.empty")
            color: Library.cacheError.length > 0 ? Theme.alertRed
                                                 : Theme.textSecondary
            font: Theme.body
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
            Layout.fillHeight: true
            wrapMode: Text.WordWrap
            Accessible.name: text
        }

        ListView {
            id: listView
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: Library.filteredCount > 0
            clip: true
            model: Library
            spacing: Theme.spacingXxSmall
            // The model is rebuilt wholesale (reset), so track the selection
            // by workout id instead of a stale index.
            currentIndex: -1

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
                implicitHeight: rowContent.implicitHeight
                          + (dayHeader.visible ? dayHeader.implicitHeight
                                               + Theme.spacingXSmall : 0)
                Accessible.name: accessible_text
                Accessible.role: Accessible.ListItem

                property bool selected: Library.selectedWorkoutId === workout_id

                Column {
                    id: rowContentColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    spacing: Theme.spacingXxSmall

                    Label {
                        id: dayHeader
                        visible: rowItem.is_section_start
                        text: rowItem.section_text.toUpperCase()
                        font: Theme.compactLabel
                        color: Theme.textTertiary
                        Accessible.ignored: true
                    }

                    Rectangle {
                        id: rowContent
                        width: parent.width
                        implicitHeight: innerColumn.implicitHeight
                                      + 2 * Theme.spacingSmall
                        radius: Theme.radiusSmall
                        color: rowItem.selected ? Theme.activeCardBackground
                                                : "transparent"

                        RowLayout {
                            id: innerColumn
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            anchors.margins: Theme.spacingSmall
                            spacing: Theme.spacingSmall

                            // Sport badge: initial letter on a tonal chip
                            // (SF Symbols have no Qt equivalent).
                            Rectangle {
                                Layout.preferredWidth: 22
                                Layout.preferredHeight: 22
                                radius: Theme.radiusSmall
                                color: Theme.panelBackground
                                Accessible.ignored: true

                                Label {
                                    anchors.centerIn: parent
                                    text: rowItem.sport_name.charAt(0)
                                    font: Theme.compactLabel
                                    color: Theme.textSecondary
                                }
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 0

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacingSmall

                                    Label {
                                        Layout.fillWidth: true
                                        text: rowItem.title
                                        font: Theme.body
                                        color: Theme.textPrimary
                                        elide: Text.ElideRight
                                        Accessible.ignored: true
                                    }

                                    // PB capsule (Studio: comparisonOrange
                                    // at 15% behind compact text).
                                    Rectangle {
                                        visible: rowItem.is_pb
                                        Layout.preferredHeight: 14
                                        Layout.preferredWidth: pbLabel.implicitWidth
                                                              + 2 * Theme.spacingXSmall
                                        radius: height / 2
                                        color: Qt.alpha(Theme.comparisonOrange, 0.15)
                                        Accessible.ignored: true

                                        Label {
                                            id: pbLabel
                                            anchors.centerIn: parent
                                            text: Tr.t("dashboard.pbTag")
                                            font: Theme.compactLabel
                                            color: Theme.comparisonOrange
                                        }
                                    }
                                }

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacingSmall

                                    Label {
                                        text: rowItem.date_text + "  "
                                              + rowItem.distance_text
                                        font: Theme.metricLabel
                                        color: Theme.textSecondary
                                        elide: Text.ElideRight
                                        Layout.fillWidth: true
                                        Accessible.ignored: true
                                    }

                                    Label {
                                        text: rowItem.pace_text
                                        font: Theme.metricValue
                                        color: Theme.textPrimary
                                        Layout.alignment: Qt.AlignRight
                                        Accessible.ignored: true
                                    }
                                }
                            }
                        }

                        MouseArea {
                            anchors.fill: parent
                            onClicked: Library.selectWorkout(rowItem.workout_id)
                        }
                    }
                }

                // Keyboard activation: ListView's arrow keys move
                // currentIndex; activate the focused row like the macOS
                // sidebar (selection follows focus).
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

            // Keep focus traversal sane: the list itself takes focus so the
            // arrows work without clicking a row first.
            activeFocusOnTab: true
        }
    }

    function applyDateRange() {
        var accepted = Library.setDateRange(dateFromField.text, dateToField.text)
        dateRangeError.visible = !accepted
    }
}
