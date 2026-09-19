// SPDX-License-Identifier: GPL-3.0-or-later
// Settings screen — a port of rowplay-studio's SettingsView onto web locale
// ids (docs/source-map.md records each substitution):
// - token section: web `token.*` / `auth.logout` instead of Studio's
//   hardcoded labels; "connected" is expressed by the Log-out control
//   appearing, exactly like the web header (no status string);
// - the reduce-motion toggle uses a desktop-supplement key
//   (settings.reduceMotion) because the web has none;
// - the timezone picker is a flat list (labels carry the UTC offset) instead
//   of the web's grouped <select>.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: screen

    signal closed()

    padding: Theme.spacingXxxLarge

    ScrollView {
        anchors.fill: parent
        clip: true

        ColumnLayout {
            width: parent.width
            spacing: Theme.spacingXxLarge

            Label {
                text: Tr.t("settings.title")
                font: Theme.pageTitle
                color: Theme.textPrimary
                Accessible.name: text
            }

            // In-app version (Phase 9, R6.5): the one user-visible string
            // that identifies the running release in a bug report. Under the
            // title so it is visible without scrolling.
            Label {
                text: Tr.t("settings.appVersion", { version: Settings.appVersion })
                font: Theme.metricLabel
                color: Theme.textTertiary
                Accessible.name: text
            }

            // Library / demo mode -------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingSmall

                Label {
                    text: Tr.t("settings.eyebrow")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                Switch {
                    id: demoToggle
                    text: Tr.t("common.demoMode")
                    checked: Settings.demoModeEnabled
                    onToggled: Settings.setDemoModeEnabled(checked)
                    Accessible.name: Tr.t("common.demoMode")
                }

                Label {
                    Layout.fillWidth: true
                    text: Tr.t("settings.factDemo")
                    font: Theme.metricLabel
                    color: Theme.textTertiary
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }
            }

            // Replay quality --------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium

                Label {
                    text: Tr.t("replay.quality")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                RowLayout {
                    spacing: Theme.spacingSmall
                    Repeater {
                        model: Settings.qualityLabels
                        Button {
                            text: modelData
                            flat: true
                            highlighted: index === Settings.qualityIndex
                            onClicked: Settings.setQualityIndex(index)
                            Accessible.name: Tr.t("replay.quality") + " " + modelData
                        }
                    }
                }
            }

            // Reduce motion ---------------------------------------------------
            // The web has no locale key for this; the desktop supplement
            // (tools/convert-locales.mjs DESKTOP_SUPPLEMENT) provides
            // "settings.reduceMotion" with the English value for all six
            // locales so the pipeline stays the single source.
            Switch {
                text: Tr.t("settings.reduceMotion")
                checked: Settings.reduceReplayMotion
                onToggled: Settings.setReduceReplayMotion(checked)
                Accessible.name: Tr.t("settings.reduceMotion")
            }

            // Concept2 token --------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium

                Label {
                    text: Tr.t("token.title")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                // Connected: the web expresses this state purely through the
                // Log-out control, so the desktop shell does too.
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingSmall
                    visible: !Settings.hasToken

                    Label {
                        text: Tr.t("token.apiToken")
                        font: Theme.metricLabel
                        color: Theme.textSecondary
                        Accessible.name: text
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spacingMedium

                        TextField {
                            id: tokenField
                            Layout.fillWidth: true
                            Layout.maximumWidth: 420
                            echoMode: TextInput.Password
                            placeholderText: Tr.t("token.placeholder")
                            Accessible.name: Tr.t("token.apiToken")
                            onAccepted: saveTokenButton.clicked()
                        }

                        Button {
                            id: saveTokenButton
                            text: Tr.t("token.connect")
                            enabled: tokenField.text.trim().length > 0
                            onClicked: {
                                // The token crosses the bridge exactly once,
                                // into SecretToken + the OS keychain, and the
                                // field is cleared immediately.
                                Settings.saveToken(tokenField.text)
                                tokenField.text = ""
                            }
                            Accessible.name: Tr.t("token.connect")
                        }
                    }

                    Label {
                        Layout.fillWidth: true
                        Layout.maximumWidth: 420
                        text: Tr.t("token.introBefore") + Tr.t("token.introLink")
                              + Tr.t("token.introAfter")
                        font: Theme.metricLabel
                        color: Theme.textTertiary
                        wrapMode: Text.WordWrap
                        Accessible.name: text
                    }
                }

                Button {
                    visible: Settings.hasToken
                    text: Tr.t("auth.logout")
                    enabled: !Sync.isRunning
                    onClicked: disconnectDialog.open()
                    Accessible.name: Tr.t("auth.logout")
                }

                Label {
                    visible: Settings.statusTextId.length > 0
                    text: Settings.statusTextId.length > 0
                          ? Tr.t(Settings.statusTextId) : ""
                    font: Theme.metricLabel
                    color: Theme.alertRed
                    Accessible.name: text
                }
            }

            // Sync -------------------------------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium

                Label {
                    text: Tr.t("settings.syncTitle")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                Label {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 480
                    text: Tr.t("settings.syncNote")
                    font: Theme.metricLabel
                    color: Theme.textTertiary
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }

                // Two modes, matching settings.syncNote: the incremental
                // button is the default; full re-sync is the slower escape
                // hatch for when something looks wrong.
                RowLayout {
                    spacing: Theme.spacingMedium

                    Button {
                        text: Tr.t("settings.syncIncremental")
                        enabled: Sync.canSync
                        onClicked: Sync.start()
                        Accessible.name: Tr.t("settings.syncIncremental")
                    }

                    Button {
                        text: Tr.t("settings.syncFull")
                        enabled: Sync.canSync
                        onClicked: Sync.startFull()
                        Accessible.name: Tr.t("settings.syncFull")
                    }

                    Button {
                        visible: Sync.isRunning
                        text: Tr.t("workoutList.compareCancel")
                        onClicked: Sync.cancel()
                        Accessible.name: Tr.t("workoutList.compareCancel")
                    }

                    BusyIndicator {
                        visible: Sync.isRunning
                        running: Sync.isRunning
                        Layout.preferredWidth: 24
                        Layout.preferredHeight: 24
                    }
                }

                // Progress: determinate while the detail pass is sized
                // (-1 before the summary walk reports a total).
                ProgressBar {
                    id: syncProgress
                    Layout.fillWidth: true
                    Layout.maximumWidth: 480
                    visible: Sync.isRunning
                    from: 0
                    to: 1
                    indeterminate: Sync.progressFraction < 0
                    // The bar shows overall progress; the label carries the
                    // counts (rendered in Rust).
                    value: Sync.progressFraction < 0 ? 0 : Sync.progressFraction
                    Accessible.name: Tr.t("sync.inProgress")
                    Accessible.description: syncProgressLabel.text
                }

                Label {
                    id: syncProgressLabel
                    Layout.fillWidth: true
                    visible: Sync.isRunning
                    // Counts/remaining are rendered in Rust (sync worker);
                    // QML only prefixes the translated status.
                    text: Sync.progressTotal > 0
                          ? Tr.t("sync.inProgress") + " " + Sync.progressText
                          : Tr.t("sync.loading")
                    font: Theme.metricLabel
                    color: Theme.textSecondary
                    Accessible.name: text
                }

                // Last result / status line.
                Label {
                    Layout.fillWidth: true
                    visible: !Sync.isRunning && text.length > 0
                    font: Theme.metricLabel
                    color: Sync.statusId === "sync.failed"
                           ? Theme.alertRed : Theme.textSecondary
                    wrapMode: Text.WordWrap
                    text: {
                        if (Settings.demoModeEnabled && !Settings.hasToken) {
                            return Tr.t("settings.syncDemo")
                        }
                        if (Sync.statusId === "sync.incrementalDone") {
                            return Tr.t("sync.incrementalDone",
                                        { total: Sync.statusTotal })
                        }
                        // The web's exact result strings carry the counts;
                        // `Sync.statusSkipped` is available for callers that
                        // want the skip count, but no new English is invented
                        // here (the i18n parity check only admits web keys).
                        if (Sync.statusId === "sync.done") {
                            return Tr.t("sync.done", { added: Sync.statusAdded,
                                                       total: Sync.statusTotal })
                        }
                        if (Sync.statusId === "sync.failed") {
                            return Sync.statusMessage.length > 0
                                   ? Tr.t("sync.errorHint",
                                          { message: Sync.statusMessage })
                                   : Tr.t("sync.failed")
                        }
                        if (Sync.statusTotal > 0) {
                            return Sync.statusDate.length > 0
                                   ? Tr.t("settings.lastSync",
                                          { total: Sync.statusTotal,
                                            date: Sync.statusDate })
                                   : Tr.t("settings.partialCache",
                                          { n: Sync.statusTotal })
                        }
                        return ""
                    }
                    Accessible.name: text
                }
            }

            // Live mode (logbook page-1 polling — not PM5 / Bluetooth) ---------
            LiveModePanel {
                Layout.fillWidth: true
            }

            // Units, timezone, language -----------------------------------------
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium

                GridLayout {
                    Layout.fillWidth: true
                    Layout.maximumWidth: 560
                    columns: 2
                    columnSpacing: Theme.spacingXLarge
                    rowSpacing: Theme.spacingMedium

                    Label {
                        text: Tr.t("workoutList.sortDistance")
                        font: Theme.metricLabel
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    ComboBox {
                        id: unitCombo
                        Layout.fillWidth: true
                        // Unit symbols are untranslated by design (km / mi),
                        // like the web's chart axis labels.
                        model: Settings.unitLabels
                        Component.onCompleted: currentIndex = Settings.distanceUnitIndex
                        onActivated: function(index) {
                            Settings.setDistanceUnitIndex(index)
                        }
                        Accessible.name: Tr.t("workoutList.sortDistance")
                    }

                    Label {
                        text: Tr.t("settings.timezoneLabel")
                        font: Theme.metricLabel
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    ComboBox {
                        id: timezoneCombo
                        Layout.fillWidth: true
                        model: [Tr.t("settings.timezoneUtcDefault")].concat(
                                   Settings.timezoneLabels)
                        Component.onCompleted: currentIndex = Settings.homeTimezoneIndex
                        onActivated: function(index) {
                            Settings.setHomeTimezoneIndex(index)
                        }
                        Accessible.name: Tr.t("settings.timezoneLabel")
                        ToolTip.visible: hovered
                        ToolTip.text: Tr.t("settings.timezoneNote")
                        ToolTip.delay: 800
                    }

                    Label {
                        text: Tr.t("lang.switch")
                        font: Theme.metricLabel
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    ComboBox {
                        id: languageCombo
                        Layout.fillWidth: true
                        model: Settings.languageNames
                        Component.onCompleted: currentIndex = Settings.languageIndex
                        onActivated: function(index) {
                            Settings.setLanguageIndex(index)
                        }
                        Accessible.name: Tr.t("lang.switch")
                    }
                }
            }

            Button {
                text: Tr.t("common.dismiss")
                onClicked: screen.closed()
                Accessible.name: Tr.t("common.dismiss")
            }
        }
    }

    Connections {
        target: Settings
        function onSettingsChanged() {
            unitCombo.currentIndex = Settings.distanceUnitIndex
            timezoneCombo.currentIndex = Settings.homeTimezoneIndex
            languageCombo.currentIndex = Settings.languageIndex
        }
    }

    // Studio wraps Disconnect in a confirmation dialog; the web's log out is
    // immediate. Desktop keeps the confirmation (keychain write is cheap, but
    // losing the token is annoying).
    Dialog {
        id: disconnectDialog
        anchors.centerIn: parent
        // Explicit sizing: with implicit sizing the Dialog and its content
        // form a binding loop (caught by the runtime-error gate).
        width: 420
        modal: true
        title: Tr.t("auth.logout")
        standardButtons: Dialog.Ok | Dialog.Cancel

        Label {
            width: parent.width
            text: Tr.t("settings.deleteConfirm")
            wrapMode: Text.WordWrap
            Accessible.name: text
        }

        onAccepted: Settings.clearToken()
    }
}
