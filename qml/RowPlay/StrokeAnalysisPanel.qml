// SPDX-License-Identifier: GPL-3.0-or-later
// Stroke analysis — a port of Studio's WorkoutStrokeAnalysisView: pace and
// power over distance with split boundaries and average rule lines, plus the
// stroke-rate and heart-rate series the Phase 4 brief adds. Missing strokes
// show Studio's empty state; synthesised (split-derived) strokes chart like
// recorded ones. All series arrive downsampled to ≤500 points and load with
// one replace() each. The charts sit in one tonal card (ADR 0013).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

ColumnLayout {
    id: panel

    spacing: Theme.spacingMedium

    // Empty state (Studio: "No Stroke Detail").
    Rectangle {
        Layout.fillWidth: true
        visible: !Detail.hasStrokes
        implicitHeight: emptyColumn.implicitHeight + 2 * Theme.spacingXLarge
        radius: Theme.radiusLarge
        color: Theme.panelBackground
        border.width: Theme.cardBorderWidth
        border.color: Theme.separator

        ColumnLayout {
            id: emptyColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            anchors.margins: Theme.spacingXLarge
            spacing: Theme.spacingXSmall

            Label {
                Layout.fillWidth: true
                text: Tr.t("workoutList.strokeNo")
                font: Theme.sectionHeadline
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
            Label {
                Layout.fillWidth: true
                text: Tr.t("inspector.noStrokeData")
                font: Theme.body
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
                Accessible.name: text
            }
        }
    }

    Rectangle {
        Layout.fillWidth: true
        visible: Detail.hasStrokes
        implicitHeight: chartsColumn.implicitHeight + 2 * Theme.spacingXLarge
        radius: Theme.radiusLarge
        color: Theme.panelBackground
        border.width: Theme.cardBorderWidth
        border.color: Theme.separator

        ColumnLayout {
            id: chartsColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: Theme.spacingXLarge
            spacing: Theme.spacingLarge

            // Header + summary numbers (labels via locale ids, values from
            // Rust). The web's per-stroke inspector section title; the
            // splits table below keeps the splits heading.
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingXSmall

                Label {
                    text: Tr.t("inspector.sectionPerStroke")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                Flow {
                    Layout.fillWidth: true
                    spacing: Theme.spacingXLarge

                    Label {
                        text: Tr.t("replay.mStrokeCount") + ": " + Detail.strokeCount
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    Label {
                        text: Tr.t("replay.mAvgPace") + ": " + Detail.averagePaceText
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    Label {
                        text: Tr.t("replay.mAvgPower") + ": "
                              + Detail.averageWattsText + " W"
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                }
            }

            // Pace (negated, fast-up) with the average-pace rule and split
            // boundaries — Studio's paceChart.
            StrokeChart {
                id: paceChart
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartStrokeHeight
                titleText: Tr.t("replay.cPace")
                yMin: Detail.paceDomainLow
                yMax: Detail.paceDomainHigh
                // Pace ticks formatted in Rust (the plotted values are negated
                // seconds per 500 m, faster up).
                axisValues: Detail.paceAxisValues
                axisLabels: Detail.paceAxisLabels
                seriesColor: Theme.metricPace
                flatSeries: Detail.paceSeries
                ruleY: Detail.averagePaceRuleY
                boundaryLines: Detail.splitBoundaries
                accessibleText: Tr.t("replay.cPace") + ", " + Detail.averagePaceText
            }

            // Power with the average-watts rule — Studio's powerChart.
            StrokeChart {
                Layout.fillWidth: true
                Layout.preferredHeight: Theme.chartStrokeHeight
                titleText: Tr.t("replay.cPower") + " (W)"
                minimumLabelOverflow: paceChart.labelOverflow
                yMin: 0
                yMax: Math.max(Detail.peakWatts, Detail.averageWatts, 1.0) * 1.1
                seriesColor: Theme.metricWatts
                flatSeries: Detail.powerSeries
                ruleY: Detail.averageWatts > 0 ? Detail.averageWatts : NaN
                boundaryLines: Detail.splitBoundaries
                accessibleText: Tr.t("replay.cPower") + ", "
                                + Detail.averageWattsText + " W"
            }

            // Stroke rate and heart rate (the Phase 4 brief extends Studio's
            // two charts with the rate/HR series). The HR chart draws the
            // first gap-free segment; belt dropouts split the data and the
            // later segments are not drawn (4b limitation, recorded in the
            // source map).
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingXLarge

                StrokeChart {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: Theme.chartStrokeHeight
                    titleText: Tr.t("replay.cRate")
                    minimumLabelOverflow: paceChart.labelOverflow
                    yMin: 0
                    yMax: Detail.rateMax > 0 ? Detail.rateMax * 1.15 : 40
                    seriesColor: Theme.metricCadence
                    flatSeries: Detail.rateSeries
                    ruleY: NaN
                    boundaryLines: Detail.splitBoundaries
                    accessibleText: Tr.t("replay.cRate")
                }

                StrokeChart {
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.preferredHeight: Theme.chartStrokeHeight
                    titleText: Tr.t("replay.cHeart")
                    minimumLabelOverflow: paceChart.labelOverflow
                    yMin: 0
                    yMax: Detail.hrMax > 0 ? Detail.hrMax * 1.15 : 180
                    seriesColor: Theme.metricHeartRate
                    flatSeries: Detail.hrSegmentsJson.length > 0
                                ? Detail.hrSegmentsJson[0] : []
                    ruleY: NaN
                    boundaryLines: Detail.splitBoundaries
                    accessibleText: Tr.t("replay.cHeart")
                }
            }

            // Axis caption (Studio's distance axis label).
            Label {
                Layout.alignment: Qt.AlignRight
                text: Tr.t("dashboard.distance") + " (" + Detail.distanceAxis + ")"
                font: Theme.metricLabel
                color: Theme.textSecondary
                Accessible.name: text
            }
        }
    }
}
