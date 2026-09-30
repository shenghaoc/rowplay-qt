// SPDX-License-Identifier: GPL-3.0-or-later
// Stand-in for the app's Settings singleton: Theme.qml reads exactly these three members.
pragma Singleton
import QtQuick

QtObject {
    readonly property string contrastOverride: ""
    readonly property bool reduceReplayMotion: false
    readonly property string languageCode: "en"
}
