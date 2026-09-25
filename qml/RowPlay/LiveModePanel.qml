// SPDX-License-Identifier: GPL-3.0-or-later
// Live-mode panel — web `LiveModePanel.svelte` / Studio `LiveModePanelView`
// over the `Live` QML singleton. Logbook polling only (no PM5 / Bluetooth).
// A settings group of the style's own controls (ADR 0015): the switch with
// its hint, the check row with its one button and spinner, and the status
// lines.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

GroupBox {
    id: panel

    title: Tr.t("liveMode.title")

    readonly property string statusText: {
        if (Live.statusId === "liveMode.rateLimitRetry") {
            return Tr.t("liveMode.rateLimitRetry", { seconds: Live.statusRetrySecs })
        }
        return Live.statusId.length > 0 ? Tr.t(Live.statusId) : ""
    }

    ColumnLayout {
        width: parent.width
        spacing: Theme.spacingMedium

        // The switch beside its label and hint, as the settings screen's
        // switch rows: they wrap where the style would elide a switch's own
        // text, and a click on them toggles it.
        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.spacingXLarge

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingXxSmall

                Label {
                    Layout.fillWidth: true
                    text: Tr.t("liveMode.enabled")
                    font: Theme.body
                    color: Theme.textPrimary
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }
                Label {
                    Layout.fillWidth: true
                    text: Tr.t("liveMode.enabledHint")
                    font: Theme.subheadline
                    color: Theme.textSecondary
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }

                TapHandler {
                    enabled: enableToggle.enabled
                    onTapped: {
                        enableToggle.toggle()
                        enableToggle.toggled()
                    }
                }
            }
            Switch {
                id: enableToggle
                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter
                checked: Live.enabled
                enabled: Live.canEnable || Live.enabled
                onToggled: {
                    Live.setEnabled(checked)
                    // The store decides (it can refuse or clear it later).
                    checked = Qt.binding(function() { return Live.enabled })
                }
                Accessible.name: Tr.t("liveMode.enabled")
                Accessible.description: Tr.t("liveMode.enabledHint")
            }
        }

        // The last check (or the check in flight) and the one control, Check
        // now.
        RowLayout {
            visible: Live.enabled
            Layout.fillWidth: true
            spacing: Theme.spacingMedium

            Label {
                Layout.fillWidth: true
                text: Live.polling
                      ? Tr.t("liveMode.polling")
                      : Tr.t("liveMode.lastPollLabel") + " "
                        + (Live.lastPollText.length > 0 ? Live.lastPollText
                                                        : Tr.t("liveMode.neverUpdated"))
                font: Theme.body
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
            // Ours, not the style's: the macOS style's spinner is a WebP
            // animation, and none of the project's Qt installs carries the
            // WebP plugin (Qt Image Formats), so it would draw nothing there.
            AppBusyIndicator {
                visible: Live.polling
                running: Live.polling
                Accessible.name: Tr.t("liveMode.polling")
            }
            Button {
                text: Tr.t("liveMode.refresh")
                enabled: Live.enabled && !Live.polling
                Accessible.name: text
                onClicked: Live.refresh()
            }
        }

        // Kept visible after a 401 / 403 signs the panel out (enabled becomes
        // false).
        Label {
            Layout.fillWidth: true
            visible: panel.statusText.length > 0 && !Live.polling
            text: panel.statusText
            font: Theme.subheadline
            color: Theme.alertRed
            wrapMode: Text.WordWrap
            Accessible.name: text
        }

        Label {
            Layout.fillWidth: true
            visible: Live.hasWarning
            text: Tr.t("liveMode.warning", { count: Live.failureCount })
            font: Theme.subheadline
            color: Theme.alertRed
            wrapMode: Text.WordWrap
            Accessible.name: text
        }
    }
}
