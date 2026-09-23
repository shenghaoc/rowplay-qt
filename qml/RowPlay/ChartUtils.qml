// SPDX-License-Identifier: GPL-3.0-or-later
// Chart data helpers shared by the Qt Graphs panels.
//
// The bridge hands series over as flat [x0,y0,x1,y1,…] arrays (one crossing
// per series); Qt Graphs' XYSeries.replace() needs a list of points and is
// the documented bulk path — convert once, replace once, never point by
// point (Phase 4 ground rule).
pragma Singleton
import QtQuick

QtObject {
    // Flat bridge array → [{x,y}, …] for XYSeries.replace(list<point>).
    function points(flat) {
        if (!flat) {
            return []
        }
        var pts = new Array(flat.length >> 1)
        for (var i = 0; i + 1 < flat.length; i += 2) {
            pts[i >> 1] = { x: flat[i], y: flat[i + 1] }
        }
        return pts
    }

    // A horizontal rule at y (Studio's RuleMark) across [minX, maxX].
    function rule(minX, maxX, y) {
        return [{ x: minX, y: y }, { x: maxX, y: y }]
    }

    // A vertical rule at x (split boundaries) across [minY, maxY].
    function vrule(x, minY, maxY) {
        return [{ x: x, y: minY }, { x: x, y: maxY }]
    }

    // A "nice" tick step (1, 2, 2.5 or 5 × 10^k) that puts about
    // `targetTicks` intervals on `span`. Layout arithmetic only: Qt Graphs'
    // automatic interval aims at about ten ticks whatever the plot height,
    // which packs a 50 px stroke chart with overlapping labels.
    function niceInterval(span, targetTicks) {
        if (!(span > 0) || !(targetTicks > 0)) {
            return 1
        }
        var raw = span / targetTicks
        var magnitude = Math.pow(10, Math.floor(Math.log(raw) / Math.LN10))
        var normalized = raw / magnitude
        var step = normalized <= 1 ? 1
                 : normalized <= 2 ? 2
                 : normalized <= 2.5 ? 2.5
                 : normalized <= 5 ? 5 : 10
        return step * magnitude
    }

    // ValueAxis tick labels are printf numbers; the pace axis needs the
    // pre-rendered pace strings from Rust. Maps the injected label text to
    // the nearest exported tick value and returns its formatted label.
    function nearestLabel(values, labels, text) {
        var v = parseFloat(text)
        if (isNaN(v) || !values || values.length === 0) {
            return text
        }
        var best = 0
        var bestDistance = Math.abs(v - values[0])
        for (var i = 1; i < values.length; i++) {
            var d = Math.abs(v - values[i])
            if (d < bestDistance) {
                bestDistance = d
                best = i
            }
        }
        return labels[best] !== undefined ? labels[best] : text
    }
}
