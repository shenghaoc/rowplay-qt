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
    // Blender Phase 4: the course dressing's structures and furniture take
    // the same blue-hour tint as the land they stand on.
    readonly property color dressingTint: dark ? "#b3bfde" : "#ffffff"

    function venueColor(name, fallback) {
        if (name.indexOf("environment:rower:") !== 0) return fallback
        // Since Blender Phase 4 every rower venue node is hidden (the
        // environment and the dressing replace them all); the walk still
        // builds their materials, and these tones are what it built since
        // Phase 3 for the island's lawn, trees and shrubs.
        if (/grass|lawn|canopy|shrub|reed/.test(name)) return dark ? "#3d4b45" : "#5a6751"
        if (/earth|beach|pontoon|path|trunk/.test(name)) return dark ? "#657077" : "#979e99"
        if (/tower-accent/.test(name)) return dark ? "#778593" : "#8f9b9e"
        if (/tower-material|pavilion-body/.test(name)) return dark ? "#a5b2c2" : "#cbd0d0"
        return fallback
    }
}
