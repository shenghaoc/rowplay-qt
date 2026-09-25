// SPDX-License-Identifier: GPL-3.0-or-later
// One grouped-settings row (ADR 0013): the label at the leading edge with an
// optional detail line under it, one control at the trailing edge (two at
// most), at least Theme.formRowHeight tall. `stacked` puts a wide control
// (the token field) under the label at full width; a row without a label
// lays its controls out from the leading edge, and so does a row too narrow
// for its label beside its controls (large text in a narrow window): the
// controls then sit under the label instead of covering it. A row whose
// control is a switch names it in `toggleTarget`: a click anywhere on the
// row toggles the switch, while keyboard focus stays on the switch itself,
// never the row.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import QtQuick.Templates as T
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
    /// The row's switch: a click on the row toggles it (it keeps the focus).
    /// Typed as the template: under the Basic style a plain `AbstractButton`
    /// names Basic's own AbstractButton.qml, which a Switch is not.
    property T.AbstractButton toggleTarget: null
    /// Marker FormRow siblings use to find the first visible row.
    readonly property bool isFormRow: true
    default property alias controls: trailing.data

    // The label keeps at least this much width beside its controls.
    readonly property real minimumLabelWidth: Theme.px(140)
    readonly property bool tooNarrow: label.length > 0 && width > 0
                                      && trailing.implicitWidth + Theme.spacingXLarge
                                         + minimumLabelWidth
                                         > width - 2 * Theme.spacingLarge
    readonly property bool leadingControls: stacked || label.length === 0 || tooNarrow
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
    implicitHeight: Math.max(Theme.formRowHeight,
                             grid.implicitHeight + 2 * Theme.spacingMedium + 2)

    // Whole-row toggle: a hover wash says the row is clickable; the tap
    // flips the switch and reports it like a user toggle, without moving
    // focus (the switch stays the only focus stop).
    Rectangle {
        anchors.fill: parent
        anchors.margins: Theme.hairline
        radius: Theme.radiusLarge - 1
        visible: row.toggleTarget !== null && rowHover.hovered
                 && row.toggleTarget.enabled
        color: Theme.hoverFill
    }
    HoverHandler {
        id: rowHover
        enabled: row.toggleTarget !== null
    }
    TapHandler {
        enabled: row.toggleTarget !== null && row.toggleTarget.enabled
        onTapped: {
            row.toggleTarget.toggle()
            row.toggleTarget.toggled()
        }
    }

    Rectangle {
        visible: !row.firstVisibleRow
        x: Theme.spacingLarge
        width: parent.width - 2 * Theme.spacingLarge
        height: Theme.hairline
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
