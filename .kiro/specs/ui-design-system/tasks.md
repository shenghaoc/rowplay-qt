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
    - the macOS menu bar with About / Settings… / Quit roles, created only
      on macOS;
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
- [ ] T4 (4/7 `ui/04-dashboard-detail`) R5.
- [ ] T5 (5/7 `ui/05-settings`) R6 and R4.5.
- [ ] T6 (6/7 `ui/06-replay`) R7.
- [ ] T7 (7/7 `ui/07-docs`) R8.2, the roadmap outcome, the README
  screenshots; #47 superseded, with links to the stack.
