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
            console.log("count before reset:", list.count)
            Backend.resetRows()
            console.log("count after reset:", list.count)
            console.log("reset survived")
            Qt.quit()
        }
    }
}
