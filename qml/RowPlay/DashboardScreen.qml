// SPDX-License-Identifier: GPL-3.0-or-later
// Dashboard — a port of Studio's DashboardView: the metric-tile grid, the
// personal-best cards and the two trend charts (distance by sport, recent
// pace). All values arrive pre-rendered from rowplay-viewmodel through the
// Library singleton; the charts load their series in bulk with
// XYSeries.replace(list<point>) — never point by point.
//
// Divergences (docs/source-map.md): the Studio "Challenge" tile is dropped
// (no web key, the web dashboard has no challenge tile); SF Symbols are
// replaced by the semantic colour on the value.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtGraphs
import RowPlay

Pane {
    id: screen

    padding: Theme.spacingXxxLarge

    ScrollView {
        anchors.fill: parent
        clip: true

        ColumnLayout {
            width: parent.width
            spacing: Theme.spacingXxxLarge

            Label {
                text: Tr.t("nav.dashboard")
                font: Theme.pageTitle
                color: Theme.textPrimary
                Accessible.name: text
            }

            // Metric tiles (adaptive 180–240 grid like Studio).
            GridLayout {
                Layout.fillWidth: true
                columns: Math.max(2, Math.floor(screen.width / 220))
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

            // Personal bests (Studio's adaptive card grid).
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingLarge
                visible: pbGrid.count > 0

                Label {
                    text: Tr.t("dashboard.pbTitle")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                GridLayout {
                    id: pbGrid
                    Layout.fillWidth: true
                    columns: Math.max(2, Math.floor(screen.width / 220))
                    columnSpacing: Theme.spacingMedium
                    rowSpacing: Theme.spacingMedium

                    Repeater {
                        model: Library.pbCardsJson

                        Pane {
                            required property var modelData
                            Layout.fillWidth: true
                            padding: Theme.spacingMedium
                            Accessible.name: modelData.label + " "
                                           + modelData.sportName + " PB"
                            Accessible.description: modelData.timeText + ", "
                                                    + modelData.paceText

                            background: Rectangle {
                                color: Theme.cardBackground
                                radius: Theme.radiusSmall
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: Theme.spacingXSmall

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacingSmall

                                    Label {
                                        text: modelData.label
                                        font: Theme.compactLabel
                                        color: Theme.textPrimary
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: modelData.sportName
                                        font: Theme.compactLabel
                                        color: Theme.textSecondary
                                        elide: Text.ElideRight
                                    }
                                }

                                Label {
                                    text: modelData.timeText
                                    // title3-ish semibold tabular value
                                    font: ({ pixelSize: 16,
                                             weight: Font.DemiBold,
                                             features: Font.TabularNumbers })
                                    color: Theme.metricDuration
                                }
                                Label {
                                    text: modelData.paceText
                                    font: Theme.compactLabel
                                    color: Theme.metricPace
                                }
                                Label {
                                    text: modelData.dateText
                                    font: Theme.compactLabel
                                    color: Theme.textTertiary
                                }
                            }
                        }
                    }
                }
            }

            // Distance by sport (Studio's bar chart panel).
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: chartColumnHeight
                color: Theme.panelBackground
                radius: Theme.radiusMedium
                Accessible.name: Tr.t("dashboard.bySport")

                property real chartColumnHeight: Theme.chartHeight + 80

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingLarge

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

                        theme: GraphsTheme {
                            colorScheme: Theme.dark ? GraphsTheme.ColorScheme.Dark
                                                    : GraphsTheme.ColorScheme.Light
                            backgroundVisible: false
                            plotAreaBackgroundVisible: false
                            labelTextColor: Theme.textSecondary
                            labelFont.pixelSize: 10
                        }

                        axisX: BarCategoryAxis {
                            categories: Library.sportBarLabels
                            gridVisible: false
                            subGridVisible: false
                        }
                        axisY: ValueAxis {
                            min: 0
                            max: sportChart.barMax
                            titleText: Library.distanceAxis
                            titleFont.pixelSize: 10
                            titleVisible: true
                            labelDecimals: 0
                        }

                        // Headroom so labels/bars never clip (a display
                        // constant, not a formatted metric).
                        readonly property real barMax: {
                            var max = 0
                            for (var i = 0; i < Library.sportBarValues.length; i++) {
                                max = Math.max(max, Library.sportBarValues[i])
                            }
                            return max > 0 ? max * 1.15 : 1
                        }

                        BarSeries {
                            id: sportBars
                            barWidth: 0.5
                            labelsVisible: false
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
                Layout.preferredHeight: Theme.chartHeight + 80
                color: Theme.panelBackground
                radius: Theme.radiusMedium
                Accessible.name: Tr.t("dashboard.trendTitle")

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingLarge

                    Label {
                        text: Tr.t("dashboard.trendTitle")
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.ignored: true
                    }
                    Label {
                        text: Tr.t("dashboard.likeForLike",
                                   { sport: Library.paceSportName })
                        font: Theme.metricLabel
                        color: Theme.textSecondary
                        Accessible.ignored: true
                    }

                    GraphsView {
                        id: paceChart
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        theme: GraphsTheme {
                            colorScheme: Theme.dark ? GraphsTheme.ColorScheme.Dark
                                                    : GraphsTheme.ColorScheme.Light
                            backgroundVisible: false
                            plotAreaBackgroundVisible: false
                            labelTextColor: Theme.textSecondary
                            labelFont.pixelSize: 10
                        }

                        axisX: ValueAxis {
                            min: -0.5
                            max: Math.max(0.5, Library.paceDateTexts.length - 0.5)
                            tickInterval: 1
                            subTickCount: 0
                            gridVisible: false
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
                                    color: Theme.textTertiary
                                }
                            }
                        }
                        axisY: ValueAxis {
                            id: paceAxis
                            min: Library.paceDomainLow
                            max: Library.paceDomainHigh
                            tickAnchor: Library.paceDomainLow
                            tickInterval: Library.paceAxisValues.length > 1
                                          ? (Library.paceDomainHigh
                                             - Library.paceDomainLow)
                                            / (Library.paceAxisValues.length - 1)
                                          : 0
                            subTickCount: 0
                            titleText: Tr.t("replay.pacePer500m")
                            titleFont.pixelSize: 10
                            titleVisible: true
                            // Pace ticks display the Rust-formatted labels.
                            labelDelegate: Item {
                                property string text
                                implicitWidth: paceLabel.implicitWidth
                                implicitHeight: paceLabel.implicitHeight
                                Text {
                                    id: paceLabel
                                    text: ChartUtils.nearestLabel(
                                              Library.paceAxisValues,
                                              Library.paceAxisLabels,
                                              parent.text)
                                    font.pixelSize: 10
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
