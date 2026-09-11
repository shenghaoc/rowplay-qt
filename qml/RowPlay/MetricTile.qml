// SPDX-License-Identifier: GPL-3.0-or-later
// Dashboard metric tile — a port of Studio's MetricTile: icon-less (no SF
// Symbols on Qt; divergence recorded), hero value in the semantic colour,
// tertiary label, tonal card background, read-only (no hover/active state).
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
        radius: Theme.radiusMedium
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Theme.spacingLarge

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

        Label {
            Layout.fillWidth: true
            text: tile.label
            font: Theme.metricLabel
            color: Theme.textTertiary
            elide: Text.ElideRight
            Accessible.ignored: true
        }
    }
}
