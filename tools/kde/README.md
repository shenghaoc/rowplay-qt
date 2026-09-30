# tools/kde/ — Fedora KDE Plasma native acceptance

`acceptance.py` turns the Plasma checks behind [ADR 0018](../../docs/decisions/0018-plasma-integration-through-qt-and-freedesktop.md)
into one repeatable, evidence-producing run, so a Qt, KDE, Mesa or toolchain upgrade is cheap to
re-validate. Nothing here is linked into the app, and nothing here adds a dependency to it: the harness
is Python's standard library, the tools Plasma and Fedora ship, `cargo`, `podman` and `pyatspi`
(only for the exact-AppImage walk).

It is a *local, native* acceptance. GitHub-hosted runners are not a Plasma desktop on a real GPU, and an
Xvfb job is not Plasma-native evidence. CI runs only what is honest there: the harness's unit tests
(`python3 -m unittest discover -s tools/kde/tests -t tools/kde`, a step of the Qt-free job), the generic
Linux gate, the release quick gate and the packaging.

The unit suite also runs on macOS: baseline paths are canonicalised, and helper-process checks use
the shared BSD/procps `pgrep` flags and recognise macOS's `Python` executable. This validates the
harness logic; it is not evidence from a live Plasma session.

## Running it

```bash
source .envrc                                   # the repository's Qt 6.11 (aqt), as for every rowplay-app command
tools/kde/acceptance.py --help                  # every mode and option
tools/kde/acceptance.py --dry-run all           # the plan; runs and changes nothing
tools/kde/acceptance.py probe guards services   # quick, read-only
tools/kde/acceptance.py all --allow-session-changes --calibrate-noise \
    --output artifacts/kde/acceptance-$(date +%Y%m%d-%H%M%S)
```

`--qt-dir` selects the Qt installation for probes, gates and both package builds. The Ubuntu wrapper
receives that resolved directory explicitly and mounts it read-only; the baseline and branch use the same Qt.

Run `all` from a **clean** branch worktree with a clean checkout of the **acceptance baseline** beside it
(`--baseline-tree`, formerly `--main-tree`; default: a worktree already at that commit); `guards` fails a
dirty tree. The baseline is a *commit*, `046c2e3` (post-#144 `main`, before any Plasma change), not "whatever `main` is today": once the
stack's layers merge `main` contains the change, so a comparison with it would compare the branch with
itself. A detached worktree at that commit is the intended checkout (`git worktree add --detach ../baseline 046c2e3`); its
branch name does not matter, and `guards` fails a checkout at any other commit, the tree under test used as its own
baseline, and a tree that does not descend from the baseline. `--baseline-sha` names another commit when the
integration is re-baselined deliberately. `--dry-run` prints the plan and what baseline the stages would need,
and resolves nothing. It never uses `git stash`, needs
no root, and each `cargo` runs in its own tree, so each worktree keeps its own Cargo target directory.

| stage | what it measures |
| --- | --- |
| `probe` | the host (Fedora, kernel, Plasma, KWin, KF, host Qt, bundled Qt, Mesa, GPU, portals), and — through the bundled Qt's own `qml` tool, in a window that is never shown — the platform theme Qt created, the palette, `Accent` vs `Highlight`, scheme, contrast, font, icon theme, and what the repository's real `Theme.qml` resolves. Whether Qt's `Accent` is unusable decides which branch of `Theme.resolveAccent` must have run; nothing names a desktop. |
| `guards` | clean trees; #143's `[profile.release.package.qtbridge-interfaces] opt-level = 0` intact and identical to the baseline's; no new lockfile package; no KDE/KF/Kirigami crate among `rowplay-app`'s production dependencies and no new dependency in core, platform or view-model; the Blender stack untouched; `git diff --check`. |
| `services` | URL opening, file chooser, notifications, tray, global menu, MPRIS and the Secret Service, classified from the source: *Qt/XDG sufficient*, *not applicable* or *needs code* (a failure). |
| `native` | the repository's own gate, unmodified, natively on Wayland with hardware GL: debug quick and release quick (#144's: fails on `BorrowError`, `role_names`, a panic or an abort) and full with phase shots and close-ups on the branch, and full at the acceptance baseline for the visual baseline. `LIBGL_ALWAYS_SOFTWARE` is removed and a software renderer fails the run. |
| `visual` | the spatial contract below. |
| `generic` | CI's Linux recipe under Xvfb with every desktop variable removed: no KDE theme is created, Fusion is the style, the palette falls back by the same rule; the captures against the acceptance baseline's within capture-diff's own bounds; the X11 identity properties. |
| `package` | both AppImages built by `tools/package/linux.sh` unchanged in `ubuntu:24.04` (`ubuntu-package.sh`), each with its own target directory; size, SHA-256, file count, plugin and QML inventory; a scan of every path and every ELF `NEEDED` entry for KDE Frameworks, Kirigami, Plasma, `org.kde.desktop`, Breeze QML, KConfig, KI18n, KIO; the launch check; `desktop-file-validate` and `appstreamcli`. Fedora-host packaging is not canonical: linuxdeploy corrupts RELR-packed libraries there. |
| `identity` | (`--allow-session-changes`) the exact branch AppImage launched through a temporary desktop entry: KWin's `desktopFileName`, one window, then two windows with one association, the AT-SPI walk below, a clean shutdown, no core dump, the app's own Qt log (theme, icon theme, themed icon lookups). The entry and icon are removed and any previous ones restored byte for byte. |
| `appearance` | (`--allow-session-changes`) a loud accent, a dark scheme and the general font at 150 %, each verified to reach a fresh Qt process and each followed by a native gate; then restoration, verified. |
| `checks` | the harness's tests, `cargo fmt`, `cargo clippy --workspace`, `cargo test`, `cargo test --workspace`, `git diff --check`. |

