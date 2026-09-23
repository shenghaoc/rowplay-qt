// SPDX-License-Identifier: GPL-3.0-or-later
// Library sidebar — a port of Studio's SidebarView: the search field with the
// sort menu, the date-range filter, the match count and the day-sectioned
// workout list backed by the Library QListModel. Keyboard: arrows navigate
// and select (macOS sidebar behaviour), Enter/Space select, Escape clears.
//
// Apple HIG Sidebars (ADR 0013): the sidebar material runs the full window
// height, section headers are small bold sentence case, each row carries a
// sport glyph badge, and the selection is the accent fill with white content
// while the list has keyboard focus, a neutral fill otherwise.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: panel

    /// Ctrl/Cmd+F from the shell: focus the search field, text selected.
    function focusSearch() {
        searchField.forceActiveFocus(Qt.ShortcutFocusReason)
        searchField.selectAll()
    }

    // The search row sits in the same 52 px band as the content toolbar.
    topPadding: (Theme.toolbarHeight - Theme.controlHeight) / 2
    leftPadding: Theme.spacingMedium + 2
    rightPadding: Theme.spacingMedium + 2
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

                Menu {
                    id: sortMenu
                    y: sortButton.height + Theme.spacingXSmall

                    Repeater {
                        model: Library.sortFieldIds

                        MenuItem {
                            id: sortItem

                            required property string modelData
                            required property int index
                            readonly property bool active: Library.sortFieldIndex === index

                            text: Tr.t(modelData)
                            onTriggered: Library.toggleSort(index)
                            Accessible.name: text

                            // Studio marks the active field and its direction:
                            // a checkmark in front, the sort arrow behind.
                            contentItem: RowLayout {
                                spacing: Theme.spacingSmall

                                Item {
                                    Layout.preferredWidth: 14
                                    Layout.preferredHeight: 14

                                    Icon {
                                        anchors.centerIn: parent
                                        visible: sortItem.active
                                        name: "checkmark"
                                        size: 14
                                        color: sortItem.highlighted ? "#ffffff"
                                                                    : Theme.textPrimary
                                    }
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: sortItem.text
                                    font: Theme.body
                                    color: sortItem.highlighted ? "#ffffff"
                                                                : Theme.textPrimary
                                    Accessible.ignored: true
                                }
                                Icon {
                                    visible: sortItem.active
                                    name: Library.sortAscending ? "arrow.up" : "arrow.down"
                                    size: 12
                                    color: sortItem.highlighted ? "#ffffff"
                                                                : Theme.textSecondary
                                }
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
            font: Theme.metricLabel
            color: Theme.alertRed
            Accessible.name: text
        }

        // Match count: small secondary text, sentence case (not a header).
        Label {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.spacingXSmall
            Layout.topMargin: Theme.spacingXxSmall
            text: Tr.t("workoutList.matching", { n: Library.filteredCount })
            font.pixelSize: 11
            color: Theme.textSecondary
            elide: Text.ElideRight
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
                    // Focused selection: accent fill, white content (HIG).
                    readonly property bool emphasized: selected && listView.activeFocus
                    readonly property color primaryText: emphasized ? "#ffffff"
                                                                    : Theme.textPrimary
                    readonly property color secondaryText: emphasized
                                                           ? Qt.rgba(1, 1, 1, 0.85)
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
                            height: Theme.sidebarRowHeight
                            radius: Theme.radiusSmall
                            color: rowItem.emphasized ? Theme.selectionFill
                                   : (rowItem.selected ? Theme.selectionFillInactive
                                      : (rowHover.containsMouse ? Theme.hoverFill
                                                                : "transparent"))

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: Theme.spacingSmall
                                anchors.rightMargin: Theme.spacingMedium
                                spacing: Theme.spacingMedium

                                // Sport badge: the sport glyph on a neutral
                                // rounded square (sports carry no metric colour).
                                Rectangle {
                                    Layout.preferredWidth: 28
                                    Layout.preferredHeight: 28
                                    radius: 7
                                    color: rowItem.emphasized ? Qt.rgba(1, 1, 1, 0.2)
                                                              : Qt.alpha(Theme.textPrimary, 0.07)
                                    Accessible.ignored: true

                                    Icon {
                                        anchors.centerIn: parent
                                        name: "sport." + rowItem.sport_key
                                        size: 16
                                        color: rowItem.emphasized ? "#ffffff"
                                                                  : Theme.textSecondary
                                    }
                                }

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 1

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

                                        // PB capsule (Studio: comparisonOrange at
                                        // 15% behind compact text).
                                        Rectangle {
                                            visible: rowItem.is_pb
                                            Layout.preferredHeight: 15
                                            Layout.preferredWidth: pbLabel.implicitWidth
                                                                  + 2 * Theme.spacingSmall
                                            radius: height / 2
                                            color: rowItem.emphasized
                                                   ? Qt.rgba(1, 1, 1, 0.22)
                                                   : Qt.alpha(Theme.comparisonOrange, 0.15)
                                            Accessible.ignored: true

                                            Label {
                                                id: pbLabel
                                                anchors.centerIn: parent
                                                text: Tr.t("dashboard.pbTag")
                                                font: Theme.compactLabel
                                                color: rowItem.emphasized ? "#ffffff"
                                                                          : Theme.comparisonOrange
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
                                            font.pixelSize: 11
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
                                    // Clicking a row focuses the list, like a
                                    // macOS sidebar: the selection turns accent.
                                    listView.forceActiveFocus(Qt.MouseFocusReason)
                                    Library.selectWorkout(rowItem.workout_id)
                                }
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

            // With nothing selected there is no accent row to show that the
            // list holds keyboard focus, so ring the list itself.
            FocusRing {
                visible: listView.activeFocus && Library.selectedWorkoutId === -1
                controlRadius: Theme.radiusSmall
            }
        }
    }

    function applyDateRange() {
        var accepted = Library.setDateRange(dateFromField.text, dateToField.text)
        dateRangeError.visible = !accepted
    }
}
