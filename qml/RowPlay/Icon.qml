// SPDX-License-Identifier: GPL-3.0-or-later
// A glyph of our own symbol set (Glyphs.qml) drawn in QML, for our content
// (the sport badges). Stock controls take their icons from the platform
// with the same glyph as an SVG fallback (Glyphs.iconName / iconSource).
//
// Colour comes from the caller (`color`), exactly like a template image, so
// a glyph follows its control's state (accent when checked, onAccent on a
// focused selection, the disabled text colour when disabled).
import QtQuick
import QtQuick.Shapes

Item {
    id: icon

    /// Glyph key in `Glyphs.paths` (SF Symbols-style names).
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

    readonly property var glyph: Glyphs.paths[name] !== undefined ? Glyphs.paths[name]
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
