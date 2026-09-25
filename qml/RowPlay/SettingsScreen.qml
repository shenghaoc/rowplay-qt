// SPDX-License-Identifier: GPL-3.0-or-later
// Settings screen — a port of rowplay-studio's SettingsView onto web locale
// ids (docs/source-map.md records each substitution):
// - token section: web `token.*` / `auth.logout` instead of Studio's
//   hardcoded labels; "connected" is expressed by the Log-out control
//   appearing, exactly like the web header (no status string);
// - the reduce-motion toggle uses a desktop-supplement key
//   (settings.reduceMotion) because the web has none;
// - the timezone picker is a flat list (labels carry the UTC offset) instead
//   of the web's grouped <select>, filtered as you type (round 2).
//
// ADR 0013, 0015: a grouped page of the style's own controls. Each group is
// a GroupBox; a row puts its label (and a detail line) beside its control,
// or above it where the two do not fit; a switch always sits beside its
// label, which wraps and toggles it; notes sit under the groups or in row
// details, never behind a hover. One column at most 640 px (scaled) wide. No dismiss
// button: back navigation, Escape and the toolbar's settings toggle close
// the page.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: screen

    // The page's margin is inside the scroll view, so the style's scroll bar
    // sits at the pane's edge and the groups keep their distance from it.
    padding: 0
    readonly property real pageMargin: Theme.spacingXxxLarge

    // The sync section's last result / status line; every branch of the
    // Phase 4 expression is kept.
    readonly property string syncStatusText: {
        if (Settings.demoModeEnabled && !Settings.hasToken) {
            return Tr.t("settings.syncDemo")
        }
        if (Sync.statusId === "sync.incrementalDone") {
            return Tr.t("sync.incrementalDone", { total: Sync.statusTotal })
        }
        // The web's exact result strings carry the counts;
        // `Sync.statusSkipped` is available for callers that want the skip
        // count, but no new English is invented here (the i18n parity check
        // only admits web keys).
        if (Sync.statusId === "sync.done") {
            return Tr.t("sync.done", { added: Sync.statusAdded,
                                       total: Sync.statusTotal })
        }
        if (Sync.statusId === "sync.failed") {
            return Sync.statusMessage.length > 0
                   ? Tr.t("sync.errorHint", { message: Sync.statusMessage })
                   : Tr.t("sync.failed")
        }
        if (Sync.statusTotal > 0) {
            return Sync.statusDate.length > 0
                   ? Tr.t("settings.lastSync", { total: Sync.statusTotal,
                                                 date: Sync.statusDate })
                   : Tr.t("settings.partialCache", { n: Sync.statusTotal })
        }
        return ""
    }

    // Whether the status line reports a failure (demo mode shows its own
    // line instead): it turns red and offers the retry.
    readonly property bool syncFailed: !(Settings.demoModeEnabled && !Settings.hasToken)
                                       && Sync.statusId === "sync.failed"

    // The token section's status line (red detail text on its row).
    readonly property string tokenStatusText: Settings.statusTextId.length > 0
                                              ? Tr.t(Settings.statusTextId) : ""

    // A row's label and its optional detail line (layout, not a control).
    component RowLabel: ColumnLayout {
        property string text: ""
        property string detail: ""
        property color detailColor: Theme.textSecondary
        Layout.fillWidth: true
        spacing: Theme.spacingXxSmall

        Label {
            Layout.fillWidth: true
            visible: parent.text.length > 0
            text: parent.text
            font: Theme.body
            color: Theme.textPrimary
            wrapMode: Text.WordWrap
            Accessible.name: text
        }
        Label {
            Layout.fillWidth: true
            visible: parent.detail.length > 0
            text: parent.detail
            font: Theme.subheadline
            color: parent.detailColor
            wrapMode: Text.WordWrap
            Accessible.name: text
        }
    }

    // A switch beside its label and detail (layout, not a control): they wrap
    // where the style would elide a switch's own text (a long translation in
    // a narrow window), and a click on them toggles the switch, as a click on
    // that text would.
    component SwitchRow: RowLayout {
        id: switchRow
        property alias text: rowLabel.text
        property alias detail: rowLabel.detail
        property alias checked: rowSwitch.checked
        signal toggled()
        Layout.fillWidth: true
        spacing: Theme.spacingXLarge

        RowLabel {
            id: rowLabel
            TapHandler {
                onTapped: {
                    rowSwitch.toggle()
                    rowSwitch.toggled()
                }
            }
        }
        Switch {
            id: rowSwitch
            Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
            onToggled: switchRow.toggled()
            Accessible.name: rowLabel.text
            Accessible.description: rowLabel.detail
        }
    }

    // A group's note, under it.
    component Note: Label {
        Layout.fillWidth: true
        Layout.leftMargin: Theme.spacingXSmall
        Layout.rightMargin: Theme.spacingXSmall
        visible: text.length > 0
        font: Theme.subheadline
        color: Theme.textSecondary
        wrapMode: Text.WordWrap
        Accessible.name: text
    }

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        Item {
            width: scroll.availableWidth
            implicitHeight: form.implicitHeight + 2 * screen.pageMargin

            ColumnLayout {
                id: form
                anchors.horizontalCenter: parent.horizontalCenter
                y: screen.pageMargin
                width: Math.min(parent.width - 2 * screen.pageMargin, Theme.px(640))
                spacing: Theme.spacingXxLarge

                // A label and its control side by side while they fit, one
                // above the other where they do not (large text in a narrow
                // window).
                readonly property int columns: width >= Theme.px(460) ? 2 : 1

                // Title + the running version (Phase 9, R6.5: the one string
                // that identifies the release in a bug report).
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingXSmall

                    Label {
                        Layout.fillWidth: true
                        text: Tr.t("settings.title")
                        font: Theme.pageTitle
                        color: Theme.textPrimary
                        wrapMode: Text.WordWrap
                        Accessible.name: text
                    }
                    Label {
                        text: Tr.t("settings.appVersion", { version: Settings.appVersion })
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                }

                // Library / demo mode ------------------------------------
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingSmall

                    GroupBox {
                        Layout.fillWidth: true
                        title: Tr.t("settings.eyebrow")

                        SwitchRow {
                            width: parent.width
                            text: Tr.t("common.demoMode")
                            checked: Settings.demoModeEnabled
                            onToggled: {
                                Settings.setDemoModeEnabled(checked)
                                checked = Qt.binding(function() {
                                    return Settings.demoModeEnabled
                                })
                            }
                        }
                    }
                    Note { text: Tr.t("settings.factDemo") }
                }

                // Replay quality + reduce motion -------------------------
                // The reduce-motion key is a desktop supplement
                // (tools/convert-locales.mjs DESKTOP_SUPPLEMENT): the web has
                // none.
                GroupBox {
                    Layout.fillWidth: true

                    GridLayout {
                        width: parent.width
                        columns: form.columns
                        columnSpacing: Theme.spacingXLarge
                        rowSpacing: Theme.spacingMedium

                        RowLabel { text: Tr.t("replay.quality") }
                        // A setting changed now and then: the platform's
                        // pop-up button (design.md, "SegmentedControl, per
                        // use").
                        ComboBox {
                            id: qualityCombo
                            Layout.preferredWidth: Theme.px(220)
                            model: Settings.qualityLabels
                            currentIndex: Settings.qualityIndex
                            onActivated: function(index) {
                                Settings.setQualityIndex(index)
                            }
                            Accessible.name: Tr.t("replay.quality")
                        }

                        SwitchRow {
                            Layout.columnSpan: form.columns
                            text: Tr.t("settings.reduceMotion")
                            checked: Settings.reduceReplayMotion
                            onToggled: {
                                Settings.setReduceReplayMotion(checked)
                                checked = Qt.binding(function() {
                                    return Settings.reduceReplayMotion
                                })
                            }
                        }
                    }
                }

                // Concept2 token -----------------------------------------
                // Connected is expressed purely through the Log-out control,
                // like the web header.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingSmall

                    GroupBox {
                        Layout.fillWidth: true
                        title: Tr.t("token.title")

                        ColumnLayout {
                            width: parent.width
                            spacing: Theme.spacingMedium

                            // Not connected: the token field and the page's
                            // one prominent action, under the row's label.
                            RowLabel {
                                visible: !Settings.hasToken
                                text: Tr.t("token.apiToken")
                                detail: screen.tokenStatusText
                                detailColor: Theme.alertRed
                            }
                            RowLayout {
                                visible: !Settings.hasToken
                                Layout.fillWidth: true
                                spacing: Theme.spacingMedium

                                TextField {
                                    id: tokenField
                                    Layout.fillWidth: true
                                    echoMode: TextInput.Password
                                    placeholderText: Tr.t("token.placeholder")
                                    Accessible.name: Tr.t("token.apiToken")
                                    onAccepted: saveTokenButton.clicked()
                                }
                                Button {
                                    id: saveTokenButton
                                    text: Tr.t("token.connect")
                                    highlighted: true
                                    enabled: tokenField.text.trim().length > 0
                                    Accessible.name: text
                                    onClicked: {
                                        // The token crosses the bridge exactly
                                        // once, into SecretToken + the OS
                                        // keychain, and the field is cleared
                                        // immediately.
                                        Settings.saveToken(tokenField.text)
                                        tokenField.text = ""
                                    }
                                }
                            }

                            // Connected: the status and Log out, instead of the
                            // token row, so the page never has both a
                            // prominent and a destructive button.
                            RowLayout {
                                visible: Settings.hasToken
                                Layout.fillWidth: true
                                spacing: Theme.spacingMedium

                                RowLabel {
                                    detail: screen.tokenStatusText
                                    detailColor: Theme.alertRed
                                }
                                Button {
                                    text: Tr.t("auth.logout")
                                    enabled: !Sync.isRunning
                                    Accessible.name: text
                                    onClicked: disconnectDialog.open()
                                }
                            }
                        }
                    }
                    Note {
                        text: Settings.hasToken
                              ? ""
                              : Tr.t("token.introBefore") + Tr.t("token.introLink")
                                + Tr.t("token.introAfter")
                    }
                }

                // Sync ---------------------------------------------------
                // Two modes, matching settings.syncNote: the incremental
                // button is the default; full re-sync is the slower escape
                // hatch for when something looks wrong.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingSmall

                    GroupBox {
                        id: syncGroup
                        Layout.fillWidth: true
                        title: Tr.t("settings.syncTitle")

                        ColumnLayout {
                            width: parent.width
                            spacing: Theme.spacingMedium

                            // The two buttons stand side by side while they
                            // fit, and one under the other when they do not
                            // (large text in a narrow window).
                            GridLayout {
                                columns: incremental.implicitWidth + columnSpacing
                                         + full.implicitWidth <= parent.width ? 2 : 1
                                columnSpacing: Theme.spacingMedium
                                rowSpacing: Theme.spacingMedium

                                Button {
                                    id: incremental
                                    text: Tr.t("settings.syncIncremental")
                                    enabled: Sync.canSync
                                    Accessible.name: text
                                    onClicked: Sync.start()
                                }
                                Button {
                                    id: full
                                    text: Tr.t("settings.syncFull")
                                    enabled: Sync.canSync
                                    Accessible.name: text
                                    onClicked: Sync.startFull()
                                }
                            }

                            // While a sync runs: the progress (determinate
                            // once the detail pass is sized; the counts are
                            // rendered in Rust) and its one control, Cancel.
                            RowLayout {
                                visible: Sync.isRunning
                                Layout.fillWidth: true
                                spacing: Theme.spacingMedium

                                RowLabel {
                                    text: Sync.progressTotal > 0
                                          ? Tr.t("sync.inProgress") + " " + Sync.progressText
                                          : Tr.t("sync.loading")
                                }
                                ProgressBar {
                                    Layout.preferredWidth: Theme.px(140)
                                    from: 0
                                    to: 1
                                    indeterminate: Sync.progressFraction < 0
                                    value: Sync.progressFraction < 0 ? 0 : Sync.progressFraction
                                    Accessible.name: Tr.t("sync.inProgress")
                                }
                                Button {
                                    text: Tr.t("workoutList.compareCancel")
                                    Accessible.name: text
                                    onClicked: Sync.cancel()
                                }
                            }

                            // The last result: a status line, never a dialog.
                            // A failure names itself ("Sync failed", the
                            // row's label) above the error it met
                            // (sync.errorHint is the error alone) and offers
                            // its retry right here, in the mode that failed.
                            RowLayout {
                                visible: !Sync.isRunning && screen.syncStatusText.length > 0
                                Layout.fillWidth: true
                                spacing: Theme.spacingMedium

                                RowLabel {
                                    text: screen.syncFailed ? Tr.t("sync.failed") : ""
                                    detail: !screen.syncFailed ? screen.syncStatusText
                                            : Sync.statusMessage.length > 0
                                              ? Tr.t("sync.errorHint",
                                                     { message: Sync.statusMessage })
                                              : ""
                                    detailColor: screen.syncFailed ? Theme.alertRed
                                                                   : Theme.textSecondary
                                }
                                Button {
                                    visible: screen.syncFailed
                                    text: Tr.t("sync.retry")
                                    enabled: Sync.canSync
                                    Accessible.name: text
                                    onClicked: Sync.retry()
                                }
                            }
                        }
                    }
                    Note { text: Tr.t("settings.syncNote") }
                }

                // Live mode (logbook page-1 polling — not PM5 / Bluetooth).
                LiveModePanel {
                    Layout.fillWidth: true
                }

                // Units, timezone, language ------------------------------
                GroupBox {
                    Layout.fillWidth: true

                    GridLayout {
                        width: parent.width
                        columns: form.columns
                        columnSpacing: Theme.spacingXLarge
                        rowSpacing: Theme.spacingMedium

                        RowLabel { text: Tr.t("workoutList.sortDistance") }
                        ComboBox {
                            id: unitCombo
                            Layout.preferredWidth: Theme.px(220)
                            // Unit symbols are untranslated by design
                            // (km / mi), like the web's chart axis labels.
                            model: Settings.unitLabels
                            Component.onCompleted: currentIndex = Settings.distanceUnitIndex
                            onActivated: function(index) {
                                Settings.setDistanceUnitIndex(index)
                            }
                            Accessible.name: Tr.t("workoutList.sortDistance")
                        }

                        // The web's explanatory note as the row's detail: it
                        // was a hover-only tooltip.
                        RowLabel {
                            text: Tr.t("settings.timezoneLabel")
                            detail: Tr.t("settings.timezoneNote")
                        }
                        // 48 entries: the field filters them, by city, UTC
                        // offset or zone name (the view-model decides), and
                        // the pop-up lists the matches. It always shows the
                        // zone in use, filtered out or not.
                        ColumnLayout {
                            Layout.preferredWidth: Theme.px(220)
                            spacing: Theme.spacingXSmall

                            TextField {
                                id: timezoneFilter
                                Layout.fillWidth: true
                                placeholderText: Tr.t("workoutList.search")
                                // The view-model reads at most 64 characters
                                // of a filter.
                                maximumLength: 64
                                Accessible.name: Tr.t("workoutList.search")
                                Accessible.description: Tr.t("settings.timezoneLabel")
                            }
                            ComboBox {
                                id: timezoneCombo
                                Layout.fillWidth: true
                                readonly property string utcLabel: Tr.t("settings.timezoneUtcDefault")
                                readonly property var labels: [utcLabel].concat(Settings.timezoneLabels)
                                /// The full-list indices the filter keeps, in order.
                                readonly property var matches: {
                                    if (timezoneFilter.text.length === 0) {
                                        var all = []
                                        for (var i = 0; i < labels.length; ++i) {
                                            all.push(i)
                                        }
                                        return all
                                    }
                                    return Settings.timezoneMatches(timezoneFilter.text, utcLabel)
                                }
                                property int homeIndex: Settings.homeTimezoneIndex
                                model: matches.map(function(i) { return labels[i] })
                                currentIndex: matches.indexOf(homeIndex)
                                displayText: labels[homeIndex] !== undefined ? labels[homeIndex] : ""
                                onActivated: function(index) {
                                    Settings.setHomeTimezoneIndex(matches[index])
                                    timezoneFilter.text = ""
                                }
                                Accessible.name: Tr.t("settings.timezoneLabel")
                                Accessible.description: Tr.t("settings.timezoneNote")
                            }
                        }

                        RowLabel { text: Tr.t("lang.switch") }
                        ComboBox {
                            id: languageCombo
                            Layout.preferredWidth: Theme.px(220)
                            model: Settings.languageNames
                            Component.onCompleted: currentIndex = Settings.languageIndex
                            onActivated: function(index) {
                                Settings.setLanguageIndex(index)
                            }
                            Accessible.name: Tr.t("lang.switch")
                        }
                    }
                }
            }
        }
    }

    Connections {
        target: Settings
        function onSettingsChanged() {
            unitCombo.currentIndex = Settings.distanceUnitIndex
            timezoneCombo.homeIndex = Settings.homeTimezoneIndex
            languageCombo.currentIndex = Settings.languageIndex
        }
    }

    // Studio wraps Disconnect in a confirmation dialog; the web's log out is
    // immediate. Desktop keeps the confirmation (keychain write is cheap, but
    // losing the token is annoying). The dialog's standard buttons, in the
    // platform's order, are named with web strings: the app's catalogues
    // never translated Qt's own OK / Cancel.
    Dialog {
        id: disconnectDialog
        anchors.centerIn: parent
        // Explicit width: with implicit sizing the Dialog and its content
        // form a binding loop (caught by the runtime-error gate).
        width: Math.min(screen.width - 2 * Theme.spacingXLarge, Theme.px(420))
        title: Tr.t("auth.logout")
        modal: true
        standardButtons: Dialog.Ok | Dialog.Cancel
        onAboutToShow: {
            standardButton(Dialog.Ok).text = Tr.t("auth.logout")
            standardButton(Dialog.Cancel).text = Tr.t("workoutList.compareCancel")
        }

        // The text wraps inside a layout that spans the dialog: a bare
        // wrapping Label sized to its parent made the dialog's implicit
        // height depend on itself (a binding loop the gate reports the first
        // time the dialog opens).
        ColumnLayout {
            width: parent.width

            Label {
                Layout.fillWidth: true
                text: Tr.t("settings.deleteConfirm")
                font: Theme.body
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
        }

        onAccepted: Settings.clearToken()
    }

    /// The runtime-error gate filters the timezone list once: it types
    /// `text` into the filter ("" clears it and closes the list), opens the
    /// list of matches and returns how many the filter keeps.
    function filterTimezones(text) {
        timezoneFilter.text = text
        if (text.length === 0) {
            timezoneCombo.popup.close()
            return 0
        }
        timezoneCombo.popup.open()
        return timezoneCombo.matches.length
    }

    /// The runtime-error gate opens the logout dialog once (demo mode never
    /// shows the button that opens it).
    function showLogoutDialog(open) {
        if (open) {
            disconnectDialog.open()
        } else {
            disconnectDialog.close()
        }
    }
}
