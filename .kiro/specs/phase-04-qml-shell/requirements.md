# Phase 4 — QML shell: requirements

The web app (`shenghaoc/rowplay`, pinned `011e830`) is canonical; rowplay-studio
(pinned `3d406a5`) is the second reference and supplies the views to port
(`ContentView`, `SidebarView`, `DashboardView`, `WorkoutDetailView`,
`WorkoutStrokeAnalysisView`, `SettingsView`, `MetricTile`, `DesignTokens`) and
`DESIGN.md`. When they disagree the web wins unless Studio's source map
documents a deviation; every choice the Qt port makes is recorded in
`docs/source-map.md`.

Phase 4 ships as two stacked PRs: **4a (foundation)** — view-model crate,
theme, app shell, settings, i18n pipeline, runtime-error gate — and
**4b (screens)** — sidebar, dashboard, workout detail, stroke analysis.

Architecture ground rules (from the phase brief):

- A Qt-free `rowplay-viewmodel` crate holds all UI logic; the qtbridge objects
  in `rowplay-app` are thin adapters with no logic.
- All numbers and dates come from `rowplay-core` formatting and datetime (via
  the view-model). QML never formats metrics: no `toFixed`, no
  `toLocaleString`, no JS `Date` for data values.
- Sync and HTTP never run on the Qt thread.
- qtbridge carries only scalars, `String`, `Vec<scalar|String>`, QObjects and
  `serde_json::Value` (qt-bridges-notes #4); lists use qtbridge's `QListModel`
  base where the docs show one, else JSON arrays for read-only data.
- No hand-written C++ (ADR 0001), no Qt Widgets; Qt Quick Controls with the
  Fusion style on all three OSes, coloured from a `Theme.qml` singleton.

## R1: `rowplay-viewmodel` crate (Qt-free)

- **R1.1** New workspace member `crates/rowplay-viewmodel`, added to
  `default-members` and to the Qt-free CI job. Dependencies: `rowplay-core`,
  `serde`, `serde_json`, `thiserror`, `chrono`/`chrono-tz` (re-used from core's
  vocabulary) — no Qt, no I/O beyond what `rowplay-platform` traits hand it.
- **R1.2** Contains all UI logic: library filtering / sorting / grouping (on
  `workout_query`), dashboard summary tiles, PB lists, chart series building,
  detail metric strips, splits table rows, stroke-analysis series (Studio's
  `downsampleStrokes`, `computePaceChartDomain`, `computeSplitBoundaryDistances`
  ported), settings validation and display strings.
- **R1.3** Display strings are produced in Rust via `rowplay-core::formatting`
  and `rowplay-core::datetime`. Locale-sensitive date display (the web's
  `fmtDate` / `monthShortName`, backed by `Intl.DateTimeFormat`) is ported as a
  six-language month/day-name table inside the view-model, keyed by the
  language preference; the divergence from the Phase 1 plan ("delegate to
  `QLocale`") is recorded in the source map.
- **R1.4** The navigation state is a pure port of Studio's
  `DetailNavigationState` (route stack, replay-unavailability policy,
  selection reset), directly unit-testable.
- **R1.5** Every ported behaviour has a plain `cargo test` unit test; floats
  compare with a stated tolerance; where Studio has a Swift test, it is
  re-expressed.

## R2: qtbridge backend objects (`rowplay-app`)

- **R2.1** Four QML singletons under the `RowPlay` URI — `Library`, `Detail`,
  `Settings`, `Sync` — registered with the note #1 workaround
  (`#[qobject(NoQmlElement, ConvertToCamelCase)]` + a hand-written
  `impl QmlRegister` with `IS_SINGLETON = true`).
- **R2.2** The singletons are thin adapters: they hold view-model state, expose
  qtbridge-supported property types (scalars, `String`, `Vec<…>`,
  `serde_json::Value`, `QListModel` items) and forward calls. No formatting,
  filtering or business logic in `rowplay-app`.
- **R2.3** The sidebar/library list uses qtbridge's `QListModel` base
  (`#[qobject(Base = QListModel)]` + `#[derive(QModelItem)]` row structs, ≤ 15
  roles) with bulk loading through `reset_unnotified` + `reset()`.
- **R2.4** Performance budget, measured with the demo library and a synthetic
  5,000-workout library: the sidebar scrolls smoothly (list model, no
  per-row QObjects) and re-filtering completes in under 50 ms.
- **R2.5** Sync runs on a `std::thread` worker. Cross-thread notification uses
  qtbridge's `QmlMethodInvoker` (`get_qml_method_invoker()` on the Qt thread,
  `invoke_method` from the worker, queued-connection slot on the Qt thread
  draining an `mpsc` channel); if that proves unusable in practice, fall back
  to a QML `Timer` (~50 ms) draining through one Rust slot, and record the
  finding in `docs/qt-bridges-notes.md`. Cancel sets the coordinator's
  `AtomicBool`.
- **R2.6** The Concept2 token never crosses the bridge: QML only ever sees
  `hasToken: bool`. The token is saved to / loaded from the keyring
  (`rowplay-platform::token_store`) inside Rust, is never echoed back, logged
  or stored in a QML-readable property.

## R3: Theme (`Theme.qml`)

- **R3.1** `Theme.qml` is a QML singleton porting `DesignTokens.swift` and
  `DESIGN.md`: the six-colour semantic palette with light and dark hex values,
  metric colours, the spacing scale (2/4/6/8/12/16/20/24), radii (6/8/12/16),
  chart heights (220/150), panel/card/active-card backgrounds and the delta
  colour helper (dead-zone threshold, `higherIsBetter`).
- **R3.2** Light/dark follows the system colour scheme
  (`Qt.styleHints.colorScheme`); every colour property re-evaluates on change.
- **R3.3** Typography keeps DESIGN.md's size and weight scale (hero 28 bold,
  headline 15 semibold, body 13, metric 13 semibold, label 11 medium, compact
  10 medium, strip metric 18 semibold monospaced) but uses the system font
  family instead of SF Pro / SF Pro Rounded.
