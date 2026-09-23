// SPDX-License-Identifier: GPL-3.0-or-later
// Workout detail — a port of Studio's WorkoutDetailView: header, metric
// strip, stroke analysis, splits/intervals table and the targets read-out.
// The Replay action keeps Studio's availability policy; tools/annotations/
// comparison/export stay deferred.
//
// Design system (ADR 0013): the title with the sport in a neutral capsule
// and Replay as the view's one prominent button; the metrics, the charts,
// the table and the targets each sit in a tonal card; labels in sentence
// case; the table has a header row, hairline rules and right-aligned
// tabular numbers.
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
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical: AppScrollBar {
            parent: scroll
            x: scroll.mirrored ? 0 : scroll.width - width
            y: scroll.topPadding
            height: scroll.availableHeight
        }

        ColumnLayout {
            id: content
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

                        // The sport in a neutral capsule (trademark name,
                        // untranslated).
                        Rectangle {
                            visible: Detail.sportName.length > 0
                            implicitWidth: sportLabel.implicitWidth + 2 * Theme.spacingMedium
                            implicitHeight: sportLabel.implicitHeight + Theme.spacingXSmall
                            radius: height / 2
                            color: Theme.segmentTrack

                            Label {
                                id: sportLabel
                                anchors.centerIn: parent
                                text: Detail.sportName
                                font: Theme.metricLabel
                                color: Theme.textPrimary
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

                // The view's one prominent action; a label, no icon.
                PushButton {
                    Layout.alignment: Qt.AlignTop
                    text: Tr.t("common.replay")
                    prominent: true
                    // Studio's policy: replay needs stroke data and no sync
                    // in flight. The disabled state explains nothing yet:
                    // the web has no string for why (tracked in #64).
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

            // Metric grid (Studio's performanceMetric row): equal, balanced
            // columns, the caption above the tabular value in its semantic
            // colour — the caption names the metric, so colour is never the
            // only cue.
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
                    columns: Theme.balancedColumns(Detail.stripJson.length, width,
                                                   Theme.px(120), columnSpacing)
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

                            Label {
                                Layout.fillWidth: true
                                text: Tr.t(modelData.labelId)
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
                                minimumPixelSize: Theme.fontPx(13)
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
            // number column, then six equal metric columns, right-aligned.
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

                    // Header row, sentence case.
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
                                Layout.preferredWidth: index > 0 ? 1 : Theme.px(36)
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
                            // Rest rows recede in the secondary text colour
                            // (their distance and pace read "—"); the metric
                            // colours are for work rows.
                            readonly property bool rest: modelData.isRest === true

                            function tone(metric) {
                                return rest ? Theme.textSecondary : metric
                            }

                            Layout.fillWidth: true
                            spacing: 0
                            Accessible.name: Tr.t("replay.segSplits") + " "
                                             + modelData.numberText + ": "
                                             + modelData.distanceText + ", "
                                             + modelData.timeText + ", "
                                             + modelData.paceText

                            // Hairline rule above every row.
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Theme.hairline
                                color: Theme.separator
                            }

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.preferredHeight: Theme.controlHeight
                                spacing: Theme.spacingLarge

                                Label {
                                    Layout.preferredWidth: Theme.px(36)
                                    // Numbered from 1 like the web
                                    // (Rust-formatted).
                                    text: splitRow.modelData.numberText
                                    font: Theme.tabularBody
                                    color: Theme.textSecondary
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.distanceText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.metricDistance)
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.timeText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.textPrimary)
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.paceText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.metricPace)
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.cadenceText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.metricCadence)
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.powerText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.metricWatts)
                                    horizontalAlignment: Text.AlignRight
                                    elide: Text.ElideLeft
                                    Accessible.ignored: true
                                }
                                Label {
                                    Layout.fillWidth: true
                                    Layout.preferredWidth: 1
                                    text: splitRow.modelData.hrText
                                    font: Theme.tabularBody
                                    color: splitRow.tone(Theme.metricHeartRate)
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
                                Layout.preferredWidth: Theme.px(180)
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
