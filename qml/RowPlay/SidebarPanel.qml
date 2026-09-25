// SPDX-License-Identifier: GPL-3.0-or-later
// Library sidebar — a port of Studio's SidebarView: the search field with the
// sort menu, the date-range filter, the match count and the day-sectioned
// workout list backed by the Library QListModel. Keyboard: arrows navigate
// and select (selection follows focus, as in Studio), Enter/Space select,
// Escape clears.
//
// ADR 0013, 0015: the sidebar surface runs the full window height and its
// search row shares the content toolbar's band; the fields, the sort menu
// and the rows are the style's own (the rows carry the workout's data);
// day headers are the list's sections, small, bold and in sentence case;
// each row carries its sport glyph; the selection uses the style's highlight
// while the list has keyboard focus and a neutral content wash otherwise.
// Below the expanded width class the shell shows the panel in
// a drawer, which a chosen workout closes.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: panel

    /// Set while the panel is the shell's drawer (below the expanded width
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

    /// The runtime-error gate enters a date range through the fields, as
    /// typing and leaving them does.
    function enterDateRange(from, to) {
        dateFromField.text = from
        dateToField.text = to
        applyDateRange()
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

            // The style's text field, not its search field: Qt 6.11's
            // SearchField draws no placeholder, and "Search comments…" says
            // what the search covers.
            TextField {
                id: searchField
                Layout.fillWidth: true
                placeholderText: Tr.t("workoutList.searchComments")
                Accessible.name: Tr.t("workoutList.search")
                // Debounced: a 5k-workout rebuild takes ~65 ms, so keystrokes
                // batch instead of re-filtering per character.
                onTextChanged: searchDebounce.restart()
            }

            ToolButton {
                id: sortButton
                display: AbstractButton.IconOnly
                icon.name: Glyphs.iconName("arrow.up.arrow.down")
                icon.source: Glyphs.iconSource("arrow.up.arrow.down")
                icon.color: enabled ? Theme.textPrimary : Theme.textDisabled
                icon.width: Theme.iconSize
                icon.height: Theme.iconSize
                text: Tr.t("workoutList.sortGroup")
                focusPolicy: Qt.TabFocus
                hoverEnabled: true
                Accessible.name: text
                ToolTip.visible: hovered && !sortMenu.visible
                ToolTip.delay: 600
                ToolTip.text: text
                onClicked: sortMenu.open()

                Menu {
                    id: sortMenu
                    y: sortButton.height + Theme.spacingXSmall

                    Repeater {
                        model: Library.sortFieldIds

                        MenuItem {
                            required property string modelData
                            required property int index
                            readonly property bool active: Library.sortFieldIndex === index

                            // Studio marks the active field and its direction:
                            // the style's check, and the sort arrow after the
                            // label (as an item icon it pushed the label out
                            // of line with the others). The arrow is a symbol,
                            // the same in every language.
                            text: active ? Tr.t(modelData) + (Library.sortAscending ? " ↑" : " ↓")
                                         : Tr.t(modelData)
                            checkable: true
                            checked: active
                            Accessible.name: text
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

            TextField {
                id: dateFromField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                placeholderText: Tr.t("workoutList.dateFrom")
                Accessible.name: Tr.t("workoutList.dateFrom")
                Accessible.description: panel.dateFromInvalid ? panel.dateErrorText : ""
                onEditingFinished: panel.applyDateRange()
            }

            TextField {
                id: dateToField
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                placeholderText: Tr.t("workoutList.dateTo")
                Accessible.name: Tr.t("workoutList.dateTo")
                Accessible.description: panel.dateToInvalid ? panel.dateErrorText : ""
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

            // A ScrollView hosts the list so the style's scroll bar gets
            // room where it is not transient (macOS with a mouse, Windows,
            // Fusion) instead of drawing over the rows' trailing edge.
            ScrollView {
                id: listScroll
                anchors.fill: parent

                ListView {
                    id: listView
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
                    Keys.onPressed: function(event) {
                        if (event.key === Qt.Key_Escape && !panel.inDrawer) {
                            Library.clearSelection()
                            event.accepted = true
                        }
                    }

                    // Day sections: section_text is populated on every row of
                    // a day, so adjacent workouts cannot create a blank group.
                    section.property: "section_text"
                    section.delegate: Label {
                        required property string section
                        width: ListView.view.width
                        topPadding: Theme.spacingMedium
                        bottomPadding: Theme.spacingXxSmall
                        leftPadding: Theme.spacingSmall
                        text: section
                        font: Theme.sidebarSection
                        color: Theme.textSecondary
                        elide: Text.ElideRight
                        Accessible.ignored: true
                    }

                    // A row is the style's item delegate: its background,
                    // selection and hover are the style's, and it carries the
                    // workout's data as its content (ADR 0015's list-delegate
                    // exception). The native highlight is used while the list
                    // has focus. The inactive row keeps a neutral wash in its
                    // content, without overriding the control's palette.
                    delegate: ItemDelegate {
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
                        required property string accessible_text

                        width: ListView.view.width
                        highlighted: selected && listView.activeFocus
                        focusPolicy: Qt.NoFocus
                        Accessible.name: accessible_text
                        Accessible.role: Accessible.ListItem

                        readonly property bool selected: Library.selectedWorkoutId === workout_id
                        readonly property color primaryText: highlighted ? palette.highlightedText
                                                                         : Theme.textPrimary
                        readonly property color secondaryText: highlighted ? palette.highlightedText
                                                                           : Theme.textSecondary

                        onClicked: {
                            // A click focuses the list, so the selection shows
                            // in the active highlight.
                            listView.forceActiveFocus(Qt.MouseFocusReason)
                            panel.choose(rowItem.workout_id)
                        }

                        contentItem: Item {
                            implicitWidth: rowContent.implicitWidth
                            implicitHeight: rowContent.implicitHeight

                            Rectangle {
                                anchors.fill: parent
                                visible: rowItem.selected && !listView.activeFocus
                                color: Theme.selectionFillInactive
                                radius: Theme.radiusSmall
                                border.width: Theme.highContrast ? Theme.px(2) : 0
                                border.color: Theme.selectionOutline
                                Accessible.ignored: true
                            }

                            RowLayout {
                                id: rowContent
                                anchors.fill: parent
                                spacing: Theme.spacingMedium

                                // Sport badge: the sport glyph on a neutral rounded
                                // square (sports carry no metric colour).
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

                                        // PB capsule: the comparison orange with
                                        // text picked for AA on it.
                                        Rectangle {
                                            visible: rowItem.is_pb
                                            Layout.preferredHeight: pbLabel.implicitHeight + Theme.px(2)
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

                                        // The date, then the distance, which moves
                                        // under the date when the two do not fit
                                        // beside the pace: Chinese and Japanese
                                        // dates at the 12 px floor elided it.
                                        // Whole-pixel widths, so the Flow places the
                                        // second on the pixel grid.
                                        Flow {
                                            id: metaFlow
                                            Layout.fillWidth: true
                                            spacing: Theme.spacingSmall

                                            Label {
                                                width: Math.min(Math.ceil(implicitWidth), metaFlow.width)
                                                text: rowItem.date_text
                                                font: Theme.metricLabel
                                                color: rowItem.secondaryText
                                                elide: Text.ElideRight
                                                Accessible.ignored: true
                                            }
                                            Label {
                                                width: Math.min(Math.ceil(implicitWidth), metaFlow.width)
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
