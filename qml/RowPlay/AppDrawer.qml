// SPDX-License-Identifier: GPL-3.0-or-later
// Drawer (ADR 0013): the sidebar below the large width class. Its surface is
// the sidebar's, with a rule on its trailing edge (2 px under high contrast)
// and the dialog's dimmed backdrop while it is modal. It slides in from the
// leading edge, and appears at once under the reduce-motion preference.
import QtQuick
import QtQuick.Controls
import RowPlay

Drawer {
    id: control

    /// The rule on the trailing edge (off where the drawer spans the
    /// window).
    property bool edgeRule: true
    /// Where the drawer is going: set as it starts to open, cleared as it
    /// starts to close. `visible` stays true through the closing slide, so
    /// a toggle button bound to it showed a closing drawer as open.
    property bool shown: false

    onAboutToShow: shown = true
    onAboutToHide: shown = false

    edge: Qt.LeftEdge
    // The Basic style pads one edge by a pixel for its own line; the rule
    // here keeps its full width clear of the content instead.
    topPadding: 0
    leftPadding: 0
    bottomPadding: 0
    rightPadding: edgeRule ? Theme.ruleWidth : 0
    // Never opened by a drag from the window's edge: on a desktop a drag
    // there is aimed at the window frame. The drawer stays interactive
    // (a swipe can close it), because Qt 6.11 closes a non-interactive
    // popup neither on Escape nor on a click outside
    // (docs/qt-bridges-notes.md).
    dragMargin: 0

    enter: Transition {
        NumberAnimation {
            duration: Theme.reduceMotion ? 0 : 200
            easing.type: Easing.OutCubic
        }
    }
    exit: Transition {
        NumberAnimation {
            duration: Theme.reduceMotion ? 0 : 200
            easing.type: Easing.OutCubic
        }
    }

    background: Rectangle {
        color: Theme.sidebarBackground

        Rectangle {
            visible: control.edgeRule
            x: parent.width - width
            width: Theme.ruleWidth
            height: parent.height
            color: Theme.separator
        }
    }

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, Theme.dark ? 0.55 : 0.32)
    }
}
