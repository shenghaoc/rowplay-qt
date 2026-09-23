// SPDX-License-Identifier: GPL-3.0-or-later
// Monochrome symbol icons drawn in QML (ADR 0013): the design system's own
// glyph set, the same on every platform. No platform icon set is used (SF
// Symbols are Apple-licensed and Apple-only, Segoe Fluent Icons and Adwaita
// are platform looks) and nothing may be added under assets/ (the tree is
// hash-pinned), so every symbol is authored here as SVG path data on a
// 16-unit grid — one 1.5-unit stroke with round caps and joins, plus an
// optional fill for solid parts. Every path is original to this repository;
// the keys only borrow familiar names.
//
// Colour comes from the caller (`color`), exactly like a template image, so
// a glyph follows its control's state (accent when checked, onAccent on a
// focused selection, the disabled text colour when disabled).
import QtQuick
import QtQuick.Shapes

Item {
    id: icon

    /// Glyph key in `glyphs` below (SF Symbols-style names).
    property string name: ""
    /// Rendered edge length in pixels; the 16-unit grid scales to it.
    property real size: 16
    property color color: "black"

    implicitWidth: size
    implicitHeight: size
    visible: name.length > 0
    // Decorative: the owning control carries the accessible name.
    Accessible.ignored: true

    // stroke: outline path (1.5 units, round caps/joins); fill: solid parts.
    readonly property var glyphs: ({
        "play": {
            stroke: "M5.5 3.9 L12.1 8 L5.5 12.1 Z",
            fill: "M5.5 3.9 L12.1 8 L5.5 12.1 Z"
        },
        "pause": {
            stroke: "M4.9 3.9 H6.5 V12.1 H4.9 Z M9.5 3.9 H11.1 V12.1 H9.5 Z",
            fill: "M4.9 3.9 H6.5 V12.1 H4.9 Z M9.5 3.9 H11.1 V12.1 H9.5 Z"
        },
        "chevron.left": { stroke: "M10 3.5 L5.5 8 L10 12.5", fill: "" },
        "chevron.right": { stroke: "M6 3.5 L10.5 8 L6 12.5", fill: "" },
        "chevron.down": { stroke: "M3.5 6 L8 10.5 L12.5 6", fill: "" },
        "chevron.updown": {
            stroke: "M5 6.25 L8 3.25 L11 6.25 M5 9.75 L8 12.75 L11 9.75",
            fill: ""
        },
        // Open circle with the gap at the upper right; the head at the top
        // points clockwise.
        "arrow.clockwise": {
            stroke: "M12.7 6.3 A5 5 0 1 1 8.87 3.08 M7.68 1.0 L9.17 3.13 L7.04 4.62",
            fill: ""
        },
        // Three slider tracks, each broken around a hollow knob.
        "sliders": {
            stroke: "M2.5 4.5 H4 M7 4.5 H13.5 M7 4.5 A1.5 1.5 0 1 1 4 4.5 A1.5 1.5 0 1 1 7 4.5"
                  + " M2.5 8 H9 M12 8 H13.5 M12 8 A1.5 1.5 0 1 1 9 8 A1.5 1.5 0 1 1 12 8"
                  + " M2.5 11.5 H5.5 M8.5 11.5 H13.5 M8.5 11.5 A1.5 1.5 0 1 1 5.5 11.5 A1.5 1.5 0 1 1 8.5 11.5",
            fill: ""
        },
        "magnifyingglass": {
            stroke: "M11.25 7 A4.25 4.25 0 1 1 2.75 7 A4.25 4.25 0 1 1 11.25 7 M10.1 10.1 L13.5 13.5",
            fill: ""
        },
        "arrow.up.arrow.down": {
            stroke: "M5 13 V3 M2.5 5.5 L5 3 L7.5 5.5 M11 3 V13 M8.5 10.5 L11 13 L13.5 10.5",
            fill: ""
        },
        "arrow.up": { stroke: "M8 13 V3 M4 7 L8 3 L12 7", fill: "" },
        "arrow.down": { stroke: "M8 3 V13 M4 9 L8 13 L12 9", fill: "" },
        "calendar": {
            stroke: "M4.5 3.75 H11.5 A1.5 1.5 0 0 1 13 5.25 V12.25 A1.5 1.5 0 0 1 11.5 13.75"
                  + " H4.5 A1.5 1.5 0 0 1 3 12.25 V5.25 A1.5 1.5 0 0 1 4.5 3.75 Z"
                  + " M3 7 H13 M5.75 2.25 V4.75 M10.25 2.25 V4.75",
            fill: ""
        },
        "checkmark": { stroke: "M3.5 8.5 L6.5 11.5 L12.5 4.5", fill: "" },
        // Three rules: the application menu button (Windows / Linux).
        "line.3.horizontal": { stroke: "M3 4.5 H13 M3 8 H13 M3 11.5 H13", fill: "" },
        "ellipsis": {
            stroke: "",
            fill: "M3.1 8 a1.1 1.1 0 1 0 2.2 0 a1.1 1.1 0 1 0 -2.2 0 Z M6.9 8 a1.1 1.1 0 1 0 2.2 0 a1.1 1.1 0 1 0 -2.2 0 Z M10.7 8 a1.1 1.1 0 1 0 2.2 0 a1.1 1.1 0 1 0 -2.2 0 Z"
        },
        // RowErg: a seated rower (head, torso, arm) in a flat-deck shell, the
        // oar running from the hands down into the water.
        "sport.rower": {
            stroke: "M1.5 10.75 H14.5 Q8 14.75 1.5 10.75 Z M6.25 6.5 L7.25 10.25"
                  + " M6.75 7.75 L10.25 8.25 L13.75 14.25",
            fill: "M7.6 4.4 A1.6 1.6 0 1 1 4.4 4.4 A1.6 1.6 0 1 1 7.6 4.4 Z"
        },
        // SkiErg: two poles with grips planted in the snow, baskets above
        // the snow line.
        "sport.skierg": {
            stroke: "M1.75 13.75 H14.25 M4.25 3.5 L6.75 12 M9.25 3.5 L11.75 12"
                  + " M5.1 10.25 H7.4 M10.1 10.25 H12.4",
            fill: "M5.35 2.75 A1.1 1.1 0 1 1 3.15 2.75 A1.1 1.1 0 1 1 5.35 2.75 Z"
                + " M10.35 2.75 A1.1 1.1 0 1 1 8.15 2.75 A1.1 1.1 0 1 1 10.35 2.75 Z"
        },
        // BikeErg: two wheels and a diamond frame with saddle and bars.
        "sport.bike": {
            stroke: "M7 10.5 A3 3 0 1 1 1 10.5 A3 3 0 1 1 7 10.5"
                  + " M15 10.5 A3 3 0 1 1 9 10.5 A3 3 0 1 1 15 10.5"
                  + " M4 10.5 L6.5 5.5 H10.75 L12 10.5 M4 10.5 H8 L10.75 5.5"
                  + " M5.25 4.25 H7.75 M10.25 3.75 L11.25 5.5",
            fill: ""
        }
    })

    readonly property var glyph: glyphs[name] !== undefined ? glyphs[name]
                                                            : { stroke: "", fill: "" }

    Shape {
        id: shape
        width: 16
        height: 16
        scale: icon.size / 16
        transformOrigin: Item.TopLeft
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            fillColor: icon.color
            strokeColor: "transparent"
            strokeWidth: -1
            PathSvg { path: icon.glyph.fill }
        }
        ShapePath {
            fillColor: "transparent"
            strokeColor: icon.color
            strokeWidth: 1.5
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: icon.glyph.stroke }
        }
    }
}
