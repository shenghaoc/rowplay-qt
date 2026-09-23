// SPDX-License-Identifier: GPL-3.0-or-later
// Dashboard metric tile — a port of Studio's MetricTile: icon-less (no SF
// Symbols on Qt; divergence recorded), read-only (no hover/active state). HIG
// layout (ADR 0013): the caption on top in secondary text, the hero value
// below it in the semantic colour (the One Hero Rule), on a tonal card.
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import RowPlay

Pane {
    id: tile

    /// Translated label (caller resolves the locale id).
    property string label: ""
    /// Pre-rendered value text (never formatted in QML).
    property string value: ""
    /// Semantic colour for the value (Theme.metricColor(role)).
    property color valueColor: Theme.textPrimary

    padding: Theme.spacingXLarge
    Accessible.name: label
    Accessible.description: value

    background: Rectangle {
        color: Theme.cardBackground
        radius: Theme.radiusLarge
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingSmall

        Label {
            Layout.fillWidth: true
            text: tile.label
            font: Theme.subheadline
            color: Theme.textSecondary
            elide: Text.ElideRight
            Accessible.ignored: true
        }

        // The One Hero Rule: this is the only hero-sized element per card.
        Label {
            Layout.fillWidth: true
            text: tile.value
            font: Theme.heroMetric
            color: tile.valueColor
            elide: Text.ElideRight
            fontSizeMode: Text.HorizontalFit
            minimumPixelSize: 18
            Accessible.ignored: true
        }
    }
}
