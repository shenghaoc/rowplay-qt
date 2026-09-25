// SPDX-License-Identifier: GPL-3.0-or-later
// A command's tool button (ADR 0015): the style's own ToolButton, showing
// the platform's icon for a glyph (Glyphs.qml) and named by its label, with
// the label and the command's shortcut, in the platform's notation, in its
// tooltip. A press dismisses the tooltip for the rest of the hover (it stayed
// up over the menu a click opened), and while the button has focus Space
// presses it, not the replay's play / pause. Used by the toolbar and the
// replay HUD.
import QtQuick
import QtQuick.Controls
import RowPlay

ToolButton {
    /// Glyph key (Glyphs.paths / Glyphs.platformNames).
    property string glyph: ""
    /// The command's shortcut (a `Shortcut`'s `nativeText`); "" for none.
    property string shortcutText: ""
    property bool tipDismissed: false

    display: AbstractButton.IconOnly
    icon.name: Glyphs.iconName(glyph)
    icon.source: Glyphs.iconSource(glyph)
    icon.color: enabled ? Theme.textPrimary : Theme.textDisabled
    icon.width: Theme.iconSize
    icon.height: Theme.iconSize
    focusPolicy: Qt.TabFocus
    hoverEnabled: true
    Accessible.name: text
    ToolTip.visible: hovered && !tipDismissed && text.length > 0
    ToolTip.delay: 600
    ToolTip.text: shortcutText.length > 0 ? text + " (" + shortcutText + ")" : text
    onPressed: tipDismissed = true
    onHoveredChanged: tipDismissed = false
    Keys.onShortcutOverride: function(event) {
        if (event.key === Qt.Key_Space && event.modifiers === Qt.NoModifier) {
            event.accepted = true
        }
    }
}
