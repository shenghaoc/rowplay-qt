# Native macOS R8.1 verification

Verified on 2026-09-26 on Apple M5 / macOS 27, Qt 6.11.2, Cocoa + Metal,
Rust 1.98.1 and qtbridge 0.2.0. The tested composed #118 code is
`edf275699aedde8ed56c778ae654595186d50551`. The subsequent #117 evidence
commit and #118 rebase change documentation only; the executable sources,
resources and build configuration are identical to the tested tree.

## Full native gates

Both runs used the full profile with phase shots and close-ups, a visible
native window and isolated demo data:

```sh
source .envrc
QT_QPA_PLATFORM=cocoa QSG_RHI_BACKEND=metal \
  ROWPLAY_FORCE_COLOR_SCHEME=light \
  ROWPLAY_PHASE_SHOTS=1 ROWPLAY_PHASE_CLOSEUPS=1 \
  ROWPLAY_SMOKE_SCREENSHOT_DIR="$PWD/artifacts/native-light" \
  ROWPLAY_SMOKE_ARTIFACT_DIR="$PWD/artifacts/native-light" \
  cargo test -p rowplay-app --test qml_runtime_gate -- --nocapture
```

Repeat with `dark` and a separate artifact directory.

| Scheme | Tests | App-log duration | Test duration | Replay entry |
| --- | --- | --- | --- | --- |
| Light | 3 passed | 107.0 s | 111.69 s | 0.2 s |
| Dark | 3 passed | 106.4 s | 110.47 s | 0.2 s |

No failed or starved holds. Both runs produced 70 captures and passed the
shadow, equipment, grip, venue and six compact locale assertions
(en/zh/de/es/fr/ja). The structured race gap and four metric chips remain
contained. These are fresh macOS results, separate from earlier Linux and
Windows evidence.

## Layout and text matrix

Five width classes × English/Spanish/Japanese × two text scales × two
schemes: **60 combinations per state, 960 distinct cases across 16 states**.
The states derive from the shell/gate: dashboard top/bottom; detail
top/bottom, with comments and without strokes; settings top/bottom; all
three replay sports with rivals; workout list/drawer; sort menu; About;
logout confirmation; and the filtered timezone popup.

| Class | 100% window width | 150% window width |
| --- | ---: | ---: |
| Compact | 480 | 720 |
| Medium | 720 | 1080 |
| Expanded | 1000 | 1500 |
| Large | 1440 | 2160 |
| Extra-large | 2048 | 3072 |

These are achieved logical dimensions, not guessed breakpoints. Each case
asserted `Theme.widthClass`. Window height was normally 800 / 1200 logical
pixels; the first 150% window was initially constrained to 852 by macOS
before subsequent resizing. Very wide native windows extend beyond the physical
display; full Qt item captures cover their complete content.

The temporary probe set the window's inherited native-control font to
13 / 19.5 points and the Theme font input to 13 / 19.5 logical pixels.
Both values, including the actual native ComboBox font, were logged in each
case. This is a controlled application-font test, not a change to macOS's
preferred-reading-size setting. The QPA remained Cocoa and the renderer
Metal. Temporary probe and font-input changes were restored after each run.

All accepted cases recorded an active native window, correct width class,
contained HUD/gap, centred and capped content, and no layout overlap.
`contentMaxWidth` stopped dashboard/detail content at 1440 / 2160;
settings and workout comments stopped at `readableWidth`, 640 / 960.
Large/extra-large feed and supporting-pane layouts activated when their
content columns had room. Top/bottom captures and text-bound traversal
covered scrollable content, controls and separate popup content. Captures
were checked for missing/blank output, and representative captures plus
all flagged categories were inspected visually.

There was no native-control label truncation or overflowing Spanish/
Japanese HUD text. At 150%, Qt Graphs' three category-axis text items report
18-pixel painted height inside 15-pixel item bounds. They have `clip:false`;
the full painted bounds fit their clipping ancestors when scrolled into
view (for example y=589..607 inside y=79..1200). Captures show complete
letters and descenders. These are item-bound diagnostic flags, not clipped
text; they were retained in the raw results rather than silently filtered.

One earlier dark attempt lost activity in its last two timezone cases
when the Mac locked, and the next light-150 attempt started locked. Those
attempts were excluded. The later dark rerun stopped safely after 184
active cases at another lock; its remaining 56 cases passed after unlock.
Only active-window cases enter the final 960-case set. The guard stops
before recording an inactive case. The light-150 comment state was added
in a separate 15-case supplement. All 1,500 matrix captures decoded
successfully, with no missing or blank main-content capture.

## Real input

QtTest `TestEvent` delivers mouse and keyboard events to the native window;
property setters are used only to arrange scenarios. The checks cover
compact/medium drawer opening, selecting a workout and closing the drawer,
a settings switch through its label, Tab traversal, clicking Replay,
clicking above the scrubber's visible rectangle, rejection outside its hit
mask, mouse speed selection, global speed shortcuts, subsequent Left/Right
from the newly focused speed, Space on a speed without toggling play, and
mouse/Space play activation. The macOS play button already occupies its
40-pixel target; its edge was clicked inside that target.

The 100% light and 150% dark real-input runs each passed all 17 assertions.

## Actual system settings and native menus

Computer Use changed macOS System Settings, with no app colour/contrast
overrides:

- Purple accent, Light: native switch, selection and focus ring followed
  the system palette; PM5 metric colours retained their semantic roles.
- Increase Contrast on, Light and Dark: inspected settings, sidebar/drawer,
  About and replay, including Japanese content. Enabled product text
  contrast measured at least 14.93:1 (light) / 16.29:1 (dark) for primary
  and secondary text; metric minima were 5.56:1 / 4.62:1. Dark distance blue
  used the documented high-contrast white fallback. Focus and borders
  remained visible. Disabled text and platform-drawn controls retain the
  documented native-style exception.
- Actual application menu: About opened its localized dialog; Preferences
  and Cmd+, opened settings; Cmd+Q and clicking Quit each exited the
  process. In Japanese, the native role titles stayed English, matching
  #80. The test app bundle was named Rowplay QA; this was a debug native
  menu check, not release-package approval.
- Original settings were recorded and visibly restored: Appearance Auto,
  accent Multicolour, highlight Automatic and Increase Contrast off. No
  text-size setting was changed, and other accessibility preferences were
  left as recorded.

## Other validation and scope

Fmt and both clippy scopes passed. `cargo test` passed 585 tests (two
existing ignored tests); app tests excluding the separately run native
walk passed 27 tests. The focused grouping regression passed, and all three
`i18n_parity` tests passed. `git diff --check` passed. No product fix or
functional change was needed for this verification.

The existing Shortcut multiple-binding diagnostic and Qt Graphs axis-colour
diagnostic can appear; neither is a new QML runtime error, and the
uninstrumented full gates pass. Known native-menu translation and
platform-owned disabled-colour/motion exceptions remain documented.

Raw logs, per-case JSON, captures, temporary probes and the original/restored
System Settings evidence are retained as the local `native-r81` QA artifact
set, referenced in the handoff. Screenshots and machine-local paths are not
committed. Instrumented QML was restored byte-for-byte after each run.
