# Repository guidelines

## Project purpose

rowplay-qt is the cross-platform (Linux, macOS, Windows) desktop port of
rowplay — Concept2 logbook analytics and real-time workout replay for RowErg,
SkiErg and BikeErg athletes. Rust implements all logic; Qt Bridges for Rust
(`qtbridge`) exposes it to QML / Qt Quick; Qt Quick 3D renders the replay and
Qt Graphs the charts. The only sources of truth are the author's two existing
repositories: **rowplay** (web, canonical behaviour) and **rowplay-studio**
(native macOS, layering and golden parity fixtures). The repository is public;
history, docs and decision records matter as much as the code.

Not affiliated with Concept2. Concept2, RowErg, SkiErg and BikeErg are
Concept2 trademarks and stay untranslated.

## Repository structure

```
crates/rowplay-core/       # pure domain logic; no Qt, no I/O beyond parsing byte slices
crates/rowplay-platform/   # services behind traits with mocks: token store, cache, Concept2 client, preferences, sync, paths
crates/rowplay-viewmodel/  # Qt-free UI logic: navigation state, locale date display, settings options, screen view models
crates/rowplay-app/        # the binary: qtbridge backend objects, QML shell, Qt Quick 3D (build.rs runs rcc + lrelease)
crates/rowplay-fixtures/   # dev-only loader for tests/fixtures
qml/                       # QML modules (RowPlay/qmldir, Main.qml, Theme/Tr singletons, screens) + rowplay.qrc + qtquickcontrols2.conf
assets/                    # vendored .glb / textures / icon with provenance (ASSET_PROVENANCE.md)
packaging/                 # Phase 9 manifests: macOS Info.plist, Linux .desktop + AppStream, Windows Inno Setup script
i18n/                      # generated ID-based Qt .ts catalogues (never hand-edited; see "Internationalisation")
tools/                     # asset / locale / fixture pipeline scripts (Python, Node, Blender); tools/package/ builds the installers
tests/fixtures/            # golden parity JSON from rowplay-studio + manifest.json + PROVENANCE.md
docs/                      # roadmap.md, source-map.md, qt-bridges-notes.md, decisions/ (ADRs)
.kiro/specs/               # per-phase requirements / design / tasks
reference/                 # git-ignored checkouts of rowplay and rowplay-studio (never committed)
```

## Agent instructions

`AGENTS.md` is the single canonical guide for coding agents. `CLAUDE.md` and
`GEMINI.md` are import-only shims and must contain only `@AGENTS.md`. Do not
add vendor-specific guides unless a tool cannot read `AGENTS.md`.

Before coding read `docs/roadmap.md`, `docs/source-map.md`,
`docs/qt-bridges-notes.md`, the ADRs in `docs/decisions/` and the phase spec
under `.kiro/specs/`. At session start check out the reference repositories at
current `main` into `reference/` and record both SHAs in `docs/source-map.md`;
copy only what you need, with provenance (source repo, path, commit, SHA-256).

## Build, test and development commands

```bash
cargo build                                   # Qt-free default members (core, platform, viewmodel, fixtures)
cargo test                                    # unit + parity + i18n tests without Qt
cargo test --workspace                        # also the app crate (needs Qt 6.11)
ROWPLAY_KEYRING_TESTS=1 cargo test -p rowplay-platform   # opt-in OS keychain round trip
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings     # add --workspace where Qt is installed
cargo build -p rowplay-app                    # needs qmake on PATH (or QMAKE=/path/to/qmake)
cargo run -p rowplay-app                      # the Phase 4 shell (demo data by default)
cargo test -p rowplay-app --test qml_runtime_gate        # QML runtime-error gate (any QPA)
# headless screenshot + gate test (Linux, Mesa):
QT_QPA_PLATFORM=xcb QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
  ROWPLAY_QT_SMOKE=1 ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts \
  ROWPLAY_SMOKE_SCREENSHOT_DIR=$PWD/artifacts \
  xvfb-run -a cargo test -p rowplay-app
# on a Wayland desktop the same tests run without Xvfb:
QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
  ROWPLAY_QT_SMOKE=1 ROWPLAY_SMOKE_ARTIFACT_DIR=$PWD/artifacts cargo test -p rowplay-app
git diff --check
tools/package/macos.sh                        # Phase 9: dist/rowplay-qt.app + .dmg (macOS)
tools/package/linux.sh                        # Phase 9: dist/*.AppImage (Linux x86_64; xvfb-run when headless)
pwsh tools/package/windows.ps1                # Phase 9: dist/*-setup.exe + .zip (Windows; Inno Setup 6)
```

