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
import RowPlay.Replay

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

                    // Replay route (Phase 5b): the 3D scene with the chase
                    // camera, athlete posing and transport controls.
                    ReplayScene {
                        Component.onCompleted: {
                            Replay.setSchemeDark(Theme.dark)
                            Replay.setReduceMotion(Settings.reduceReplayMotion)
                            Replay.setQualityIndex(Settings.qualityIndex)
                        }
                        Connections {
                            target: Theme
                            function onDarkChanged() {
                                Replay.setSchemeDark(Theme.dark)
                            }
                        }
                        Connections {
                            target: Settings
                            function onSettingsChanged() {
                                Replay.setReduceMotion(Settings.reduceReplayMotion)
                                Replay.setQualityIndex(Settings.qualityIndex)
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
        // Replay route: load the workout and switch the stack index.
        function onIsReplayPresentedChanged() {
            if (Library.isReplayPresented) {
                root.screenIndex = 3
                Replay.loadWorkout(Library.selectedWorkoutId)
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
    // The sync worker writes to the cache on its own thread; every few
    // details (and on completion) it asks the shell to re-read the library so
    // new rows appear while the sync runs instead of after it.
    Connections {
        target: Sync
        function onLibraryRefreshRequested() {
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

    /// Probes every `Singleton.member` pair the QML sources reference (the
    /// list is scanned from `qml/` by the gate test and handed over as
    /// `ROWPLAY_GATE_MEMBER_CHECK`). A property that is not registered on the
    /// qtbridge object reads as `undefined` with no QML error or binding
    /// warning — the exact hole that let `Sync.progressText` ship — so an
    /// unresolved member is reported loudly for the test to fail on.
    function checkGateMembers() {
        if (Settings.gateMemberCheck.length === 0) {
            return
        }
        var objects = {
            "Library": Library, "Detail": Detail,
            "Settings": Settings, "Sync": Sync, "Replay": Replay
        }
        var pairs = Settings.gateMemberCheck.split(",")
        var missing = []
        for (var i = 0; i < pairs.length; ++i) {
            var pair = pairs[i]
            if (pair.length === 0) {
                continue
            }
            var dot = pair.indexOf(".")
            if (dot < 0) {
                continue
            }
            var object = objects[pair.slice(0, dot)]
            var member = pair.slice(dot + 1)
            if (object === undefined || typeof object[member] === "undefined") {
                missing.push(pair)
            }
        }
        console.log("gate members:", pairs.length - missing.length, "of",
                    pairs.length, "resolved")
        if (missing.length > 0) {
            console.log("gate members unresolved:", missing.join(" "))
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
    // Set when a gate step starts a sync; the walk then holds until the
    // worker is genuinely idle, so the following step reports real counts
    // instead of a mid-run snapshot (the full re-sync is the slowest).
    // Bench mode: the gate holds while the ReplayScene collects 600 frames.
    property bool gateAwaitingBench: false
    function benchRun(label) {
        // Find the ReplayScene component (it's the 4th child of detailColumn)
        var replayScene = detailColumn.children[3]
        if (replayScene && replayScene.benchStart) {
            replayScene.benchStart(label)
            root.gateAwaitingBench = true
        }
    }

    property bool gateAwaitingSync: false
    // Set while the replay scene applies its rules; the walk holds until the
    // scene reports ready (bounded, so a broken pack cannot hang the gate —
    // the error is logged and the pixel assertion fails instead).
    property bool gateAwaitingReplay: false
    property int gateReplayWaits: 0
    // A sport switch (or a finished load) changes materials, template
    // visibility and the rebuilt procedural sky texture, and those land over
    // one or two *rendered frames*, not instantly. grabToImage composites
    // the last rendered View3D frame, so grabbing on the next 300 ms tick
    // can still capture the previous state — observed under llvmpipe and
    // under load as a row-palette ski frame and a half-uploaded hull. The
    // walk holds until two frames have rendered since the scene change.
    property int gateRenderedFrames: 0
    onAfterAnimating: if (root.gateMode) root.gateRenderedFrames += 1
    property bool gateAwaitingScene: false
    property int gateSceneFramesTarget: 0
    property int gateSceneWaits: 0
    property int gateSceneTicks: 0
    property string gateSceneGrabName: ""

    // An idle scene renders exactly one frame per change, so a "+N frames"
    // settle target is unreachable and mesh-buffer uploads (which only
    // progress on rendered frames) stall mid-geometry. Driving frames for
    // as long as a replay hold is active advances both.
    FrameAnimation {
        running: root.gateMode
                 && (root.gateAwaitingReplay || root.gateAwaitingScene)
    }

    // The release needs frames AND a minimum wall time: under llvmpipe the
    // big rig pack's vertex buffers upload over several slow frames, and a
    // grab before they land renders zero-filled geometry as a crumpled ball
    // around the origin (observed on the row hull while the athlete, whose
    // buffers had landed, rendered fine).
    readonly property int gateSceneMinTicks: 15
    readonly property int gateSceneMaxTicks: 60

    function grabSettledScene(name) {
        if (Settings.screenshotDir.length === 0) {
            return
        }
        gateAwaitingScene = true
        gateSceneFramesTarget = gateRenderedFrames + 3
        gateSceneWaits = 0
        gateSceneTicks = 0
        gateSceneGrabName = name
        console.log("gate scene: settling", name, "from",
                    gateRenderedFrames, "frames")
    }

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
            // A PPM twin (same callback, same pixels) feeds the gate test's
            // pixel-diversity assertions — uncompressed P6 is trivial to
            // parse without an image crate.
            result.saveToFile(Settings.screenshotDir + "/" + name + ".ppm")
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
                // A replay grab re-renders the 3D scene into the grab layer;
                // on a slow software renderer that pass takes seconds.
                if (root.grabWaits < 40) {
                    return
                }
                console.log("gate screenshot: grab timed out, continuing")
                root.grabPending = false
            }
            if (root.gateAwaitingSync) {
                if (Sync.isRunning) {
                    return
                }
                root.gateAwaitingSync = false
            }
            if (root.gateAwaitingBench) {
                var rs = detailColumn.children[3]
                if (rs && !rs.benchCollecting) {
                    root.gateAwaitingBench = false
                } else {
                    return  // still collecting
                }
            }
            if (root.gateAwaitingReplay) {
                if (Replay.loadState === "ready") {
                    root.gateAwaitingReplay = false
                } else if (Replay.loadState === "error") {
                    console.log("gate replay error:", Replay.errorText)
                    root.gateAwaitingReplay = false
                } else {
                    root.gateReplayWaits += 1
                    if (root.gateReplayWaits < 60) {
                        return
                    }
                    console.log("gate replay: load timed out, continuing")
                    root.gateAwaitingReplay = false
                }
            }
            if (root.gateAwaitingScene) {
                // Release and grab, but do NOT advance the step this tick:
                // grabScreen's callback renders asynchronously, and the next
                // sport switch must not run before it has — otherwise each
                // capture shows the following sport's palette. The
                // grabPending hold above paces the walk from here.
                root.gateSceneTicks += 1
                if (root.gateRenderedFrames >= root.gateSceneFramesTarget
                        && root.gateSceneTicks >= root.gateSceneMinTicks) {
                    root.gateAwaitingScene = false
                    console.log("gate scene: settled", root.gateSceneGrabName,
                                "after", root.gateSceneTicks, "ticks,",
                                root.gateRenderedFrames, "frames")
                    root.grabScreen(root.gateSceneGrabName)
                    return
                }
                root.gateSceneWaits += 1
                if (root.gateSceneWaits < root.gateSceneMaxTicks) {
                    return
                }
                console.log("gate scene: frames never settled, grabbing anyway")
                root.gateAwaitingScene = false
                root.grabScreen(root.gateSceneGrabName)
                return
            }
            root.gateStep += 1
            switch (root.gateStep) {
            case 1:
                root.screenIndex = 0
                console.log("gate i18n en:", Tr.t("nav.dashboard"), "|",
                            Tr.t("workoutList.matching",
                                 { n: Library.filteredCount }),
                            "| uiLanguage:", Qt.uiLanguage)
                root.checkGateMembers()
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
                    root.gateAwaitingSync = true
                    Sync.start()          // incremental
                }
                break
            case 26: case 27: case 28: case 29: case 30:
                break   // unreachable while a sync runs
            case 31:
                if (Settings.syncMockMode) {
                    Library.reload()
                    console.log("gate sync:", Sync.statusId,
                                "added", Sync.statusAdded,
                                "skipped", Sync.statusSkipped,
                                "total", Sync.statusTotal,
                                "library", Library.totalCount,
                                Library.isDemoLibrary ? "demo" : "cache")
                    // Second pass over the now-caught-up library: an
                    // incremental sync must fetch nothing.
                    root.gateAwaitingSync = true
                    Sync.start()
                }
                break
            case 32: case 33: case 34: case 35: case 36: case 37:
                break   // unreachable while a sync runs
            case 38:
                if (Settings.syncMockMode) {
                    Library.reload()
                    console.log("gate resync:", Sync.statusId,
                                "added", Sync.statusAdded,
                                "skipped", Sync.statusSkipped,
                                "library", Library.totalCount)
                    // Full mode re-downloads everything.
                    root.gateAwaitingSync = true
                    Sync.startFull()
                }
                break
            case 39: case 40: case 41: case 42:
                break   // unreachable while a sync runs
            case 43:
                if (Settings.syncMockMode) {
                    Library.reload()
                    console.log("gate full:", Sync.statusId,
                                "added", Sync.statusAdded,
                                "skipped", Sync.statusSkipped,
                                "library", Library.totalCount)
                    Settings.setDemoModeEnabled(true)
                    Settings.clearCachedWorkouts()
                    Library.reload()
                }
                break
            case 44: Library.toggleSort(3); break        // pace ascending
            case 45: Library.setDateRange("2024-01-01", "2024-12-31"); break
            case 46: Library.setDateRange("nope", ""); break   // rejected
            case 47: Library.setDateRange("", ""); break       // cleared
            case 48: Library.toggleSort(0); Library.selectWorkout(9001); break
            case 49: root.grabScreen("detail-nostrokes"); break
            case 50: Library.selectWorkout(1005); break
            case 51: root.grabScreen("detail-full"); break
            case 52:                                 // route push + rower
                Library.requestReplay(false)
                root.gateAwaitingReplay = true
                break
            case 53: Replay.loadWorkout(1001); break     // rower demo workout
            case 54: root.grabSettledScene("replay-row"); break
            case 55: Replay.loadWorkout(1003); break     // skierg demo workout
            case 56: root.grabSettledScene("replay-ski"); break
            case 57: Replay.loadWorkout(1004); break     // bike demo workout
            case 58: root.grabSettledScene("replay-bike"); break
            // Ghost: load workout 1002 as rival alongside the rower (1001).
            case 59: Replay.loadWorkout(1001); Replay.loadGhost(1002); break
            case 60: root.grabSettledScene("replay-ghost"); break
            case 61: Replay.loadGhost(-1); break  // dismiss ghost
            // Tier cycling: exercise all four quality tiers on the rower scene
            // so the gate can assert texture set counts per tier.
            case 62: Replay.setQualityIndex(0); break  // Low
            case 63: Replay.setQualityIndex(2); break  // High
            case 64: Replay.setQualityIndex(3); break  // Ultra
            case 65: Replay.setQualityIndex(1); break  // back to Medium
            // Phase shots (ROWPLAY_PHASE_SHOTS=1): stroke-phase captures
            // per sport with the ghost loaded — the T8 visual baseline.
            // Fractions below are deterministic mid-workout stroke phases
            // computed from the demo timelines (see the phase-scan notes in
            // the Phase 7 tasks); each seek-then-settle lands the same
            // frame on every run.
            case 66:
                if (!Settings.phaseShots) { root.gateStep = 81; break }
                break
            case 67: Replay.loadWorkout(1001); Replay.loadGhost(1002); break
            case 68: Replay.seek(0.49995); root.grabSettledScene("phase-row-catch"); break
            case 69: Replay.seek(0.50091); root.grabSettledScene("phase-row-middrive"); break
            case 70: Replay.seek(0.50187); root.grabSettledScene("phase-row-finish"); break
            case 71: Replay.seek(0.50343); root.grabSettledScene("phase-row-midrecovery"); break
            case 72: Replay.loadWorkout(1003); Replay.loadGhost(1005); break
            case 73: Replay.seek(0.50073); root.grabSettledScene("phase-ski-catch"); break
            case 74: Replay.seek(0.50201); root.grabSettledScene("phase-ski-middrive"); break
            case 75: Replay.seek(0.49710); root.grabSettledScene("phase-ski-finish"); break
            case 76: Replay.seek(0.50500); root.grabSettledScene("phase-ski-midrecovery"); break
            case 77: Replay.loadWorkout(1004); Replay.loadGhost(1006); break
            case 78: Replay.seek(0.49937); root.grabSettledScene("phase-bike-catch"); break
            case 79: Replay.seek(0.50001); root.grabSettledScene("phase-bike-middrive"); break
            case 80: Replay.seek(0.49960); root.grabSettledScene("phase-bike-finish"); break
            case 81: Replay.seek(0.50025); root.grabSettledScene("phase-bike-midrecovery"); break
            case 82: Replay.setQualityIndex(1); Replay.loadGhost(-1); Library.closeReplay(); Library.clearSelection(); break
            // Bench mode (ROWPLAY_REPLAY_BENCH=1): measure 600 frames per
            // sport × tier on hardware GL. Runs after the normal gate.
            case 83:
                if (!Settings.benchMode) { root.gateStep = 999; break }
                Library.selectWorkout(1001)
                Library.requestReplay(false)
                root.gateAwaitingReplay = true
                break
            // Bench with ghost: load the ghost one step before each sport's
            // measurement so the doubled geometry is settled by the time the
            // 60-frame warmup starts.
            // RowErg × 4 tiers
            case 84: Replay.loadWorkout(1001); Replay.loadGhost(1002); break
            case 85: Replay.setQualityIndex(0); Replay.play(); root.benchRun("row-low"); break
            case 86: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("row-medium"); break
            case 87: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("row-high"); break
            case 88: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("row-ultra"); break
            // SkiErg × 4 tiers
            case 89: Replay.loadWorkout(1003); Replay.loadGhost(1005); break
            case 90: Replay.setQualityIndex(0); Replay.play(); root.benchRun("ski-low"); break
            case 91: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("ski-medium"); break
            case 92: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("ski-high"); break
            case 93: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("ski-ultra"); break
            // BikeErg × 4 tiers
            case 94: Replay.loadWorkout(1004); Replay.loadGhost(1006); break
            case 95: Replay.setQualityIndex(0); Replay.play(); root.benchRun("bike-low"); break
            case 96: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("bike-medium"); break
            case 97: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("bike-high"); break
            case 98: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("bike-ultra"); break
            case 99: Replay.setQualityIndex(1); Library.closeReplay(); Library.clearSelection(); break
            default:
                gateTimer.running = false
                Qt.exit(0)
            }
        }
    }
}
