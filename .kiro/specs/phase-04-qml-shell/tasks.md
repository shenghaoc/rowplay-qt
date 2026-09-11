# Phase 4 — QML shell: tasks

Spec first, then one small commit per deliverable. Two stacked PRs: 4a on
`phase-4a-shell` (off `main`), 4b on `phase-4b-screens` (off `phase-4a-shell`).

## PR 4a — foundation

- [x] Write the Phase 4 spec (requirements, design, tasks) and commit it before
      any code.
- [ ] `crates/rowplay-viewmodel`: crate skeleton (forbid unsafe, pedantic
      lints, SPDX headers), added to `default-members` and the Qt-free CI job.
- [ ] `viewmodel::nav`: `DetailNavigationState` port with unit tests
      (re-expressed from Studio's `ReplayNavigationTests`).
- [ ] `viewmodel::dates`: six-language month/day-name tables + `fmt_date_short`
      / `fmt_date_full` / `fmt_time_of_day` on core datetime, with tests.
- [ ] `viewmodel::settings`: language list, grouped timezone list (chrono-tz),
      unit mapping and validation, with tests.
- [ ] `rowplay-platform::preferences`: add the `language` preference
      (default `en`), round-trip test.
- [ ] Theme: `qml/RowPlay/Theme.qml` singleton porting `DesignTokens.swift` +
      `DESIGN.md` (palette light/dark via `Qt.styleHints.colorScheme`, metric
      colours, spacing, radii, typography scale, delta-colour helper).
- [ ] Fusion style: `qtquickcontrols2.conf` in the qrc; palette coloured from
      `Theme.qml`.
- [ ] Backend singletons: `Settings` and `Sync` (thin adapters; `hasToken`
      only; keyring save/disconnect; preferences persistence).
- [ ] Sync worker thread: `std::thread` + `mpsc` + `QmlMethodInvoker`
      (`pumpEvents` slot), cancel via the coordinator `AtomicBool`; findings
      recorded in `docs/qt-bridges-notes.md`.
- [ ] App shell: `Main.qml` `ApplicationWindow` (title, 1000×680 minimum),
      split view with sidebar/detail placeholders, toolbar (sport filter,
      reload, settings), keyboard shortcuts, empty state.
- [ ] Settings screen: token section (password field, save, disconnect with
      confirmation), preferences (units, timezone, language), demo-mode
      toggle, sync section (progress, cancel, last result).
- [ ] i18n: `tools/convert-locales.mjs` (six locales → ID-based `.ts`, English
      fill, `--check` mode); committed `i18n/rowplay_*.ts`.
- [ ] i18n: `Tr.qml` (`t(id, vars)` = `qsTrId` + web-`interpolate` parity);
      every QML string in 4a through `Tr.t`.
- [ ] i18n: `build.rs` runs `lrelease` and bundles `qml_<lang>.qm` into the
      rcc under `:/qt/qml/RowPlay/i18n/`; `Qt.uiLanguage` bound to the
      language preference; live switch verified by hand in all six languages.
- [ ] i18n parity tests (Qt-free, in CI): identical id sets (908), QML id
      scan, regeneration check when `reference/` exists.
- [ ] QML runtime-error gate: smoke mode walks the shell screens; test fails
      on `TypeError` / `ReferenceError` / `Binding loop` / `Unable to assign`
      / `is not defined`; wired into CI next to the smoke screenshot.
- [ ] Docs: roadmap, source-map rows + divergences, qt-bridges-notes (new
      findings incl. no QTranslator/engine access on QApp), AGENTS.md (new
      crate + i18n workflow), README (RHEL recipe corrections: pip→uv/pipx,
      Wayland smoke without Xvfb).
- [ ] Validation: fmt / clippy (`RUSTFLAGS="-D warnings"`, `--workspace
      --all-targets`) / test / `git diff --check`; manual Wayland pass light +
      dark; open the PR with scope, results, versions, findings.

## PR 4b — screens

- [ ] Backend: `Library` singleton + sidebar `QListModel` (`SidebarRow`
      `QModelItem` ≤ 15 roles), bulk reset loading; `Detail` singleton
      (header, metric strip, splits model, stroke series).
- [ ] `viewmodel::library` / `dashboard` / `detail` / `strokes` with tests
      (Studio statics ported: `toggleSort`, `pbLabel`, `powerText`,
      `downsampleStrokes`, chart domains, split boundaries; dashboard summary
      + PB derivations from core analytics).
- [ ] Sidebar screen: grouped list, sport/date-range/text filters, sort menu,
      PB badges, keyboard navigation, `Accessible.name`.
- [ ] Dashboard screen: metric tiles (`MetricTile.qml`), personal bests,
      period summary, Qt Graphs bar + line charts fed by `replace(...)`.
- [ ] Detail screen: header, metric strip, splits/intervals table, HR and
      targets display.
- [ ] Stroke analysis: pace/power charts with split boundaries and average
      rules, empty-strokes state, handling synthesised strokes like Studio.
- [ ] Performance: 5,000-workout synthetic library; filter < 50 ms; smooth
      sidebar scroll; numbers recorded in the PR.
- [ ] Runtime-error gate walks every screen; CI screenshots (dashboard,
      detail, settings) uploaded as artifacts.
- [ ] Docs: source-map rows for every ported view, roadmap Phase 4 done,
      README screenshots (demo data), deferred list in the PR.
- [ ] Validation as 4a, plus manual pass over every screen (light/dark,
      Wayland); open the PR against `phase-4a-shell` (retarget to `main` when
      4a merges).

## Deferred (listed in both PRs, not built)

Replay (Phase 5), live mode (Phase 8), HR import, annotations, comparison
panel, file actions/export, rival controls.