Commands run in one of three environments, never mixed: **host** (`host_env`: the host's `plasmashell`, `kwin_wayland`, `rpm` and Qt run
with every Qt loader, plugin-path, platform-theme and style variable of the caller removed, among them `LD_LIBRARY_PATH`, `QT_PLUGIN_PATH`,
`QML_IMPORT_PATH`, `QML2_IMPORT_PATH`, `QT_QPA_PLATFORM_PLUGIN_PATH`, `QT_QPA_PLATFORMTHEME`, `QT_STYLE_OVERRIDE` and `QT_QUICK_CONTROLS_STYLE`),
**bundled Qt** (`bundled_qt_env`: the host environment plus only the repository Qt's own library directory, and the platform variables a
stage sets explicitly) and **generic Linux** (`generic_env`: the host environment with every desktop variable removed as well). A command
that times out is recorded as `timed_out` with whatever it had printed. The probe's icon theme must be *measured* (any theme Qt names
counts; none measured fails the check).

Every check ends in `PASS`, `FAIL`, `SKIPPED` (a flag was not given), `MANUAL_OPTIONAL` (automation cannot
do it safely), or `UNAVAILABLE` (the machine lacks what it needs). Only `FAIL` fails a run; a stage that
raised fails it too, and the exit status is nonzero.

## Evidence

Each run writes one self-contained directory (`artifacts/kde/acceptance-<timestamp>/`, git-ignored):
`manifest.json` (the source of truth), `summary.md` (generated from it), `host.json`, `commands.log` (one JSON
line per command, with exit status and time), and a subdirectory per stage with the gate logs, captures,
probe JSON, package inventories, KWin and AT-SPI reports and the representative screenshots for a person to
look at (`appearance/screenshots/{accent,dark,font150}/`, `identity/*-fullscreen.png`).

## The visual contract (`expected-visual-diff.json`)

