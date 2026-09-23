// SPDX-License-Identifier: GPL-3.0-or-later
// One stroke-chart surface: a line series over distance with optional split
// boundary rules and an average rule (Studio's splitBoundaryMarks /
// RuleMark). Series load through a single bulk replace(); boundary and rule
// lines are two-point LineSeries (the verified Qt 6.11 RuleMark equivalent).
//
// A chart whose values are not plain numbers — the negated pace axis — passes
// Rust-formatted ticks (`axisValues` / `axisLabels`) exactly like the
// dashboard's pace chart, so it never prints the raw negated seconds.
import QtQuick
import QtQuick.Controls
import QtQml.Models
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
    /// Optional Rust-formatted Y ticks: axis values spanning yMin…yMax and
    /// their labels (Detail.paceAxisValues / paceAxisLabels).
    property var axisValues: []
    property var axisLabels: []
    /// Left inset shared by stacked charts so their plot origins line up
    /// (the widest label overflow in the panel).
    property real minimumLabelOverflow: 0

    readonly property bool customTicks: axisValues !== undefined && axisValues !== null
                                        && axisValues.length > 1
    // Room the Rust labels need beyond Qt Graphs' fixed 40 px label column.
    readonly property real labelOverflow: customTicks
                                          ? ChartUtils.yLabelOverflow(axisLabels, tickMetrics)
                                          : 0

    FontMetrics {
        id: tickMetrics
        font.pixelSize: 9
    }

    // No axis line or tick marks under wide Rust labels; other axes keep
    // the theme's default colour (the Binding restores it when inactive).
    Binding {
        target: yAxis
        property: "color"
        value: "transparent"
        when: chart.customTicks
    }

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
            marginLeft: Math.max(chart.labelOverflow, chart.minimumLabelOverflow)

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
                id: yAxis
                min: chart.yMin
                max: chart.yMax
                subTickCount: 0
                labelDecimals: 0
                // About four ticks: the automatic interval packed eight to
                // ten overlapping labels into the short plot. Rust-labelled
                // axes put one tick on each exported value instead.
                tickAnchor: chart.customTicks ? chart.yMin : 0
                tickInterval: chart.customTicks
                              ? ChartUtils.spanInterval(chart.yMin, chart.yMax,
                                                        chart.axisValues.length)
                              : ChartUtils.niceInterval(chart.yMax - chart.yMin, 4)
                // Qt Graphs sizes this item to its 40 px column (height 0 at
                // the tick): right-align and centre the text on the tick.
                labelDelegate: Item {
                    property string text
                    implicitWidth: tickLabel.implicitWidth
                    implicitHeight: tickLabel.implicitHeight
                    Text {
                        id: tickLabel
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        text: chart.customTicks
                              ? ChartUtils.nearestLabel(chart.axisValues,
                                                        chart.axisLabels, parent.text)
                              : parent.text
                        font.pixelSize: 9
                        color: Theme.textTertiary
                    }
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

    // Split boundaries (Studio: secondary colour, dashed). A Repeater cannot
    // create them — a LineSeries is not an Item, so a Repeater instantiates
    // nothing and the boundaries never drew — so an Instantiator builds one
    // series per boundary and hands it to the view, behind the data.
    Instantiator {
        id: boundarySeries
        model: chart.boundaryLines

        delegate: LineSeries {
            required property var modelData
            color: Qt.alpha(Theme.textSecondary, 0.35)
            width: 1
            strokeStyle: LineSeries.StrokeStyle.DashLine
            Component.onCompleted: replace(
                ChartUtils.vrule(modelData, chart.yMin, chart.yMax))
        }

        onObjectAdded: function(index, object) {
            view.insertSeries(0, object)
        }
        onObjectRemoved: function(index, object) {
            view.removeSeries(object)
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
        // A new model re-creates the series (each loads its two points in
        // Component.onCompleted); a new Y range re-spans the existing ones.
        for (var i = 0; i < boundarySeries.count; ++i) {
            var series = boundarySeries.objectAt(i)
            if (series) {
                series.replace(ChartUtils.vrule(series.modelData, yMin, yMax))
            }
        }
    }
}
