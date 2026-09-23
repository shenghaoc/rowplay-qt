// SPDX-License-Identifier: GPL-3.0-or-later
// Live-mode panel — web `LiveModePanel.svelte` / Studio `LiveModePanelView`
// over the `Live` QML singleton. Logbook polling only (no PM5 / Bluetooth).
// A grouped-settings section (ADR 0013): the switch row (a click on the row
// toggles it), the check row with its one button, and the status lines.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

FormSection {
    id: panel

    title: Tr.t("liveMode.title")

    readonly property string statusText: {
        if (Live.statusId === "liveMode.rateLimitRetry") {
            return Tr.t("liveMode.rateLimitRetry", { seconds: Live.statusRetrySecs })
        }
        return Live.statusId.length > 0 ? Tr.t(Live.statusId) : ""
    }

    FormRow {
        label: Tr.t("liveMode.enabled")
        detail: Tr.t("liveMode.enabledHint")
        toggleTarget: enableToggle

        ToggleSwitch {
            id: enableToggle
            checked: Live.enabled
            enabled: Live.canEnable || Live.enabled
            onToggled: {
                Live.setEnabled(checked)
                // The store decides (it can refuse or clear it later).
                checked = Qt.binding(function() { return Live.enabled })
            }
            Accessible.name: Tr.t("liveMode.enabled")
        }
    }

    // The last check (or the check in flight) and the one control, Check now.
    FormRow {
        visible: Live.enabled
        label: Live.polling
               ? Tr.t("liveMode.polling")
               : Tr.t("liveMode.lastPollLabel") + " "
                 + (Live.lastPollText.length > 0 ? Live.lastPollText
                                                 : Tr.t("liveMode.neverUpdated"))

        AppBusyIndicator {
            visible: Live.polling
            running: Live.polling
            Accessible.name: Tr.t("liveMode.polling")
        }

        PushButton {
            text: Tr.t("liveMode.refresh")
            enabled: Live.enabled && !Live.polling
            onClicked: Live.refresh()
        }
    }

    // Kept visible after a 401 / 403 signs the panel out (enabled becomes
    // false).
    FormRow {
        visible: panel.statusText.length > 0 && !Live.polling
        detail: panel.statusText
        detailColor: Theme.alertRed
    }

    FormRow {
        visible: Live.hasWarning
        detail: Tr.t("liveMode.warning", { count: Live.failureCount })
        detailColor: Theme.alertRed
    }
}