`tools/capture-diff.py` says whether a capture is within the repository's noise bound. A *meant* change (the
replay scrubber's fill turning from black to the accent, a theme icon replacing ours, a focus ring) is far
outside it, and waving those captures through would also wave through a wrong pixel beside them. So each
expected change is described spatially:

* `regions`: inclusive rectangles where pixels may change;
* `bands`: the outline of a rectangle, some pixels thick, never its interior (a focus ring);
* `must_change`: the capture must actually change inside the allowed area, so a fix that quietly stops
  working fails as loudly as a stray pixel.

A capture with no rule is held to capture-diff's own bound. A pixel is *changed* when its channel delta
exceeds that capture type's noise delta. Acceptance requires **zero unexpected pixels**. Native
hardware-GL captures carry delta-1 dithering the Xvfb + llvmpipe bound was not measured on, so the file also holds a
named noise profile with its calibration (two same-tree runs: 2D up to 450 px, 3D up to 76 px, all delta 1);
`--calibrate-noise` re-measures it on every run. The global bounds are untouched.

What the two committed sets record (each derived from a real pair, both on Fedora 44 Plasma 6.7.5, hardware GL):

* **`baseline-vs-branch`** (`baseline_sha` `046c2e3`), the branch's full native walk against the acceptance baseline's (post-#144 `main`): 40 of 70 captures exceed the
  generic noise bound, in three families and nowhere else. The 30 replay captures (`phase-*`, `replay-gap-*`)
  differ only in the scrubber's fill, `#000000` → the accent, plus a dozen anti-aliasing pixels beside it; the
  9 toolbar screens differ only in the 16×16 Settings icon (our glyph replaces a full-colour category icon,
  where the baseline drew a solid silhouette); and `detail-nostrokes` has the sidebar list's focus ring, a band 5 px
  thick with no interior, going from black to the fitted accent (its toolbar icon aside). The other 30 are noise.
* **`accent-vs-default`**, the quick walk under Plasma's default accent against a loud one: 6 of 14 captures
  change, in four accent-driven places: the prominent Replay button's fill, a settings switch, the selected
  sidebar row's highlight and the focus ring.

The rules are **derived from real capture pairs**, never guessed (`derive_rules.py`), and a changed rule is a
reviewed change: the diff of that file shows which pixels a change may now move. Re-derive after an
intended visual change, look at the family summary it prints (the commonest before→after colours), and
commit the file with the change. A ring is a cluster with at least 98 % of its pixels within a few pixels of its
border: the band is as thick as those border pixels, and the few deeper ones stay exact, as padded regions of their own.

**The rules belong to one baseline.** `baseline-vs-branch` describes a change made on top of `046c2e3` and records it as
`baseline_sha`; once the stack's layers merge, `main` already contains that change and applying the rules to it would
report dozens of missing changes. The `native` stage therefore writes a `provenance.json` (role, commit, clean or not) beside
every capture directory it takes, and `visual` checks the *baseline captures'* provenance against the rules' `baseline_sha`,
and the rules' against this run's baseline, before it applies one rule. A mismatch fails once, by name (`baseline mismatch`);
missing provenance fails as an evidence error saying what to add. Neither needs a Git worktree, so
`acceptance.py visual --baseline-captures DIR --branch-captures DIR` classifies copied evidence on a machine with one
checkout (`--main-captures` is the old name). `derive_rules.py` takes the commit from the `before` directory's provenance
(and refuses a dirty tree or a disagreeing `--baseline-sha`); `--stamp-baseline SHA` records it on a set derived earlier without
touching a coordinate. A set derived between two states of one tree (`accent-vs-default`) names no baseline.

## Driving the exact AppImage (`atspi_walk.py`)

Wayland gives an unprivileged process no key synthesis: `ydotool`/`wtype`/`xdotool` are not installed,
`/dev/uinput` is root-only, and the RemoteDesktop portal needs a consent dialog every session. The harness
adds none of them. Instead the exact AppImage is launched with `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` (an
environment variable, no app change) and driven through AT-SPI: the walk moves **focus** between the search
and both date fields and invokes **actions** (Replay, Play, Pause). Leaving a date field is what a Tab does
to it, and it runs the same `editingFinished` → `Library.setDateRange` → model reset path issue #143
aborted release builds on. The key **chords** are covered where key events are real, in the gate: `GateKeys.qml`
sends `QKeyEvent`s (Ctrl+F, Tab and Shift+Tab through both date fields, Esc, the platform's Refresh, Preferences,
Back and sidebar chords (read from the shortcuts, F5/F9/Alt+Left on Linux and Windows, ⌘R/⌃⌘S/⌘[ on a Mac),
Space, Left, Right, `[`, `]`, and a real Ctrl+Q ending the walk) through the same key
path, on the same release code (the release quick gate), and `qml_runtime_gate.rs` asserts every one.

If fewer than three editable, focusable text fields are exposed, that requirement fails and the date checks
are skipped. The independent Replay, Play and Pause checks still run, without a second generic exception.

## Restoring the desktop, and interruption

`appearance` and `identity` change the live session, so each is a transaction (`kdeacc/plasma.py`): snapshot,
change, and on every way out (success, failed check, exception, SIGINT, SIGTERM) re-apply the snapshot's
colour scheme with `plasma-apply-colorscheme`, put the snapshot's exact `kdeglobals` bytes back, then re-read
everything and compare it with the snapshot: the file's SHA-256, the current scheme, the portal's appearance
keys, the general font, and what a fresh bundled-Qt process reports. Both steps are needed: `kwriteconfig6
--delete` writes a `key[$d]` marker and the colour tool an explicit `ColorScheme=` line, neither of which is
what the file held before. A restoration that does not verify **fails the run** and the summary lists what is
still changed. Panel pinning is never touched; it stays a manual optional smoke.

## Not automated, and why

* **Task Manager pin/unpin** is the user's panel configuration. The run records whether the entry is pinned
  and marks the check `MANUAL_OPTIONAL`.
* **Key chords into the packaged binary** (see above): covered by the gate on the same code path and by
  AT-SPI focus/actions on the exact AppImage.
* **Whether the screenshots look right** is a person's judgement; they are saved for that.

**Interruption.** SIGINT, SIGTERM and SIGHUP (a closed terminal) end a run the same way, whichever stage they arrive in, including inside
the appearance transaction. The transaction restores and verifies the desktop first; that stage, with its restoration evidence, is then
recorded in the manifest and summary; **no later stage runs** (`checks` in particular); the processes this run launched are ended by PID
(never by the generic AppImage wrapper's name); the leftovers check and the manifest are written; and the command exits `128 + signal`
(130, 143, 129), the manifest's `overall` being `INTERRUPTED`.

Identity teardown uses the same owned-PID cleanup: TERM, a bounded grace period, then KILL and a wait for exit.
Requiring forced cleanup still fails the clean-shutdown check; a zombie awaiting reaping is not a running leftover.
Zombie state comes from Linux's `/proc`, or `ps` on macOS; neither path reaps another caller's child.
The fake Ubuntu fixture supplies its own release record and never reads the validation host's distribution file.

**Gate variables.** A run the harness calls a gate owns every variable that changes what the walk does (`gates.GATE_VARS`: profile, smoke and
exit-after-frames hooks, scale and font, renderer, capture directories, forced scheme and contrast): it sets the value it means or removes the
caller's, so a shell exporting `ROWPLAY_GATE_PROFILE=quick` cannot turn the generic walk, which is the *full* profile, into a quick one.
`ROWPLAY_GATE_NO_WINDOW_MANAGER` is one of them: the generic Xvfb walk sets it (no window manager, so the keyboard contract's
inactive-window skip is expected), and a native walk removes it, so an inherited value can never excuse a skipped contract on Plasma.
The X11 identity probe runs with the Qt given by `--qt-dir`.
