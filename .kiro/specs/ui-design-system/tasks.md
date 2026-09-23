# UI design system — tasks

A stack of seven PRs (`design.md`, "Stack"). Each task is ticked in the PR
that delivers it.

- [x] T1 (1/7 `ui/01-fixes`) Pre-existing defects fixed one commit at a
  time, each with its own issue (#48–#61). Recorded in `docs/roadmap.md`.
- [x] T2 (2/7 `ui/02-foundation`) The Basic style and the tokens.
  - [x] T2.1 `qtquickcontrols2.conf` selects Basic. `Main.qml` sets every
    palette role from the tokens, mapped by Basic's use of each role (R2.3).
    No `font.pixelSize` on the window: the system font is the base.
  - [x] T2.2 `Theme.qml` (R1):
    - the PM5 palette and metric colours unchanged;
    - the neutral ramps and control-part tokens;
    - `destructiveText` (the PM5 red measures 4.46:1 on the dark control
      fill);
    - the system-font scale (`basePx`, `scale`, `px`, `fontPx`) on every
      length and font;
    - the accent with the brand-blue fallback and `onAccent`;
    - the high-contrast variant, with `ROWPLAY_FORCE_CONTRAST` →
      `Settings.contrastOverride`;
    - the reduce-motion flag for control motion.
  - [x] T2.3 The shared controls rebuilt on Basic, plus the popups and
    indicators Basic needs themed (R2.1, R2.2), registered in `qmldir` and
    the qrc. The glyph set gains `line.3.horizontal` and `ellipsis`. No
    screen uses them yet.
  - [x] T2.4 Rendered in the scratch gallery (`design.md`, "Verification
    harness"):
    - light, dark, and high contrast in both schemes;
    - focus forced on each control;
    - the menu, pop-up list, tooltip and dialog open;
    - 125 % and 150 % text (base 15 and 18 px) with nothing clipped.

    The gallery found two defects, both fixed:
    - `FormRow.toggleTarget` rejected a `ToggleSwitch` under Basic, so it is
      now typed `T.AbstractButton`;
    - Basic's `DialogButtonBox` stretched the buttons and painted the window
      colour, so `AppDialogButtonBox` replaces it.

    The contrast table comes from the QML runtime on the real tokens
    (`docs/source-map.md`). Read again at four decimals, it found a third
    defect: the PM5 duration colour measured 4.496:1 on the light grouped
    surface, which printed as 4.50. The light sidebar and group surface
    moved from `#F3F4F6` to `#F4F5F7` (4.53:1), and the table's figures
    are now truncated, never rounded up.
  - [x] T2.5 ADR 0013, this spec, `docs/source-map.md` (the Theme row, the
    surface and text divergence), `docs/qt-bridges-notes.md` (accent,
    contrast, font scale and Basic findings), `docs/roadmap.md`, and the
    AGENTS.md coding-style line (Basic, not Fusion).
  - [x] T2.6 Validation (R9): see the PR.
  - [x] T2.7 Large text in a narrow window, found by driving the screens
    of 3/7–5/7 at 150 % text in the 1000 px minimum window: a
    `SegmentedControl` capped below its implicit width becomes a pop-up
    button with the same choices, and a `FormRow` too narrow for its label
    beside its controls stacks them under the label.
  - [x] T2.8 Text measured in a binding follows its font.
    `FontMetrics.advanceWidth()` registers no dependency, so
    `SegmentedControl.widestTitle` and `ChartUtils.yLabelOverflow` read the
    metrics' font themselves. Without that, a measure taken before the font
    landed stayed (5/7's quality control drew 308 px wide instead of 292),
    and so did one taken before a live change of the system font.
  - [x] T2.9 The compact form's floor, from the Codex review of #70 (its
    toolbar could cap the sport filter at 0 px, where the control shows
    nothing). `SegmentedControl.compactWidth` is the narrowest width that
    still shows every choice whole: the pop-up button around its widest
    title, measured in the button's own font. The pop-up button is never
    narrower. `ComboBox.WidestText`, which the compact form had set for
    this, measures only a `TextInput` content item and read 0 here
    (`docs/qt-bridges-notes.md`).
  - [x] T2.10 The second-reviewer pass (2026-09-24). Each defect was
    reproduced before its fix, with a scratch Qt Quick Test on the real
    control or Qt's own sources:
    - `ToolbarButton` set `Accessible.Button` explicitly, hiding the
      CheckBox role the template gives a checkable button (3/7's settings
      toggle);
    - a window shortcut on Space took the key from a focused toolbar
      button (6/7's replay: Space on Close toggled playback);
    - the pop-up list indented only the current row's label (2/7's own
      gallery capture shows it);
    - scroll bars vanished into Basic's track under the OS contrast
      preference, a track `ROWPLAY_FORCE_CONTRAST` never shows;
    - the indeterminate progress segment froze off-centre when reduce
      motion stopped its sweep;
    - parts centred inside a control could sit on half pixels.

    `AppScrollBar.maximumThickness` serves 4/7's splits table.
- [x] T3 (3/7 `ui/03-shell`) The shell and the platform layer.
  - [x] T3.1 Full-height sidebar beside a content column. The toolbar holds
    the segmented sport filter and the icon-only reload and settings
    buttons, and the split handle is a hairline with a 9 px drag area.
    Sidebar rows (R3):
    - InputFields for search and dates;
    - the sort menu as an AppMenu with a checkmark and a direction arrow;
    - day headers in sentence case;
    - sport glyphs;
    - the PB capsule in solid orange with `Theme.textOn` text (the 15 %
      wash measured 4.16:1);
    - the focused / unfocused selection.

    The empty state now replaces only the content area.
  - [x] T3.2 The platform layer (R4.1–R4.4):
    - StandardKey Preferences (Ctrl+, where the platform has none), Quit,
      Close, Find, Refresh and Back (primary chord only);
    - the sidebar toggle (F9, or Ctrl+Cmd+S on macOS; off in the replay);
    - the macOS menu bar with About / Preferences… / Quit roles (Qt titles
      them, in English: #80), created only on macOS;
    - the Windows / Linux menu button with the shell's commands and their
      shortcut texts;
    - the About dialog of existing strings.

    Dialog order comes from `AppDialogButtonBox` (2/7), and title bars stay
    native. The gate walks the menu, the toggle, About and the sort menu.
  - [x] T3.3 Driven under Xvfb with xdotool:
    - the menu opens with its shortcut hints;
    - F9 hides and restores the sidebar at its width (mean difference
      0.06);
    - Ctrl+F rings the search field;
    - Ctrl+, opens settings;
    - Alt+Left walks back from settings to the workout, then to the
      dashboard;
    - Ctrl+Q exits 0.

    The Tab walk runs search → sort → From → To → list (accent selection)
    → sport filter → reload → settings → menu, each stop with its ring. It
    found that a toolbar button's tooltip stayed over the popup its click
    opened (fixed).
  - [x] T3.4 `tools/package/linux.sh` deploys `Qt/labs/platform` and
    `QtQuick/Shapes` into the AppImage, and its launch check rendered 30
    frames and exited 0. macOS and Windows packaging was not run: the
    release workflow does not trigger on these paths.
  - [x] T3.5 Large text in a narrow window. At 150 % text in the 1000 px
    minimum window (Spanish), the centred sport filter ran under the
    trailing buttons. It is now clamped clear of them, and where it cannot
    fit its width is capped, so it takes the segmented control's compact
    pop-up form (driven under Xvfb: the segmented track's pixels drop to 0
    in the narrow window and come back at 1200 px, and a choice made in
    the pop-up filters the list and shows in the segments).
  - [x] T3.6 Validation (R9): see the PR.
  - [x] T3.7 The Codex review of #70 (head `5efc104`): three findings, each
    reproduced before its fix.
    - A Space press on a focused toolbar button, with the pointer
      elsewhere, hid the tooltip from the next hover (a scratch Qt Quick
      Test on the real control). Any hover change now clears the press's
      dismissal.
    - Settings opened over the replay (the toolbar toggle, Preferences, the
      menus) left the replay presented: Back returned to the workout, and
      Replay was refused as already presented. `showSettings()` and
      `showDashboard()` now close the replay first, a change 6/7 had made
      for itself. Gate steps 206–209 log the route and the gate test
      asserts it.
    - Very large text with a wide sidebar capped the sport filter at 0 px,
      where it shows nothing, and pushed the trailing buttons past the
      toolbar's leading edge (base 23 px, the sidebar dragged to its
      maximum, in a probe of the real controls). The filter keeps its
      compact form's width (`compactWidth`, T2.9), a dragged sidebar stops
      where the toolbar would get less, and the window's minimum grows past
      1000 px only where very large text needs it. Nothing moves at 100 %
      or 150 % text.
  - [x] T3.8 The second-reviewer pass (2026-09-24):
    - the sport filter's centring and the sidebar's top padding could land
      on half pixels (x 295.5 at the reference in the default window); both
      round;
    - the sort menu's direction was only an icon, so screen readers lost
      the ↑ / ↓ main's label showed; the active item's name carries it;
    - the focused list showed nothing while its selected row was filtered
      out or scrolled away; it rings itself then (checked on a plain
      ListView in a scratch Qt Quick Test);
    - the menu titles are Qt's "Preferences…", not "Settings…" (#80).
- [x] T4 (4/7 `ui/04-dashboard-detail`) R5.
  - [x] T4.1 Dashboard:
    - tiles and PB cards as tonal cards in balanced grids
      (`Theme.balancedColumns`);
    - the PB value on `Theme.cardMetric`, a scaled token for the inline
      font that 1/7 gave its tabular figures;
    - both charts on `ChartTheme`, with about four nice ticks on the bars;
    - the `AppScrollBar`.
  - [x] T4.2 Detail:
    - Replay as the one prominent button (a label, no icon);
    - the metric grid in a card with sentence-case labels;
    - the stroke charts in one card on `ChartTheme`, the title height
      measured;
    - the splits table in a card with right-aligned tabular numbers;
    - rest rows in the secondary text colour instead of 55 % opacity
      (below AA);
    - the targets card.
  - [x] T4.3 No `toUpperCase()` is left in `qml/` (R5.2). Nothing on these
    screens relies on colour alone (R5.3):
    - every metric colour sits under the label that names it;
    - rest rows keep their "—";
    - the PB capsule carries "PB".

    The replay's race verdict is 6/7's.
  - [x] T4.4 Large text in a narrow window. The splits table's equal
    columns elided the pace at 125 % text in the 1000 px window, and most
    values at 150 %, even at 1200 px. Its columns are now sized by their
    content and the spare width shared, so no value is elided; at 150 %
    in the narrow window the table scrolls sideways inside its card with a
    visible scroll bar, while a vertical wheel over it still scrolls the
    page (driven under Xvfb in Spanish, the longest labels). The columns
    are whole pixels: Qt Quick Layouts round each width up, and fractional
    shares clipped the last column wherever the table fits, the default
    size included (found by the default-size capture).
  - [x] T4.5 Validation (R9): see the PR.
  - [x] T4.6 The second-reviewer pass (2026-09-24), each checked in a
    scratch Qt Quick Test with the real controls:
    - Replay sat flush against the scroll view's clipped top-right corner,
      which cut its focus ring's top and trailing sides, and the overlay
      scroll bar took clicks on its trailing edge. It keeps room for its
      ring and sits clear of the bar's widest thickness.
    - The splits card reserved its sideways scroll bar's current height,
      which grows under the pointer, so hovering the bar moved the page
      (2 px at the 12 px base). It reserves the bar's `maximumThickness`
      (2/7).
- [x] T5 (5/7 `ui/05-settings`) R6 and R4.5.
  - [x] T5.1 The grouped page:
    - FormSections of FormRows with at most two trailing controls (the
      sync progress and Cancel moved to their own row);
    - whole-row switch toggles for demo mode, reduce motion and live mode,
      each switch re-bound to its store;
    - the SegmentedControl quality, PopupButton pickers and InputField
      token;
    - one prominent or destructive button at a time;
    - the live-mode panel as a section.
  - [x] T5.2 The timezone note is the row's detail, no longer a hover-only
    tooltip (#63).
  - [x] T5.3 The logout confirmation: an AppDialog with web-string
    PushButtons in the platform order. The gate opens it (it logs a marker
    and holds for a tick), and its first run found a binding loop on the
    dialog's implicit height (fixed).
  - [x] T5.4 No dismiss button (R4.5): back, Escape and the toolbar toggle
    close the page.
  - [x] T5.5 Driven under Xvfb: a click on the reduce-motion row's label
    turned the switch on and back off, changing only the switch region.
  - [x] T5.6 Large text in a narrow window, driven at 150 % text in the
    1000 px minimum window (Spanish): the sync buttons stack onto a second
    line (a two-column `GridLayout` that drops to one column; a `Flow` put
    the second button off the pixel grid), and the rows too narrow for
    their label beside their controls stack them under it (`FormRow`,
    2/7). The quality control's segments fit under its label there; it
    may shrink into its compact pop-up form (2/7) where even they would
    not. A click on a switch itself turns it on and off once per click,
    like a click on its row.
  - [x] T5.7 Validation (R9): see the PR.
  - [x] T5.8 The second-reviewer pass (2026-09-24) found no defect in this
    layer's own code. Two older string defects show on this page and are
    tracked: the logout confirmation promises to delete cached workouts
    that the action keeps (#77), and the quality labels are Rust literals,
    English in every language (#78). The two-way distance-unit choice is
    a pop-up button where ADR 0013 calls for a segmented control; that is
    left to the owner.
- [x] T6 (6/7 `ui/06-replay`) R7 and the rest of R2.2.
  - [x] T6.1 The transport as a floating, opaque HUD over a full-route
    scene (R7.1): the round play / pause button, the `AppSlider`
    scrubber, the speed `SegmentedControl`, the metric chips (caption over
    value, in their metric colours), the race gap, and the verdict on its
    own line. The loading / error overlay uses the same surface.
  - [x] T6.2 Auto-hide (R7.2), driven under Xvfb. The HUD hides about 3 s
    into playback and comes back on a pointer move, the Right key, a pause
    or Tab. It stays up 4.5 s after opening idle and after a pause, and
    while focus is inside it. The first build never hid, because of Qt
    Quick's per-frame synthetic hover. A review then found that it hid
    while paused or idle: `Timer.restart()` overrode the timer's `running`
    binding, which the drive confirmed before the fix
    (`docs/qt-bridges-notes.md` on both). The metric chips now update in
    place instead of rebuilding per frame.
  - [x] T6.3 The sidebar is hidden during the replay and comes back at its
    width (R7.3, #62). The toolbar holds the way back and the workout's
    title, guarded so a replay loaded without selecting its workout shows
    no other workout's name. Escape closes the replay, as Back did.
  - [x] T6.4 `AppSlider` themes the replay scrubber (R2.2). The stock types
    left are containers: the screens' `Pane`s paint the palette's window
    colour, every `ScrollView` uses `AppScrollBar`, and the `SplitView`
    handle is the shell's own.
  - [x] T6.5 No camera framing inset (R7.4).
  - [x] T6.6 Validation (R9): see the PR.
- [ ] T7 (7/7 `ui/07-docs`) R8.2, the roadmap outcome, the README
  screenshots; #47 superseded, with links to the stack.
