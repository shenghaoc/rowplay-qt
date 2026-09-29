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

## Running it

```bash
source .envrc                                   # the repository's Qt 6.11 (aqt), as for every rowplay-app command
tools/kde/acceptance.py --help                  # every mode and option
tools/kde/acceptance.py --dry-run all           # the plan; runs and changes nothing
tools/kde/acceptance.py probe guards services   # quick, read-only
tools/kde/acceptance.py all --allow-session-changes --calibrate-noise \
    --output artifacts/kde/acceptance-$(date +%Y%m%d-%H%M%S)
```

Run `all` from a **clean** branch worktree with a clean checkout of `main` beside it (`--main-tree`,
default: the worktree on branch `main`); `guards` fails a dirty tree. It never uses `git stash`, needs
no root, and gives each worktree its own Cargo target directory (each `cargo` runs in its own tree, and
each container build gets its own directory under `~/.cache/rowplay-ubuntu2404-target/`).

| stage | what it measures |
| --- | --- |
| `probe` | the host (Fedora, kernel, Plasma, KWin, KF, host Qt, bundled Qt, Mesa, GPU, portals), and — through the bundled Qt's own `qml` tool, in a window that is never shown — the platform theme Qt created, the palette, `Accent` vs `Highlight`, scheme, contrast, font, icon theme, and what the repository's real `Theme.qml` resolves. Whether Qt's `Accent` is unusable decides which branch of `Theme.resolveAccent` must have run; nothing names a desktop. |
| `guards` | clean trees; #143's `[profile.release.package.qtbridge-interfaces] opt-level = 0` intact and identical to `main`'s; no new lockfile package; no KDE/KF/Kirigami crate among `rowplay-app`'s production dependencies and no new dependency in core, platform or view-model; the Blender stack untouched; `git diff --check`. |
| `services` | URL opening, file chooser, notifications, tray, global menu, MPRIS and the Secret Service, classified from the source: *Qt/XDG sufficient*, *not applicable* or *needs code* (a failure). |
| `native` | the repository's own gate, unmodified, natively on Wayland with hardware GL: debug quick and release quick (#144's: fails on `BorrowError`, `role_names`, a panic or an abort) and full with phase shots and close-ups on the branch, and full on `main` for the visual baseline. `LIBGL_ALWAYS_SOFTWARE` is removed and a software renderer fails the run. |
| `visual` | the spatial contract below. |
| `generic` | CI's Linux recipe under Xvfb with every desktop variable removed: no KDE theme is created, Fusion is the style, the palette falls back by the same rule; the captures against `main`'s within capture-diff's own bounds; the X11 identity properties. |
| `package` | both AppImages built by `tools/package/linux.sh` unchanged in `ubuntu:24.04` (`ubuntu-package.sh`), each with its own target directory; size, SHA-256, file count, plugin and QML inventory; a scan of every path and every ELF `NEEDED` entry for KDE Frameworks, Kirigami, Plasma, `org.kde.desktop`, Breeze QML, KConfig, KI18n, KIO; the launch check; `desktop-file-validate` and `appstreamcli`. Fedora-host packaging is not canonical: linuxdeploy corrupts RELR-packed libraries there. |
| `identity` | (`--allow-session-changes`) the exact branch AppImage launched through a temporary desktop entry: KWin's `desktopFileName`, one window, then two windows with one association, the AT-SPI walk below, a clean shutdown, no core dump, the app's own Qt log (theme, icon theme, themed icon lookups). The entry and icon are removed and any previous ones restored byte for byte. |
| `appearance` | (`--allow-session-changes`) a loud accent, a dark scheme and the general font at 150 %, each verified to reach a fresh Qt process and each followed by a native gate; then restoration, verified. |
| `checks` | the harness's tests, `cargo fmt`, `cargo clippy --workspace`, `cargo test`, `cargo test --workspace`, `git diff --check`. |

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

* **`main-vs-branch`**, the branch's full native walk against post-#144 `main`'s: 40 of 70 captures exceed the
  generic noise bound, in three families and nowhere else. The 30 replay captures (`phase-*`, `replay-gap-*`)
  differ only in the scrubber's fill, `#000000` → the accent, plus a dozen anti-aliasing pixels beside it; the
  9 toolbar screens differ only in the 16×16 Settings icon (our glyph replaces a full-colour category icon,
  where `main` drew a solid silhouette); and `detail-nostrokes` has the sidebar list's focus ring, a band 5 px
  thick with no interior, going from black to the fitted accent (its toolbar icon aside). The other 30 are noise.
* **`accent-vs-default`**, the quick walk under Plasma's default accent against a loud one: 6 of 14 captures
  change, in four accent-driven places: the prominent Replay button's fill, a settings switch, the selected
  sidebar row's highlight and the focus ring.

The rules are **derived from real capture pairs**, never guessed (`derive_rules.py`), and a changed rule is a
reviewed change: the diff of that file shows which pixels a change may now move. Re-derive after an
intended visual change, look at the family summary it prints (the commonest before→after colours), and
commit the file with the change.

## Driving the exact AppImage (`atspi_walk.py`)

Wayland gives an unprivileged process no key synthesis: `ydotool`/`wtype`/`xdotool` are not installed,
`/dev/uinput` is root-only, and the RemoteDesktop portal needs a consent dialog every session. The harness
adds none of them. Instead the exact AppImage is launched with `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` (an
environment variable, no app change) and driven through AT-SPI: the walk moves **focus** between the search
and both date fields and invokes **actions** (Replay, Play, Pause). Leaving a date field is what a Tab does
to it, and it runs the same `editingFinished` → `Library.setDateRange` → model reset path issue #143
aborted release builds on. The key **chords** are covered where key events are real, in the gate: `GateKeys.qml`
sends `QKeyEvent`s (Ctrl+F, Tab and Shift+Tab through both date fields, F5, Esc, the platform's Preferences
chord, Alt+Left, F9, Space, Left, Right, `[`, `]`, and a real Ctrl+Q ending the walk) through the same key
path, on the same release code (the release quick gate), and `qml_runtime_gate.rs` asserts every one.

## Restoring the desktop

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
