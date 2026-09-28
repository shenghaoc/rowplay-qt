// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import ResetRepro
Window {
    visible: true
    width: 320; height: 200
    ListView {
        id: list
        anchors.fill: parent
        model: Backend
        delegate: Text { required property int value; text: value }
    }
    Timer {
        interval: 500; running: true
        onTriggered: {
            const before = list.count
            console.log("count before reset:", before)
            Backend.resetRows()
            const after = list.count
            console.log("count after reset:", after)
            // A pass needs the view's reset too, not only a return: exit 2
            // unless the count went from 0 to 1.
            if (before !== 0 || after !== 1) {
                console.error("count went", before, "->", after, "instead of 0 -> 1")
                Qt.exit(2)
                return
            }
            console.log("reset survived")
            Qt.exit(0)
        }
    }
}
