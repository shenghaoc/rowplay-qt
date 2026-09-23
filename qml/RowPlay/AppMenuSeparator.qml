// SPDX-License-Identifier: GPL-3.0-or-later
// Menu separator (ADR 0013): an inset hairline in Theme.separator.
import QtQuick
import QtQuick.Controls
import RowPlay

MenuSeparator {
    topPadding: Theme.spacingXSmall
    bottomPadding: Theme.spacingXSmall
    leftPadding: Theme.spacingMedium
    rightPadding: Theme.spacingMedium

    contentItem: Rectangle {
        implicitWidth: Theme.px(180)
        implicitHeight: Theme.hairline
        color: Theme.separator
    }
}
