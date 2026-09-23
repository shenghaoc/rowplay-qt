// SPDX-License-Identifier: GPL-3.0-or-later
// Progress bar (ADR 0013): a rounded track in the segment-track tone with the
// accent fill; indeterminate progress shows a sliding segment, held still
// (centred) under the reduce-motion preference.
import QtQuick
import QtQuick.Controls
import RowPlay

ProgressBar {
    id: control

    implicitHeight: Theme.px(6)

    contentItem: Item {
        implicitWidth: Theme.px(200)
        implicitHeight: Theme.px(6)
        clip: true

        Rectangle {
            visible: !control.indeterminate
            width: control.visualPosition * parent.width
            height: parent.height
            radius: height / 2
            color: Theme.accentColor
        }

        Rectangle {
            id: segment
            visible: control.indeterminate
            width: parent.width * 0.3
            height: parent.height
            radius: height / 2
            color: Theme.accentColor
            x: (parent.width - width) / 2

            SequentialAnimation on x {
                running: control.indeterminate && control.visible && !Theme.reduceMotion
                loops: Animation.Infinite
                NumberAnimation {
                    from: -segment.width
                    to: segment.parent.width
                    duration: 1200
                    easing.type: Easing.InOutQuad
                }
            }
        }
    }

    background: Rectangle {
        implicitWidth: Theme.px(200)
        implicitHeight: Theme.px(6)
        radius: height / 2
        color: Theme.segmentTrack
        border.width: Theme.highContrast ? Theme.hairline : 0
        border.color: Theme.controlBorder
    }
}
