// SPDX-License-Identifier: GPL-3.0-or-later
// Push button (ADR 0013): a bezel on the control fill with a hairline
// outline, about 32 px tall at the reference text size. A label only —
// outside the toolbar a button shows a label or an icon, never both.
// `prominent` is the view's default action: the accent fill with an onAccent
// label (white or near-black, whichever passes WCAG AA against the accent in
// use). `destructive` tints the label red on the neutral bezel. At most one
// prominent or destructive button per view.
import QtQuick
import QtQuick.Controls
import RowPlay

Button {
    id: control

    property bool prominent: false
    property bool destructive: false

    // A disabled default button loses its accent: the neutral bezel with a
    // disabled label, so it can never read as live.
    readonly property bool accentFill: prominent && enabled
    readonly property color labelColor: !enabled ? Theme.textDisabled
                                        : accentFill ? Theme.onAccent
                                        : (destructive ? Theme.destructiveText : Theme.textPrimary)

    implicitHeight: Theme.controlHeight
    leftPadding: Theme.spacingLarge
    rightPadding: Theme.spacingLarge
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    font: Theme.bodyEmphasized
    Accessible.name: text

    contentItem: Label {
        text: control.text
        font: control.font
        color: control.labelColor
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
        Accessible.ignored: true
    }

    background: Rectangle {
        implicitWidth: Theme.px(72)
        radius: Theme.radiusSmall
        color: control.accentFill ? Theme.accentColor : Theme.controlBackground
        // Under high contrast the accent is the system highlight, which can
        // sit close to the window colour (1.64:1 in macOS's light palette),
        // so the prominent button keeps its outline there.
        border.width: control.accentFill && !Theme.highContrast ? 0 : Theme.hairline
        border.color: control.enabled ? Theme.controlBorder : Theme.separator

        // Hover / press wash over either fill.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            color: control.down ? (control.accentFill ? Qt.rgba(0, 0, 0, 0.18)
                                                      : Theme.pressedFill)
                                : (control.hovered && control.enabled
                                   ? (control.accentFill ? Qt.rgba(1, 1, 1, 0.10)
                                                         : Theme.hoverFill)
                                   : "transparent")
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: Theme.radiusSmall
        }
    }
}
