// SPDX-License-Identifier: GPL-3.0-or-later
// Busy indicator (ADR 0013): a three-quarter ring in the secondary text
// colour that turns while running — held still under the reduce-motion
// preference (the ring alone still says "working"). ADR 0015 keeps it as a
// bounded exception: the macOS style's spinner is a WebP animation, and no
// Qt install of the project carries the WebP plugin yet.
import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Shapes
import RowPlay

BusyIndicator {
    id: control

    implicitWidth: Theme.px(18)
    implicitHeight: Theme.px(18)
    padding: 0

    contentItem: Item {
        id: spinner
        opacity: control.running ? 1 : 0

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                fillColor: "transparent"
                strokeColor: Theme.textSecondary
                strokeWidth: Math.max(1.5, Theme.px(2))
                capStyle: ShapePath.RoundCap

                PathAngleArc {
                    centerX: spinner.width / 2
                    centerY: spinner.height / 2
                    radiusX: spinner.width / 2 - Theme.px(2)
                    radiusY: spinner.height / 2 - Theme.px(2)
                    startAngle: -90
                    sweepAngle: 270
                }
            }
        }

        // A turn's period, not a transition, so no motion token: under
        // reduce motion the ring holds still instead.
        RotationAnimator on rotation {
            running: control.running && control.visible && !Theme.reduceMotion
            from: 0
            to: 360
            duration: 900
            loops: Animation.Infinite
        }
    }
}
