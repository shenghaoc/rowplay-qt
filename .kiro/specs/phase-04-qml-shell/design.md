# Phase 4 — QML shell: design

## Module map

| Screen | Studio source | Web canonical | QML | Rust |
| --- | --- | --- | --- | --- |
| Shell / navigation | `Views/ContentView.swift`, `Views/DetailNavigationState.swift`, `App/RowPlayStudioApp.swift` | `src/routes/+layout.svelte` | `qml/RowPlay/Main.qml`, `ShellWindow` pieces | `rowplay_viewmodel::nav` |
| Theme | `Views/DesignTokens.swift`, `DESIGN.md` | — (web CSS vars) | `qml/RowPlay/Theme.qml` (singleton) | — |
| Settings | `Views/SettingsView.swift` | `src/routes/settings/`, `src/routes/auth/token/` | `qml/RowPlay/SettingsScreen.qml` | `rowplay_viewmodel::settings`, `rowplay-app/src/backend/settings.rs`, `…/sync.rs` |
| Sidebar | `Views/SidebarView.swift` | `src/lib/workoutList*` | `qml/RowPlay/SidebarPanel.qml` | `rowplay_viewmodel::library`, `rowplay-app/src/backend/library.rs` (`QListModel`) |
| Dashboard | `Views/DashboardView.swift`, `Views/MetricTile.swift` | `src/routes/dashboard/` | `qml/RowPlay/DashboardScreen.qml`, `MetricTile.qml` | `rowplay_viewmodel::dashboard` |
| Workout detail | `Views/WorkoutDetailView.swift` | `src/routes/workout/` | `qml/RowPlay/DetailScreen.qml` | `rowplay_viewmodel::detail` |
| Stroke analysis | `Views/WorkoutStrokeAnalysisView.swift` | `replay` chart derivations | `qml/RowPlay/StrokeAnalysisPanel.qml` | `rowplay_viewmodel::strokes` |
| i18n | — | `src/lib/i18n*.ts`, `src/lib/locales/*.ts` | `qml/RowPlay/Tr.qml` (singleton) | `tools/convert-locales.mjs`, `crates/rowplay-app/build.rs`, `crates/rowplay-viewmodel/tests/i18n_parity.rs` |

## `rowplay-viewmodel` crate

Qt-free, `#![forbid(unsafe_code)]`, pedantic clippy like the rest of the
workspace. Public modules:

```rust
pub mod nav;        // DetailNavigationState port: Route, ReplayUnavailability,
                    // is_replay_presented, show_replay, replay_unavailability,
                    // reset_for_selection_change — all pure, all tested
pub mod library;    // LibraryViewModel: holds Vec<Workout> + WorkoutListQuery;
                    // filtered_rows() -> Vec<SidebarRow> (display strings done),
                    // grouped sections, pb ids, sort toggling (Studio's toggleSort),
                    // count strings via locale ids handed to QML as {n} vars
pub mod dashboard;  // tiles (sessions/total distance/challenge/time/avg pace),
                    // by-sport bar series, recent-pace line series + domain,
                    // pb cards (pbLabel port: Half/Marathon/k/m)
pub mod detail;     // header fields, metric strip entries (label id + value +
                    // colour role), split table rows, powerText port
pub mod strokes;    // downsample_strokes (limit 500, endpoints kept),
                    // pace_chart_domain, split_boundary_distances,
                    // stroke series as flat Vec<f64> pairs for Qt Graphs
pub mod settings;   // language list (code + endonym), timezone groups from the
                    // web settings.timezoneGroup* keys over chrono-tz,
                    // unit mapping, validation
pub mod dates;      // six-language month/day-name tables; fmt_date_short,
                    // fmt_date_full, fmt_time_of_day on top of core::datetime
pub mod theme;      // colour roles as an enum -> QML reads role names, Theme.qml
                    // maps role -> hex (palette stays in QML, roles in Rust)
```

