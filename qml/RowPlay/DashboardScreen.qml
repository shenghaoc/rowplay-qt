// SPDX-License-Identifier: GPL-3.0-or-later
// Dashboard — a port of Studio's DashboardView: the metric-tile grid, the
// personal-best cards and the two trend charts (distance by sport, recent
// pace). All values arrive pre-rendered from rowplay-viewmodel through the
// Library singleton; the charts load their series in bulk with
// XYSeries.replace(list<point>) — never point by point.
//
// Divergences (docs/source-map.md): the Studio "Challenge" tile is dropped
// (no web key, the web dashboard has no challenge tile); SF Symbols are
// replaced by the semantic colour on the value. HIG layout (ADR 0013): the
// tiles and PB cards flow in equal-width columns, and the charts sit in
// cards on the quiet shared ChartTheme.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtGraphs
import RowPlay

Pane {
    id: screen

    padding: Theme.spacingXxxLarge

    background: Rectangle {
        color: Theme.windowBackground
    }

    // Equal-width adaptive columns: as many ≥ minWidth columns as fit (at
    // least two), never more than there are items, then balanced across the
    // rows that takes — four tiles at the default window make one row of
    // four, and five cards make 3 + 2 rather than 4 + an orphan.
    function gridColumns(count, width, minWidth, gap) {
        if (count <= 0) {
            return 1
        }
        var fit = Math.min(count, Math.max(2, Math.floor((width + gap) / (minWidth + gap))))
        var rows = Math.ceil(count / fit)
        return Math.max(1, Math.ceil(count / rows))
    }

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: scroll.availableWidth
            spacing: Theme.spacingXxLarge

            Label {
                text: Tr.t("nav.dashboard")
                font: Theme.pageTitle
                color: Theme.textPrimary
                Accessible.name: text
            }

            // Metric tiles.
            GridLayout {
                Layout.fillWidth: true
                columns: screen.gridColumns(Library.tilesJson.length, scroll.availableWidth,
                                            170, columnSpacing)
                uniformCellWidths: true
                columnSpacing: Theme.spacingLarge
                rowSpacing: Theme.spacingLarge

                Repeater {
                    model: Library.tilesJson

                    MetricTile {
                        required property var modelData
                        Layout.fillWidth: true
                        label: Tr.t(modelData.labelId)
                        value: modelData.valueText
                        valueColor: Theme.metricColor(modelData.role)
                    }
                }
            }

            // Personal bests (Studio's adaptive card grid, same column rule).
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium
                visible: pbRepeater.count > 0

                Label {
                    text: Tr.t("dashboard.pbTitle")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: screen.gridColumns(Library.pbCardsJson.length,
                                                scroll.availableWidth, 150, columnSpacing)
                    uniformCellWidths: true
                    columnSpacing: Theme.spacingLarge
                    rowSpacing: Theme.spacingLarge

                    Repeater {
                        id: pbRepeater
                        model: Library.pbCardsJson

                        Pane {
                            required property var modelData
                            Layout.fillWidth: true
                            padding: Theme.spacingLarge
                            Accessible.name: modelData.label + " "
                                           + modelData.sportName + " PB"
                            Accessible.description: modelData.timeText + ", "
                                                    + modelData.paceText

                            background: Rectangle {
                                color: Theme.cardBackground
                                radius: Theme.radiusLarge
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: Theme.spacingXSmall

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacingSmall

                                    Label {
                                        text: modelData.label
                                        font: Theme.metricLabel
                                        color: Theme.textPrimary
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: modelData.sportName
                                        font: Theme.metricLabel
                                        color: Theme.textSecondary
                                        elide: Text.ElideRight
                                    }
                                }

                                Label {
                                    text: modelData.timeText
                                    // title3-ish semibold tabular value
                                    font: ({ pixelSize: 17,
                                             weight: Font.DemiBold,
                                             features: { "tnum": 1 } })
                                    color: Theme.metricDuration
                                }
                                Label {
                                    text: modelData.paceText
                                    font: Theme.metricLabel
                                    color: Theme.metricPace
                                }
                                Label {
                                    text: modelData.dateText
                                    font: Theme.metricLabel
                                    color: Theme.textSecondary
                                }
                            }
                        }
                    }
                }
            }

            // Distance by sport (Studio's bar chart panel).
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartHeight + 72
                color: Theme.panelBackground
                radius: Theme.radiusLarge
                Accessible.name: Tr.t("dashboard.bySport")

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingMedium

                    Label {
                        text: Tr.t("dashboard.bySport")
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.ignored: true
                    }

                    GraphsView {
                        id: sportChart
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        marginLeft: 0
                        marginRight: Theme.spacingXSmall
                        marginTop: Theme.spacingSmall
                        marginBottom: 0

                        theme: ChartTheme {}

                        // Four or five quiet gridlines: a nice step from the
                        // largest bar, the axis topped at a whole step (a
                        // display constant, not a formatted metric).
                        readonly property real barPeak: {
                            var max = 0
                            for (var i = 0; i < Library.sportBarValues.length; i++) {
                                max = Math.max(max, Library.sportBarValues[i])
                            }
                            return max
                        }
                        readonly property real tickStep: ChartUtils.niceInterval(barPeak, 4)
                        readonly property real barMax: barPeak > 0
                                                       ? Math.ceil(barPeak * 1.08 / tickStep) * tickStep
                                                       : 1

                        axisX: BarCategoryAxis {
                            categories: Library.sportBarLabels
                            gridVisible: false
                            subGridVisible: false
                        }
                        axisY: ValueAxis {
                            min: 0
                            max: sportChart.barMax
                            tickInterval: sportChart.tickStep
                            subTickCount: 0
                            lineVisible: false
                            // No Y axis line or tick marks (HIG-quiet; the ticks also ran
                            // into the wider Rust labels): gridlines carry the scale.
                            color: "transparent"
                            subGridVisible: false
                            labelDecimals: 0
                            titleText: Library.distanceAxis
                            titleFont.pixelSize: 10
                            titleColor: Theme.textSecondary
                            titleVisible: true
                        }

                        BarSeries {
                            id: sportBars
                            barWidth: 0.4
                            labelsVisible: false
                            // Rounded top corners (the series sizes each
                            // delegate; the colour comes from the BarSet).
                            barDelegate: Component {
                                Rectangle {
                                    property color barColor
                                    property color barBorderColor
                                    property real barBorderWidth
                                    property real barValue
                                    property string barLabel
                                    property bool barSelected
                                    property int barIndex
                                    color: barColor
                                    topLeftRadius: 4
                                    topRightRadius: 4
                                }
                            }
                            BarSet {
                                id: sportBarSet
                                color: Theme.metricDistance
                            }
                        }
                    }
                }
            }

            // Recent pace (Studio's line chart panel; negated pace axis so
            // faster is up, ticks pre-rendered in Rust).
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartHeight + 90
                color: Theme.panelBackground
                radius: Theme.radiusLarge
                Accessible.name: Tr.t("dashboard.trendTitle")

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingXSmall

                    Label {
                        text: Tr.t("dashboard.trendTitle")
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.ignored: true
                    }
                    Label {
                        Layout.bottomMargin: Theme.spacingSmall
                        text: Tr.t("dashboard.likeForLike",
                                   { sport: Library.paceSportName })
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.ignored: true
                    }

                    FontMetrics {
                        id: paceTickMetrics
                        font.pixelSize: 10
                        font.features: { "tnum": 1 }
                    }

                    GraphsView {
                        id: paceChart
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        // Room for the Rust pace labels beyond Qt Graphs'
                        // fixed 40 px label column (ChartUtils.yLabelOverflow).
                        marginLeft: ChartUtils.yLabelOverflow(Library.paceAxisLabels,
                                                              paceTickMetrics)
                        marginRight: Theme.spacingMedium
                        marginTop: Theme.spacingSmall
                        marginBottom: 0

                        theme: ChartTheme {}

                        axisX: ValueAxis {
                            min: -0.5
                            max: Math.max(0.5, Library.paceDateTexts.length - 0.5)
                            tickInterval: 1
                            subTickCount: 0
                            gridVisible: false
                            subGridVisible: false
                            // Chronological index axis; the locale date per
                            // point comes pre-rendered from Rust.
                            labelDelegate: Item {
                                property string text
                                implicitWidth: dateLabel.implicitWidth
                                implicitHeight: dateLabel.implicitHeight
                                Text {
                                    id: dateLabel
                                    text: {
                                        var idx = Math.round(parseFloat(parent.text))
                                        if (isNaN(idx)) {
                                            return ""
                                        }
                                        var dates = Library.paceDateTexts
                                        return idx >= 0 && idx < dates.length
                                               ? dates[idx] : ""
                                    }
                                    font.pixelSize: 9
                                    color: Theme.textSecondary
                                }
                            }
                        }
                        axisY: ValueAxis {
                            id: paceAxis
                            min: Library.paceDomainLow
                            max: Library.paceDomainHigh
                            tickAnchor: Library.paceDomainLow
                            tickInterval: ChartUtils.spanInterval(Library.paceDomainLow,
                                                                  Library.paceDomainHigh,
                                                                  Library.paceAxisValues.length)
                            subTickCount: 0
                            lineVisible: false
                            // No Y axis line or tick marks (HIG-quiet; the ticks also ran
                            // into the wider Rust labels): gridlines carry the scale.
                            color: "transparent"
                            subGridVisible: false
                            // No rotated axis title: every tick already reads
                            // "m:ss.t/500m", and Qt Graphs sizes the label
                            // column for the raw tick numbers, so the title
                            // ran into the wider pace labels.
                            titleVisible: false
                            // Pace ticks display the Rust-formatted labels.
                            labelDelegate: Item {
                                property string text
                                implicitWidth: paceLabel.implicitWidth
                                implicitHeight: paceLabel.implicitHeight
                                Text {
                                    id: paceLabel
                                    anchors.right: parent.right
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: ChartUtils.nearestLabel(
                                              Library.paceAxisValues,
                                              Library.paceAxisLabels,
                                              parent.text)
                                    font.pixelSize: 10
                                    font.features: { "tnum": 1 }
                                    color: Theme.textSecondary
                                }
                            }
                        }

                        LineSeries {
                            id: paceLine
                            color: Theme.metricPace
                            width: 2
                        }
                    }
                }
            }
        }
    }

    // Bulk series loading: one replace() per change (ground rule).
    function reloadSeries() {
        sportBarSet.values = Library.sportBarValues
        paceLine.replace(ChartUtils.points(Library.paceSeries))
    }

    Component.onCompleted: reloadSeries()

    Connections {
        target: Library
        function onLibraryChanged() {
            screen.reloadSeries()
        }
    }
}
