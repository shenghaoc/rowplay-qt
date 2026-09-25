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
            id: detailScrollBar
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
                            border.width: Theme.cardBorderWidth
                            border.color: Theme.separator

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
                    // Room for its focus ring (FocusRing reaches
                    // focusRingExtent outside the button, and the scroll
                    // view clips at its top edge), and clear of the overlay
                    // scroll bar, which would otherwise take clicks on its
                    // trailing edge.
                    Layout.topMargin: Theme.focusRingExtent
                    Layout.rightMargin: detailScrollBar.maximumThickness
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
                border.width: Theme.cardBorderWidth
                border.color: Theme.separator

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
            // number column, then six metric columns, right-aligned. Every
            // column is as wide as its widest header or value, and spare
            // width is shared equally among the metric columns, so the
            // header and every row line up and no value is ever elided.
            // Where even the content widths do not fit (large text in a
            // narrow window), the table scrolls sideways inside its card.
            Rectangle {
                id: splitsCard
                Layout.fillWidth: true
                visible: Detail.splitsJson.length > 0
                implicitHeight: splitsColumn.implicitHeight + 2 * Theme.spacingXLarge
                radius: Theme.radiusLarge
                color: Theme.panelBackground
                border.width: Theme.cardBorderWidth
                border.color: Theme.separator

                // One key per column, in Detail.splitColumnIds' order.
                readonly property var cellKeys: [
                    "numberText", "distanceText", "timeText", "paceText",
                    "cadenceText", "powerText", "hrText"
                ]
                // Content widths: the widest of the header and every row. The
                // binding reads both metrics' fonts: advanceWidth() registers
                // no dependency (docs/qt-bridges-notes.md).
                readonly property var contentWidths: {
                    void headerMetrics.font
                    void valueMetrics.font
                    var ids = Detail.splitColumnIds
                    var rows = Detail.splitsJson
                    var widths = []
                    for (var c = 0; c < cellKeys.length; ++c) {
                        var w = headerMetrics.advanceWidth(Tr.t(ids[c]))
                        for (var r = 0; r < rows.length; ++r) {
                            w = Math.max(w, valueMetrics.advanceWidth(rows[r][cellKeys[c]]))
                        }
                        widths.push(Math.ceil(w) + Theme.px(4))
                    }
                    return widths
                }
                readonly property real minimumTableWidth: {
                    var sum = 0
                    for (var c = 0; c < contentWidths.length; ++c) {
                        sum += contentWidths[c]
                    }
                    return sum + (contentWidths.length - 1) * Theme.spacingLarge
                }
                readonly property real tableWidth: Math.max(splitsScroller.width,
                                                            minimumTableWidth)
                // The final widths, in whole pixels: Qt Quick Layouts round
                // each width up to a whole pixel, so fractional shares would
                // sum past the table and clip its last column. The spare
                // width goes to the metric columns in equal shares, with the
                // rounding remainder on the last one; the number column keeps
                // its content width.
                readonly property var columnWidths: {
                    var n = contentWidths.length - 1
                    var spare = Math.max(0, Math.floor(tableWidth - minimumTableWidth))
                    var share = Math.floor(spare / n)
                    var widths = []
                    for (var c = 0; c < contentWidths.length; ++c) {
                        widths.push(contentWidths[c] + (c > 0 ? share : 0)
                                    + (c === n ? spare - share * n : 0))
                    }
                    return widths
                }

                FontMetrics { id: headerMetrics; font: Theme.metricLabel }
                FontMetrics { id: valueMetrics; font: Theme.tabularBody }

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

                    Flickable {
                        id: splitsScroller
                        Layout.fillWidth: true
                        Layout.preferredHeight: splitsTable.implicitHeight
                                                + (interactive ? splitsScrollBar.maximumThickness : 0)
                        contentWidth: splitsCard.tableWidth
                        contentHeight: splitsTable.implicitHeight
                        flickableDirection: Flickable.HorizontalFlick
                        boundsBehavior: Flickable.StopAtBounds
                        interactive: contentWidth > width + 0.5
                        clip: true
                        ScrollBar.horizontal: AppScrollBar {
                            id: splitsScrollBar
                            policy: splitsScroller.interactive ? ScrollBar.AlwaysOn
                                                               : ScrollBar.AlwaysOff
                        }

                        ColumnLayout {
                            id: splitsTable
                            width: splitsCard.tableWidth
                            spacing: 0

                            // Header row, sentence case.
                            RowLayout {
                                Layout.bottomMargin: Theme.spacingSmall
                                spacing: Theme.spacingLarge

                                Repeater {
                                    model: Detail.splitColumnIds

                                    Label {
                                        required property string modelData
                                        required property int index
                                        Layout.preferredWidth: splitsCard.columnWidths[index]
                                        text: Tr.t(modelData)
                                        font: Theme.metricLabel
                                        color: Theme.textSecondary
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
                                    // Rest rows recede in the secondary text
                                    // colour (their distance and pace read
                                    // "—"); the metric colours are for work
                                    // rows.
                                    readonly property bool rest: modelData.isRest === true
                                    readonly property var tones: [
                                        Theme.textSecondary, Theme.metricDistance,
                                        Theme.textPrimary, Theme.metricPace,
                                        Theme.metricCadence, Theme.metricWatts,
                                        Theme.metricHeartRate
                                    ]

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
                                        Layout.preferredHeight: Theme.controlHeight
                                        spacing: Theme.spacingLarge

                                        // Numbered from 1 like the web
                                        // (Rust-formatted), then the six
                                        // metrics.
                                        Repeater {
                                            model: splitsCard.cellKeys

                                            Label {
                                                required property string modelData
                                                required property int index
                                                Layout.preferredWidth: splitsCard.columnWidths[index]
                                                text: splitRow.modelData[modelData]
                                                font: Theme.tabularBody
                                                color: index === 0 || splitRow.rest
                                                       ? Theme.textSecondary
                                                       : splitRow.tones[index]
                                                horizontalAlignment: index > 0 ? Text.AlignRight
                                                                               : Text.AlignLeft
                                                Accessible.ignored: true
                                            }
                                        }
                                    }
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
                border.width: Theme.cardBorderWidth
                border.color: Theme.separator

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
