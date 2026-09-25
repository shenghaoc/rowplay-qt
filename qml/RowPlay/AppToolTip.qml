// SPDX-License-Identifier: GPL-3.0-or-later
// Tooltip (ADR 0013): the popup surface with a hairline outline below its
// control, secondary-size text. Supplementary only: whatever a tooltip says
// is also the control's accessible name and reachable another way (a menu
// entry, a shortcut, a visible label) — nothing is reachable by hover alone.
import QtQuick
import QtQuick.Controls.Basic
import RowPlay

ToolTip {
    id: control

    x: parent ? Math.round((parent.width - implicitWidth) / 2) : 0
    y: parent ? parent.height + Theme.spacingXSmall : 0
    delay: 600
    timeout: 8000
    margins: Theme.spacingMedium
    leftPadding: Theme.spacingMedium
    rightPadding: Theme.spacingMedium
    topPadding: Theme.spacingXSmall + Theme.px(1)
    bottomPadding: Theme.spacingXSmall + Theme.px(1)
    font: Theme.subheadline

    contentItem: Label {
        text: control.text
        font: control.font
        color: Theme.textPrimary
        wrapMode: Text.Wrap
    }

    background: Rectangle {
        color: Theme.popupBackground
        radius: Theme.radiusSmall
        border.width: Theme.outlineWidth
        border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator
    }
}