- **R3.4** Fusion style is configured for Qt Quick Controls on all platforms;
  the palette is coloured from `Theme.qml` (accent = Monitor Blue).

## R4: App shell

- **R4.1** `Main.qml` is a split view: sidebar column (min 260, ideal 320) and
  detail area, with placeholders in 4a replaced by real screens in 4b.
- **R4.2** Window title and minimum size come from Studio: "RowPlay" title
  semantics with minimum 1000×680 (Studio's `WindowGroup("RowPlay Studio")` +
  `windowMinimumWidth/Height`); the qt port names the window after the app.
- **R4.3** Navigation state (dashboard vs workout detail vs replay route
  stack) is driven by the view-model port of `DetailNavigationState`
  (R1.4); selection changes reset the route stack.
- **R4.4** Toolbar equivalents from `ContentView`: sport filter (All + three
  sports, segmented, 280 px), reload action, and the settings entry point;
  ⌘/Ctrl+1 selects the dashboard, ⌘/Ctrl+R reloads.
- **R4.5** Empty state (library empty, demo mode off) ports `ContentView`'s
  "Ready When You Are" panel: enable-demo action and open-settings action,
  strings from the web locale keys.

## R5: Settings screen

- **R5.1** Token section: a password field (`echoMode: Password`) for the
  Concept2 personal access token; save writes it to the keyring through Rust
  (R2.6); the field clears after save; the section shows connection state
  (`hasToken`) and a disconnect action with a confirmation step.
- **R5.2** Preferences section: distance unit (metric/imperial), home timezone
  (grouped IANA picker with a UTC default, from `chrono-tz` via core) and
  language (the six supported languages, endonyms). Changes persist through
  `rowplay-platform::preferences` (the `Preferences` struct gains a `language`
  field).
- **R5.3** Demo mode: a toggle, on by default when there is no token; sync is
  disabled while demo mode is on.
