// SPDX-License-Identifier: GPL-3.0-or-later
// Workout detail — a port of Studio's WorkoutDetailView: header, metric
// strip, stroke analysis, splits/intervals table and the targets read-out.
// The Replay action keeps Studio's availability policy; tools/annotations/
// comparison/export stay deferred.
//
// HIG layout (ADR 0013): the title with the sport in a capsule and Replay as
// the view's one prominent button; the metric strip, the charts, the table
// and the targets each sit in a card; the table has a header row, hairline
// row separators and right-aligned tabular numbers.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: screen

    padding: Theme.spacingXxxLarge

    background: Rectangle {
        color: Theme.windowBackground
    }

    ScrollView {
        id: scroll
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: scroll.availableWidth
            spacing: Theme.spacingXxLarge

            // Header (Studio: title + sport, date/time/source/intervals,
            // comments).
            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.spacingLarge

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Theme.spacingSmall

                    Label {
                        Layout.fillWidth: true
                        text: Detail.workoutType
                        font: Theme.pageTitle
                        color: Theme.textPrimary
                        elide: Text.ElideRight
                        Accessible.name: text
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Theme.spacingMedium

                        // The sport in a subtle capsule (trademark name,
                        // untranslated).
                        Rectangle {
                            visible: Detail.sportName.length > 0
                            implicitWidth: sportLabel.implicitWidth + 2 * Theme.spacingMedium
                            implicitHeight: sportLabel.implicitHeight + Theme.spacingXSmall
                            radius: height / 2
                            color: Qt.alpha(Theme.textPrimary, 0.07)

                            Label {
                                id: sportLabel
                                anchors.centerIn: parent
                                text: Detail.sportName
                                font: Theme.metricLabel
                                color: Theme.textSecondary
                                Accessible.name: text
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            text: [Detail.dateText, Detail.timeText,
                                   Detail.sourceText,
                                   Detail.isInterval
                                   ? Tr.t("workout.tag.interval") : ""]
                                  .filter(function(part) { return part.length > 0 })
                                  .join("  ")
                            font: Theme.subheadline
                            color: Theme.textSecondary
                            elide: Text.ElideRight
                            Accessible.name: Detail.headerAccessible
                        }
                    }
                }

                PushButton {
                    Layout.alignment: Qt.AlignTop
                    text: Tr.t("common.replay")
                    iconName: "play"
                    prominent: true
                    // Studio's policy: replay needs stroke data and no sync
                    // in flight.
                    enabled: Detail.hasStrokeData && !Sync.isRunning
                    onClicked: Library.requestReplay(Sync.isRunning)
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

            // Metric strip (Studio's performanceMetric row): equal columns,
            // four across (two when narrow), the caption above the tabular
            // value in its semantic colour.
            Rectangle {
                Layout.fillWidth: true
                visible: Detail.stripJson.length > 0
                implicitHeight: stripGrid.implicitHeight + 2 * Theme.spacingXLarge
                radius: Theme.radiusLarge
                color: Theme.panelBackground

                GridLayout {
                    id: stripGrid
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.spacingXLarge
                    columns: width >= 520 ? 4 : 2
                    uniformCellWidths: true
                    columnSpacing: Theme.spacingLarge
                    rowSpacing: Theme.spacingXLarge

                    Repeater {
                        model: Detail.stripJson

                        ColumnLayout {
                            required property var modelData
                            Layout.fillWidth: true
                            spacing: Theme.spacingXSmall
                            Accessible.name: modelData.accessibleLabelId
                                             ? Tr.t(modelData.accessibleLabelId)
                                             : Tr.t(modelData.labelId)
                            Accessible.description: modelData.accessibleValue
                                                    ? modelData.accessibleValue
                                                    : modelData.valueText

                            // Uppercase is a caption style here, not number
                            // formatting.
                            Label {
                                Layout.fillWidth: true
                                text: Tr.t(modelData.labelId).toUpperCase()
                                font: Theme.metricLabel
                                color: Theme.textSecondary
                                elide: Text.ElideRight
                                Accessible.ignored: true
                            }
                            Label {
                                Layout.fillWidth: true
                                text: modelData.valueText
                                font: Theme.stripMetric
                                color: Theme.metricColor(modelData.role)
                                elide: Text.ElideRight
                                fontSizeMode: Text.HorizontalFit
                                minimumPixelSize: 13
                                Accessible.ignored: true
                            }
                        }
                    }
                }
            }

            StrokeAnalysisPanel {
                Layout.fillWidth: true
            }

            // Splits / intervals table (Studio's Grid; web th* keys): the
            // index column, then six equal metric columns, right-aligned.
            Rectangle {
                Layout.fillWidth: true
                visible: Detail.splitsJson.length > 0
                implicitHeight: splitsColumn.implicitHeight + 2 * Theme.spacingXLarge
                radius: Theme.radiusLarge
                color: Theme.panelBackground

                ColumnLayout {
                    id: splitsColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.spacingXLarge
                    spacing: 0

                    Label {
                        Layout.bottomMargin: Theme.spacingLarge
                        text: Tr.t(Detail.splitsSectionId)
                        font: Theme.sectionHeadline
                        color: Theme.textPrimary
                        Accessible.name: text
                    }

                    // Header row.
                    RowLayout {
                        Layout.fillWidth: true
                        Layout.bottomMargin: Theme.spacingSmall
                        spacing: Theme.spacingLarge

                        Repeater {
                            model: Detail.splitColumnIds

                            Label {
                                required property string modelData
                                required property int index
                                Layout.fillWidth: index > 0
                                Layout.preferredWidth: index > 0 ? 1 : 36
                                text: Tr.t(modelData)
                                font: Theme.metricLabel
                                color: Theme.textSecondary
                                elide: Text.ElideRight
                                horizontalAlignment: index > 0 ? Text.AlignRight
                                                               : Text.AlignLeft
                                Accessible.ignored: true
                            }
                        }
                    }

                    Repeater {
                        model: Detail.splitsJson

                        ColumnLayout {
                            id: splitRow

                            required property var modelData
                            Layout.fillWidth: true
                            spacing: 0
                            Accessible.name: Tr.t("replay.segSplits") + " "
                                           + (modelData.index + 1) + ": "
                                           + modelData.distanceText + ", "
                                           + modelData.timeText + ", "
                                           + modelData.paceText

                            // Hairline rule above every row.
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 1
                                color: Theme.separator
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 32
                                spacing: Theme.spacingLarge
                                // Rest intervals recede.
                                opacity: splitRow.modelData.isRest ? 0.55 : 1.0

                                Label {
                                    Layout.preferredWidth: 36
                                    text: splitRow.modelData.index
                                    font: Theme.tabularBody
                                    color: Theme.textSecondary
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.distanceText
                                    font: Theme.tabularBody
                                    color: Theme.metricDistance
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.timeText
                                    font: Theme.tabularBody
                                    color: Theme.textPrimary
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.paceText
                                    font: Theme.tabularBody
                                    color: Theme.metricPace
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.cadenceText
                                    font: Theme.tabularBody
                                    color: Theme.metricCadence
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.powerText
                                    font: Theme.tabularBody
                                    color: Theme.metricWatts
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.hrText
                                    font: Theme.tabularBody
                                    color: Theme.metricHeartRate
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                            }
                        }
                    }
                }
            }

            // Targets read-out (web replay.mTarget* keys).
            Rectangle {
                Layout.fillWidth: true
                visible: Detail.targetsJson.length > 0
                implicitHeight: targetsColumn.implicitHeight + 2 * Theme.spacingXLarge
                radius: Theme.radiusLarge
                color: Theme.panelBackground

                ColumnLayout {
                    id: targetsColumn
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: Theme.spacingXLarge
                    spacing: Theme.spacingMedium

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
                                font: Theme.subheadline
                                color: Theme.textSecondary
                                elide: Text.ElideRight
                                Accessible.ignored: true
                            }
                            Label {
                                Layout.fillWidth: true
                                text: modelData.valueText
                                font: Theme.tabularBody
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
}