Data handed to the bridge is either plain structs the app layer flattens into
`QModelItem` roles, or `serde_json::Value` for read-only blobs. The view-model
never allocates per-frame; recomputation happens on explicit invalidation
(query changed, library reloaded, unit/language changed).

Display-string rule: every user-visible number/date string is built here with
`rowplay_core::formatting` (`fmt_time`, `fmt_pace`, `fmt_distance_in`, …) and
`dates`. QML only concatenates translated labels with these values through
`Tr.t(id, vars)`.

## qtbridge backend (rowplay-app)

Four singletons, all `#[qobject(NoQmlElement, ConvertToCamelCase)]` +
manual `impl QmlRegister { URI = "RowPlay"; IS_SINGLETON = true; }`
(note #1/#2 workaround):

- `Library` — owns the workout set (`Vec<Workout>` from cache/demo via
  `rowplay_platform::library::load_library`), the `WorkoutListQuery`, the
  sidebar `QListModel` (`#[qobject(Base = QListModel)]` inner object with a
  `SidebarRow` `QModelItem`: `id`, `title`, `dateText`, `distanceText`,
  `paceText`, `sportIcon`, `isPb` — ≤ 15 roles), section headers as a parallel
  `Vec<String>` model, plus `selectedWorkoutId`, `countText` inputs
  (`workoutCount`), and slots `setSportFilter`, `setSearchText`, `toggleSort`,
  `select`, `reload`. Bulk loads go through `reset_unnotified` + `reset()`.
- `Detail` — `hasSelection`, and for the selected workout the header strings,
  metric-strip `serde_json::Value` array, splits `QListModel`, stroke series
  (`Vec<f64>` flattened x/y pairs for Qt Graphs `replace`), `powerText` etc.
  Recomputed on selection change; strokes downsampled to ≤ 500 points in the
  view-model.
- `Settings` — `hasToken` (bool, the only token-derived value that crosses the
  bridge), `demoMode`, `distanceUnitIndex`, `languageIndex`/`languageCodes`,
  `timezoneIndex`/grouped names, slots `saveToken(String)` (Rust immediately
  moves it into `SecretToken` + keyring and the QML string is the only copy —
  the adapter overwrites its local buffer with an empty string afterwards),
  `disconnect()`, `setDemoMode`, `setUnit`, `setLanguage`, `setTimezone`.
  Persistence via `rowplay_platform::preferences`.
- `Sync` — `state` (idle/running/done/failed as int + `stateText` vars),
  `progress` (0..1 + counts), `lastResultText` vars, slots `start()`,
  `cancel()`.

Threading: `Sync.start()` spawns a `std::thread` running
`WorkoutSyncCoordinator::sync_with` with the progress callback pushing
`SyncProgress` into an `mpsc::Sender`. The backend is created on the Qt thread
(`default_with_attached_qobject`), grabs `get_qml_method_invoker()` before
spawning, and the worker calls `invoke_method("pumpEvents")` after each send
(queued connection). The `pumpEvents` `#[qslot]` drains the channel, updates
members and emits notify signals. Cancel flips the coordinator's `AtomicBool`.
Fallback if the invoker misbehaves (RefCell double-borrow, note in
qt-bridges-notes): a QML `Timer { interval: 50 }` calling the same slot.

State shared between the worker and the Qt thread lives in
`Arc<Mutex<SyncShared>>` (cancel flag, last result); the channel carries
progress events. The worker never touches a QObject directly.

The app composition root (`main.rs` / a small `app_state` module) wires
platform services: `paths` → cache/preferences files, `KeyringTokenStore`,
`Concept2HttpClient`, demo fallback — mirroring Studio's
`Concept2SyncController` (deferred from Phase 3 into this layer).

## QML tree

```
qml/RowPlay/
  qmldir            # adds Theme, Tr, singletons are Rust-registered
  Main.qml          # ApplicationWindow shell: toolbar, SplitView, screen stack
  Theme.qml         # singleton: palettes, spacing, radii, typography, roles
  Tr.qml            # singleton: t(id, vars) over qsTrId + interpolate
  SettingsScreen.qml
  SidebarPanel.qml  # 4b
  DashboardScreen.qml, MetricTile.qml   # 4b
  DetailScreen.qml, StrokeAnalysisPanel.qml, ChartsPanel.qml  # 4b
  i18n/qml_<lang>.qm                    # generated into the rcc by build.rs
```

`Main.qml` becomes a `Controls.ApplicationWindow`. qtbridge has no
`QQuickStyle` binding and `std::env::set_var` is `unsafe` in edition 2024
(forbidden), so the Fusion style is forced with a `qtquickcontrols2.conf`
(`[Controls]` / `Style=Fusion`) embedded in the qrc at
`:/qtquickcontrols2.conf`, which Qt Quick Controls reads at startup — the
documented resource-based configuration path. Window: title "rowplay",
`minimumWidth: 1000`, `minimumHeight: 680`.

Theme singleton: `property bool dark: Qt.styleHints.colorScheme === Qt.Dark`
(with `Unknown` resolved from the palette brightness), every colour a
`dark ? … : …` binding so re-evaluation is automatic. Roles mirror
`AppDesign.MetricColor`; `deltaColor(delta, threshold, higherIsBetter)` ports
the helper. Fonts: `Font.system` family; pixel sizes/weights per DESIGN.md;
the OpenType `tnum` feature (`font.features: { "tnum": 1 }`) replaces
`.monospacedDigit()`; the hero style keeps weight Bold (no rounded system
font — divergence recorded). This design first named `Font.TabularNumbers`,
which Qt 6.11 does not have: a font object drops the undefined value
silently, so no text had tabular figures until the UI fixes (refs #54,
`docs/qt-bridges-notes.md`).

Navigation: `nav` state in the `Library`/app backend mirrors
`DetailNavigationState`; QML `StackLayout` (dashboard | detail) + the replay
route slot disabled in Phase 4 (Phase 5). `Escape` clears selection
(Studio's hidden clear button), Ctrl/Cmd+1 → dashboard, Ctrl/Cmd+R → reload.

Charts (4b): Qt Graphs `GraphsView` with `LineSeries`/`BarSeries`; series
filled via `replace(...)` from flat `Vec<f64>` bridge properties — never point
by point. Axis label formatting uses pre-formatted tick strings from the
view-model where the axis is data-driven (pace axis).

## i18n pipeline

Conversion (`tools/convert-locales.mjs`, Node ≥ 22 type stripping, no deps):

1. Import the six `reference/rowplay/src/lib/locales/*.ts` (named exports).
2. Flatten to dotted keys (908 per locale; identical key sets — verified).
3. Emit `i18n/rowplay_<lang>.ts` (Qt TS 2.1): one `<context name="rowplay">`,
   one `<message id="dotted.key">` per key, `<source>` = English text,
   `<translation>` = locale text (English fill for missing keys, `type`
   attribute omitted; `en` translates to itself so the id lookup always
   resolves). XML-escaped, `{} placeholders untouched, no numerus forms.
4. `--check` mode diffs against committed files (used by the parity test when
   `reference/` exists).

`Tr.qml`:

```qml
pragma Singleton
QtObject {
    function t(id, vars) {
        var s = qsTrId(id)
        if (s.length === 0) s = id      // no translator loaded: degrade visibly
        if (!vars) return s
        for (var k in vars) s = s.replaceAll("{" + k + "}", String(vars[k]))
        return s
    }
}
```

(web `interpolate` parity: replaceAll per variable, String(value)).

`build.rs`: find `lrelease` next to `rcc` (same qmake query), run it per
`i18n/rowplay_<lang>.ts` into `OUT_DIR/i18n/qml_<lang>.qm`, and a generated
qrc fragment (or direct entries in `qml/rowplay.qrc` pointing at
`../../target/…` — rejected: qrc paths must be stable) — instead build.rs
writes `OUT_DIR/rowplay_i18n.qrc` mapping
`i18n/qml_<lang>.qm` under prefix `/qt/qml/RowPlay`, runs `rcc --binary` on
both qrc files and the app registers both blobs. When `lrelease` is missing
the build fails with an actionable message (Qt is required for this crate
anyway).

Language switch: `Settings.setLanguage` persists the choice and the backend
mirrors it into a `language` property; `Main.qml` binds
`Qt.uiLanguage = Settings.languageCode`. QQmlApplicationEngine reloads
`qml_<lang>.qm` from `:/qt/qml/RowPlay/i18n/` (Qt 6.11 documented behaviour).
Startup: the backend's `Default` reads preferences before QML loads, so the
first binding already carries the stored language.

Parity tests (`crates/rowplay-viewmodel/tests/i18n_parity.rs`, Qt-free):

- parse the six committed `i18n/rowplay_*.ts`; assert identical id sets and
  908 ids; assert every `<source>` matches across locales; assert `{var}`
  placeholder sets are equal per id across locales.
- scan `qml/**/*.qml` for `Tr.t("literal")` / `t("literal")` ids; assert each
  id is in the key set (ids built dynamically are not allowed — lint rule).
- when `reference/rowplay/src/lib/locales/` exists, re-run the generator
  in-memory and assert it matches the committed files byte for byte.

## Runtime-error gate

`crates/rowplay-app/tests/qml_runtime_gate.rs`: launches the binary with
`ROWPLAY_QT_SMOKE=1` + `ROWPLAY_SMOKE_GATE=1` (demo data, offscreen-capable
platform), the app walks every screen (select dashboard → settings → sidebar
selection via backend slots on a timer), captures stderr, and the test fails
on `TypeError`, `ReferenceError`, `Binding loop`, `Unable to assign`,
`is not defined`. The gate also runs as part of the existing screenshot flow
in CI (Xvfb) and produces the 4b screen screenshots
(`ROWPLAY_SMOKE_SCREEN=dashboard|detail|settings` → grabToImage PNGs uploaded
as artifacts).

## Performance harness (4b)

`crates/rowplay-viewmodel/benches/`-style test (plain `cargo test`, measured
and printed, `#[ignore]` by default to keep CI fast; run manually + in the PR
report): generate 5,000 synthetic workouts (seeded, demo-generator based),
time `library.filtered_rows()` cold/warm for representative queries, assert
< 50 ms; time `QListModel` reset path in the app crate behind the smoke env.
Scroll smoothness: the list model only realises visible delegates (Qt
behaviour), verified by manual pass + frame timing printout in smoke mode.

## Key decisions and divergences (for source-map)

1. View-model crate inserts between core and app: app → viewmodel → platform →
   core (extends ADR 0006 layering; viewmodel may not use platform I/O types
   directly, it consumes their outputs).
2. Date display via a six-language table in `dates` instead of `QLocale`
   (overrides the Phase 1 source-map note; keeps rule "no formatting in QML"
   and stays Qt-free/testable).
3. Studio's hardcoded English settings labels → web locale ids
   (`token.connect` for "Save Token", `dashboard.sync` for "Sync Now",
   `auth.logout` for "Disconnect", `common.demoMode` for the demo toggle);
   metric/imperial picker uses the unit symbols `km`/`mi` (untranslated by
   design, like the web's chart axis labels).
4. Fusion style via an embedded `:/qtquickcontrols2.conf` (qtbridge 0.2.0 has
   no `QQuickStyle` binding and `set_var` is `unsafe` in edition 2024).
5. Window title: "rowplay" (the desktop app's own name; Studio's
   "RowPlay Studio" is product-specific).
6. Imperial distance formatting uses core's `fmt_distance_in` (Studio
   behaviour, already ported in Phase 1).
