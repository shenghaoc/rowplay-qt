// SPDX-License-Identifier: GPL-3.0-or-later
// Modal dialog (ADR 0013): the popup surface with the large radius and a
// hairline outline, the title in the group-title face, a dimmed backdrop.
// Buttons go in a DialogButtonBox footer that keeps its default layout, so
// Qt orders them the platform's way (macOS, Windows, KDE and GNOME differ);
// callers give each PushButton its DialogButtonBox.buttonRole.
import QtQuick
import QtQuick.Controls
import RowPlay

Dialog {
    id: control

    modal: true
    padding: Theme.spacingXLarge
    topPadding: Theme.spacingLarge
    font: Theme.body

    header: Label {
        visible: control.title.length > 0
        text: control.title
        font: Theme.groupTitle
        color: Theme.textPrimary
        wrapMode: Text.WordWrap
        leftPadding: Theme.spacingXLarge
        rightPadding: Theme.spacingXLarge
        topPadding: Theme.spacingXLarge
        Accessible.name: text
    }

    background: Rectangle {
        color: Theme.popupBackground
        radius: Theme.radiusLarge
        border.width: Theme.outlineWidth
        border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator
    }

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, Theme.dark ? 0.55 : 0.32)
    }
}