Set `LANG=C.UTF-8` when Qt warns about the C locale. `ROWPLAY_RCC` /
`ROWPLAY_LRELEASE` override the `rcc` / `lrelease` executables found through
`qmake`. The repo root has a committed `.envrc` exporting the local Qt 6.11.2
paths (`~/Qt/6.11.2/gcc_64` on Linux, `~/Qt/6.11.2/macos` on macOS, per the
README install recipe); source it (or `direnv allow`) before any
`rowplay-app` command when qmake is not already on `PATH`. macOS needs no
`DYLD_*` variable: `build.rs` emits the Qt lib dir as an `LC_RPATH`
(qt-bridges-notes #10).

Phase 5 onward requires a local Qt: the replay scene, materials and athlete
must be run and looked at by hand on this machine (`cargo run -p rowplay-app`
under Wayland, plus the headless screenshot tests above), never inferred from
a green CI job alone. Do not proceed with 3D work while `cargo build
-p rowplay-app` fails locally — fix the environment first.

Test/QA environment hooks: `ROWPLAY_SMOKE_GATE=1` walks every screen
and exits and `ROWPLAY_SYNC_MOCK=1` runs syncs against the deterministic
mock client (no token); both are compiled out of release builds
(`backend::test_env`). `ROWPLAY_FORCE_COLOR_SCHEME=dark|light` pins the
palette, `ROWPLAY_FORCE_CONTRAST=high|normal` pins the contrast variant and
`ROWPLAY_SMOKE_SCREENSHOT_DIR` saves per-screen PNGs during the gate walk;
these are plain environment reads, documented overrides that change nothing
but the look (the screenshot directory is used only by the gate walk).
`ROWPLAY_EXIT_AFTER_FRAMES=N` is the one hook read in every build: the shell
quits with status 0 after N rendered frames, which is how the packaged
release bundles are proven to start (`tools/package/launch-check.py`); it can
only shorten a run.

## Packaging and releases (Phase 9, ADR 0012)

`tools/package/{macos.sh,linux.sh,windows.ps1}` build the `.app`/`.dmg`,
the AppImage and the Inno Setup installer + zip from a release build with
Qt's own deployment tools (`macdeployqt`, `windeployqt`) and pinned,
SHA-256-verified `linuxdeploy` tools; each script launch-checks its output
from a clean environment before it keeps it. `.github/workflows/release.yml`
runs all three on pull requests touching `packaging/**`, `tools/package/**`,
`assets/icon/**`, the app `build.rs` or the Cargo manifests, on dispatch,
and on `v*` tags, where it drafts a GitHub release. Rules:

- Release artifacts come only from that workflow. Never upload a hand-built
  package to a release.
- A Qt bump or a qtbridge bump must be re-packaged and launch-checked on
  all three OSes in the same PR.
- The launch check proves start / load / render / exit, not pixels. macOS
  and Windows rendering is verified by a human looking at the packaged
  app (the gate's visual assertions run on the Linux leg only, note #17);
  say so in any release PR.
- Icons live under `assets/icon/`, pinned like the replay assets; regenerate
  the `.icns`/`.ico` with `tools/package/gen-icons.py` and re-pin.

## Architecture boundaries

Dependency direction is **app → viewmodel → platform → core** (ADR 0006,
extended by the Phase 4 view-model crate).

- **rowplay-core** — pure domain logic. It may use `serde`, `serde_json`,
  `regex`, `chrono` and `chrono-tz`; it must not depend on Qt, perform file or
  network I/O, or read the clock beyond `datetime::now_*`. Everything with a
  web equivalent has a parity test. `serde_json` is there for the Concept2
  payload mapper (`concept2`), which takes byte slices.
- **rowplay-platform** — non-UI services as traits with production and mock
  implementations: `TokenStore` (`KeyringTokenStore` over the OS keychain),
  `WorkoutCache` (`SqliteWorkoutCache` over rusqlite), `Concept2Client`
  (`Concept2HttpClient` over ureq), `PreferencesStore`
  (`FilePreferencesStore`, JSON), the sync coordinator and state tracker,
  `load_library`, `paths` and log sinks. No Qt, no tokio. Its only extra crates
  are the ones ADR 0007 names — `ureq`, `keyring`, `rusqlite`, `directories`
  and, for tests, `tempfile`. Do not add a dependency that the platform layer
  does not strictly need (ADR 0007 explains why `url` was left out).
- **rowplay-viewmodel** — all UI logic, Qt-free and unit-testable: filtering,
  sorting, summary tiles, PB lists, display strings (via `rowplay-core`
  formatting), locale date display (`dates`, golden-tested against the web's
  `Intl` output), chart series building, settings validation and the
  `DetailNavigationState` port (`nav`). It may use `serde`, `serde_json`,
  `chrono`/`chrono-tz` and `thiserror`; no Qt, no I/O. All user-visible
  numbers and dates are formatted here or in core — **never in QML** (no
  `toFixed`, `toLocaleString` or JS `Date` for data values).
- **rowplay-app** — the only crate that links Qt. Backend objects are thin
  adapters over the view-model with no logic of their own: the `Library`,
  `Detail`, `Settings` and `Sync` QML singletons (registered under the
  `RowPlay` URI with the qt-bridges-notes #1 workaround). QML drives per-frame
  work with `FrameAnimation` calling one `tick(dt)` slot and reads a compact
  result; measure bridge crossings per frame before adding more. Sync and
  HTTP run on a `std::thread` worker; events come back over `mpsc` plus
  qtbridge's `QmlMethodInvoker` (50 ms QML timer as a safety net).

**No hand-written C++.** If something needs a C++-only Qt API (subclassing
`QQuick3DGeometry`, `QQuick3DTextureData`, …), stop and write an ADR with
options (asset baking, a QML-only alternative, a CXX-Qt island). Qt tooling
invoked from `build.rs` (`rcc`, `balsam`, `lupdate`) is fine.

**qtbridge is newer than any model's training data.** Read the crate sources
(`qtbridge`, `qtbridge-runtime`, `qtbridge-interfaces`, `qtbridge-gen`) and
`github.com/qt/qtbridge-rust-examples` before using an API; never write it from
memory. Keep the version pinned exactly. Log every friction point, bug or
missing feature in `docs/qt-bridges-notes.md` with a minimal repro — they are
sent upstream.

## Decisions already made

Recorded as ADRs in `docs/decisions/`; do not relitigate them: Rust + qtbridge +
Qt Quick 3D + Qt Graphs on Qt 6.11 with no C++ (0001); GPL-3.0-or-later with
SPDX headers and asset provenance (0002); `.glb` only, Blender authoring, no
USDZ (0003); procedural-sky IBL, no HDRI files (0004); bake venues rather than
port `renderer3dEnvironment.ts` (0005); Rust core first (0006).

## Parity is the test oracle

- Every ported web helper gets a golden-fixture test or a test re-expressed
  from the web `*.test.ts` suite. Compare floats with a per-fixture tolerance
  and state it in the test.
- If the web and Swift versions disagree, the web wins unless Studio's source
  map documents a deliberate deviation (for example the interpolated finish
  crossing). Record every divergence in `docs/source-map.md`.
- Fixtures under `tests/fixtures/` are never edited by hand; refresh them with
  `tools/vendor-fixtures.py` and update `PROVENANCE.md`.
- Replay-related fixtures may land early with `#[ignore]` tests that name the
  phase that will enable them.
- Enable the ignored fixture *before* porting to it, not after: three
  geometry defects in Phase 7 (two in slice 1, the palm-normal facing in
  slice 2) all looked correct in isolation with passing unit tests and were
  caught only because a fixture consumed them. Unit tests cannot substitute
  for parity fixtures on geometry code.
- A green parity suite means the *covered surface* agrees, not that the port
  is correct. Phase 7's rower rig stroked backwards against the web for two
  slices because the corpus covered the web's static rig contracts but had
  zero samples of the stroke-phase calibration — Studio (the only source for
  it) was itself inverted, and nothing failed. When porting a calibration or
  a mapping (not just pure functions), ask what fixture consumes it; if the
  answer is nothing, write the web-derived fixture first
  (`tools/gen-row-phase-parity.mjs` is the pattern: evaluate the pinned web
  commit under Node, record source hashes inside the JSON).
- Record what the web *renders*, not how the web *computes*. A generator
  that re-derives the web's formula in JavaScript inherits whatever the
  porter believed at port time and then asserts the port matches itself; it
  can only ever confirm the port's own reading of the source. A generator
  that builds the web's real object and samples its animated output records
  independent observables, so a wrong channel or a missing layer shows up as
  a diff. Phase 7's `replay-row-phase-parity.json` re-implemented
  `OAR_YAW_CATCH + handleTravel·span` in the generator (the port's own
  channel choice) and passed; the audit's `replay-rig-phase-parity.json`
  sampled the avatar's rendered transforms and caught that the web actually
  drives the oar from `armDraw` — a 1.05 rad error, plus the reach solve the
  port never composed. Where a quantity is only reachable as a pure function,
  record that function's *inputs and output* at each sample and feed them to
  the port verbatim; and have the generator assert its reconstruction equals
  the rendered value, so wrong observables cannot be recorded silently.
- Residuals cannot validate orientation: a re-solve absorbs an orientation
  error and reports convergence, so a contact residual measures "did the
  solver find *a* solution", not "is the solution anatomically real" (Phase
  7 slice 3: the left scull shaft pointed outboard with green residuals).
  Orientation claims need anatomy assertions — shaft directions, twist
  budgets, envelope checks — not just closed contacts.
- The harness is a suspect equal to the port. Four parity failures turned
  out to be instrument faults, each presenting as an apparent port defect: a
  corpus that re-derived the web's formula (agreeing with the port by
  construction), a recorder animating the avatar detached from a scene graph
  (an early return collapsed its targets to the pelvis), a recorder that
  mirrored one side's inputs into the other, and — on the Rust side of the
  harness — serde_json's default float parser landing 1 ULP off the
  fixtures' long literals, silently perturbing every float-bearing corpus's
  parsed inputs. The round-trip through the file is part of the instrument:
  every float literal in every fixture must read back bit-identical to its
  nearest double through the parser the tests use
  (`crates/rowplay-fixtures/tests/float_roundtrip.rs` asserts it over the
  whole fixtures directory). When a parity comparison fails, verify the
  recorder and the round-trip before changing port code.

## Privacy and security invariants (from rowplay-studio)

- Concept2 tokens live only in the OS keychain via `keyring` — never in files,
  logs, fixtures, preferences or analytics payloads. `SecretToken` has no
  `Display` and is not serialisable.
- All logging of user data goes through `rowplay_core::privacy::redact` /
  `PrivacySafeLogger`.
- Parsers bound raw input length before scanning (pace input, day keys,
  logbook timestamps, redaction).
- Share / export strips hardware-identifying metadata (`serial_number`, `device`).
- Network clients (Phase 3) are HTTPS-only, allow same-host redirects only,
  use an ephemeral session and strict timeouts. In the Rust port
  (`Concept2HttpClient`, ADR 0007): plain `http` is accepted only for
  `localhost`, `127.0.0.1` and `::1`; automatic redirects are off and at most
  three same-origin ones are followed by hand; a cross-host redirect or an
  HTTPS→HTTP downgrade is a typed `InsecureRedirectBlocked` failure and the
  token is never sent anywhere else; timeouts are 30 s per request and 300 s
  overall; a response body over 25 MiB is rejected. It is bring-your-own-token
  only — there is no OAuth flow.
- keyring's native backend is named explicitly for every target and a test
  fails if the crate's default credential store is keyring's in-memory mock.

## Deterministic demo mode

Demo mode is first-class. `rowplay_core::demo` reproduces the web app's seeded
generator byte for byte; the app must be fully explorable with it and no
Concept2 token. Cache failures never silently fall back to demo data.

## Coding style

- Rust 2024 edition, MSRV 1.87; `cargo fmt`; clippy pedantic with the
  workspace allow-list in `Cargo.toml`; `#![forbid(unsafe_code)]`;
  `thiserror` for library errors; `must_use` on pure functions.
- The Rust toolchain is pinned in `rust-toolchain.toml` and CI; it is bumped
  deliberately, in its own PR (new clippy lints land with each stable).
- Snake-case ports keep the web names (`fmt_time`, `pb_workout_ids`,
  `workout_local_day_key`) so `docs/source-map.md` stays greppable.
- Every source file (Rust, QML, scripts, workflows, qrc) starts with
  `SPDX-License-Identifier: GPL-3.0-or-later`.
- QML: one module per directory with a `qmldir`; strings through
  `Tr.t("dotted.web.key", { vars })` (never `qsTr` with inline English, never
  hardcoded user-visible text); the Basic style, drawn from the tokens in
  `Theme.qml` (ADR 0013: one design system on every OS, lengths and type
  scaled from the system font, the system accent and contrast preference);
  `Accessible.name` on every control and tile; no metric formatting and no
  inline per-frame arithmetic that belongs in Rust.
- Screens use the shared controls in `qml/RowPlay/`: `PushButton`,
  `ToolbarButton`, `SegmentedControl`, `ToggleSwitch`, `InputField`,
  `PopupButton`, `FormSection` / `FormRow`, `AppSlider`, and the `App*`
  menus, tool tips, dialogs, scroll bars and indicators. Never a raw
  `Button`, `ComboBox`, `Switch`, `TextField`, `Slider`, `Menu`, `ToolTip`
  or `Dialog`. New symbols are original path data in `Icon.qml`, never
  image files. Text is in sentence case (no `toUpperCase()`), nothing is
  conveyed by colour alone, and nothing is reachable only by hover.

## Internationalisation

The six web locales are the only string source. `tools/convert-locales.mjs`
(Node ≥ 23.6, no dependencies) regenerates the ID-based Qt catalogues in
`i18n/` from `reference/rowplay/src/lib/locales/*.ts`; the committed `.ts`
files are build inputs (`build.rs` runs `lrelease` and bundles
`qml_<lang>.qm` into the rcc at `:/qt/qml/RowPlay/i18n/`). Rules:

- The message id is the web's dotted key; `<source>` must stay **empty** (a
  non-empty source silently breaks `qsTrId` lookup — qt-bridges-notes #12);
  the English text lives in `<oldsource>`.
- Never hand-edit `i18n/*.ts`; re-run the generator and commit the result.
  `cargo test -p rowplay-viewmodel --test i18n_parity` fails on stale files
  (when `reference/` exists), on key-set drift, on extra placeholders and on
  any `Tr.t("…")` id in `qml/` that is not a web key.
- The language preference drives `Qt.uiLanguage`; `QQmlApplicationEngine`
  reloads the catalogues live. New UI strings need a web key first — if the
  web has none, record the substitution in `docs/source-map.md` instead of
  inventing an id.

## Review priorities

P1 (must fix before merge):

- Missing or outdated documentation for the change: `docs/source-map.md`
  (including divergences), `docs/roadmap.md` and the phase's `.kiro/specs/`
  tasks, `docs/qt-bridges-notes.md` for any bridge friction, an ADR for any
  architectural decision, provenance files for any vendored asset or fixture.
- A privacy invariant above weakened.
- New C++ or a runtime-downloaded asset.

## PR validation checklist

1. `cargo fmt --all -- --check` passes.
2. `cargo clippy --all-targets -- -D warnings` passes (with `--workspace` where Qt is installed).
3. `cargo test --workspace` passes (state which crates ran if Qt was unavailable).
4. `git diff --check` passes.
5. The app builds wherever Qt is available; attach the smoke screenshot for QML / 3D changes.
6. The PR lists scope, validation commands with results, toolchain versions,
   pinned reference SHAs, what was deferred, known gaps and every Qt Bridges issue hit.

## Commit and PR style

One PR per phase; commits scoped per logical step with subjects in the
rowplay-studio style, e.g. `feat: Phase 1 - Core parity foundation`.

## Operational lessons

Each rule exists because the failure happened.

- **A port that mirrors an architecture must mirror its law, not just its
  shape.** Issue #40 was filed as "the V4 contact architecture couples a
  wrist snap into hand position" — and the web *does* use that architecture
  (bone-chain IK closing a `target − R·offset` contact), so the coupling
  looked inherited. It was not: the port aimed the chain at the contact
  point while the web aims it at the terminal *origin* and lets the wrist
  bone absorb the offset, and the port closed on the contract's authored
  palm point where the web overrides it per sport. Both differences were
  visible only by **driving the web's own code** — the "the Node harness
  cannot drive the hero chain" assumption in the issue was wrong, and one
  headless install of the real controller turned three weeks of
  architectural explanation into two measured arithmetic errors
  (2026-09-21). When a defect is attributed to an architecture the port
  copies, drive the original and measure it before writing the
  attribution into a comment or a doc.
- **A widened tolerance and a skipped window are the same defect wearing
  different clothes.** The guard's two carved-out cycle windows were the
  record of issue #40 for two phases; the fix deleted them rather than
  re-tuning them. If a bound needs an exception, the exception is a TODO
  with a date and an issue number, and it is deleted with the fix — never
  quietly widened.
- **An 11x improvement is not parity, and the residual is the next issue.**
  The issue #40 fix took the rendered hand's snap from 0.096 m to 0.0405
  (the web's 0.0379) — close enough to read as "fixed". The oracle's own
  residual showed it was not: the web holds its contact point exact
  through its snaps (≤ 9.4e-7) while the port's snap still reached the
  elbow 2.1x further than the web's (0.0411 vs 0.0193), and the surviving
  9 mm grip residual was passing under a pre-existing 10 mm budget with
  ~10 % margin. Both became issues (#44) rather than a claim of parity.
  When a fix moves a number most of the way, compare the *structure* of
  the remaining error against the oracle's, and file it by name instead of
  letting a green assert imply the mechanism is gone.
- **A closing keyword is a scope wider than the sentence it sits in.**
  GitHub treats `close` / `fix` / `resolve` in **any tense**, followed by
  an issue reference, as closing that issue — in **commit bodies** and PR
  descriptions alike. A commit body that filed the coupling's residue as
  its own issue and, in the same paragraph, wrote "claimed fixed" beside
  its number closed that issue at the merge: the parser read the phrase as
  a closing keyword, not as prose. (The first attempt to write this lesson
  *quoted* the offending phrase, and closed the same issue a second time —
  quotation is not an escape, and neither are backticks.) When an issue
  must stay open, write "tracked in", "see" or "refs" before the number,
  and never a close/fix/resolve word beside it. Check the issue list after
  a merge that names issues, and write this rule with placeholders rather
  than a live example.

- After any conflict resolution, grep the whole tree for conflict markers
  (`rg -n '^(<{7}|={7}|>{7})'`) before continuing the rebase.
- Inside a fold rebase (`git rebase --onto` + `amend`), stage **explicit
  paths** — never `git add -A` or `git add .`. A fold runs while the working
  tree still carries untracked build output, and `-A` has committed 30 MB of
  linuxdeploy caches and 2,276 `dist/` files including a 64 MB AppImage into
  otherwise-clean histories (2026-09-20, #35 review; the second sweep was
  caught by GitHub's large-file warning at push time, not by any check of
  ours). Run `git status --porcelain` before every commit inside a rebase,
  and confirm `git branch --show-current` (or the intended detached commit)
  before every commit, amend or push.
- A vision model's description of a capture is **not evidence**. Compute
  pixel statistics from the bytes first (region luminance and variance,
  tile-wise diffs between expected states); use a vision model only to
  corroborate what those numbers already show, never as the observation
  that establishes a state. On 2026-09-20 a vision pass produced confident,
  detailed descriptions of detail views and three 3D venues for captures
  that pixel-diffing proved were identical dashboard frames whose only
  variation was a blinking text cursor; every readout was discarded.
- A flat luminance sample does **not** mean an empty region — sparse text
  on a plain background reads as flat. Confirm with a crop or a glyph-edge
  count before concluding a pane is empty (2026-09-20: the first-launch
  main pane, actually the styled "No stroke data" empty state, was recorded
  as blank and the claim had to be corrected in the spec).
- A capture's colours are only as honest as its alpha. A gate grab stores
  pixels that nothing opaque covers as translucent, the PPM beside it keeps
  their bare colour, and `PIL.Image.convert("RGB")` drops alpha without
  compositing — so a 10 % grey hairline read back as pure black
  (2026-09-23, the unmerged HIG pass; an `xwd` capture of the screen
  showed the grey, and painting the grab root cured it; see #61). Check
  that a capture's alpha is 255 everywhere before reading its colours, and
  when a colour looks wrong, capture the screen as well before blaming the
  code.
- Compare a measurement with its threshold **unrounded**. WCAG does not
  round: 4.496:1 fails 4.5:1. The design system's contrast table printed
  two decimals, so the PM5 duration colour on the light grouped surface
  read "4.50" and stood in four stacked PRs before a four-decimal read
  caught it (2026-09-23; the surface moved one step, to `#F4F5F7`). Two of
  the table's passing figures had been rounded up as well. Assert on the
  raw value, and state measured figures truncated, never rounded up.
- A fix aimed at one configuration is checked in all the others. The
  narrow-window fixes of the design-system stack were driven at 150 % text
  in the minimum window and passed every test, yet they changed the
  default size three times (2026-09-23): the settings quality control grew
  16 px, because a `FontMetrics` measure had never depended on its font
  and a new `Layout` binding read it early; a sync button moved a pixel
  off the grid, placed by a `Flow`; and the new splits table clipped its
  last column, because Qt Quick Layouts round fractional widths up. The
  first two surfaced only in a pixel diff of the default-size captures
  against the heads before the fixes, the third only once that view was
  captured at all. After fixing one size, scheme or language, capture the
  others, diff them against the head before the fix, and look at every
  difference, explained ones included: beyond the known run-to-run glyph
  noise, each is a finding until checked.
- `pkill -f <pattern>` matches your own shell's command line, because the
  pattern appears in it — twice this killed the driving script mid-run
  (same family as `git add -A`: a command whose scope is wider than the
  mental model of it). Kill by PID: record `$!` or read it from
  `pgrep -x <process-name>`, then `kill <pid>`.
- A capture handed to a third-party vision/describe service leaves this
  machine and may be retained there. Only send captures whose contents you
  would publish; the 2026-09-20 T9 captures carried demo workout data only,
  and that is the bar.
- A failure that does not reproduce is recorded as **unexplained**, never
  dismissed as "flake" — not reproducing is what flakiness does, so the word
  asserts a mechanism nobody demonstrated. Name what was checked, what the
  suspected interference was, and leave the cause open (2026-09-20: one
  per-commit-gate execution failed once and passed unchanged on re-run;
  recorded in the QML gate's docs together with the gate's real hermeticity
  boundary — it defaults to `offscreen` but keeps a caller-provided
  `QT_QPA_PLATFORM`, so local runs after visual work are session-dependent
  where CI's clean Xvfb is not. The same record shows what checking the
  environment buys: the failing exec had `QT_QPA_PLATFORM` unset (offscreen
  — the session-dependence path excluded as the cause), and because the
  gate chain piped `cargo test` through a counting grep, the failing
  test's name and output were discarded — nothing to compare a recurrence
  against.
- Check **exit codes, not output patterns**. A check that greps a command's
  output for failure markers passes whenever the command never got far
  enough to emit them: a `cargo test` compile error produces no
  `test result: FAILED` lines, so a counting grep reports zero and
  succeeds (2026-09-20: the ad-hoc per-commit rebase gate used exactly
  that idiom). Verified by grep that it appears nowhere in
  `.github/workflows/` or `tools/`; the one output grep in `tools/`
  (`macos.sh`'s `otool | grep -F`) is the safe orientation — it fails the
  build on the *presence* of a leak signature, not on the absence of a
  success marker.
