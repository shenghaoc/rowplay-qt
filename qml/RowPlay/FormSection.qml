// SPDX-License-Identifier: GPL-3.0-or-later
// Grouped-settings section (ADR 0013): an optional title above a rounded
// group box that holds FormRow children, and an optional footer note below
// it — footers carry the notes, rows carry the controls. Each FormRow draws
// the inset hairline above itself unless it is the first visible row, so
// rows can come and go.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ColumnLayout {
    id: section

    /// Translated section title ("" for an untitled group).
    property string title: ""
    /// Translated footer note ("" for none).
    property string footer: ""
    default property alias rows: box.data

    Layout.fillWidth: true
    spacing: Theme.spacingSmall

    Label {
        Layout.fillWidth: true
        Layout.leftMargin: Theme.spacingXSmall
        visible: section.title.length > 0
        text: section.title
        font: Theme.groupTitle
        color: Theme.textPrimary
        wrapMode: Text.WordWrap
        Accessible.name: text
    }

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: box.implicitHeight
        radius: Theme.radiusLarge
        color: Theme.groupBackground
        border.width: Theme.hairline
        border.color: Theme.separator

        ColumnLayout {
            id: box
            anchors.left: parent.left
            anchors.right: parent.right
            spacing: 0
        }
    }

    Label {
        Layout.fillWidth: true
        Layout.leftMargin: Theme.spacingXSmall
        Layout.rightMargin: Theme.spacingXSmall
        visible: section.footer.length > 0
        text: section.footer
        font: Theme.subheadline
        color: Theme.textSecondary
        wrapMode: Text.WordWrap
        Accessible.name: text
    }
}
