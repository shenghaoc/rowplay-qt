// SPDX-License-Identifier: GPL-3.0-or-later
// Dialog button row (ADR 0013): PushButtons at their natural width on the
// trailing edge, drawn on the dialog's own surface (Basic's box paints the
// window colour, which differs from the popup surface in dark mode). The
// buttonLayout stays Qt's default, so the platform theme orders the roles
// (macOS, Windows, KDE and GNOME differ); callers add PushButtons with a
// DialogButtonBox.buttonRole and translated text.
import QtQuick
import QtQuick.Controls
import RowPlay

DialogButtonBox {
    alignment: Qt.AlignRight
    spacing: Theme.spacingMedium
    padding: Theme.spacingXLarge
    topPadding: 0
    delegate: PushButton {}
    background: Item {}
}
