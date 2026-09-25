// SPDX-License-Identifier: GPL-3.0-or-later
// Switch (ADR 0013): a pill in the accent when on, the neutral track with an
// outline when off, a white knob, and no text of its own — in a grouped form
// the label sits at the row's leading edge and a click anywhere on the row
// toggles the switch (FormRow), so the caller sets `Accessible.name` to that
// label. Sizes scale with the system font.
import QtQuick
import QtQuick.Controls
import RowPlay

Switch {
    id: control

    /// Knob slide; off under the reduce-motion preference.
    property bool animated: !Theme.reduceMotion

    text: ""
    padding: 0
    spacing: 0
    implicitWidth: indicator.implicitWidth
    implicitHeight: Math.max(indicator.implicitHeight, Theme.px(22))
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    indicator: Rectangle {
        readonly property int inset: Theme.px(2)

        implicitWidth: Theme.px(36)
        implicitHeight: Theme.px(20)
        x: control.leftPadding
        y: Math.round((control.height - height) / 2)
        radius: height / 2
        color: !control.enabled ? Theme.segmentTrack
             : control.checked ? Theme.accentColor : Theme.switchTrackOff
        // The off track's outline carries the control's shape (≥ 3:1); under
        // high contrast the on track keeps it too, since the system highlight
        // can sit close to the window colour.
        border.width: control.checked && control.enabled && !Theme.highContrast
                      ? 0 : Theme.hairline
        border.color: control.enabled ? Theme.controlBorder : Theme.separator

        Rectangle {
            width: parent.height - 2 * parent.inset
            height: width
            radius: width / 2
            y: parent.inset
            x: control.checked ? parent.width - width - parent.inset : parent.inset
            color: !control.enabled ? Theme.groupBackground
                 : control.checked ? Theme.switchKnobOn : Theme.switchKnob
            border.width: Theme.hairline
            border.color: control.down ? Theme.controlBorder : Theme.separator

            Behavior on x {
                enabled: control.animated
                NumberAnimation { duration: Theme.motionDuration; easing.type: Easing.OutCubic }
            }
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: parent.radius
        }
    }

    contentItem: Item {}
}
