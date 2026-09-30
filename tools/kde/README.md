# tools/kde/ — Fedora KDE Plasma native acceptance

`acceptance.py` turns the Plasma checks behind [ADR 0018](../../docs/decisions/0018-plasma-integration-through-qt-and-freedesktop.md)
into one repeatable, evidence-producing run, so a Qt, KDE, Mesa or toolchain upgrade is cheap to
re-validate. Nothing here is linked into the app, and nothing here adds a dependency to it: the harness
is Python's standard library, the tools Plasma and Fedora ship, and `cargo`.

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
tools/kde/acceptance.py all --output artifacts/kde/acceptance-$(date +%Y%m%d-%H%M%S)
```

Run `all` from a **clean** branch worktree with a clean checkout of `main` beside it (`--main-tree`,
default: the worktree on branch `main`); `guards` fails a dirty tree. It never uses `git stash`, needs
no root, and each `cargo` runs in its own tree, so each worktree keeps its own Cargo target directory.

| stage | what it measures |
| --- | --- |
| `probe` | the host (Fedora, kernel, Plasma, KWin, KF, host Qt, bundled Qt, Mesa, GPU, portals), and — through the bundled Qt's own `qml` tool, in a window that is never shown — the platform theme Qt created, the palette, `Accent` vs `Highlight`, scheme, contrast, font, icon theme, and what the repository's real `Theme.qml` resolves. Whether Qt's `Accent` is unusable decides which branch of `Theme.resolveAccent` must have run; nothing names a desktop. |
| `guards` | clean trees; #143's `[profile.release.package.qtbridge-interfaces] opt-level = 0` intact and identical to `main`'s; no new lockfile package; no KDE/KF/Kirigami crate among `rowplay-app`'s production dependencies and no new dependency in core, platform or view-model; the Blender stack untouched; `git diff --check`. |
| `services` | URL opening, file chooser, notifications, tray, global menu, MPRIS and the Secret Service, classified from the source: *Qt/XDG sufficient*, *not applicable* or *needs code* (a failure). |
| `checks` | the harness's tests, `cargo fmt`, `cargo clippy --workspace`, `cargo test`, `cargo test --workspace`, `git diff --check`. |

Every check ends in `PASS`, `FAIL`, `SKIPPED` (a flag was not given), `MANUAL_OPTIONAL` (automation cannot
do it safely), or `UNAVAILABLE` (the machine lacks what it needs). Only `FAIL` fails a run; a stage that
raised fails it too, and the exit status is nonzero.

## Evidence

Each run writes one self-contained directory (`artifacts/kde/acceptance-<timestamp>/`, git-ignored):
`manifest.json` (the source of truth), `summary.md` (generated from it), `host.json`, `commands.log` (one JSON
line per command, with exit status and time), and a subdirectory per stage with its logs and probe JSON.
