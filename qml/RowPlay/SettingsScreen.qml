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
//
// Design system (ADR 0013): a grouped page — the label at each row's leading
// edge, at most two controls at its trailing edge, a click anywhere on a
// switch's row toggles it (the switch keeps the focus), notes in section
// footers or row details, never behind a hover. One column at most 640 px
// (scaled) wide. No dismiss button: back navigation, Escape and the
// toolbar's settings toggle close the page.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: screen

    padding: Theme.spacingXxxLarge

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

    // The token section's status line (red detail text on its row).
    readonly property string tokenStatusText: Settings.statusTextId.length > 0
                                              ? Tr.t(Settings.statusTextId) : ""

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical: AppScrollBar {
            parent: scroll
            x: scroll.mirrored ? 0 : scroll.width - width
            y: scroll.topPadding
            height: scroll.availableHeight
        }

        Item {
            width: scroll.availableWidth
            implicitHeight: form.implicitHeight

            ColumnLayout {
                id: form
                anchors.horizontalCenter: parent.horizontalCenter
                width: Math.min(parent.width, Theme.px(640))
                spacing: Theme.spacingXxLarge

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
                FormSection {
                    title: Tr.t("settings.eyebrow")
                    footer: Tr.t("settings.factDemo")

                    FormRow {
                        label: Tr.t("common.demoMode")
                        toggleTarget: demoSwitch

                        ToggleSwitch {
                            id: demoSwitch
                            checked: Settings.demoModeEnabled
                            onToggled: {
                                Settings.setDemoModeEnabled(checked)
                                checked = Qt.binding(function() {
                                    return Settings.demoModeEnabled
                                })
                            }
                            Accessible.name: Tr.t("common.demoMode")
                        }
                    }
                }

                // Replay quality + reduce motion -------------------------
                // The reduce-motion key is a desktop supplement
                // (tools/convert-locales.mjs DESKTOP_SUPPLEMENT): the web has
                // none.
                FormSection {
                    FormRow {
                        label: Tr.t("replay.quality")

                        SegmentedControl {
                            model: Settings.qualityLabels
                            currentIndex: Settings.qualityIndex
                            label: Tr.t("replay.quality")
                            onActivated: function(index) {
                                Settings.setQualityIndex(index)
                            }
                        }
                    }

                    FormRow {
                        label: Tr.t("settings.reduceMotion")
                        toggleTarget: motionSwitch

                        ToggleSwitch {
                            id: motionSwitch
                            checked: Settings.reduceReplayMotion
                            onToggled: {
                                Settings.setReduceReplayMotion(checked)
                                checked = Qt.binding(function() {
                                    return Settings.reduceReplayMotion
                                })
                            }
                            Accessible.name: Tr.t("settings.reduceMotion")
                        }
                    }
                }

                // Concept2 token -----------------------------------------
                // Connected is expressed purely through the Log-out control,
                // like the web header.
                FormSection {
                    title: Tr.t("token.title")
                    footer: Settings.hasToken
                            ? ""
                            : Tr.t("token.introBefore") + Tr.t("token.introLink")
                              + Tr.t("token.introAfter")

                    FormRow {
                        visible: !Settings.hasToken
                        stacked: true
                        label: Tr.t("token.apiToken")
                        detail: screen.tokenStatusText
                        detailColor: Theme.alertRed

                        InputField {
                            id: tokenField
                            Layout.fillWidth: true
                            echoMode: TextInput.Password
                            placeholderText: Tr.t("token.placeholder")
                            Accessible.name: Tr.t("token.apiToken")
                            onAccepted: saveTokenButton.clicked()
                        }

                        // The page's one prominent action.
                        PushButton {
                            id: saveTokenButton
                            text: Tr.t("token.connect")
                            prominent: true
                            enabled: tokenField.text.trim().length > 0
                            onClicked: {
                                // The token crosses the bridge exactly once,
                                // into SecretToken + the OS keychain, and the
                                // field is cleared immediately.
                                Settings.saveToken(tokenField.text)
                                tokenField.text = ""
                            }
                        }
                    }

                    // Shown instead of the token row, so the page never has
                    // both a prominent and a destructive button.
                    FormRow {
                        visible: Settings.hasToken
                        detail: screen.tokenStatusText
                        detailColor: Theme.alertRed

                        PushButton {
                            text: Tr.t("auth.logout")
                            destructive: true
                            enabled: !Sync.isRunning
                            onClicked: disconnectDialog.open()
                        }
                    }
                }

                // Sync ---------------------------------------------------
                // Two modes, matching settings.syncNote: the incremental
                // button is the default; full re-sync is the slower escape
                // hatch for when something looks wrong.
                FormSection {
                    title: Tr.t("settings.syncTitle")
                    footer: Tr.t("settings.syncNote")

                    FormRow {
                        id: syncRow

                        // The two buttons stand side by side while the row
                        // has room for both, and one under the other when
                        // it has not (large text in a narrow window). A
                        // grid, not a Flow: like the row's own layout it
                        // puts them on whole pixels. FormRow insets its
                        // controls by spacingLarge on each side.
                        GridLayout {
                            columns: incremental.implicitWidth + columnSpacing
                                     + full.implicitWidth
                                     <= syncRow.width - 2 * Theme.spacingLarge ? 2 : 1
                            columnSpacing: Theme.spacingMedium
                            rowSpacing: Theme.spacingMedium

                            PushButton {
                                id: incremental
                                text: Tr.t("settings.syncIncremental")
                                enabled: Sync.canSync
                                onClicked: Sync.start()
                            }

                            PushButton {
                                id: full
                                text: Tr.t("settings.syncFull")
                                enabled: Sync.canSync
                                onClicked: Sync.startFull()
                            }
                        }
                    }

                    // While a sync runs: the progress (determinate once the
                    // detail pass is sized; the counts are rendered in Rust)
                    // and its one control, Cancel.
                    FormRow {
                        visible: Sync.isRunning
                        label: Sync.progressTotal > 0
                               ? Tr.t("sync.inProgress") + " " + Sync.progressText
                               : Tr.t("sync.loading")

                        AppProgressBar {
                            Layout.preferredWidth: Theme.px(140)
                            from: 0
                            to: 1
                            indeterminate: Sync.progressFraction < 0
                            value: Sync.progressFraction < 0 ? 0 : Sync.progressFraction
                            Accessible.name: Tr.t("sync.inProgress")
                        }

                        PushButton {
                            text: Tr.t("workoutList.compareCancel")
                            onClicked: Sync.cancel()
                        }
                    }

                    // Last result / status line.
                    FormRow {
                        visible: !Sync.isRunning && screen.syncStatusText.length > 0
                        detail: screen.syncStatusText
                        detailColor: Sync.statusId === "sync.failed"
                                     ? Theme.alertRed : Theme.textSecondary
                    }
                }

                // Live mode (logbook page-1 polling — not PM5 / Bluetooth).
                LiveModePanel {
                    Layout.fillWidth: true
                }

                // Units, timezone, language ------------------------------
                FormSection {
                    FormRow {
                        label: Tr.t("workoutList.sortDistance")

                        PopupButton {
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
                    }

                    FormRow {
                        label: Tr.t("settings.timezoneLabel")
                        // The web's explanatory note as the row's detail:
                        // it was a hover-only tooltip.
                        detail: Tr.t("settings.timezoneNote")

                        PopupButton {
                            id: timezoneCombo
                            Layout.preferredWidth: Theme.px(220)
                            model: [Tr.t("settings.timezoneUtcDefault")].concat(
                                       Settings.timezoneLabels)
                            Component.onCompleted: currentIndex = Settings.homeTimezoneIndex
                            onActivated: function(index) {
                                Settings.setHomeTimezoneIndex(index)
                            }
                            Accessible.name: Tr.t("settings.timezoneLabel")
                            Accessible.description: Tr.t("settings.timezoneNote")
                        }
                    }

                    FormRow {
                        label: Tr.t("lang.switch")

                        PopupButton {
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
            timezoneCombo.currentIndex = Settings.homeTimezoneIndex
            languageCombo.currentIndex = Settings.languageIndex
        }
    }

    // Studio wraps Disconnect in a confirmation dialog; the web's log out is
    // immediate. Desktop keeps the confirmation (keychain write is cheap, but
    // losing the token is annoying). PushButtons with web strings replace
    // Qt's standard OK / Cancel (never translated by the app's catalogues),
    // and AppDialogButtonBox keeps Qt's default layout, so the platform
    // orders them.
    AppDialog {
        id: disconnectDialog
        anchors.centerIn: parent
        // Explicit width: with implicit sizing the Dialog and its content
        // form a binding loop (caught by the runtime-error gate).
        width: Math.min(screen.width - 2 * Theme.spacingXLarge, Theme.px(420))
        title: Tr.t("auth.logout")

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

        footer: AppDialogButtonBox {
            PushButton {
                text: Tr.t("workoutList.compareCancel")
                DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
            }
            PushButton {
                text: Tr.t("auth.logout")
                destructive: true
                DialogButtonBox.buttonRole: DialogButtonBox.AcceptRole
            }
        }

        onAccepted: Settings.clearToken()
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
