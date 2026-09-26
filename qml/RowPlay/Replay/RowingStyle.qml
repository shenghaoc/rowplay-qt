// SPDX-License-Identifier: GPL-3.0-or-later
pragma Singleton
import QtQuick
import RowPlay

QtObject {
    readonly property bool dark: Replay.schemeDark
    readonly property color water: "#809296"
    readonly property color fog: dark ? "#94a8bf" : "#c7cdd1"
    readonly property color key: dark ? "#bbcde5" : "#f3f1ea"
    readonly property real keyBrightness: Replay.effectiveQuality < 2
        ? (dark ? 0.5 : 0.85) : (dark ? 1.4 : 1.2)
    readonly property color hull: "#315784"
    readonly property color carbon: "#24282d"
    readonly property color metal: "#a0a5aa"
    readonly property color blade: "#edeee9"
    // Blender Phase 3: the environment's albedo lives in its vertex colours;
    // blue hour tints it cooler and darker, as the venue colours below do.
    readonly property color landTint: dark ? "#b3bfde" : "#ffffff"
    readonly property color farTint: dark ? "#b3bfde" : "#ffffff"
    readonly property color foliageTint: dark ? "#b3bfde" : "#ffffff"

    function venueColor(name, fallback) {
        if (name.indexOf("environment:rower:") !== 0) return fallback
        // Since Blender Phase 3 this reaches only the retained island, whose
        // lawn, trees and shrubs have shared one green since Phase 1: they
        // take the authored environment's mown-lawn tone (blue hour: the
        // same albedo under the environment's tint).
        if (/grass|lawn|canopy|shrub|reed/.test(name)) return dark ? "#3d4b45" : "#5a6751"
        if (/earth|beach|pontoon|path|trunk/.test(name)) return dark ? "#657077" : "#979e99"
        if (/tower-accent/.test(name)) return dark ? "#778593" : "#8f9b9e"
        if (/tower-material|pavilion-body/.test(name)) return dark ? "#a5b2c2" : "#cbd0d0"
        return fallback
    }
}
