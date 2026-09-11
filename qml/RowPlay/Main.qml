// SPDX-License-Identifier: GPL-3.0-or-later
// Phase 0 bootstrap window. Later phases replace this with the dashboard shell.
import QtQuick
import RowPlay

Window {
    id: root
    visible: true
    width: 960
    height: 600
    title: "rowplay-qt — Phase 0 smoke test"
    color: "#101418"

    property bool grabRequested: false

    SmokeScene {
        id: scene
        anchors.fill: parent
    }

    // One bridge crossing per frame: QML hands the frame delta to Rust and
    // reads back the compact result through properties (ground rule: keep the
    // bridge thin, do per-frame work in Rust).
    FrameAnimation {
        id: frames
        running: true
        onTriggered: {
            Smoke.tick(frameTime)
            if (Smoke.exitAfterFrames > 0 && Smoke.frames >= Smoke.exitAfterFrames && !root.grabRequested) {
                root.grabRequested = true
                if (Smoke.screenshotPath === "") {
                    Qt.exit(0)
                } else {
                    root.captureAndExit()
                }
            }
        }
    }

    function captureAndExit() {
        var ok = scene.grabToImage(function(result) {
            var path = Smoke.screenshotPath
            var saved = result.saveToFile(path)
            if (path.endsWith(".png")) {
                // A PPM sibling lets the Rust test inspect pixels without an image decoder.
                saved = result.saveToFile(path.slice(0, -4) + ".ppm") && saved
            }
            console.log("smoke screenshot", saved ? "saved" : "FAILED", path)
            Qt.exit(saved ? 0 : 2)
        })
        if (!ok) {
            console.log("smoke screenshot: grabToImage refused")
            Qt.exit(3)
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: 16
        width: statusColumn.implicitWidth + 24
        height: statusColumn.implicitHeight + 16
        radius: 6
        color: "#c0101418"
        Column {
            id: statusColumn
            anchors.centerIn: parent
            spacing: 2
            Text { color: "#f2f4f6"; font.pixelSize: 15; text: "rowplay-qt smoke test" }
            Text { color: "#c8ccd2"; font.pixelSize: 13; text: Smoke.status }
            Text { color: "#c8ccd2"; font.pixelSize: 13; text: "frames " + Smoke.frames + "  elapsed " + Smoke.elapsed.toFixed(2) + " s" }
        }
    }
}
