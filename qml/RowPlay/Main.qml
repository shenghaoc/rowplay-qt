// SPDX-License-Identifier: GPL-3.0-or-later
// The application shell — a port of rowplay-studio's ContentView:
// a split view (sidebar column min 260 / ideal 320, detail area), the sport
// filter and reload toolbar, the dashboard/detail navigation state and the
// "Ready When You Are" empty state. The window title and 1000x680 minimum
// come from Studio's app scene.
//
// Design system (ADR 0013): the sidebar runs the full window height; the
// toolbar sits over the content column with icon-only buttons and the sport
// filter as a segmented control; the empty state replaces only the content
// area.
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

    // Detail column routing: 0 = dashboard, 1 = workout detail, 2 = settings,
    // 3 = the replay route.
    property int screenIndex: 0
    // The runtime-error gate walks every screen and exercises the language
    // switch (driven from Rust by ROWPLAY_SMOKE_GATE=1).
    readonly property bool gateMode: Settings.gateMode
    property int gateStep: 0
    // Temporary step-529 strip (demo 1003, close-up camera, steps 520–540).
    property int step529Cursor: 520

    // Live retranslation: QQmlApplicationEngine reloads
    // :/qt/qml/RowPlay/i18n/qml_<lang>.qm whenever Qt.uiLanguage changes.
    // `Qt` is a JS global, not a QObject, so a Binding element cannot target
    // it — assign imperatively at startup and on every settings change.
    Component.onCompleted: {
        Qt.uiLanguage = Settings.languageCode
        // The demo library starts with its default workout selected (Studio's
        // SceneStorage initial value); route to the detail screen if so. The
        // Library made that selection while it was constructed, before the
        // onSelectionChanged wiring below existed, so hand it to Detail here —
        // otherwise the first launch showed an empty detail pane.
        Detail.selectWorkout(Library.selectedWorkoutId)
        screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
    }

    // The Basic style (qtquickcontrols2.conf), with every palette role set
    // from Theme.qml (ADR 0013). The app's own controls draw themselves from
    // the tokens; the palette is the safety net for any stock Basic control,
    // mapped by how Basic uses each role: `button` fills buttons and combo
    // boxes, `mid` outlines fields and draws scroll bars, `midlight` is the
    // off track of switches and progress bars, and `dark` is Basic's "on"
    // colour (checked buttons, switch-on, progress, busy) with `brightText`
    // on it. The text size is the system font's (Qt.application.font; Theme
    // derives its whole scale from it), so no font is set here.
    palette.window: Theme.windowBackground
    palette.windowText: Theme.textPrimary
    palette.base: Theme.controlBackground
    palette.alternateBase: Theme.groupBackground
    palette.text: Theme.textPrimary
    palette.button: Theme.segmentTrack
    palette.buttonText: Theme.textPrimary
    palette.brightText: Theme.windowBackground
    palette.highlight: Theme.accentColor
    palette.highlightedText: Theme.onAccent
    palette.accent: Theme.accentColor
    palette.link: Theme.accentColor
    palette.placeholderText: Theme.textTertiary
    palette.toolTipBase: Theme.popupBackground
    palette.toolTipText: Theme.textPrimary
    palette.light: Theme.controlBackground
    palette.midlight: Theme.switchTrackOff
    palette.mid: Theme.controlBorder
    palette.dark: Theme.textSecondary
    palette.shadow: Theme.dark ? "#000000" : "#15181d"

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
    // of the content, not the window `header`. It is painted in the window
    // colour because an item grab has no window background under it: on an
    // unpainted root, translucent pixels were stored translucent — alpha in
    // the PNG, the bare colour in the PPM the tests read — instead of what
    // the screen shows.
    Rectangle {
        id: shellRoot
        anchors.fill: parent
        color: Theme.windowBackground

        SplitView {
            id: splitView
            anchors.fill: parent

            // A hairline divider with a wider invisible drag area: the
            // containment mask enlarges a handle's hit region without
            // changing its look.
            handle: Rectangle {
                id: splitHandle
                implicitWidth: Theme.hairline
                implicitHeight: Theme.hairline
                color: Theme.separator
                containmentMask: Item {
                    x: (splitHandle.width - width) / 2
                    width: Theme.px(9)
                    height: splitHandle.height
                }
            }

            // Sidebar column (Studio: min 260, ideal 320), full height.
            SidebarPanel {
                id: sidebarColumn
                SplitView.preferredWidth: Theme.px(320)
                SplitView.minimumWidth: Theme.px(260)
                SplitView.maximumWidth: Theme.px(480)
            }

            // Content column: the toolbar over the routed screens.
            ColumnLayout {
                SplitView.fillWidth: true
                spacing: 0

                Rectangle {
                    id: toolbar
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.toolbarHeight
                    color: Theme.toolbarBackground

                    // Principal: the sport filter (Studio's segmented picker),
                    // bound to the Library so a filter set from anywhere —
                    // including the gate walk — shows here.
                    SegmentedControl {
                        id: sportFilter
                        anchors.centerIn: parent
                        model: [Tr.t("dashboard.all")].concat(Library.sportNames)
                        currentIndex: Library.sportFilterIndex
                        label: Tr.t("workoutList.filtersTitle")
                        onActivated: function(index) {
                            Library.setSportFilter(index)
                        }
                    }

                    // Trailing: reload and the settings toggle.
                    RowLayout {
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.spacingLarge
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.spacingXSmall

                        ToolbarButton {
                            iconName: "arrow.clockwise"
                            label: Tr.t("pwa.reload")
                            enabled: !Sync.isRunning
                            onClicked: Library.reload()

                            Shortcut {
                                sequences: ["Ctrl+R", "Meta+R"]
                                onActivated: Library.reload()
                            }
                        }

                        ToolbarButton {
                            iconName: "sliders"
                            label: Tr.t("settings.title")
                            checkable: true
                            checked: root.screenIndex === 2
                            onClicked: root.toggleSettings()
                        }
                    }
                }

                // The toolbar's bottom hairline.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.hairline
                    color: Theme.separator
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    // Detail column: dashboard | workout detail | settings |
                    // replay route (Phase 5 renders the route itself).
                    StackLayout {
                        id: detailColumn
                        anchors.fill: parent
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

                    // Studio's empty state (library empty and demo mode off)
                    // replaces the content area, like Studio's detail column.
                    Rectangle {
                        anchors.fill: parent
                        visible: Library.isEmpty && !Settings.demoModeEnabled
                                 && root.screenIndex !== 2
                        color: Theme.windowBackground

                        ColumnLayout {
                            anchors.centerIn: parent
                            width: Math.min(parent.width - 2 * Theme.spacingXxxLarge,
                                            Theme.px(440))
                            spacing: Theme.spacingXxxLarge

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: Theme.spacingLarge

                                Icon {
                                    Layout.alignment: Qt.AlignHCenter
                                    name: "sport.rower"
                                    size: Theme.px(48)
                                    color: Theme.metricDuration
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: Tr.t("landing.title1")
                                    font: Theme.pageTitle
                                    color: Theme.textPrimary
                                    wrapMode: Text.WordWrap
                                    horizontalAlignment: Text.AlignHCenter
                                    Accessible.name: text
                                }
                                Label {
                                    Layout.fillWidth: true
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

                                // The view's one prominent action.
                                PushButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: Tr.t("landing.exploreDemo")
                                    prominent: true
                                    onClicked: Settings.setDemoModeEnabled(true)
                                }
                                PushButton {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: Tr.t("landing.connect")
                                    onClicked: root.showSettings()
                                }
                            }
                        }
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
    Connections {
        target: Live
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
    // Same pattern for live-mode polls.
    Timer {
        interval: 50
        repeat: true
        running: Live.polling
        onTriggered: Live.pumpEvents()
    }
    // Live cadence: the view-model decides when a poll is due; this timer
    // only asks. Runs while enabled so polls continue with Settings closed.
    Timer {
        interval: 1000
        repeat: true
        running: Live.enabled
        triggeredOnStart: true
        onTriggered: Live.tick()
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
            "Settings": Settings, "Sync": Sync, "Live": Live,
            "Replay": Replay
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
    // ROWPLAY_PHASE_CLOSEUPS: after a phase grab settles and saves, take a
    // second grab of the same seek through the close-up camera (the
    // torso-and-hands framing the wrist/posture judgement needs).
    property bool gateSceneCloseup: false

    // An idle scene renders exactly one frame per change, so a "+N frames"
    // settle target is unreachable and mesh-buffer uploads (which only
    // progress on rendered frames) stall mid-geometry. Driving frames for
    // as long as a replay hold is active advances both.
    FrameAnimation {
        running: root.gateMode
                 && (root.gateAwaitingReplay || root.gateAwaitingScene)
    }

    // Packaged-launch probe (ROWPLAY_EXIT_AFTER_FRAMES=N, Phase 9): the
    // release bundle has no gate hooks (they are compiled out), so the
    // package scripts start the deployed binary, let the shell render N
    // frames — every Qt framework, QML module and platform plugin the
    // bundle must carry is loaded by then — and require a clean exit. An
    // idle shell renders no frames at all, hence the driver.
    FrameAnimation {
        running: Settings.exitAfterFrames > 0
        onTriggered: if (currentFrame >= Settings.exitAfterFrames) Qt.quit()
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
        gateSceneCloseup = Settings.phaseCloseups && name.indexOf("phase-") === 0
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
            if (root.gateSceneCloseup && name.indexOf("-closeup") < 0) {
                // Same seek, close-up camera: settle again so the swapped
                // lens has rendered, then re-enter grabScreen for the twin.
                // gateAwaitingScene holds the walk; grabPending releases so
                // the settle branch is reachable on the next ticks.
                root.gateSceneCloseup = false
                Replay.setCloseupCamera(true)
                root.grabPending = false
                root.gateAwaitingScene = true
                root.gateSceneFramesTarget = root.gateRenderedFrames + 3
                root.gateSceneWaits = 0
                root.gateSceneTicks = 0
                root.gateSceneGrabName = name + "-closeup"
                console.log("gate scene: settling", root.gateSceneGrabName,
                            "from", root.gateRenderedFrames, "frames")
                return
            }
            if (name.indexOf("-closeup") >= 0) {
                Replay.setCloseupCamera(false)
            }
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
                // different workout so the selection actually changes. The
                // walk is still on the settings screen here, so route to the
                // detail screen too — otherwise the "detail" capture at step
                // 13 shows settings.
                Library.selectWorkout(1003)
                root.screenIndex = 1
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
                if (!Settings.phaseShots) { root.gateStep = 83; break }
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
            // Temporary step-529 strip: Replay.loadWorkout(1003)
            // "1000m SkiErg" 2026-05-21. Do not Library.selectWorkout —
            // that leaves the 3D replay for the detail-chart screen.
            // Dismiss the rower ghost from case 72. Close-up camera
            // (SkiErg: surged chest, stand-off from the aim, CLOSEUP_DISTANCE
            // 2.4 m — the tightest framing the app has). Pose via
            // Replay.setGuardCycleStep so frame `stepN` is guard step N
            // (fallback 30 spm, metres = N·3) — the strip must show the
            // guard's own sample, and live 1003 (41 spm / drive_frac 0.46)
            // puts the same cycle_frac's discontinuity at step 522.
            case 77:
                Replay.loadGhost(-1)
                Replay.loadWorkout(1003)
                Replay.setCloseupCamera(true)
                root.step529Cursor = 520
                break
            case 78:
                if (root.step529Cursor > 540) {
                    Replay.setGuardCycleStep(-1)
                    Replay.setCloseupCamera(false)
                    break
                }
                Replay.setGuardCycleStep(root.step529Cursor)
                root.grabSettledScene("step" + root.step529Cursor)
                root.step529Cursor += 1
                root.gateStep = 77
                break
            case 79: Replay.loadWorkout(1004); Replay.loadGhost(1006); break
            case 80: Replay.seek(0.49937); root.grabSettledScene("phase-bike-catch"); break
            case 81: Replay.seek(0.50001); root.grabSettledScene("phase-bike-middrive"); break
            case 82: Replay.seek(0.49960); root.grabSettledScene("phase-bike-finish"); break
            case 83: Replay.seek(0.50025); root.grabSettledScene("phase-bike-midrecovery"); break
            case 84: Replay.setQualityIndex(1); Replay.loadGhost(-1); Library.closeReplay(); Library.clearSelection(); break
            // Bench mode (ROWPLAY_REPLAY_BENCH=1): measure 600 frames per
            // sport × tier on hardware GL. Runs after the normal gate.
            case 85:
                if (!Settings.benchMode) { root.gateStep = 999; break }
                Library.selectWorkout(1001)
                Library.requestReplay(false)
                root.gateAwaitingReplay = true
                break
            // Bench with ghost: load the ghost one step before each sport's
            // measurement so the doubled geometry is settled by the time the
            // 60-frame warmup starts.
            // RowErg × 4 tiers
            case 86: Replay.loadWorkout(1001); Replay.loadGhost(1002); break
            case 87: Replay.setQualityIndex(0); Replay.play(); root.benchRun("row-low"); break
            case 88: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("row-medium"); break
            case 89: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("row-high"); break
            case 90: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("row-ultra"); break
            // SkiErg × 4 tiers
            case 91: Replay.loadWorkout(1003); Replay.loadGhost(1005); break
            case 92: Replay.setQualityIndex(0); Replay.play(); root.benchRun("ski-low"); break
            case 93: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("ski-medium"); break
            case 94: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("ski-high"); break
            case 95: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("ski-ultra"); break
            // BikeErg × 4 tiers
            case 96: Replay.loadWorkout(1004); Replay.loadGhost(1006); break
            case 97: Replay.setQualityIndex(0); Replay.play(); root.benchRun("bike-low"); break
            case 98: Replay.seek(0); Replay.setQualityIndex(1); Replay.play(); root.benchRun("bike-medium"); break
            case 99: Replay.seek(0); Replay.setQualityIndex(2); Replay.play(); root.benchRun("bike-high"); break
            case 100: Replay.seek(0); Replay.setQualityIndex(3); Replay.play(); root.benchRun("bike-ultra"); break
            case 101: Replay.setQualityIndex(1); Library.closeReplay(); Library.clearSelection(); break
            default:
                gateTimer.running = false
                Qt.exit(0)
            }
        }
    }
}
