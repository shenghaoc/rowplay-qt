// SPDX-License-Identifier: GPL-3.0-or-later
// The gate's chord parser (tests/qml/GateKeys.qml) against the strings each platform's shortcuts
// print, macOS's glyphs included: what Qt would send for "Cmd+comma" or "F5" must be exactly the
// key and modifiers Qt's own constants give. Run by the harness's probe stage with the bundled qml.
import QtQuick
import QtTest
Item {
    Loader { id: l; source: Qt.resolvedUrl("../../../crates/rowplay-app/tests/qml/GateKeys.qml") }
    Component.onCompleted: {
        const k = l.item
        const C = Qt.ControlModifier, S = Qt.ShiftModifier, A = Qt.AltModifier, M = Qt.MetaModifier
        // [native text, key, modifiers]: what each platform's shortcuts print, and what Qt sends for them
        const cases = [
            ["Ctrl+Shift+,", Qt.Key_Comma, C | S],      // Preferences under KDE
            ["Ctrl+,", Qt.Key_Comma, C],                // Preferences fallback
            ["F5", Qt.Key_F5, 0],                       // Refresh, Linux and Windows
            ["F9", Qt.Key_F9, 0],                       // sidebar, Linux and Windows
            ["Alt+Left", Qt.Key_Left, A],               // Back, Linux and Windows
            ["Ctrl+Q", Qt.Key_Q, C],
            ["⌘,", Qt.Key_Comma, C],               // Preferences on macOS (Cmd is Qt's Control)
            ["⌘R", Qt.Key_R, C],                   // Refresh on macOS
            ["⌘[", Qt.Key_BracketLeft, C],         // Back on macOS
            ["⌃⌘S", Qt.Key_S, M | C],         // sidebar on macOS (Control is Qt's Meta)
            ["⌘⇧F", Qt.Key_F, C | S],
            ["Ctrl+Alt+Del", Qt.Key_Delete, C | A],
            ["Escape", Qt.Key_Escape, 0]
        ]
        let bad = 0
        for (const [text, key, mods] of cases) {
            const r = k.chord(text)
            const ok = r.key === key && r.modifiers === mods
            if (!ok) ++bad
            console.log("CHORD", ok ? "ok  " : "FAIL", JSON.stringify(text), "-> key", r.key, "mods", r.modifiers, ok ? "" : "(wanted " + key + " / " + mods + ")")
        }
        console.log("CHORD summary:", cases.length - bad, "of", cases.length, "correct")
        Qt.quit()
    }
}
