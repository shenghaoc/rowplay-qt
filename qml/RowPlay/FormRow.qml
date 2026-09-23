// SPDX-License-Identifier: GPL-3.0-or-later
// One grouped-form row (HIG Settings): the label at the leading edge with an
// optional detail line under it, the control(s) at the trailing edge, at
// least 44 px tall. `stacked` puts wide controls (the token field) under the
// label at full width; a row without a label lays its controls out from the
// leading edge.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Item {
    id: row

    /// Translated row label ("" for a controls-only row).
    property string label: ""
    /// Translated secondary line under the label ("" for none).
    property string detail: ""
    property color detailColor: Theme.textSecondary
    /// Controls under the label, full width (token entry).
    property bool stacked: false
    /// Marker FormRow siblings use to find the first visible row.
    readonly property bool isFormRow: true
    default property alias controls: trailing.data

    readonly property bool leadingControls: stacked || label.length === 0
    // The inset separator goes above every visible row except the first.
    readonly property bool firstVisibleRow: {
        var siblings = parent ? parent.children : []
        for (var i = 0; i < siblings.length; ++i) {
            if (siblings[i].isFormRow === true && siblings[i].visible) {
                return siblings[i] === row
            }
        }
        return true
    }

    Layout.fillWidth: true
    implicitWidth: grid.implicitWidth + 2 * Theme.spacingLarge
    implicitHeight: Math.max(44, grid.implicitHeight + 2 * Theme.spacingMedium + 2)

    Rectangle {
        visible: !row.firstVisibleRow
        x: Theme.spacingLarge
        width: parent.width - 2 * Theme.spacingLarge
        height: 1
        color: Theme.separator
    }

    GridLayout {
        id: grid
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: Theme.spacingLarge
        anchors.rightMargin: Theme.spacingLarge
        columns: row.leadingControls ? 1 : 2
        columnSpacing: Theme.spacingXLarge
        rowSpacing: Theme.spacingMedium

        ColumnLayout {
            Layout.fillWidth: true
            visible: row.label.length > 0 || row.detail.length > 0
            spacing: Theme.spacingXxSmall

            Label {
                Layout.fillWidth: true
                visible: row.label.length > 0
                text: row.label
                font: Theme.body
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
            Label {
                Layout.fillWidth: true
                visible: row.detail.length > 0
                text: row.detail
                font: Theme.subheadline
                color: row.detailColor
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
        }

        RowLayout {
            id: trailing
            Layout.fillWidth: row.leadingControls
            Layout.alignment: row.leadingControls ? Qt.AlignLeft | Qt.AlignVCenter
                                                  : Qt.AlignRight | Qt.AlignVCenter
            spacing: Theme.spacingMedium
        }
    }
}
