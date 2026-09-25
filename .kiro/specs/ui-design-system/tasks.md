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
  - [x] T6.7 The second-reviewer pass (2026-09-24): `AppSlider` centred its
    track and knob on a half pixel at some font sizes (the knob at y 2.5 at
    the 12 px base); both round. Space on the focused Close button toggled
    playback, because the replay's window shortcut took the key; 2/7's
    `ToolbarButton` now claims Space while focused (T2.10). The spec's
    layer list and R2.1 now name `AppSlider`.
- [x] T7 (7/7 `ui/07-docs`) R8.2, the roadmap outcome, the README
  screenshots; #47 superseded, with links to the stack.
  - [x] T7.1 README: one current set of five screenshots (256-colour PNGs,
    240 KB) replaces `docs/screenshots/phase-04-*` (514 KB). The Phase 5a
    replay shots leave the README but stay on disk: the Phase 5a spec
    records the row, ski and bike captures, and
    `phase-05a-replay-row-dark.png` is now referenced nowhere. The status list gains a design-system bullet.
  - [x] T7.2 AGENTS.md: the shared-controls rule beside the QML style
    line, and three operational lessons (a capture's alpha; thresholds
    compared unrounded; a fix aimed at one configuration checked in all
    the others).
  - [x] T7.3 `docs/roadmap.md`: the stack's outcome. `docs/source-map.md`:
    the three reduce-motion rows become one current row, and the timezone
    picker's note says what "4b polish" left undone (the region groups).
  - [x] T7.4 #47 closed unmerged, with a comment linking the stack; its
    branch kept.
  - [x] T7.5 The outcome in `docs/roadmap.md` records the Codex reviews of
    #68 and #70 (T3.7, T2.9 and 1/7's two review commits).
  - [x] T7.6 The second-reviewer pass (2026-09-24): the outcome records its
    findings (T2.10, T3.8, T4.6, T5.8, T6.7) and the native macOS check
    (#66). Windows is split out to #81. T7.1 and T7.3 are corrected: one
    Phase 5a capture is referenced nowhere, and the timezone note rewords
    "4b polish" rather than dropping it.

## Round 2 — Windows, KDE and HarmonyOS guidance (2026-09-25)

Rules the Apple and GNOME guidelines did not cover, from Microsoft's
Windows and Fluent guidance (WinUI as the reference implementation), the
KDE HIG and HarmonyOS's layout breakpoints, each checked against the code
first. A stack of five PRs, merged bottom-up: `ui/focus-ring-high-contrast`
(#100), `ui/min-text-cjk` (#101), `ui/breakpoints-min-window` (#103),
`ui/shortcut-tooltips-timezone-errors` (#105) and
`docs/design-system-round-2`.

- [x] T8 (`ui/focus-ring-high-contrast`) The two-tone focus ring and high
  contrast from the system palette (R1.6, R2.4).
  - [x] T8.1 `FocusRing` draws a 1 px inner band in the window colour and
    a 2 px outer band in the accent (the text colour under high contrast),
    3 px outside the control instead of a 2 px band after a 2 px gap.
    Every control draws it through that one component.
  - [x] T8.2 Under the OS contrast preference, `Theme` takes every role
    from `SystemPalette` (Active and Disabled), composited to opaque
    colours, and keeps a metric or status colour only where it reaches
    4.5:1 on the window. The scheme follows the palette. The prominent
    button and an on switch keep their outline, an unfocused sidebar
    selection takes a 2 px highlight outline. Content on a control's own
    fill takes the button text (`Theme.controlText`), since a contrast
    theme may set it apart from the window text (Codex review).
  - [x] T8.3 2 px outlines under high contrast only: the sidebar's edge,
    the toolbar's rule, cards and chart panels, grouped forms, popups,
    dialogs, tooltips and the replay HUD.
  - [x] T8.4 `ROWPLAY_FORCE_COLOR_SCHEME` also asks Qt for its scheme, so
    forced high contrast can be checked in dark on macOS.
  - [x] T8.5 Checked natively on macOS with a scratch probe that gives
    Tab focus to twelve real controls (the prominent button, the sidebar
    list, the search field, the sport filter, a switch, the quality
    control, a pop-up button, the dialog's buttons, the HUD's play button,
    scrubber and speed control), in light, dark and forced high contrast
    in both: the ring's pixel runs read two bands, and the colours match
    the palette Qt reports. macOS's "Increase contrast" itself is for the
    owner to switch (the probe cannot), and Windows' four contrast themes
    are on #81's checklist.
- [x] T9 (`ui/min-text-cjk`) Minimum text sizes and CJK (R1.3).
  - [x] T9.1 `Theme.textFloor` under `fontPx`: 12 px on Windows and Linux,
    11 px on macOS, 12 px in Chinese and Japanese. Captions it lifts to the
    body's size take regular weight (`Theme.floored`).
  - [x] T9.2 A scratch probe walked every visible text item on the
    dashboard, detail, settings and replay screens in English, Chinese,
    Japanese and Spanish, and listed the elided ones and the ones whose
    text overflows their box. Three configurations: macOS natively (13 px
    base), and the offscreen platform at 12 px and at KDE's 14 pt test
    font (18 px) through `QT_FONT_DPI`, which cocoa ignores.
    - Before the floor: nothing flagged natively and at 12 px; the smallest
      text was 10 px natively and 9 px at 12 px.
    - With the floor alone, Chinese and Japanese sidebar rows elided their
      distance at 12 px. The date and the distance now sit in a `Flow`,
      and the distance moves under the date where the two do not fit.
    - Found at every size, the floor aside: the dashboard's trend chart
      labelled every point: English dates touched at the default size and
      overlapped at 18 px, Japanese ones overlapped at the default size
      already. It labels every n-th
      point now, anchored on the newest, with half a label of margin so
      the newest one stays whole.
    - Still flagged at 18 px, and not a defect: the bar chart's category
      labels overflow the 15 px boxes Qt Graphs gives them, which do not
      clip (checked in the capture).
- [x] T10 (`ui/breakpoints-min-window`) Width breakpoints and a smaller
  minimum window (R3.5).
  - [x] T10.1 `Theme.widthClass` with HarmonyOS's breakpoints: compact
    below 600 px, medium below 840 px, large from 840 px, all scaled with
    the text. Large is the layout as it was.
  - [x] T10.2 Below large the one sidebar moves into `AppDrawer`: modal
    over the content at medium, and at compact the list's page under the
    toolbar, not modal, so the shell's shortcuts stay live. A leading
    toolbar button (a new `sidebar.left` glyph, named with the web's
    `dashboard.sectionWorkoutsEyebrow`), the sidebar toggle and Find open
    it, and a chosen workout closes it. The sport filter takes its
    compact form.
  - [x] T10.3 Grids fall to one column where two do not fit. The detail's
    rate and heart-rate charts, and the replay HUD's speed and chips,
    stack where they do not fit side by side.
  - [x] T10.4 The minimum window drops from Studio's 1000 × 680 to
    480 × 480, the width scaled with the text.
  - [x] T10.5 The gate walks medium and compact (steps 213–229): the
    drawer, the list's page, settings, the dashboard and the replay. It
    asserts that a chosen workout closed the drawer and that the sidebar
    returned beside the content.
  - [x] T10.6 A scratch probe walked the dashboard, the detail, settings,
    the replay HUD and the list at large, medium and compact, in English,
    Spanish and Japanese, light and dark: natively on macOS (13 px) and on
    the offscreen platform at 150 % text (18 px), 180 combinations. It
    listed every text that was elided, overflowed its box or ran past the
    window's edge. Natively nothing; at 150 % only the bar chart's
    category labels, as in T9. Its first run found the HUD's second row
    running past the HUD's edge at 480 px, fixed in T10.3. Forced high
    contrast was checked on the drawer, the list's page and the HUD.
  - [x] T10.7 A second probe pressed real keys and clicked through QtTest's
    `TestEvent` in the running app (offscreen): the sidebar toggle, the
    arrows, Enter, Escape from the list and from the search field, Find,
    Ctrl/Cmd+1, the drawer button, a row, the wheel over the list and the
    dimmed strip, at medium and compact. Every path behaves as designed.
    Its first run found that Escape and a click on the strip left the
    medium drawer open: Qt 6.11 ties both to a popup's `interactive`,
    which `AppDrawer` had turned off to stop edge drags. It stays
    interactive now, with `dragMargin: 0`.
  - [x] T10.8 Large unchanged: the native gate walks (light, phase shots
    and close-ups) before and after differ in every shared capture at
    1200 × 800 only at a channel delta of 1: at most 618 px on a 2D screen
    and 165 px on a 3D capture, and no pixel by more. The repository's
    noise bounds (AGENTS.md, "Capture noise") are measured on Linux and do
    not apply to these macOS walks. The reference here is one pair of
    walks of an unchanged `main` on the same Mac, which differed by up to
    664 px (2D) and 144 px (3D), also at delta 1: a single pair, not a
    bound. A delta of 1 on an 8-bit channel is below any visible change,
    so the large layout is read as unchanged. The first comparison caught
    the race gap 1 px lower in every capture with a ghost (delta 223):
    the HUD's chips had moved into a nested layout, which rounds its own
    centring. The chips' row now takes the speed control's height beside
    it.
  - [x] T10.9 The drawer button, the sidebar toggle and the in-drawer
    shortcuts follow the drawer's target state (`AppDrawer.shown`, set as
    a transition starts). Bound to `visible`, the button stayed checked
    through the 200 ms closing slide, so a capture taken then showed it
    checked, and a click during the slide did nothing (found comparing
    the next PR's native walk with this one's).
- [x] T11 (`ui/shortcut-tooltips-timezone-errors`) Shortcuts in tooltips,
  the timezone filter, and the menu, error and accessible-name audit
  (R4.6, R5.4, R6.4, R6.5).
  - [x] T11.1 `ToolbarButton.shortcutText`: the tooltip names the
    shortcut in the platform's notation, from each `Shortcut`'s
    `nativeText` (Reload, Account & data, the drawer, the replay's Close
    and Play). Read natively on macOS: "Reload (⌘R)", "Workouts (⌃⌘S)",
    "Close (⎋)"; the offscreen platform, with no macOS key scheme, gives
    "Reload (F5)".
  - [x] T11.2 The timezone picker filters as you type (`PopupButton`
    `filterable`, the view-model's `timezone_matches` with unit tests), its
    field named with `workoutList.search`. The language picker stays
    plain.
  - [x] T11.3 Errors and status, audited path by path: the cache error
    (the list, Reload retries), the token errors (the token row), a
    failed sync (now "Sync failed" above its error, with "Retry sync" in
    the row), the live-mode errors (their panel, with automatic retry and
    Check now), a refused date (now its own field, with the form it takes
    under it), a replay that fails to load (the scene's overlay, with
    Close). No success opens a dialog; the Replay button's disabled state
    still explains nothing (#64, no web string).
  - [x] T11.4 The application menu holds Dashboard, Search, Reload and
    Account & data: no Quit or window-management items (checked, no
    change).
  - [x] T11.5 Accessible names: a sidebar row leads with its title (Studio
    led with the sport, which many rows share), a split row with its
    number, the table's title as its description. The detail header's
    name said "intervals" in English in every language; the tag is
    translated now.
  - [x] T11.6 A scratch probe pressed keys and clicked through QtTest's
    `TestEvent`, natively and offscreen: the tooltip on hover, "nope" typed
    into From (refused alone, the message under it), the filter opened by
    typing on the closed picker, Escape, Space, "-05" and Enter choosing
    Toronto, the focus back on the picker, the failure row with its
    retry, and every name above. The gate walks the date range through
    the fields and the timezone filter.
  - [x] T11.7 Access keys (mnemonics): assessed only, in #104. Every
    string is a web key and the web has no `&`; the letters would need a
    desktop supplement per language, a uniqueness test and a mnemonic mode
    in three shared controls.
- [x] T12 (`docs/design-system-round-2`) ADR 0013's round-2 notes, the
  AGENTS.md style line and the documentation pass.
  - [x] T12.1 ADR 0013's "Round 2 notes": what the round adopted, and what
    it did not, with the reasons. Not adopted: Mica and acrylic, Kirigami
    and qqc2-desktop-style, and HarmonyOS Sans with the Huawei visual
    language. For Plasma it records what the sources say: the accent
    follows the colour scheme, but no KDE theme reports a contrast
    preference. It lists what was checked where.
  - [x] T12.2 AGENTS.md: the shared-controls line names drawers
    (`AppDrawer`; never a raw `Drawer`), and a new line gives round 2's
    rules: `FocusRing`, `Theme.fontPx` and the floor, colours from the
    tokens, `Theme.widthClass`.
  - [x] T12.3 The bridge notes' per-OS entry: Plasma sets the accent
    through KDE's own platform theme (qtbase's themes do not), and neither
    KDE theme reports a contrast preference.
  - [x] T12.4 Not tried on a Plasma desktop (none here, and the Linux CI
    leg has no desktop). Linux rendering was looked at in the CI captures
    of #100, #101, #103 and #105 (Xvfb + llvmpipe, a 12 px system font).
    The CI run times stayed usual (the Ubuntu App job 5.4 min on #103's
    run), so no slow-run issue was filed.
