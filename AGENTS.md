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
assets/                    # vendored .glb / textures with provenance (ASSET_PROVENANCE.md)
i18n/                      # generated ID-based Qt .ts catalogues (never hand-edited; see "Internationalisation")
tools/                     # asset / locale / fixture pipeline scripts (Python, Node, Blender)
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
```

Set `LANG=C.UTF-8` when Qt warns about the C locale. `ROWPLAY_RCC` /
`ROWPLAY_LRELEASE` override the `rcc` / `lrelease` executables found through
`qmake`. The repo root has a committed `.envrc` exporting the local Qt 6.11.2
paths (`~/Qt/6.11.2/gcc_64`, per the README install recipe); source it (or
`direnv allow`) before any `rowplay-app` command when qmake is not already on
`PATH`.

Phase 5 onward requires a local Qt: the replay scene, materials and athlete
must be run and looked at by hand on this machine (`cargo run -p rowplay-app`
under Wayland, plus the headless screenshot tests above), never inferred from
a green CI job alone. Do not proceed with 3D work while `cargo build
-p rowplay-app` fails locally — fix the environment first.

Test/QA environment hooks: `ROWPLAY_SMOKE_GATE=1` walks every screen
and exits, `ROWPLAY_SYNC_MOCK=1` runs syncs against the deterministic mock
client (no token), `ROWPLAY_FORCE_COLOR_SCHEME=dark|light` pins the palette,
`ROWPLAY_SMOKE_SCREENSHOT_DIR` saves per-screen PNGs during the gate walk.

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
  hardcoded user-visible text); Fusion style coloured from `Theme.qml`;
  `Accessible.name` on every control and tile; no metric formatting and no
  inline per-frame arithmetic that belongs in Rust.

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
