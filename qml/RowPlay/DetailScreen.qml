// SPDX-License-Identifier: GPL-3.0-or-later
// Workout detail — a port of Studio's WorkoutDetailView: header, metric
// strip, stroke analysis, splits/intervals table and the targets read-out.
// The Replay action keeps Studio's availability policy (the route itself
// renders in Phase 5); tools/annotations/comparison/export stay deferred.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: screen

    padding: Theme.spacingXxxLarge

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        // The column spans the view: sized by parent.width it followed the
        // Flickable content item, i.e. its own implicit width, and stopped a
        // quarter to a third short of the pane.
        contentWidth: availableWidth

        ColumnLayout {
            width: scroll.availableWidth
            spacing: Theme.spacingXxxLarge

            // Header (Studio: title + sport, date/time/source/intervals,
            // comments).
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        Layout.fillWidth: true
                        text: Detail.workoutType
                        font: Theme.pageTitle
                        color: Theme.textPrimary
                        elide: Text.ElideRight
                        Accessible.name: text
                    }
                    Label {
                        text: Detail.sportName
                        font: Theme.sectionHeadline
                        color: Theme.textSecondary
                        Accessible.name: text
                    }
                    Button {
                        text: Tr.t("common.replay")
                        // Studio's policy: replay needs stroke data and no
                        // sync in flight; Phase 5 renders the route.
                        enabled: Detail.hasStrokeData && !Sync.isRunning
                        onClicked: Library.requestReplay(Sync.isRunning)
                        Accessible.name: Tr.t("common.replay")
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingMedium

                    Label {
                        text: [Detail.dateText, Detail.timeText,
                               Detail.sourceText,
                               Detail.isInterval
                               ? Tr.t("workout.tag.interval") : ""]
                              .filter(function(part) { return part.length > 0 })
                              .join("  ")
                        font: Theme.subheadline
                        color: Theme.textSecondary
                        Accessible.name: Detail.headerAccessible
                    }
                }

                Label {
                    Layout.fillWidth: true
                    visible: Detail.hasComments
                    text: Detail.comments
                    font: Theme.body
                    color: Theme.textSecondary
                    wrapMode: Text.WordWrap
                    Accessible.name: text
                }
            }

            // Metric strip (Studio's performanceMetric row: uppercased
            // labels, tabular values in semantic colours).
            Flow {
                Layout.fillWidth: true
                spacing: 0

                Repeater {
                    model: Detail.stripJson

                    ColumnLayout {
                        required property var modelData
                        width: Math.max(140, screen.width
                                        / Math.max(1, Detail.stripJson.length))
                        spacing: Theme.spacingSmall
                        Accessible.name: modelData.accessibleLabelId
                                         ? Tr.t(modelData.accessibleLabelId)
                                         : Tr.t(modelData.labelId)
                        Accessible.description: modelData.accessibleValue
                                                ? modelData.accessibleValue
                                                : modelData.valueText

                        Label {
                            Layout.leftMargin: Theme.spacingXLarge
                            text: Tr.t(modelData.labelId).toUpperCase()
                            font: Theme.metricLabel
                            color: Theme.textSecondary
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.leftMargin: Theme.spacingXLarge
                            Layout.rightMargin: Theme.spacingXLarge
                            text: modelData.valueText
                            font: Theme.stripMetric
                            color: Theme.metricColor(modelData.role)
                            elide: Text.ElideRight
                            fontSizeMode: Text.HorizontalFit
                            minimumPixelSize: 12
                            Accessible.ignored: true
                        }
                    }
                }
            }

            StrokeAnalysisPanel {
                Layout.fillWidth: true
            }

            // Splits / intervals table (Studio's Grid; web th* keys). The
            // column widths mirror Studio's grid: leading index column, then
            // the six metric columns.
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingLarge
                visible: Detail.splitsJson.length > 0

                Label {
                    text: Tr.t(Detail.splitsSectionId)
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingLarge

                    Repeater {
                        model: Detail.splitColumnIds

                        Label {
                            required property string modelData
                            required property int index
                            Layout.preferredWidth: [40, 110, 110, 100, 80,
                                                    90, 70][index]
                            text: Tr.t(modelData).toUpperCase()
                            font: Theme.metricLabel
                            color: Theme.textSecondary
                            Accessible.ignored: true
                        }
                    }
                }

                Repeater {
                    model: Detail.splitsJson

                    RowLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: Theme.spacingLarge
                        opacity: modelData.isRest ? 0.55 : 1.0
                        Accessible.name: Tr.t("replay.segSplits") + " "
                                       + modelData.numberText + ": "
                                       + modelData.distanceText + ", "
                                       + modelData.timeText + ", "
                                       + modelData.paceText

                        Label {
                            Layout.preferredWidth: 40
                            // Numbered from 1 like the web (Rust-formatted).
                            text: modelData.numberText
                            font: Theme.metricValue
                            color: Theme.textPrimary
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 110
                            text: modelData.distanceText
                            font: Theme.metricValue
                            color: Theme.metricDistance
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 110
                            text: modelData.timeText
                            font: Theme.metricValue
                            color: Theme.textPrimary
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 100
                            text: modelData.paceText
                            font: Theme.metricValue
                            color: Theme.metricPace
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 80
                            text: modelData.cadenceText
                            font: Theme.metricValue
                            color: Theme.metricCadence
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 90
                            text: modelData.powerText
                            font: Theme.metricValue
                            color: Theme.metricWatts
                            Accessible.ignored: true
                        }
                        Label {
                            Layout.preferredWidth: 70
                            text: modelData.hrText
                            font: Theme.metricValue
                            color: Theme.metricHeartRate
                            Accessible.ignored: true
                        }
                    }
                }
            }

            // Targets read-out (web replay.mTarget* keys).
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingMedium
                visible: Detail.targetsJson.length > 0

                Label {
                    text: Tr.t("replay.targetsTitle")
                    font: Theme.sectionHeadline
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                Repeater {
                    model: Detail.targetsJson

                    RowLayout {
                        required property var modelData
                        Layout.fillWidth: true
                        spacing: Theme.spacingLarge

                        Label {
                            Layout.preferredWidth: 180
                            text: Tr.t(modelData.labelId)
                            font: Theme.metricLabel
                            color: Theme.textSecondary
                            Accessible.ignored: true
                        }
                        Label {
                            text: modelData.valueText
                            font: Theme.metricValue
                            color: Theme.textPrimary
                            Accessible.name: Tr.t(modelData.labelId) + ": "
                                               + modelData.valueText
                        }
                    }
                }
            }
        }
    }
}
