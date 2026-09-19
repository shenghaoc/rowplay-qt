// SPDX-License-Identifier: GPL-3.0-or-later
// Live-mode panel — web `LiveModePanel.svelte` / Studio `LiveModePanelView`
// over the `Live` QML singleton. Logbook polling only (no PM5 / Bluetooth).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ColumnLayout {
    id: panel
    spacing: Theme.spacingMedium

    Label {
        text: Tr.t("liveMode.title")
        font: Theme.sectionHeadline
        color: Theme.textPrimary
        Accessible.name: text
    }

    Label {
        Layout.fillWidth: true
        Layout.maximumWidth: 480
        text: Tr.t("liveMode.enabledHint")
        font: Theme.metricLabel
        color: Theme.textTertiary
        wrapMode: Text.WordWrap
        Accessible.name: text
    }

    Switch {
        id: enableToggle
        text: Tr.t("liveMode.enabled")
        checked: Live.enabled
        enabled: Live.canEnable || Live.enabled
        onToggled: Live.setEnabled(checked)
        Accessible.name: Tr.t("liveMode.enabled")
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.spacingMedium
        visible: Live.enabled

        Button {
            text: Tr.t("liveMode.refresh")
            enabled: Live.enabled && !Live.polling
            onClicked: Live.refresh()
            Accessible.name: Tr.t("liveMode.refresh")
        }

        BusyIndicator {
            visible: Live.polling
            running: Live.polling
            Layout.preferredWidth: 24
            Layout.preferredHeight: 24
            Accessible.name: Tr.t("liveMode.polling")
        }

        Label {
            visible: Live.polling
            text: Tr.t("liveMode.polling")
            font: Theme.metricLabel
            color: Theme.textSecondary
            Accessible.name: text
        }
    }

    Label {
        Layout.fillWidth: true
        visible: Live.enabled && !Live.polling
        text: Tr.t("liveMode.lastPollLabel") + " "
              + (Live.lastPollText.length > 0
                 ? Live.lastPollText
                 : Tr.t("liveMode.neverUpdated"))
        font: Theme.metricLabel
        color: Theme.textSecondary
        Accessible.name: text
    }

    Label {
        Layout.fillWidth: true
        // Keep visible after 401/403 signed-out (enabled becomes false).
        visible: Live.statusId.length > 0 && !Live.polling
        text: {
            if (Live.statusId === "liveMode.rateLimitRetry")
                return Tr.t("liveMode.rateLimitRetry",
                            { seconds: Live.statusRetrySecs })
            return Live.statusId.length > 0 ? Tr.t(Live.statusId) : ""
        }
        font: Theme.metricLabel
        color: Theme.alertRed
        wrapMode: Text.WordWrap
        Accessible.name: text
    }

    Label {
        Layout.fillWidth: true
        visible: Live.hasWarning
        text: Tr.t("liveMode.warning", { count: Live.failureCount })
        font: Theme.metricLabel
        color: Theme.alertRed
        wrapMode: Text.WordWrap
        Accessible.name: text
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
