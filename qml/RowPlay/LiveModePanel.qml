// SPDX-License-Identifier: GPL-3.0-or-later
// Live-mode panel — web `LiveModePanel.svelte` / Studio `LiveModePanelView`
// over the `Live` QML singleton. Logbook polling only (no PM5 / Bluetooth).
//
// A grouped-form section (HIG Settings, ADR 0013): the enable switch row,
// then while enabled the last-check line with the manual refresh, and the
// status / failure-warning lines as red detail text.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

FormSection {
    id: panel

    title: Tr.t("liveMode.title")
    footer: Tr.t("liveMode.enabledHint")

    // Status line: kept visible after a 401/403 sign-out, when `enabled`
    // has already gone false.
    readonly property string statusText: {
        if (Live.statusId.length === 0 || Live.polling)
            return ""
        if (Live.statusId === "liveMode.rateLimitRetry")
            return Tr.t("liveMode.rateLimitRetry", { seconds: Live.statusRetrySecs })
        return Tr.t(Live.statusId)
    }

    FormRow {
        label: Tr.t("liveMode.enabled")
        detail: panel.statusText
        detailColor: Theme.alertRed

        ToggleSwitch {
            id: enableToggle
            checked: Live.enabled
            enabled: Live.canEnable || Live.enabled
            onToggled: Live.setEnabled(checked)
            Accessible.name: Tr.t("liveMode.enabled")
        }
    }

    FormRow {
        visible: Live.enabled
        label: Live.polling
               ? Tr.t("liveMode.polling")
               : Tr.t("liveMode.lastPollLabel") + " "
                 + (Live.lastPollText.length > 0 ? Live.lastPollText
                                                 : Tr.t("liveMode.neverUpdated"))

        BusyIndicator {
            visible: Live.polling
            running: Live.polling
            Layout.preferredWidth: 22
            Layout.preferredHeight: 22
            Accessible.name: Tr.t("liveMode.polling")
        }

        PushButton {
            text: Tr.t("liveMode.refresh")
            enabled: Live.enabled && !Live.polling
            onClicked: Live.refresh()
        }
    }

    FormRow {
        visible: Live.hasWarning
        detail: Tr.t("liveMode.warning", { count: Live.failureCount })
        detailColor: Theme.alertRed
    }

    // Keep the enable switch in sync when the backend clears it (401 / stop).
    Connections {
        target: Live
        function onLiveChanged() {
            if (enableToggle.checked !== Live.enabled) {
                enableToggle.checked = Live.enabled
            }
        }
    }
}
