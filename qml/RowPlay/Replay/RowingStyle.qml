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

    function venueColor(name, fallback) {
        if (name.indexOf("environment:rower:") !== 0) return fallback
        if (/horizon-far|ridge-far/.test(name)) return dark ? "#68788b" : "#95a3a4"
        if (/horizon-mid|ridge-mid/.test(name)) return dark ? "#455b68" : "#6f8580"
        if (/grass|lawn|canopy|shrub|reed/.test(name)) return dark ? "#485e5d" : "#667d6b"
        if (/earth|beach|pontoon|path|trunk/.test(name)) return dark ? "#657077" : "#979e99"
        if (/tower-accent/.test(name)) return dark ? "#778593" : "#8f9b9e"
        if (/tower-material|pavilion-body/.test(name)) return dark ? "#a5b2c2" : "#cbd0d0"
        return fallback
    }
}
