// SPDX-License-Identifier: GPL-3.0-or-later
// Segmented control (HIG Segmented controls): a recessed track of equal-width
// segments with a raised thumb behind the selection, hairline dividers between
// unselected neighbours, Left/Right to move the selection once focused.
//
// `currentIndex` is normally bound to the backing store (Library, Settings,
// Replay); a click or arrow key only emits `activated(index)` and the caller
// writes the store, so the binding is never broken and a store that rejects
// or clamps the value stays the single source of truth.
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
    /// Thumb slide; off under Settings' reduce-motion preference.
    property bool animated: !Settings.reduceReplayMotion

    signal activated(int index)

    readonly property int count: model ? model.length : 0
    readonly property real segmentWidth: count > 0 ? availableWidth / count : 0
    // Equal segments sized for the widest title in the selected weight.
    readonly property real widestTitle: {
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

    implicitWidth: leftPadding + rightPadding + count * Math.ceil(widestTitle + 24)
    implicitHeight: Theme.controlHeight
    padding: 2
    font.pixelSize: 12
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    Accessible.role: Accessible.PageTabList
    Accessible.name: label

    Keys.onLeftPressed: select(currentIndex - 1)
    Keys.onRightPressed: select(currentIndex + 1)

    FontMetrics {
        id: metrics
        font.pixelSize: control.font.pixelSize
        font.weight: Font.DemiBold
    }

    background: Rectangle {
        radius: 7
        color: Theme.segmentTrack

        // Hairline dividers between neighbours that are both unselected.
        Repeater {
            model: Math.max(0, control.count - 1)

            Rectangle {
                required property int index
                x: control.leftPadding + (index + 1) * control.segmentWidth - 0.5
                y: 7
                width: 1
                height: parent.height - 14
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
            radius: 5
            color: Theme.segmentThumb
            border.width: Theme.dark ? 0 : 1
            border.color: Qt.rgba(0, 0, 0, 0.06)

            Behavior on x {
                enabled: control.animated
                NumberAnimation { duration: 160; easing.type: Easing.OutCubic }
            }
        }

        FocusRing {
            visible: control.visualFocus
            controlRadius: 7
        }
    }

    contentItem: Row {
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
                    radius: 5
                    visible: !segment.selected && hoverArea.containsMouse
                    color: Theme.hoverFill
                }

                Label {
                    anchors.centerIn: parent
                    width: Math.min(implicitWidth, parent.width - 8)
                    text: String(segment.modelData)
                    font.pixelSize: control.font.pixelSize
                    font.weight: segment.selected ? Font.DemiBold : Font.Normal
                    color: Theme.textPrimary
                    opacity: control.enabled ? 1.0 : 0.45
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
