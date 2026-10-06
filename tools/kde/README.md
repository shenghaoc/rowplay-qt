# tools/kde/ — Fedora KDE Plasma native acceptance

`acceptance.py` turns the Plasma checks behind [ADR 0018](../../docs/decisions/0018-plasma-integration-through-qt-and-freedesktop.md)
into one repeatable, evidence-producing run, so a Qt, KDE, Mesa or toolchain upgrade is cheap to
re-validate. Nothing here is linked into the app, and nothing here adds a dependency to it: the harness
is Python's standard library, the tools Plasma and Fedora ship, and `cargo`.

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
tools/kde/acceptance.py all --output artifacts/kde/acceptance-$(date +%Y%m%d-%H%M%S)
```

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
line per command, with exit status and time), and a subdirectory per stage with its logs and probe JSON.
