// SPDX-License-Identifier: GPL-3.0-or-later
// Slider (ADR 0013): a thin rounded track in the segment-track tone with the
// accent fill up to a white knob that carries a hairline outline, and the
// focus ring on keyboard focus. The track thickens under the pointer, and
// takes an outline under high contrast. Horizontal only (the replay
// scrubber). Sizes scale with the system font.
import QtQuick
import QtQuick.Controls.Basic
import RowPlay

Slider {
    id: control

    readonly property bool emphasised: hovered || pressed || visualFocus

    implicitHeight: Theme.px(22)
    padding: Theme.px(2)
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    background: Rectangle {
        x: control.leftPadding
        y: control.topPadding + Math.round((control.availableHeight - height) / 2)
        implicitWidth: Theme.px(200)
        implicitHeight: control.emphasised ? Theme.px(6) : Theme.px(4)
        width: control.availableWidth
        height: implicitHeight
        radius: height / 2
        color: Theme.segmentTrack
        border.width: Theme.highContrast ? Theme.hairline : 0
        border.color: Theme.controlBorder

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: height / 2
            color: control.enabled ? Theme.accentColor : Theme.textDisabled
        }
    }

    handle: Rectangle {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + Math.round((control.availableHeight - height) / 2)
        implicitWidth: Theme.px(16)
        implicitHeight: Theme.px(16)
        radius: width / 2
        color: Theme.switchKnob
        border.width: Theme.hairline
        border.color: control.pressed || Theme.highContrast ? Theme.controlBorder
                                                            : Theme.separator
        // The knob shows while the pointer is over the track, while it is
        // dragged and on keyboard focus; the fill carries the position
        // otherwise.
        opacity: control.emphasised ? 1 : 0

        Behavior on opacity {
            enabled: !Theme.reduceMotion
            NumberAnimation { duration: Theme.motionDuration }
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: parent.radius
        }
    }
}
