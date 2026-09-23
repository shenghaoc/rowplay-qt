// SPDX-License-Identifier: GPL-3.0-or-later
// The shared Qt Graphs theme (HIG Charts: "prefer quiet axes and gridlines so
// the data stands out"): transparent backgrounds, 1 px main grid in
// Theme.chartGrid with no sub-grid, a hairline X axis in Theme.chartAxis,
// 10 px secondary labels. Axis-level switches (the hidden Y axis line, no
// X gridlines, tick spacing) stay on each chart's ValueAxis.
import QtQuick
import QtGraphs
import RowPlay

GraphsTheme {
    colorScheme: Theme.dark ? GraphsTheme.ColorScheme.Dark
                            : GraphsTheme.ColorScheme.Light
    backgroundVisible: false
    plotAreaBackgroundVisible: false
    gridVisible: true
    labelBackgroundVisible: false
    labelBorderVisible: false
    labelTextColor: Theme.textSecondary
    labelFont.pixelSize: 10
    axisXLabelFont.pixelSize: 10
    axisYLabelFont.pixelSize: 10

    grid.mainColor: Theme.chartGrid
    grid.subColor: "transparent"
    grid.mainWidth: 1
    grid.subWidth: 1

    axisX.mainColor: Theme.chartAxis
    axisX.subColor: "transparent"
    axisX.mainWidth: 1
    axisX.subWidth: 1
    axisX.labelTextColor: Theme.textSecondary

    axisY.mainColor: Theme.chartAxis
    axisY.subColor: "transparent"
    axisY.mainWidth: 1
    axisY.subWidth: 1
    axisY.labelTextColor: Theme.textSecondary
}
