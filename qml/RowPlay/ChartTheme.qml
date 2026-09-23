// SPDX-License-Identifier: GPL-3.0-or-later
// The shared Qt Graphs theme (ADR 0013): quiet axes and gridlines so the data
// stands out — transparent backgrounds, a 1 px main grid in Theme.chartGrid
// with no sub-grid, a hairline axis in Theme.chartAxis, secondary-colour
// labels at the chart label size (scaled with the system font; stronger
// lines under high contrast). Axis-level switches (the hidden Y axis line,
// no X gridlines, tick spacing) stay on each chart's ValueAxis.
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
    labelFont.pixelSize: Theme.chartLabel.pixelSize
    axisXLabelFont.pixelSize: Theme.chartLabel.pixelSize
    axisYLabelFont.pixelSize: Theme.chartLabel.pixelSize

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
