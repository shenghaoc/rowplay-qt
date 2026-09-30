// SPDX-License-Identifier: GPL-3.0-or-later
// What the bundled Qt reports, and what the repository's real Theme.qml resolves from it.
// A window that is never shown (no visible surface); prints one JSON line and quits.
// Built into a scratch module by kdeacc/qtprobe.py: Theme.qml is copied from qml/RowPlay/
// and Settings.qml is the stub beside this file.
import QtQuick
import QtQuick.Controls
import RowPlay

Window {
    id: root
    visible: false
    // A control with an icon name makes Qt initialise its icon loader, which logs the theme.
    property var probeButton: Button { icon.name: "document-open" }
    SystemPalette { id: active; colorGroup: SystemPalette.Active }

    function css(c) { return c.toString() }

    Timer {
        interval: 300
        running: true
        onTriggered: {
            const info = {
                platform: Qt.platform.pluginName,
                window: css(active.window), windowText: css(active.windowText),
                base: css(active.base), text: css(active.text), button: css(active.button),
                highlight: css(active.highlight), highlightedText: css(active.highlightedText),
                accent: css(active.accent),
                colorScheme: Qt.styleHints.colorScheme,          // 0 Unknown, 1 Light, 2 Dark
                contrast: Qt.styleHints.accessibility.contrastPreference,  // 0 NoPreference, 1 High
                fontFamily: Qt.application.font.family,
                fontPointSize: Qt.application.font.pointSize,
                fontPixelSize: Qt.application.font.pixelSize,
                themeAccentColor: css(Theme.accentColor),
                themeSystemAccentAvailable: Theme.systemAccentAvailable,
                themeFocusRing: css(Theme.focusRing),
                themeDark: Theme.dark,
                themeHighContrast: Theme.highContrast,
                themePx13: Theme.px(13),
                buttonBackground: String(probeButton.background)  // Fusion: ButtonPanel; Basic: Rectangle
            }
            console.log("PROBE " + JSON.stringify(info))
            Qt.quit()
        }
    }
}
