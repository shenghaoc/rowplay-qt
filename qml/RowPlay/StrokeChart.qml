// SPDX-License-Identifier: GPL-3.0-or-later
// One stroke-chart surface: a line series over distance with optional split
// boundary rules and an average rule (Studio's splitBoundaryMarks /
// RuleMark). Series load through a single bulk replace(); boundary and rule
// lines are two-point LineSeries (the verified Qt 6.11 RuleMark equivalent).
import QtQuick
import QtQuick.Controls
import QtGraphs
import RowPlay

Rectangle {
    id: chart

    /// Translated panel title (drawn above the plot).
    property string titleText: ""
    property real yMin: 0
    property real yMax: 1
    property color seriesColor: Theme.metricPace
    /// Flat [x0,y0,…] series from the bridge.
    property var flatSeries: []
    /// Horizontal rule value; NaN hides the rule.
    property real ruleY: NaN
    /// Vertical split-boundary positions (chart units).
    property var boundaryLines: []
    property string accessibleText: titleText

    color: "transparent"
    Accessible.name: accessibleText

    readonly property real xMax: {
        var max = 0
        for (var i = 0; i < flatSeries.length; i += 2) {
            max = Math.max(max, flatSeries[i])
        }
        for (var b = 0; b < boundaryLines.length; b++) {
            max = Math.max(max, boundaryLines[b])
        }
        return max > 0 ? max * 1.02 : 1
    }

    Column {
        anchors.fill: parent
        spacing: Theme.spacingXxSmall

        Label {
            text: chart.titleText
            font: Theme.metricLabel
            color: Theme.textSecondary
            Accessible.ignored: true
        }

        GraphsView {
            id: view
            width: parent.width
            height: chart.titleText.length > 0 ? parent.height - 18
                                               : parent.height

            theme: GraphsTheme {
                colorScheme: Theme.dark ? GraphsTheme.ColorScheme.Dark
                                        : GraphsTheme.ColorScheme.Light
                backgroundVisible: false
                plotAreaBackgroundVisible: false
                gridVisible: true
                labelTextColor: Theme.textTertiary
                labelFont.pixelSize: 9
            }

            axisX: ValueAxis {
                min: 0
                max: chart.xMax
                labelsVisible: false
                gridVisible: false
                subGridVisible: false
            }
            axisY: ValueAxis {
                min: chart.yMin
                max: chart.yMax
                subTickCount: 0
                labelDecimals: 0
                // About four ticks: the automatic interval packed eight to
                // ten overlapping labels into the short plot.
                tickInterval: ChartUtils.niceInterval(chart.yMax - chart.yMin, 4)
            }

            // Split boundaries (Studio: secondary colour, dashed).
            Repeater {
                model: chart.boundaryLines

                LineSeries {
                    required property var modelData
                    color: Qt.alpha(Theme.textSecondary, 0.35)
                    width: 1
                    strokeStyle: LineSeries.StrokeStyle.DashLine
                    Component.onCompleted: replace(
                        ChartUtils.vrule(modelData, chart.yMin, chart.yMax))
                }
            }

            // Average rule (Studio: 55% opacity, dashed [5,4]).
            LineSeries {
                id: ruleLine
                visible: isFinite(chart.ruleY)
                color: Qt.alpha(chart.seriesColor, 0.55)
                width: 1
                strokeStyle: LineSeries.StrokeStyle.DashLine
                dashPattern: [5, 4]
            }

            // The data series.
            LineSeries {
                id: dataLine
                color: chart.seriesColor
                width: 2
            }
        }
    }

    // One bulk replace per data change — never point by point.
    onFlatSeriesChanged: reload()
    onYMinChanged: reloadDecorations()
    onYMaxChanged: reloadDecorations()
    onXMaxChanged: reloadDecorations()
    onRuleYChanged: reloadDecorations()
    onBoundaryLinesChanged: reloadBoundaries()
    Component.onCompleted: {
        reload()
        reloadDecorations()
    }

    function reload() {
        dataLine.replace(ChartUtils.points(flatSeries))
        reloadDecorations()
    }

    function reloadDecorations() {
        if (isFinite(ruleY)) {
            ruleLine.replace(ChartUtils.rule(0, xMax, ruleY))
        }
        reloadBoundaries()
    }

    function reloadBoundaries() {
        // The Repeater re-instantiates on model change; each boundary line
        // loads its two points in Component.onCompleted.
    }
}