- **R5.4** Sync section: a button with progress, cancel, and last-result
  display (counts, timestamps, typed error messages — all produced by the
  view-model / platform layer, redacted through `PrivacySafeLogger`).
- **R5.5** Every string uses `Tr.t` with web locale ids (R7); unmapped
  Studio-only labels are matched to the closest web key or to unit symbols,
  and each substitution is recorded in the source map.

## R6: QML UI rules

- **R6.1** No metric formatting in QML (ground rule 2); QML displays the
  strings the bridge hands it.
- **R6.2** `Accessible.name` on every control and tile; the tab order and the
  sidebar work from the keyboard alone.
- **R6.3** Strings through `Tr.t(id, vars)` (R7.2); no hard-coded user-visible
  text.

## R7: i18n pipeline

- **R7.1** A `tools/` script converts the six web locale files
  (`reference/rowplay/src/lib/locales/{en,de,es,fr,ja,zh}.ts`, nested key
  objects, `{name}` placeholders) into ID-based Qt `.ts` files in `i18n/`:
  the message id is the web dotted key (e.g. `dashboard.title`), the English
  text is the `<source>`, and any key missing in a locale is filled with the
  English value (matching the web's per-key fallback). The script runs with
  Node's type stripping and no dependencies; the generated `.ts` files are
  committed.
- **R7.2** A QML singleton `Tr` exposes `t(id, vars)`: `qsTrId(id)` plus
  `{name}` interpolation identical to the web's `interpolate` (replaceAll per
  variable, values stringified). No numerus forms — the web has no plural
  rules beyond an unused `_one` helper.
- **R7.3** `build.rs` runs `lrelease` (found through qmake's binaries path,
  like `rcc`) over the six `.ts` files and the generated `.qm` files are
  bundled into the existing rcc at `:/qt/qml/RowPlay/i18n/qml_<lang>.qm`.
- **R7.4** The language preference drives `Qt.uiLanguage`;
  `QQmlApplicationEngine` then loads and re-translates live (verified against
  the Qt 6.11 docs: "Automatically loads translation files from an i18n
  directory adjacent to the main QML file … reloaded when Qt.uiLanguage is
  changed"; files need the `qml_` prefix). If the mechanism fails in practice
  with qtbridge's engine, record it in the notes and ask before working
  around it.
- **R7.5** Parity checks, Qt-free and run in CI: (a) all six `.ts` files have
  exactly the web `en` key set (908 ids); (b) every `Tr.t("…")` id used in QML
  exists in that key set; (c) the committed `.ts` files match a fresh
  regeneration from the pinned reference checkout when it is present (skipped
  in CI, which has no `reference/`).

## R8: QML runtime-error gate

- **R8.1** A smoke mode runs the app on demo data, exercises the shell (and
  from 4b each screen) and fails when stderr contains a QML `TypeError`,
  `ReferenceError`, `Binding loop`, `Unable to assign` or `is not defined`.
- **R8.2** The gate runs in CI under Xvfb like the existing smoke screenshot
  test, and locally on this host natively under Wayland with software GL.

## R9: CI, docs and validation

- **R9.1** CI: the Qt-free job and the MSRV job gain `rowplay-viewmodel`; the
  Linux app job gains the runtime-error gate and (4b) screenshots of the
  dashboard, detail view and settings as artifacts next to the smoke
  screenshot.
- **R9.2** Docs: roadmap Phase 4 status, source-map rows for every ported view
  and divergence, qt-bridges-notes entries for new friction (at minimum:
  `QListModel` findings, `QmlMethodInvoker` threading findings, no
  `QTranslator`/engine access on `QApp`), AGENTS.md updated for the new crate
  and the i18n workflow, README screenshots (demo data only).
- **R9.3** Validation per PR: `cargo fmt --all -- --check`,
  `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `git diff --check origin/main...HEAD`, and a
  manual pass over every screen under Wayland in light and dark.

## Deferred (not built in Phase 4)

Replay (Phase 5), live mode (Phase 8), HR import, annotations, comparison
panel, file actions/export, rival controls. The UI keeps their entry points
disabled or hidden, and the PRs list them.
