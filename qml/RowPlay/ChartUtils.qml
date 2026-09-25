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

    // Qt Graphs 6.11 lays the Y axis out in a fixed strip: a 40 px label
    // column, a 5 px gap and 15 px of ticker (QGraphsViewPrivate's
    // m_defaultAxisLabelsWidth and friends — private, no API; "Add properties
    // for these" in the header). Each label item gets exactly that 40 px and
    // is asked to right-align in it, so a wider Rust label (a pace such as
    // "2:54.3/500m") must right-anchor its text and the chart must reserve the
    // overflow on the left through marginLeft. Returns that overflow for the
    // widest of `labels` in `metrics`' font.
    readonly property real yLabelColumn: 40

    function yLabelOverflow(labels, metrics) {
        return Math.max(0, Math.ceil(widestLabel(labels, metrics)) - yLabelColumn)
    }

    // The widest of `labels` in `metrics`' font.
    function widestLabel(labels, metrics) {
        if (!labels || !metrics) {
            return 0
        }
        // Read the font so the caller's binding follows it: advanceWidth()
        // registers no dependency (docs/qt-bridges-notes.md).
        void metrics.font
        var widest = 0
        for (var i = 0; i < labels.length; ++i) {
            widest = Math.max(widest, metrics.advanceWidth(String(labels[i])))
        }
        return widest
    }

    // The step, in points, between labelled points on an index axis whose
    // labels (at most `widest` wide) share `width`, so neighbouring labels
    // keep `gap` between them: 1 when every label fits. Layout arithmetic
    // only.
    function labelStep(count, widest, width, gap) {
        if (!(count > 0) || !(width > 0)) {
            return 1
        }
        return Math.max(1, Math.ceil((widest + gap) / (width / count)))
    }

    // The tick step that puts `count` ticks on low…high, both ends included
    // (the anchor is `low`). Shrunk by a part in 10^9: the floating-point sum
    // low + (count − 1) · step can land a hair above `high`, and Qt Graphs
    // then drops the top tick and its label. nearestLabel() still maps every
    // tick onto its exact Rust label.
    function spanInterval(low, high, count) {
        if (!(count > 1) || !(high > low)) {
            return 0
        }
        return (high - low) / (count - 1) * (1 - 1e-9)
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
