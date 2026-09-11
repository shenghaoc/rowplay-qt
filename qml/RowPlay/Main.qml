// SPDX-License-Identifier: GPL-3.0-or-later
// The application shell — a port of rowplay-studio's ContentView:
// a split view (sidebar column min 260 / ideal 320, detail area), the sport
// filter and reload toolbar, the dashboard/detail navigation state and the
// "Ready When You Are" empty state. The sidebar and detail *screens* are
// placeholders in Phase 4a and become the real ports in 4b; the window title
// and 1000x680 minimum come from Studio's app scene.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ApplicationWindow {
    id: root

    visible: true
    width: 1200
    height: 800
    minimumWidth: 1000
    minimumHeight: 680
    title: "rowplay"

    // Detail column routing: 0 = dashboard, 1 = workout detail, 2 = settings.
    property int screenIndex: 0
    // The runtime-error gate walks every screen and exercises the language
    // switch (driven from Rust by ROWPLAY_SMOKE_GATE=1).
    readonly property bool gateMode: Settings.gateMode
    property int gateStep: 0

    // Live retranslation: QQmlApplicationEngine reloads
    // :/qt/qml/RowPlay/i18n/qml_<lang>.qm whenever Qt.uiLanguage changes.
    // `Qt` is a JS global, not a QObject, so a Binding element cannot target
    // it — assign imperatively at startup and on every settings change.
    Component.onCompleted: Qt.uiLanguage = Settings.languageCode

    // Fusion, coloured from Theme.qml (DESIGN.md: flat, tonal, no shadows).
    palette.window: Theme.windowBackground
    palette.windowText: Theme.textPrimary
    palette.base: Theme.windowBackground
    palette.alternateBase: Theme.panelBackground
    palette.text: Theme.textPrimary
    palette.button: Theme.windowBackground
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accentColor
    palette.highlightedText: "#ffffff"
    palette.toolTipBase: Theme.overlayBackground
    palette.toolTipText: Theme.textPrimary
    font.pixelSize: 13

    function showDashboard() {
        Library.clearSelection()
        screenIndex = 0
    }

    function showSettings() {
        screenIndex = 2
    }

    function toggleSettings() {
        screenIndex = screenIndex === 2 ? (Library.selectedWorkoutId === -1 ? 0 : 1) : 2
    }

    header: ToolBar {
        padding: Theme.spacingMedium

        RowLayout {
            anchors.fill: parent
            spacing: Theme.spacingLarge

            // Studio: segmented sport picker, 280 wide ("All" + the sports).
            ComboBox {
                id: sportFilter
                Layout.preferredWidth: 280
                model: [Tr.t("dashboard.all")].concat(Library.sportNames)
                currentIndex: Library.sportFilterIndex
                onActivated: Library.setSportFilter(currentIndex)
                Accessible.name: Tr.t("workoutList.filtersTitle")
            }

            Item { Layout.fillWidth: true }

            Button {
                text: Tr.t("pwa.reload")
                enabled: !Sync.isRunning
                onClicked: Library.reload()
                Accessible.name: Tr.t("pwa.reload")
                ToolTip.visible: hovered
                ToolTip.text: Tr.t("sync.inProgress")
                ToolTip.delay: 800

                Shortcut {
                    sequences: ["Ctrl+R", "Meta+R"]
                    onActivated: Library.reload()
                }
            }

            Button {
                text: Tr.t("nav.settings")
                checkable: true
                checked: root.screenIndex === 2
                onClicked: root.toggleSettings()
                Accessible.name: Tr.t("nav.settings")
            }
        }
    }

    SplitView {
        anchors.fill: parent

        // Sidebar column (placeholder panel in 4a, the grouped library list
        // in 4b). Studio: min 260, ideal 320.
        Pane {
            id: sidebarColumn
            SplitView.preferredWidth: 320
            SplitView.minimumWidth: 260
            SplitView.maximumWidth: 480
            padding: Theme.spacingLarge

            ColumnLayout {
                anchors.fill: parent
                spacing: Theme.spacingMedium

                Label {
                    text: Tr.t("workoutList.matching", { n: Library.filteredCount })
                    font: Theme.compactLabel
                    color: Theme.textTertiary
                    Accessible.name: text
                }

                TextField {
                    id: searchField
                    Layout.fillWidth: true
                    placeholderText: Tr.t("workoutList.searchComments")
                    Accessible.name: Tr.t("workoutList.search")
                    onTextChanged: Library.setSearchText(text)
                }

                Label {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    visible: Library.filteredCount === 0
                    text: Tr.t("workoutList.empty")
                    color: Theme.textSecondary
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }

                // Sidebar list placeholder: the 4b QListModel lands here.
                Label {
                    Layout.fillWidth: true
                    visible: Library.filteredCount > 0
                    text: Tr.t("nav.dashboard")
                    color: Theme.textTertiary
                    horizontalAlignment: Text.AlignHCenter
                    Accessible.name: text
                }
            }
        }

        // Detail column.
        StackLayout {
            id: detailColumn
            SplitView.fillWidth: true
            currentIndex: root.screenIndex

            // Dashboard placeholder (4b ports DashboardView).
            Pane {
                padding: Theme.spacingXxxLarge
                ColumnLayout {
                    spacing: Theme.spacingLarge
                    Label {
                        text: Tr.t("nav.dashboard")
                        font: Theme.pageTitle
                        color: Theme.textPrimary
                        Accessible.name: text
                    }
                    Label {
                        text: Tr.t("dashboard.title")
                        font: Theme.sectionHeadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                }
            }

            // Workout detail placeholder (4b ports WorkoutDetailView).
            Pane {
                padding: Theme.spacingXxxLarge
                ColumnLayout {
                    spacing: Theme.spacingLarge
                    Label {
                        text: Detail.hasSelection ? Detail.workoutType : ""
                        font: Theme.pageTitle
                        color: Theme.textPrimary
                        Accessible.name: text
                    }
                    Label {
                        text: Detail.dateText + "  " + Detail.sportName
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                }
            }

            SettingsScreen {
                onClosed: root.toggleSettings()
            }
        }
    }

    // Studio's empty state: library empty and demo mode off.
    Rectangle {
        anchors.fill: parent
        visible: Library.isEmpty && !Settings.demoModeEnabled && root.screenIndex !== 2
        color: Theme.windowBackground

        ColumnLayout {
            anchors.centerIn: parent
            spacing: Theme.spacingXxxLarge

            ColumnLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: Theme.spacingLarge

                Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: Tr.t("landing.title1")
                    font: Theme.pageTitle
                    color: Theme.metricDuration
                    Accessible.name: text
                }
                Label {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.maximumWidth: 420
                    text: Tr.t("landing.lead")
                    font: Theme.body
                    color: Theme.textSecondary
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                    Accessible.name: text
                }
            }

            ColumnLayout {
                Layout.alignment: Qt.AlignHCenter
                spacing: Theme.spacingMedium

                Button {
                    Layout.alignment: Qt.AlignHCenter
                    text: Tr.t("landing.exploreDemo")
                    highlighted: true
                    onClicked: Settings.setDemoModeEnabled(true)
                    Accessible.name: text
                }
                Button {
                    Layout.alignment: Qt.AlignHCenter
                    text: Tr.t("landing.connect")
                    onClicked: root.showSettings()
                    Accessible.name: text
                }
            }
        }
    }

    // Singleton wiring: the Library owns the selection, Detail mirrors it,
    // Settings changes invalidate both, and Sync refreshes its policy.
    Connections {
        target: Library
        function onSelectionChanged() {
            Detail.selectWorkout(Library.selectedWorkoutId)
            if (root.screenIndex !== 2) {
                root.screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
            }
        }
        function onLibraryChanged() {
            Detail.refresh()
        }
    }
    Connections {
        target: Settings
        function onSettingsChanged() {
            Qt.uiLanguage = Settings.languageCode
            Sync.refresh()
            Library.reload()
        }
    }

    // Safety net for the worker thread's cross-thread pokes: while a sync
    // runs, poll the event pump too (see docs/qt-bridges-notes.md).
    Timer {
        interval: 50
        repeat: true
        running: Sync.isRunning
        onTriggered: Sync.pumpEvents()
    }

    // Keyboard navigation (Studio: Cmd+1 dashboard, Escape clears selection).
    Shortcut {
        sequences: ["Ctrl+1", "Meta+1"]
        onActivated: root.showDashboard()
    }
    Shortcut {
        sequence: "Escape"
        onActivated: {
            if (root.screenIndex === 2) {
                root.toggleSettings()
            } else {
                Library.clearSelection()
            }
        }
    }

    // CI runtime-error gate: walk the screens, flip through all six
    // languages (live retranslation) and exit. Any QML TypeError /
    // ReferenceError / binding loop on stderr fails the gate test.
    Timer {
        id: gateTimer
        interval: 300
        repeat: true
        running: root.gateMode
        onTriggered: {
            root.gateStep += 1
            switch (root.gateStep) {
            case 1:
                root.screenIndex = 0
                console.log("gate i18n en:", Tr.t("nav.dashboard"), "|",
                            Tr.t("workoutList.matching", { n: Library.filteredCount }),
                            "| uiLanguage:", Qt.uiLanguage)
                break
            case 2: root.showSettings(); break
            case 3:
                Settings.setLanguageIndex(1)   // zh
                console.log("gate i18n zh:", Tr.t("nav.dashboard"))
                break
            case 4: Settings.setLanguageIndex(2); break   // de
            case 5: Settings.setLanguageIndex(3); break   // es
            case 6: Settings.setLanguageIndex(4); break   // fr
            case 7: Settings.setLanguageIndex(5); break   // ja
            case 8: Settings.setLanguageIndex(0); break   // en
            case 9: root.screenIndex = 0; Library.selectWorkout(1001); break
            case 10: Library.setSportFilter(1); break
            case 11: Library.setSearchText("steady"); break
            case 12: Library.setSearchText(""); Library.setSportFilter(0); break
            case 13: Settings.setDistanceUnitIndex(1); break
            case 14: Settings.setDistanceUnitIndex(0); break
            case 15: Settings.setHomeTimezoneIndex(9); break
            case 16: Settings.setHomeTimezoneIndex(0); break
            case 17: Library.clearSelection(); break
            case 18: root.showSettings(); break
            case 19: root.screenIndex = 0; break
            default:
                gateTimer.running = false
                Qt.exit(0)
            }
        }
    }
}
