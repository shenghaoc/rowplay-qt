// SPDX-License-Identifier: GPL-3.0-or-later
//
// The runtime-error gate's keyboard (issue #143). QtTest's TestEvent sends
// real QKeyEvents to the window, which delivers them to the focused item as
// it does a person's keystrokes: Tab reaches Qt Quick's focus chain, and the
// field that loses focus emits editingFinished itself. Nothing is called on
// the fields directly.
//
// This file stays outside the RowPlay QML module on purpose: the gate loads
// it by URL (ROWPLAY_GATE_KEYS_QML, set by tests/qml_runtime_gate.rs), so
// `import QtTest` never reaches the app's own QML and the packaging tools
// never deploy the test module.

import QtQuick
import QtTest

Item {
    TestEvent {
        id: events
    }

    /// Types `text` one character at a time into the focused item.
    function type(text) {
        for (let i = 0; i < text.length; ++i)
            events.keyClickChar(text[i], Qt.NoModifier, -1)
    }

    /// Presses and releases Tab.
    function tab() {
        events.keyClick(Qt.Key_Tab, Qt.NoModifier, -1)
    }

    /// Presses and releases Shift+Tab (Qt Quick's backwards focus chain
    /// listens for Key_Backtab).
    function shiftTab() {
        events.keyClick(Qt.Key_Backtab, Qt.ShiftModifier, -1)
    }

    /// A shortcut's native text as the key and modifiers that send it: "Ctrl+Shift+,", "Alt+Left",
    /// "F5", and macOS's glyphs ("⌘,", "⌃⌘S", "⌘[": ⌘ is Qt's Control, ⌃ is Qt's Meta). The shell's
    /// chords differ by platform (Preferences is Ctrl+Shift+, under KDE and ⌘, on a Mac; Refresh is
    /// F5 or ⌘R; the sidebar's is F9 or ⌃⌘S), so the gate reads them from the shortcuts and never
    /// assumes one. Returns {text, key, modifiers}; key is 0 for a name it does not know.
    function chord(text) {
        const glyphs = { "\u2318": Qt.ControlModifier, "\u2303": Qt.MetaModifier, "\u2325": Qt.AltModifier, "\u21e7": Qt.ShiftModifier }
        const words = { "Ctrl": Qt.ControlModifier, "Shift": Qt.ShiftModifier, "Alt": Qt.AltModifier, "Meta": Qt.MetaModifier }
        let modifiers = Qt.NoModifier
        let name = text
        if (text.indexOf("+") < 0 || glyphs[text[0]] !== undefined) {
            let i = 0
            while (i < text.length && glyphs[text[i]] !== undefined) {
                modifiers |= glyphs[text[i]]
                ++i
            }
            name = text.slice(i)
        } else {
            const parts = text.split("+")
            name = parts[parts.length - 1] === "" && parts.length > 1 ? "+" : parts[parts.length - 1]
            for (let i = 0; i < parts.length - 1; ++i)
                if (words[parts[i]] !== undefined)
                    modifiers |= words[parts[i]]
        }
        return { text: text, key: keyCode(name), modifiers: modifiers }
    }

    function keyCode(name) {
        const named = { "Left": Qt.Key_Left, "Right": Qt.Key_Right, "Up": Qt.Key_Up, "Down": Qt.Key_Down,
                        "Esc": Qt.Key_Escape, "Escape": Qt.Key_Escape, "Tab": Qt.Key_Tab, "Space": Qt.Key_Space,
                        "Backspace": Qt.Key_Backspace, "Del": Qt.Key_Delete, "Delete": Qt.Key_Delete,
                        "Return": Qt.Key_Return, "Enter": Qt.Key_Enter, "Home": Qt.Key_Home, "End": Qt.Key_End,
                        "PgUp": Qt.Key_PageUp, "PgDown": Qt.Key_PageDown }
        if (named[name] !== undefined)
            return named[name]
        const f = /^F([0-9]{1,2})$/.exec(name)
        if (f)
            return Qt.Key_F1 + Number(f[1]) - 1
        return name.length === 1 ? name.toUpperCase().charCodeAt(0) : 0
    }

    /// Presses and releases any key with modifiers, as a person's chord.
    /// Shortcuts see it the way they see a keystroke: QtTest hands the
    /// event to the window's own key path, ShortcutOverride included.
    function press(key, modifiers) {
        events.keyClick(key, modifiers === undefined ? Qt.NoModifier : modifiers, -1)
    }
}
