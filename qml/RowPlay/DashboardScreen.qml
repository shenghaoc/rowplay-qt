// SPDX-License-Identifier: GPL-3.0-or-later
// Dashboard — a port of Studio's DashboardView: the metric-tile grid, the
// personal-best cards and the two trend charts (distance by sport, recent
// pace). All values arrive pre-rendered from rowplay-viewmodel through the
// Library singleton; the charts load their series in bulk with
// XYSeries.replace(list<point>) — never point by point.
//
// Design system (ADR 0013): tonal cards on the window, balanced equal-width
// grids, charts on the shared ChartTheme, every size from the Theme scale.
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

    // The page's margin is inside the scroll view (the column's x and y), so
    // the style's scroll bar sits at the pane's edge and the content keeps
    // its distance from it.
    padding: 0
    readonly property real pageMargin: Theme.spacingXxxLarge

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        // The column spans the view: sized by parent.width it followed the
        // Flickable content item, i.e. its own implicit width, and stopped a
        // quarter to a third short of the pane.
        contentWidth: availableWidth
        contentHeight: content.implicitHeight + 2 * screen.pageMargin
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

        ColumnLayout {
            id: content
            x: screen.pageMargin
            y: screen.pageMargin
            width: scroll.availableWidth - 2 * screen.pageMargin
            spacing: Theme.spacingXxLarge

            Label {
                text: Tr.t("nav.dashboard")
                font: Theme.pageTitle
                color: Theme.textPrimary
                Accessible.name: text
            }

            // Metric tiles (Studio's adaptive grid): equal-width columns,
            // balanced so no tile is left alone on a row.
            GridLayout {
                Layout.fillWidth: true
                columns: Theme.balancedColumns(Library.tilesJson.length, content.width,
                                               Theme.px(180), columnSpacing)
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

            // Personal bests (Studio's adaptive card grid).
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium
                // The Repeater has the count; a GridLayout has none.
                visible: pbRepeater.count > 0

                Label {
                    text: Tr.t("dashboard.pbTitle")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: Theme.balancedColumns(Library.pbCardsJson.length, content.width,
                                                   Theme.px(150), columnSpacing)
                    uniformCellWidths: true
                    columnSpacing: Theme.spacingMedium
                    rowSpacing: Theme.spacingMedium

                    Repeater {
                        id: pbRepeater
                        model: Library.pbCardsJson

                        Pane {
                            required property var modelData
                            Layout.fillWidth: true
                            padding: Theme.spacingLarge
                            Accessible.name: modelData.label + " "
                                             + modelData.sportName + " "
                                             + Tr.t("dashboard.pbTag")
                            Accessible.description: modelData.timeText + ", "
                                                    + modelData.paceText + ", "
                                                    + modelData.dateText

                            background: Rectangle {
                                color: Theme.cardBackground
                                radius: Theme.radiusMedium
                                border.width: Theme.cardBorderWidth
                                border.color: Theme.separator
                            }

                            ColumnLayout {
                                anchors.fill: parent
                                spacing: Theme.spacingXSmall

                                RowLayout {
                                    Layout.fillWidth: true
                                    spacing: Theme.spacingSmall

                                    Label {
                                        text: modelData.label
                                        font: Theme.bodyEmphasized
                                        color: Theme.textPrimary
                                        Accessible.ignored: true
                                    }
                                    Label {
                                        Layout.fillWidth: true
                                        text: modelData.sportName
                                        font: Theme.subheadline
                                        color: Theme.textSecondary
                                        elide: Text.ElideRight
                                        Accessible.ignored: true
                                    }
                                }

                                Label {
                                    Layout.fillWidth: true
                                    text: modelData.timeText
                                    font: Theme.cardMetric
                                    color: Theme.metricDuration
                                    elide: Text.ElideRight
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: modelData.paceText
                                    font: Theme.metricLabel
                                    color: Theme.metricPace
                                    elide: Text.ElideRight
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    text: modelData.dateText
                                    font: Theme.metricLabel
                                    color: Theme.textSecondary
                                    elide: Text.ElideRight
                                    Accessible.ignored: true
                                }
                            }
                        }
                    }
                }
            }

            // Distance by sport (Studio's bar chart panel).
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartHeight + sportTitle.implicitHeight
                                        + 3 * Theme.spacingXLarge
                color: Theme.panelBackground
                radius: Theme.radiusLarge
                border.width: Theme.cardBorderWidth
                border.color: Theme.separator
                Accessible.name: Tr.t("dashboard.bySport")

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingLarge

                    Label {
                        id: sportTitle
                        text: Tr.t("dashboard.bySport")
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.ignored: true
                    }

                    GraphsView {
                        id: sportChart
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        theme: ChartTheme {}

                        axisX: BarCategoryAxis {
                            categories: Library.sportBarLabels
                            gridVisible: false
                            subGridVisible: false
                            labelsVisible: true
                        }
                        axisY: ValueAxis {
                            min: 0
                            max: sportChart.barMax
                            titleText: Library.distanceAxis
                            titleFont.pixelSize: Theme.chartLabel.pixelSize
                            titleVisible: true
                            labelDecimals: 0
                            subTickCount: 0
                            lineVisible: false
                            tickInterval: ChartUtils.niceInterval(sportChart.barMax, 4)
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
                                borderWidth: 0
                            }
                        }
                    }
                }
            }

            // Recent pace (Studio's line chart panel; negated pace axis so
            // faster is up, ticks pre-rendered in Rust).
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartHeight + trendTitle.implicitHeight
                                        + trendSubtitle.implicitHeight
                                        + 4 * Theme.spacingXLarge
                color: Theme.panelBackground
                radius: Theme.radiusLarge
                border.width: Theme.cardBorderWidth
                border.color: Theme.separator
                Accessible.name: Tr.t("dashboard.trendTitle")

                ColumnLayout {
                    anchors.fill: parent
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingSmall

                    Label {
                        id: trendTitle
                        text: Tr.t("dashboard.trendTitle")
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.ignored: true
                    }
                    Label {
                        id: trendSubtitle
                        Layout.bottomMargin: Theme.spacingMedium
                        text: Tr.t("dashboard.likeForLike",
                                   { sport: Library.paceSportName })
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.ignored: true
                    }

                    // Qt Graphs gives Y labels a fixed 40 px column; the Rust
                    // pace strings are wider, so the chart reserves the
                    // overflow on the left (ChartUtils.yLabelOverflow).
                    FontMetrics {
                        id: paceTickMetrics
                        font: Theme.chartLabel
                    }

                    GraphsView {
                        id: paceChart
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        marginLeft: ChartUtils.yLabelOverflow(Library.paceAxisLabels,
                                                              paceTickMetrics)
                        theme: ChartTheme {}

                        // Every point had a date label, and they ran into
                        // each other once dates were long or the text large
                        // (Chinese and Japanese dates at the default size
                        // already overlapped). Label every step-th point,
                        // anchored on the newest, so the labels keep a gap,
                        // and keep half a label of room on the right for the
                        // newest one, which is centred on the last point.
                        readonly property real widestDate: ChartUtils.widestLabel(
                            Library.paceDateTexts, paceTickMetrics)
                        readonly property int dateLabelStep: ChartUtils.labelStep(
                            Library.paceDateTexts.length, widestDate, plotArea.width,
                            Theme.spacingMedium)
                        marginRight: Math.max(20, Math.ceil(widestDate / 2))

                        axisX: ValueAxis {
                            min: -0.5
                            max: Math.max(0.5, Library.paceDateTexts.length - 0.5)
                            tickAnchor: Math.max(0, Library.paceDateTexts.length - 1)
                            tickInterval: paceChart.dateLabelStep
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
                                    font: Theme.chartLabel
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
                            // No axis line or tick marks under the long
                            // labels, and no rotated title: it was drawn on
                            // top of the labels, and every tick already
                            // reads "/500m".
                            color: "transparent"
                            titleVisible: false
                            // Pace ticks display the Rust-formatted labels,
                            // right-anchored on the tick (the delegate is
                            // sized to the fixed 40 px column).
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
                                    font: Theme.chartLabel
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
