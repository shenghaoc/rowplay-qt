// SPDX-License-Identifier: GPL-3.0-or-later
// Menu (ADR 0013): the popup surface with a hairline outline and rounded
// corners; items are AppMenuItem (also for Actions placed in the menu) and
// AppMenuSeparator. Used for in-window menus (the sidebar's sort menu, the
// Windows / Linux application menu button); macOS gets the native menu bar.
import QtQuick
import QtQuick.Controls
import RowPlay

Menu {
    id: control

    padding: Theme.spacingXSmall
    font: Theme.body
    delegate: AppMenuItem {}

    background: Rectangle {
        implicitWidth: Theme.px(220)
        color: Theme.popupBackground
        radius: Theme.radiusMedium
        border.width: Theme.outlineWidth
        border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator
    }
}
