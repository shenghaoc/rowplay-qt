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
    Component.onCompleted: {
        Qt.uiLanguage = Settings.languageCode
        // The demo library starts with its default workout selected (Studio's
        // SceneStorage initial value); route to the detail screen if so.
        screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
    }

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

    // Everything lives inside one QML-created item: ApplicationWindow's
    // C++ contentItem cannot grabToImage ("item has no QML engine"), and the
    // gate screenshots need a grabbable root. The toolbar is therefore part
    // of the content, not the window `header`.
    Item {
        id: shellRoot
        anchors.fill: parent

        ColumnLayout {
            anchors.fill: parent
            spacing: 0

            ToolBar {
                Layout.fillWidth: true
                padding: Theme.spacingMedium

                RowLayout {
                    anchors.fill: parent
                    spacing: Theme.spacingLarge

                    // Studio: segmented sport picker, 280 wide ("All" + sports).
                    ComboBox {
                        id: sportFilter
                        Layout.preferredWidth: 280
                        model: [Tr.t("dashboard.all")].concat(Library.sportNames)
                        onActivated: function(index) {
                            Library.setSportFilter(index)
                        }
                        Accessible.name: Tr.t("workoutList.filtersTitle")
                    }

                    Item { Layout.fillWidth: true }

                    Button {
                        text: Tr.t("pwa.reload")
                        enabled: !Sync.isRunning
                        onClicked: Library.reload()
                        Accessible.name: Tr.t("pwa.reload")

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
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Sidebar column (Studio: min 260, ideal 320).
                SidebarPanel {
                    id: sidebarColumn
                    SplitView.preferredWidth: 320
                    SplitView.minimumWidth: 260
                    SplitView.maximumWidth: 480
                }

                // Detail column: dashboard | workout detail | settings |
                // replay route (Phase 5 renders the route itself).
                StackLayout {
                    id: detailColumn
                    SplitView.fillWidth: true
                    currentIndex: root.screenIndex

                    DashboardScreen {}

                    DetailScreen {}

                    SettingsScreen {
                        onClosed: root.toggleSettings()
                    }

                    // Replay route placeholder — the navigation policy is
                    // live (Library.requestReplay); the 3D scene is Phase 5.
                    Pane {
                        padding: Theme.spacingXxxLarge
                        ColumnLayout {
                            spacing: Theme.spacingLarge
                            Label {
                                text: Tr.t("common.replay")
                                font: Theme.pageTitle
                                color: Theme.textPrimary
                                Accessible.name: text
                            }
                            Label {
                                text: Tr.t("common.loading")
                                font: Theme.body
                                color: Theme.textSecondary
                                Accessible.name: text
                            }
                            Button {
                                text: Tr.t("common.dismiss")
                                onClicked: Library.closeReplay()
                                Accessible.name: text
                            }
                        }
                    }
                }
            }
        }

        // Studio's empty state: library empty and demo mode off.
        Rectangle {
            anchors.fill: parent
            visible: Library.isEmpty && !Settings.demoModeEnabled
                     && root.screenIndex !== 2
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
    }

    // Singleton wiring: the Library owns the selection, Detail mirrors it,
    // Settings changes invalidate both, and Sync refreshes its policy.
    Connections {
        target: Library
        function onSelectionChanged() {
            Detail.selectWorkout(Library.selectedWorkoutId)
            if (root.screenIndex !== 2 && !Library.isReplayPresented) {
                root.screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
            }
        }
        function onLibraryChanged() {
            Detail.refresh()
        }
        // Replay route presentation (Phase 5 renders it; the shell routes).
        function onIsReplayPresentedChanged() {
            if (Library.isReplayPresented) {
                root.screenIndex = 3
            } else if (root.screenIndex === 3) {
                root.screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
            }
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

    // Safety-net poll for the worker thread's cross-thread pokes: while a
    // sync runs, drain the event pump from a timer too (qt-bridges-notes).
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
    // languages (live retranslation), grab per-screen PNGs when
    // ROWPLAY_SMOKE_SCREENSHOT_DIR is set, and exit. Any QML TypeError /
    // ReferenceError / binding loop on stderr fails the gate test.
    // grabToImage renders when its callback runs, so the walk must not
    // advance until the grab landed — otherwise the PNG captures a later
    // state (observed on Wayland, where frames arrive lazily). The bailout
    // keeps platforms that never produce frames (offscreen) moving.
    property bool grabPending: false
    property int grabWaits: 0

    function grabScreen(name) {
        if (Settings.screenshotDir.length === 0) {
            return
        }
        grabPending = true
        grabWaits = 0
        shellRoot.grabToImage(function(result) {
            var path = Settings.screenshotDir + "/" + name + ".png"
            console.log("gate screenshot",
                        result.saveToFile(path) ? "saved" : "FAILED", path)
            root.grabPending = false
        })
    }

    Timer {
        id: gateTimer
        interval: 300
        repeat: true
        running: root.gateMode
        onTriggered: {
            if (root.grabPending) {
                root.grabWaits += 1
                if (root.grabWaits < 20) {
                    return
                }
                console.log("gate screenshot: grab timed out, continuing")
                root.grabPending = false
            }
            root.gateStep += 1
            switch (root.gateStep) {
            case 1:
                root.screenIndex = 0
                console.log("gate i18n en:", Tr.t("nav.dashboard"), "|",
                            Tr.t("workoutList.matching",
                                 { n: Library.filteredCount }),
                            "| uiLanguage:", Qt.uiLanguage)
                break
            case 2: root.grabScreen("dashboard"); break
            case 3: root.showSettings(); break
            case 4: root.grabScreen("settings"); break
            case 5: Settings.setLanguageIndex(1); break   // zh
            case 6:
                console.log("gate i18n zh:", Tr.t("nav.dashboard"))
                break
            case 7: Settings.setLanguageIndex(2); break   // de
            case 8: Settings.setLanguageIndex(3); break   // es
            case 9: Settings.setLanguageIndex(4); break   // fr
            case 10: Settings.setLanguageIndex(5); break  // ja
            case 11: Settings.setLanguageIndex(0); break  // en
            case 12:
                // 1001 is already selected at startup (demo default); pick a
                // different workout so the selection actually changes.
                Library.selectWorkout(1003)
                break
            case 13: root.grabScreen("detail"); break
            case 14: Library.setSportFilter(1); break
            case 15: Library.setSearchText("steady"); break
            case 16: Library.setSearchText(""); Library.setSportFilter(0); break
            case 17: Settings.setDistanceUnitIndex(1); break
            case 18: Settings.setDistanceUnitIndex(0); break
            case 19: Settings.setHomeTimezoneIndex(9); break
            case 20: Settings.setHomeTimezoneIndex(0); break
            case 21: Library.clearSelection(); break
            case 22: root.showSettings(); break
            case 23: root.screenIndex = 0; break
            case 24:
                if (Settings.syncMockMode) {
                    // End-to-end worker-thread sync against the deterministic
                    // mock client: demo mode off, start, wait, verify.
                    Settings.setDemoModeEnabled(false)
                    Sync.refresh()
                }
                break
            case 25:
                if (Settings.syncMockMode) {
                    Sync.start()
                }
                break
            case 26: case 27: case 28: case 29: case 30:
                break   // let the worker run; the safety-net timer pumps
            case 31:
                if (Settings.syncMockMode) {
                    Library.reload()
                    console.log("gate sync:", Sync.statusId,
                                "added", Sync.statusAdded,
                                "total", Sync.statusTotal,
                                "library", Library.totalCount,
                                Library.isDemoLibrary ? "demo" : "cache")
                }
                break
            case 32:
                if (Settings.syncMockMode) {
                    // Restore the shipped defaults and wipe the mock-synced
                    // cache so the next run starts clean.
                    Settings.setDemoModeEnabled(true)
                    Settings.clearCachedWorkouts()
                    Library.reload()
                }
                break
            case 33: Library.toggleSort(3); break        // pace ascending
            case 34: Library.setDateRange("2024-01-01", "2024-12-31"); break
            case 35: Library.setDateRange("nope", ""); break   // rejected
            case 36: Library.setDateRange("", ""); break       // cleared
            case 37: Library.toggleSort(0); Library.selectWorkout(9001); break
            case 38: root.grabScreen("detail-nostrokes"); break
            case 39: Library.selectWorkout(1005); break
            case 40: root.grabScreen("detail-full"); break
            case 41: Library.requestReplay(false); break       // route push
            case 42: Library.closeReplay(); Library.clearSelection(); break
            default:
                gateTimer.running = false
                Qt.exit(0)
            }
        }
    }
}
