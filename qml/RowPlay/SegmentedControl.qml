// SPDX-License-Identifier: GPL-3.0-or-later
// Segmented control (ADR 0013) — for an exclusive, instantly applied choice:
// a recessed track of equal-width segments with a raised thumb behind the
// selection, hairline dividers between unselected neighbours, Left/Right to
// move the selection once focused. Sizes scale with the system font.
//
// `currentIndex` is normally bound to the backing store (Library, Settings,
// Replay); a click or arrow key only emits `activated(index)` and the caller
// writes the store, so the binding is never broken and a store that rejects
// or clamps the value stays the single source of truth.
//
// Given less width than its segments need (large text in a narrow window),
// the control shows the same choices as a pop-up button instead of eliding
// them; a caller lets it shrink by capping its width below `implicitWidth`.
import QtQuick
import QtQuick.Controls
import RowPlay

Control {
    id: control

    /// Segment titles (already translated or untranslated trademarks).
    property var model: []
    property int currentIndex: 0
    /// Translated group label: the accessible name of the tab list.
    property string label: ""
    /// Thumb slide; off under the reduce-motion preference.
    property bool animated: !Theme.reduceMotion

    signal activated(int index)

    readonly property int count: model ? model.length : 0
    readonly property bool compact: width > 0 && width < implicitWidth - 0.5
    readonly property real segmentWidth: count > 0 ? availableWidth / count : 0
    // Equal segments sized for the widest title in the selected weight. The
    // binding reads the metrics' font itself: advanceWidth() registers no
    // dependency, and the width must follow a font that lands after this
    // binding first ran or changes with the system font.
    readonly property real widestTitle: {
        void metrics.font
        var widest = 0
        for (var i = 0; i < count; ++i) {
            widest = Math.max(widest, metrics.advanceWidth(String(model[i])))
        }
        return widest
    }

    function select(index) {
        if (index >= 0 && index < count && index !== currentIndex) {
            activated(index)
        }
    }

    implicitWidth: leftPadding + rightPadding + count * Math.ceil(widestTitle + Theme.px(24))
    implicitHeight: Theme.controlHeight
    padding: Theme.px(2)
    font.pixelSize: Theme.fontPx(12)
    hoverEnabled: true
    focusPolicy: compact ? Qt.NoFocus : Qt.TabFocus

    Accessible.role: Accessible.PageTabList
    Accessible.name: label
    // In the compact form the pop-up button below is the accessible control.
    Accessible.ignored: compact

    Keys.onLeftPressed: select(currentIndex - 1)
    Keys.onRightPressed: select(currentIndex + 1)
    // While focused the arrows belong to the control, not to a window
    // shortcut (the replay route binds Left/Right to seeking).
    Keys.onShortcutOverride: function(event) {
        if (event.key === Qt.Key_Left || event.key === Qt.Key_Right) {
            event.accepted = true
        }
    }

    // The compact form: the same choices in a pop-up button, at its own
    // natural width from the leading edge.
    PopupButton {
        visible: control.compact
        width: Math.min(control.width, implicitWidth)
        height: control.height
        implicitContentWidthPolicy: ComboBox.WidestText
        model: control.model
        currentIndex: control.currentIndex
        enabled: control.enabled
        onActivated: function(index) {
            control.select(index)
        }
        Accessible.name: control.label
    }

    FontMetrics {
        id: metrics
        font.pixelSize: control.font.pixelSize
        font.weight: Font.DemiBold
    }

    background: Rectangle {
        visible: !control.compact
        radius: Theme.radiusSmall + 1
        color: Theme.segmentTrack
        border.width: Theme.highContrast ? Theme.hairline : 0
        border.color: Theme.controlBorder

        // Hairline dividers between neighbours that are both unselected.
        Repeater {
            model: Math.max(0, control.count - 1)

            Rectangle {
                required property int index
                x: control.leftPadding + (index + 1) * control.segmentWidth - 0.5
                y: Theme.px(8)
                width: Theme.hairline
                height: parent.height - 2 * Theme.px(8)
                color: Theme.separator
                visible: index !== control.currentIndex
                         && index + 1 !== control.currentIndex
            }
        }

        // The raised thumb behind the selected segment.
        Rectangle {
            visible: control.currentIndex >= 0 && control.currentIndex < control.count
            x: control.leftPadding + control.currentIndex * control.segmentWidth
            y: control.topPadding
            width: control.segmentWidth
            height: parent.height - control.topPadding - control.bottomPadding
            radius: Theme.radiusSmall - 1
            color: Theme.segmentThumb
            border.width: Theme.hairline
            border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator

            Behavior on x {
                enabled: control.animated
                NumberAnimation { duration: Theme.motionDuration; easing.type: Easing.OutCubic }
            }
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: Theme.radiusSmall + 1
        }
    }

    contentItem: Row {
        visible: !control.compact

        Repeater {
            model: control.model

            Item {
                id: segment

                required property var modelData
                required property int index
                readonly property bool selected: index === control.currentIndex

                width: control.segmentWidth
                height: control.availableHeight

                Accessible.role: Accessible.PageTab
                Accessible.name: String(modelData)
                Accessible.selected: selected
                Accessible.selectable: true
                Accessible.onPressAction: control.select(index)

                Rectangle {
                    anchors.fill: parent
                    radius: Theme.radiusSmall - 1
                    visible: !segment.selected && hoverArea.containsMouse
                    color: Theme.hoverFill
                }

                Label {
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, parent.width - Theme.spacingMedium)
                    text: String(segment.modelData)
                    font.pixelSize: control.font.pixelSize
                    font.weight: segment.selected ? Font.DemiBold : Font.Normal
                    color: control.enabled ? Theme.textPrimary : Theme.textDisabled
                    elide: Text.ElideRight
                    horizontalAlignment: Text.AlignHCenter
                    Accessible.ignored: true
                }

                MouseArea {
                    id: hoverArea
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: control.select(segment.index)
                }
            }
        }
    }
}
