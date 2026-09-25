// SPDX-License-Identifier: GPL-3.0-or-later
// The application shell — a port of rowplay-studio's ContentView:
// a split view (sidebar column min 260 / ideal 320, detail area), the sport
// filter and reload toolbar, the dashboard/detail navigation state and the
// "Ready When You Are" empty state. The window title comes from Studio's app
// scene; its 1000x680 minimum gave way to the width classes below.
//
// Design system (ADR 0013): the sidebar runs the full window height and can
// be hidden (it always is during the immersive replay), and below the large
// width class it becomes a drawer; the toolbar sits over the content column
// with icon-only buttons and the sport filter as a segmented control, and
// shows only the way back and the workout's title during the replay; the
// empty state replaces only the content area. The platform layer lives here too:
// StandardKey shortcuts, the native macOS menu bar (menu-item roles, so Qt
// writes the titles) or the toolbar's menu button on Windows and Linux,
// and the sidebar toggle.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Qt.labs.platform as Platform
import RowPlay
import RowPlay.Replay

ApplicationWindow {
    id: root

    visible: true
    width: 1200
    height: 800
    // The compact layout, checked down to 480 px (scaled with the text,
    // like the breakpoints), or more where the toolbar needs it (very large
    // text). The height leaves the replay scene room above its HUD.
    minimumWidth: Math.max(Theme.px(480), toolbarMinimumWidth)
    minimumHeight: 480
    title: "rowplay"

    // The width class (Theme.widthClass). Large keeps the sidebar beside
    // the content. Below it the sidebar moves into a drawer: at medium a
    // modal one over the content, at compact the list's own page under the
    // toolbar, so the window shows one column at a time.
    readonly property int widthClass: Theme.widthClass(width)
    readonly property bool sidebarInDrawer: widthClass !== Theme.widthLarge
    readonly property bool compactLayout: widthClass === Theme.widthCompact

    // The narrowest toolbar that keeps everything reachable: its four gaps,
    // the drawer button, the sport filter in its compact form and the
    // trailing buttons. The content column never gets less, so a dragged
    // sidebar stops there instead of squeezing the filter to nothing.
    readonly property real toolbarMinimumWidth: 4 * Theme.spacingLarge
                                                + drawerButton.implicitWidth
                                                + sportFilter.implicitWidth
                                                + trailingButtons.implicitWidth

    // Detail column routing: 0 = dashboard, 1 = workout detail, 2 = settings,
    // 3 = the replay route.
    property int screenIndex: 0
    // The platform layer's only branch: macOS has the native menu bar and
    // its own sidebar-toggle chord.
    readonly property bool isMac: Qt.platform.os === "osx" || Qt.platform.os === "macos"
    // The sidebar toggle (F9 / Ctrl+Cmd+S) at the large width class;
    // SplitView keeps the column's width while it is hidden. Below it the
    // toggle opens and closes the drawer instead.
    property bool sidebarShown: true
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
        // The scheme pin (ROWPLAY_FORCE_COLOR_SCHEME) also asks Qt for that
        // scheme, so the system palette, which high contrast draws in,
        // follows it where the platform honours the request (macOS,
        // Windows).
        if (Settings.colorSchemeOverride === "dark") {
            Qt.styleHints.colorScheme = Qt.Dark
        } else if (Settings.colorSchemeOverride === "light") {
            Qt.styleHints.colorScheme = Qt.Light
        }
        // The demo library starts with its default workout selected (Studio's
        // SceneStorage initial value); route to the detail screen if so. The
        // Library made that selection while it was constructed, before the
        // onSelectionChanged wiring below existed, so hand it to Detail here —
        // otherwise the first launch showed an empty detail pane.
        Detail.selectWorkout(Library.selectedWorkoutId)
        screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
        placeSidebar()
    }

    // Each platform's own Qt Quick Controls style and system palette (ADR
    // 0015): no style is forced and no palette role is set, so stock
    // controls draw as the platform draws them. The text size is the
    // system font's (Qt.application.font; Theme derives its scale from it).

    // Leaving the replay route by another path closes the replay first, so
    // the Library's presentation flag never outlives the route: settings
    // opened over a replay would otherwise return to the workout with the
    // replay still presented, and Replay would refuse to open again.
    function showDashboard() {
        if (Library.isReplayPresented) {
            Library.closeReplay()
        }
        Library.clearSelection()
        screenIndex = 0
        sidebarDrawer.close()
    }

    function showSettings() {
        if (Library.isReplayPresented) {
            Library.closeReplay()
        }
        screenIndex = 2
        sidebarDrawer.close()
    }

    function toggleSettings() {
        if (screenIndex === 2) {
            screenIndex = Library.selectedWorkoutId === -1 ? 0 : 1
            sidebarDrawer.close()
        } else {
            showSettings()
        }
    }

    // StandardKey.Back: one level up — the replay to its workout, settings
    // to the workout or the dashboard (a replay was closed on the way in),
    // a workout to the dashboard.
    function goBack() {
        if (screenIndex === 3) {
            Library.closeReplay()
        } else if (screenIndex === 2) {
            toggleSettings()
        } else if (screenIndex === 1) {
            showDashboard()
        }
    }

    // `fromKeyboard`: the list takes the keyboard as the drawer opens (the
    // sidebar toggle, or the drawer button pressed from the keyboard). A
    // click leaves the focus with the drawer itself, so it paints no focus
    // ring on the list.
    function toggleSidebar(fromKeyboard) {
        if (screenIndex === 3) {
            return
        }
        if (!sidebarInDrawer) {
            sidebarShown = !sidebarShown
        } else if (sidebarDrawer.shown) {
            sidebarDrawer.close()
        } else {
            sidebarDrawer.open()
            if (fromKeyboard) {
                sidebarColumn.focusList()
            }
        }
    }

    // StandardKey.Find: bring the sidebar back if it was hidden (or open
    // its drawer) and focus its search field.
    function focusSearch() {
        if (sidebarInDrawer) {
            sidebarDrawer.open()
        } else {
            sidebarShown = true
        }
        sidebarColumn.focusSearch()
    }

    // The one sidebar moves between the split view (large) and the drawer,
    // so its search text, date range and scroll position go with it. The
    // drawer closes first when the window widens past the breakpoint.
    function placeSidebar() {
        if (sidebarInDrawer) {
            if (sidebarColumn.parent !== drawerHost) {
                splitView.takeItem(0)
                sidebarColumn.parent = drawerHost
            }
        } else if (sidebarColumn.parent === drawerHost) {
            sidebarDrawer.close()
            splitView.insertItem(0, sidebarColumn)
        }
    }
    onSidebarInDrawerChanged: placeSidebar()
    // Between medium and compact the drawer changes its form (a modal
    // overlay or the list's page); an open one closes rather than morph.
    onCompactLayoutChanged: sidebarDrawer.close()

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
                implicitWidth: Theme.ruleWidth
                implicitHeight: Theme.ruleWidth
                color: Theme.separator
                containmentMask: Item {
                    x: (splitHandle.width - width) / 2
                    width: Theme.px(9)
                    height: splitHandle.height
                }
            }

            // Sidebar column (Studio: min 260, ideal 320), full height;
            // hidden during the replay, and SplitView restores its width.
            // Below the large width class it lives in the drawer
            // (placeSidebar) and shows whenever the drawer does.
            SidebarPanel {
                id: sidebarColumn
                visible: root.sidebarInDrawer
                         || (root.sidebarShown && root.screenIndex !== 3)
                inDrawer: root.sidebarInDrawer
                SplitView.preferredWidth: Theme.px(320)
                SplitView.minimumWidth: Theme.px(260)
                SplitView.maximumWidth: Theme.px(480)
                // A workout chosen in the drawer is shown: the drawer
                // closes, and the workout replaces settings too, which the
                // drawer covered (beside the sidebar, settings stays).
                onWorkoutChosen: {
                    if (!root.sidebarInDrawer) {
                        return
                    }
                    sidebarDrawer.close()
                    if (root.screenIndex === 2 && Library.selectedWorkoutId !== -1) {
                        root.screenIndex = 1
                    }
                }
            }

            // Content column: the toolbar over the routed screens.
            ColumnLayout {
                SplitView.fillWidth: true
                SplitView.minimumWidth: root.toolbarMinimumWidth
                spacing: 0

                ToolBar {
                    id: toolbar
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.toolbarHeight

                    readonly property bool replayShown: root.screenIndex === 3

                    // Leading, below the large width class: the drawer with
                    // the workout list (named like the web's workouts
                    // section). Checked while the list shows.
                    CommandButton {
                        id: drawerButton
                        visible: root.sidebarInDrawer && !toolbar.replayShown
                        anchors.left: parent.left
                        anchors.leftMargin: Theme.spacingMedium
                        anchors.verticalCenter: parent.verticalCenter
                        glyph: "sidebar.left"
                        text: Tr.t("dashboard.sectionWorkoutsEyebrow")
                        shortcutText: sidebarShortcut.nativeText
                        checkable: true
                        checked: sidebarDrawer.shown
                        onClicked: root.toggleSidebar(visualFocus)
                    }

                    // Leading, during the replay only: the way back and the
                    // workout's title.
                    RowLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.leftMargin: Theme.spacingMedium
                        anchors.rightMargin: Theme.spacingMedium
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.spacingSmall
                        visible: toolbar.replayShown

                        // Named "Close" (the web's `replay.closePanel`): it
                        // returns to the workout, where the web's
                        // "Back to dashboard" would name the wrong place.
                        CommandButton {
                            glyph: "chevron.left"
                            text: Tr.t("replay.closePanel")
                            shortcutText: escapeShortcut.nativeText
                            onClicked: Library.closeReplay()
                        }
                        Label {
                            Layout.fillWidth: true
                            // The route always opens on the selected workout;
                            // the guard keeps a replay loaded any other way
                            // (the gate walk loads demo workouts straight into
                            // Replay) from showing another workout's title.
                            text: Replay.workoutId === Library.selectedWorkoutId
                                  ? Detail.workoutType : ""
                            font: Theme.bodyEmphasized
                            color: Theme.textPrimary
                            elide: Text.ElideRight
                            Accessible.name: text
                        }
                    }

                    // Principal: the sport filter, a pop-up button (ADR 0015:
                    // a filter changed now and then, one tab stop, the
                    // platform's own pop-up), bound to the Library so a
                    // filter set from anywhere, the gate walk included, shows
                    // here. Centred, but never under the leading or trailing
                    // buttons; as wide as its widest sport, so choosing one
                    // does not move it.
                    ComboBox {
                        id: sportFilter
                        readonly property real leadingEdge: drawerButton.visible
                                                            ? drawerButton.x + drawerButton.width
                                                              + Theme.spacingLarge
                                                            : Theme.spacingLarge
                        visible: !toolbar.replayShown
                        anchors.verticalCenter: parent.verticalCenter
                        implicitContentWidthPolicy: ComboBox.WidestText
                        x: Math.max(leadingEdge,
                                    Math.min(Math.round((parent.width - width) / 2),
                                             trailingButtons.x - Theme.spacingLarge - width))
                        model: [Tr.t("dashboard.all")].concat(Library.sportNames)
                        currentIndex: Library.sportFilterIndex
                        onActivated: function(index) {
                            Library.setSportFilter(index)
                        }
                        Accessible.name: Tr.t("workoutList.filtersTitle")
                    }

                    // Trailing: reload, the settings toggle and, on Windows
                    // and Linux, the application menu (not during the replay).
                    RowLayout {
                        id: trailingButtons
                        visible: !toolbar.replayShown
                        anchors.right: parent.right
                        anchors.rightMargin: Theme.spacingMedium
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: Theme.spacingXSmall

                        CommandButton {
                            glyph: "arrow.clockwise"
                            text: Tr.t("pwa.reload")
                            shortcutText: refreshShortcut.nativeText
                            enabled: !Sync.isRunning
                            onClicked: Library.reload()
                        }

                        CommandButton {
                            glyph: "sliders"
                            text: Tr.t("settings.title")
                            shortcutText: preferencesShortcut.nativeText.length > 0
                                          ? preferencesShortcut.nativeText
                                          : preferencesFallback.nativeText
                            checkable: true
                            checked: root.screenIndex === 2
                            onClicked: root.toggleSettings()
                        }

                        CommandButton {
                            id: menuButton
                            visible: !root.isMac
                            glyph: "line.3.horizontal"
                            text: Tr.t("nav.menuOpen")
                            onClicked: appMenu.open()

                            // The Windows / Linux application menu: the
                            // shell's commands (macOS gets the native menu
                            // bar instead). The style's menu items show no
                            // shortcut; the toolbar's tooltips name them.
                            Menu {
                                id: appMenu
                                x: menuButton.width - width
                                y: menuButton.height + Theme.spacingXSmall

                                MenuItem {
                                    text: Tr.t("nav.dashboard")
                                    Accessible.name: text
                                    onTriggered: root.showDashboard()
                                }
                                MenuItem {
                                    text: Tr.t("workoutList.search")
                                    Accessible.name: text
                                    enabled: findShortcut.enabled
                                    onTriggered: root.focusSearch()
                                }
                                MenuItem {
                                    text: Tr.t("pwa.reload")
                                    Accessible.name: text
                                    enabled: !Sync.isRunning
                                    onTriggered: Library.reload()
                                }
                                MenuSeparator {}
                                MenuItem {
                                    text: Tr.t("settings.title")
                                    Accessible.name: text
                                    onTriggered: root.showSettings()
                                }
                            }
                        }
                    }
                }

                // The rule between the toolbar and the content (2 px under
                // high contrast): layout, drawn beside the style's toolbar.
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.ruleWidth
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

                        // No dismiss button (ADR 0013): back navigation,
                        // Escape and the toolbar's settings toggle close it.
                        SettingsScreen {}

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
                                Button {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: Tr.t("landing.exploreDemo")
                                    highlighted: true
                                    Accessible.name: text
                                    onClicked: Settings.setDemoModeEnabled(true)
                                }
                                Button {
                                    Layout.alignment: Qt.AlignHCenter
                                    text: Tr.t("landing.connect")
                                    Accessible.name: text
                                    onClicked: root.showSettings()
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // The sidebar's drawer below the large width class. At medium it is a
    // modal overlay beside a strip of the dimmed content: a click on the
    // strip, Escape or the sidebar toggle closes it. At compact it is the
    // list's page: under the toolbar, the window's full width, and neither
    // modal nor closed by Escape itself, because Qt blocks every window
    // shortcut outside a popup that is either (QQuickShortcutContext): the
    // toolbar and the shell's shortcuts stay live, the drawer button and
    // the sidebar toggle close the page, and so do Escape (the shortcut
    // below), settings and the menu's commands on their way.
    Drawer {
        id: sidebarDrawer
        /// Where the drawer is going: set as it starts to open, cleared as
        /// it starts to close. `visible` stays true through the closing
        /// slide, so a toggle button bound to it showed a closing drawer as
        /// open.
        property bool shown: false
        onAboutToShow: shown = true
        onAboutToHide: shown = false

        edge: Qt.LeftEdge
        y: root.compactLayout ? Theme.toolbarHeight + Theme.ruleWidth : 0
        width: root.compactLayout ? root.width
                                  : Math.min(sidebarColumn.SplitView.preferredWidth,
                                             root.width - Theme.px(56))
        height: root.height - y
        // Never opened by a drag from the window's edge: on a desktop a drag
        // there is aimed at the window frame. The drawer stays interactive
        // (a swipe can close it), because Qt 6.11 closes a non-interactive
        // popup neither on Escape nor on a click outside
        // (docs/qt-bridges-notes.md).
        dragMargin: 0
        // It slides in, and appears at once under the reduce-motion
        // preference.
        enter: Transition {
            NumberAnimation { duration: Theme.reduceMotion ? 0 : 200; easing.type: Easing.OutCubic }
        }
        exit: Transition {
            NumberAnimation { duration: Theme.reduceMotion ? 0 : 200; easing.type: Easing.OutCubic }
        }
        modal: !root.compactLayout
        focus: true
        closePolicy: root.compactLayout ? Popup.NoAutoClose
                                        : Popup.CloseOnEscape | Popup.CloseOnPressOutside

        contentItem: Item {
            id: drawerHost

            // The modal drawer blocks the shell's shortcuts, so the two that
            // concern the sidebar work inside it too: the toggle closes it
            // and Find focuses its search field.
            Shortcut {
                sequence: root.isMac ? "Meta+Ctrl+S" : "F9"
                enabled: sidebarDrawer.modal && sidebarDrawer.shown
                onActivated: sidebarDrawer.close()
            }
            Shortcut {
                sequences: [StandardKey.Find]
                enabled: sidebarDrawer.modal && sidebarDrawer.shown
                onActivated: sidebarColumn.focusSearch()
            }
        }
    }
    // The sidebar fills the drawer while it is there; back in the split
    // view, SplitView sizes it again.
    Binding {
        target: sidebarColumn
        property: "width"
        value: drawerHost.width
        when: sidebarColumn.parent === drawerHost
        restoreMode: Binding.RestoreNone
    }
    Binding {
        target: sidebarColumn
        property: "height"
        value: drawerHost.height
        when: sidebarColumn.parent === drawerHost
        restoreMode: Binding.RestoreNone
    }

    // macOS About (the native application menu's About item): the product
    // name, tagline, version and the not-affiliated note, all existing
    // strings. Windows and Linux show the version on the settings page.
    Dialog {
        id: aboutDialog
        anchors.centerIn: parent
        width: Math.min(root.width - 2 * Theme.spacingXxxLarge, Theme.px(400))
        title: root.title
        modal: true
        // The platform's button, named with the web's word ("Dismiss").
        standardButtons: Dialog.Ok
        onAboutToShow: standardButton(Dialog.Ok).text = Tr.t("common.dismiss")

        ColumnLayout {
            width: parent.width
            spacing: Theme.spacingSmall

            Label {
                Layout.fillWidth: true
                text: Tr.t("common.tagline")
                font: Theme.body
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
            Label {
                Layout.fillWidth: true
                text: Tr.t("settings.appVersion", { version: Settings.appVersion })
                font: Theme.subheadline
                color: Theme.textSecondary
                Accessible.name: text
            }
            Label {
                Layout.fillWidth: true
                text: Tr.t("common.notAffiliated")
                font: Theme.subheadline
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
        }
    }

    // The native macOS menu bar. Its About, Preferences and Quit items carry
    // menu-item roles, so Qt places them in the application menu under its
    // own titles ("About rowplay", "Preferences…", "Quit rowplay"; English
    // in every language, #80) and key equivalents (⌘, and ⌘Q); a menu whose
    // items all move there is hidden. Created only on macOS: elsewhere
    // Qt.labs.platform has no native menu bar and reports an error.
    Instantiator {
        active: root.isMac
        delegate: Platform.MenuBar {
            window: root

            Platform.Menu {
                title: root.title

                Platform.MenuItem {
                    role: Platform.MenuItem.AboutRole
                    text: root.title
                    onTriggered: aboutDialog.open()
                }
                Platform.MenuItem {
                    role: Platform.MenuItem.PreferencesRole
                    text: Tr.t("settings.title")
                    onTriggered: root.showSettings()
                }
                Platform.MenuItem {
                    role: Platform.MenuItem.QuitRole
                    text: root.title
                    onTriggered: Qt.quit()
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
                sidebarDrawer.close()
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

    // Keyboard — the platform layer (ADR 0013): QKeySequence.StandardKey
    // wherever Qt defines one, so each OS gets its own chord (Qt's table:
    // docs/qt-bridges-notes.md); a custom chord only where none exists, and
    // none over the platform's Quit or Close. The replay route keeps its
    // own keys (ReplayScene.qml).

    // Studio: Cmd+1 shows the dashboard.
    Shortcut {
        id: dashboardShortcut
        sequences: ["Ctrl+1", "Meta+1"]
        onActivated: root.showDashboard()
    }
    Shortcut {
        id: preferencesShortcut
        sequences: [StandardKey.Preferences]
        // On macOS the application menu's Preferences… item owns ⌘,.
        enabled: !root.isMac
        onActivated: root.showSettings()
    }
    // Qt maps Preferences on macOS and KDE only; Windows and the other Linux
    // desktops get the common Ctrl+, where the platform has no chord.
    Shortcut {
        id: preferencesFallback
        sequence: "Ctrl+,"
        enabled: !root.isMac && preferencesShortcut.nativeText.length === 0
        onActivated: root.showSettings()
    }
    Shortcut {
        // Linux: Ctrl+Q. macOS: the application menu's Quit item owns ⌘Q.
        // Windows has no Quit chord (the window's Alt+F4 closes the app).
        sequences: [StandardKey.Quit]
        enabled: !root.isMac
        context: Qt.ApplicationShortcut
        onActivated: Qt.quit()
    }
    Shortcut {
        sequences: [StandardKey.Close]
        onActivated: root.close()
    }
    Shortcut {
        id: findShortcut
        sequences: [StandardKey.Find]
        enabled: root.screenIndex !== 3
        onActivated: root.focusSearch()
    }
    Shortcut {
        id: refreshShortcut
        sequences: [StandardKey.Refresh]
        enabled: !Sync.isRunning
        onActivated: Library.reload()
    }
    Shortcut {
        // `sequence`, not `sequences`: only the platform's primary chord
        // (Alt+Left; ⌘[ on macOS). Windows also lists Backspace, which
        // belongs to the text fields.
        sequence: StandardKey.Back
        onActivated: root.goBack()
    }
    Shortcut {
        id: sidebarShortcut
        // The sidebar toggle: F9 on Windows and Linux, Ctrl+Cmd+S on macOS
        // (Qt's "Ctrl" is Command there and "Meta" is Control). Off during
        // the replay.
        sequence: root.isMac ? "Meta+Ctrl+S" : "F9"
        enabled: root.screenIndex !== 3
        onActivated: root.toggleSidebar(true)
    }
    Shortcut {
        id: escapeShortcut
        sequence: "Escape"
        onActivated: {
            if (sidebarDrawer.visible) {
                sidebarDrawer.close()
            } else if (root.screenIndex === 3) {
                Library.closeReplay()
            } else if (root.screenIndex === 2) {
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
    // Counted on presented frames (`frameSwapped`), not animation ticks.
    property int gateRenderedFrames: 0
    // The gate step whose first presented frame is still to be logged; the
    // test turns "gate step N" / "gate frame after step N" into latencies
    // (replay entry is step 52's).
    property int gateFrameMarkStep: 0
    onFrameSwapped: {
        if (!root.gateMode) return
        root.gateRenderedFrames += 1
        if (root.gateFrameMarkStep > 0) {
            console.log("gate frame after step", root.gateFrameMarkStep)
            root.gateFrameMarkStep = 0
        }
    }
    // A capture right after the scene changed (sport, effective tier,
    // workout or ghost) keeps the full settle below: new geometry uploads
    // over rendered frames. A seek or a camera swap in an unchanged scene
    // only needs frames rendered after it.
    property bool gateSceneFresh: true
    readonly property int gateSeenSport: Replay.sportIndex
    readonly property int gateSeenTier: Replay.effectiveQuality
    readonly property int gateSeenWorkout: Replay.workoutId
    readonly property bool gateSeenGhost: Replay.hasGhost
    onGateSeenSportChanged: gateSceneFresh = true
    onGateSeenTierChanged: gateSceneFresh = true
    onGateSeenWorkoutChanged: gateSceneFresh = true
    onGateSeenGhostChanged: gateSceneFresh = true
    property bool gateAwaitingScene: false
    property int gateSceneFramesTarget: 0
    property int gateSceneWaits: 0
    property int gateSceneTicks: 0
    // gateSceneMinTicks after a scene change, one tick otherwise.
    property int gateSceneTicksNeeded: 0
    property string gateSceneGrabName: ""
    // ROWPLAY_PHASE_CLOSEUPS: after a phase grab settles and saves, take a
    // second grab of the same seek through the close-up camera (the
    // torso-and-hands framing the wrist/posture judgement needs).
    property bool gateSceneCloseup: false
    // The shadow check's twin (tests/common, assert_shadows): after the
    // rower's High-tier grab, the same frame again with the key light's
    // shadow off. Only High and Ultra cast shadows.
    property bool gateSceneUnshadowed: false

    // An idle scene renders exactly one frame per change, so a "+N frames"
    // settle target is unreachable and mesh-buffer uploads (which only
    // progress on rendered frames) stall mid-geometry. Driving frames for
    // as long as a replay hold is active advances both. A FrameAnimation
    // only fires on frames something else caused, so it must request the
    // next one itself: until 2026-09-24 the replay scene re-dirtied itself
    // every few frames (a diagnostics-driven rule walk) and that kept these
    // frames coming; without it a paused scene renders once and every
    // settle ran out its tick bound. The gate timer kicks the first frame.
    FrameAnimation {
        running: root.gateMode
                 && (root.gateAwaitingReplay || root.gateAwaitingScene)
        onTriggered: root.update()
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
        var fresh = gateSceneFresh
        gateSceneFresh = false
        gateAwaitingScene = true
        gateSceneFramesTarget = gateRenderedFrames + 3
        gateSceneWaits = 0
        gateSceneTicks = 0
        gateSceneTicksNeeded = fresh ? gateSceneMinTicks : 1
        gateSceneGrabName = name
        gateSceneCloseup = Settings.phaseCloseups && name.indexOf("phase-") === 0
        gateSceneUnshadowed = name === "replay-row-high"
        console.log("gate scene: settling", name, "from",
                    gateRenderedFrames, "frames",
                    fresh ? "(scene changed)" : "(same scene)")
    }

    // Grabs the shell, or `item` (a popup, which lives in the window's
    // overlay, outside the shell).
    function grabScreen(name, item) {
        if (Settings.screenshotDir.length === 0) {
            return
        }
        grabPending = true
        grabWaits = 0
        var target = item ? item : shellRoot
        target.grabToImage(function(result) {
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
                root.gateSceneTicksNeeded = 1
                root.gateSceneGrabName = name + "-closeup"
                console.log("gate scene: settling", root.gateSceneGrabName,
                            "from", root.gateRenderedFrames, "frames")
                return
            }
            if (name.indexOf("-closeup") >= 0) {
                Replay.setCloseupCamera(false)
            }
            if (root.gateSceneUnshadowed) {
                // Same frame, the key light's shadow off: settle again so it
                // has rendered, then re-enter grabScreen for the twin.
                root.gateSceneUnshadowed = false
                detailColumn.children[3].shadowsSuppressed = true
                root.grabPending = false
                root.gateAwaitingScene = true
                root.gateSceneFramesTarget = root.gateRenderedFrames + 3
                root.gateSceneWaits = 0
                root.gateSceneTicks = 0
                root.gateSceneTicksNeeded = 1
                root.gateSceneGrabName = name + "-unshadowed"
                console.log("gate scene: settling", root.gateSceneGrabName,
                            "from", root.gateRenderedFrames, "frames")
                return
            }
            if (name.indexOf("-unshadowed") >= 0) {
                detailColumn.children[3].shadowsSuppressed = false
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
            if (root.gateAwaitingReplay || root.gateAwaitingScene) {
                root.update()
            }
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
                        && root.gateSceneTicks >= root.gateSceneTicksNeeded) {
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
            // Quick profile (ROWPLAY_GATE_PROFILE=quick): the shell, all six
            // languages, the member check, the mock syncs and the first
            // replay load, then teardown and the closing steps from 200 on
            // (menus, sidebar, settings over a replay, dialogs). No other
            // sport, ghost, tiers, phase shots or strip.
            if (Settings.gateQuick && root.gateStep === 55) root.gateStep = 84
            if (root.gateStep === 1)
                console.log("gate profile:", Settings.gateQuick ? "quick" : "full")
            console.log("gate step", root.gateStep)
            root.gateFrameMarkStep = root.gateStep
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
            // The date range through the sidebar's fields: a range, a
            // refused From (marked on that field alone), then cleared.
            case 45: sidebarColumn.enterDateRange("2024-01-01", "2024-12-31"); break
            case 46:
                sidebarColumn.enterDateRange("nope", "")
                console.log("gate date range: from",
                            sidebarColumn.dateFromInvalid ? "refused" : "accepted",
                            "to", sidebarColumn.dateToInvalid ? "refused" : "accepted")
                break
            case 47: sidebarColumn.enterDateRange("", ""); break
            case 48: Library.toggleSort(0); Library.selectWorkout(9001); break
            case 49: root.grabScreen("detail-nostrokes"); break
            case 50: Library.selectWorkout(1005); break
            case 51: root.grabScreen("detail-full"); break
            case 52:                                 // route push + rower
                // Pin the requested tier: the governor otherwise steps an
                // llvmpipe scene down mid-walk, so which venue a capture
                // shows would depend on the machine's speed.
                Replay.setGovernorAuto(false)
                Library.requestReplay(false)
                root.gateAwaitingReplay = true
                break
            case 53: Replay.loadWorkout(1001); break     // rower demo workout
            case 54:
                console.log("gate speed focus:", detailColumn.children[3].gateSpeedFocusTracksSelection())
                root.grabSettledScene("replay-row")
                break
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
            // High casts shadows: the shadow check's pair (the grab and its
            // twin with the key light's shadow off).
            case 63: Replay.setQualityIndex(2); root.grabSettledScene("replay-row-high"); break
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
                if (!Settings.benchMode) { root.gateStep = 199; break }
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
            // The shell's platform layer (ADR 0013), after the main walk:
            // the application menu, the sidebar toggle, the About dialog and
            // the sort menu are opened once each for the runtime-error scan.
            case 200: root.screenIndex = 0; appMenu.open(); break
            case 201: appMenu.close(); root.toggleSidebar(); break
            case 202: root.grabScreen("sidebar-hidden"); break
            case 203: root.toggleSidebar(); aboutDialog.open(); break
            case 204: aboutDialog.close(); sidebarColumn.showSortMenu(true); break
            case 205: sidebarColumn.showSortMenu(false); break
            // Settings opened over a replay: the replay closes on the way
            // in, Back returns to the workout, and Replay opens again (the
            // gate test asserts both lines).
            case 206:
                Library.selectWorkout(1001)
                Library.requestReplay(false)
                root.gateReplayWaits = 0
                root.gateAwaitingReplay = true
                break
            case 207: root.showSettings(); break
            case 208:
                root.goBack()
                console.log("gate settings over the replay: screen", root.screenIndex,
                            Library.isReplayPresented ? "presented" : "closed")
                Library.requestReplay(false)
                console.log("gate replay reopened: screen", root.screenIndex,
                            "block", Library.replayBlockReason)
                root.gateReplayWaits = 0
                root.gateAwaitingReplay = true
                break
            case 209: Library.closeReplay(); Library.clearSelection(); break
            // The logout dialog (demo mode never shows the button); held
            // open for one extra tick so a screen capture can see it.
            case 210:
                root.showSettings()
                detailColumn.children[2].showLogoutDialog(true)
                console.log("gate: logout dialog open")
                break
            case 211: break
            case 212: detailColumn.children[2].showLogoutDialog(false); root.screenIndex = 0; break
            // The width classes (Theme.widthClass), at widths that scale
            // with the text like the breakpoints: a medium window with its
            // drawer, which a chosen workout closes, then the compact one
            // (the list's page, settings, the dashboard and the replay HUD
            // in two rows), then the full layout again.
            case 213: root.width = Theme.px(720); break
            case 214: root.grabScreen("dashboard-medium"); break
            case 215: root.toggleSidebar(); break
            case 216: root.grabScreen("drawer-medium", sidebarDrawer.contentItem.parent); break
            case 217:
                sidebarColumn.choose(1005)
                console.log("gate width classes: medium", root.widthClass,
                            "drawer", sidebarDrawer.opened ? "open" : "closed",
                            "screen", root.screenIndex)
                break
            case 218: root.width = Theme.px(480); break
            case 219: root.grabScreen("detail-compact"); break
            case 220: root.toggleSidebar(); break
            case 221: root.grabScreen("sidebar-compact", sidebarDrawer.contentItem.parent); break
            case 222: root.showSettings(); break
            case 223: root.grabScreen("settings-compact"); break
            case 224: root.showDashboard(); break
            case 225: root.grabScreen("dashboard-compact"); break
            case 226:
                Library.selectWorkout(1001)
                Library.requestReplay(false)
                root.gateReplayWaits = 0
                root.gateAwaitingReplay = true
                break
            case 227: root.grabScreen("replay-compact"); break
            case 228:
                console.log("gate width classes: compact", root.widthClass,
                            "drawer", sidebarDrawer.opened ? "open" : "closed",
                            "screen", root.screenIndex)
                Library.closeReplay()
                Library.clearSelection()
                root.width = 1200
                break
            case 229:
                console.log("gate width classes: large", root.widthClass,
                            "sidebar", sidebarColumn.parent === drawerHost ? "in the drawer"
                                                                             : "beside the content")
                break
            // Settings: the timezone list filtered as typed.
            case 230:
                root.showSettings()
                console.log("gate timezone filter: york keeps",
                            detailColumn.children[2].filterTimezones("york"),
                            "of", Settings.timezoneLabels.length + 1)
                break
            case 231:
                detailColumn.children[2].filterTimezones("")
                root.screenIndex = 0
                if (!Settings.gateQuick) Library.selectWorkout(1004)
                break
            // The localized gap must fit together with all four gauges at the
            // compact width. Check every locale after live retranslation.
            case 232:
                if (Settings.gateQuick) { gateTimer.running = false; Qt.exit(0); break }
                Library.requestReplay(false)
                root.gateAwaitingReplay = true
                root.width = Theme.px(480)
                break
            case 233:
                Replay.loadWorkout(1004)
                Replay.loadGhost(1006)
                Replay.seek(0.5) // four gauges and a longer real gap (shorter rival)
                break
            case 234: case 236: case 238: case 240: case 242: case 244:
                // Exercise live QML translation without Settings' library
                // reload, which closes the replay route.
                Qt.uiLanguage = ["en", "zh", "de", "es", "fr", "ja"][(root.gateStep - 234) / 2]
                break
            case 235: case 237: case 239: case 241: case 243: case 245:
                console.log("gate compact gap:", Qt.uiLanguage, "fits",
                            detailColumn.children[3].gateGapLayoutFits())
                root.grabSettledScene("replay-gap-compact-" + Qt.uiLanguage)
                break
            case 246:
                Qt.uiLanguage = Settings.languageCode
                Library.closeReplay()
                root.width = 1200
                break
            default:
                gateTimer.running = false
                Qt.exit(0)
            }
        }
    }
}
