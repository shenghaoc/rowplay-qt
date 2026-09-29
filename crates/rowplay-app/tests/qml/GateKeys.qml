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

    /// Presses and releases any key with modifiers, as a person's chord.
    /// Shortcuts see it the way they see a keystroke: QtTest hands the
    /// event to the window's own key path, ShortcutOverride included.
    function press(key, modifiers) {
        events.keyClick(key, modifiers === undefined ? Qt.NoModifier : modifiers, -1)
    }
}
