# Source map: web → Swift → Rust

This file maps the rowplay web app (canonical behaviour) and rowplay-studio
(second reference) onto rowplay-qt, and records every deliberate divergence.
When the web and Swift versions disagree, the web wins unless Studio's source
map documents a deliberate deviation; the "Divergences" section lists what the
Rust port chose and why.

## Pinned references

| Repository | Branch | Commit | Local checkout (git-ignored) |
| --- | --- | --- | --- |
| `https://github.com/shenghaoc/rowplay` | `main` | `011e8303b66b4d2265a6f1ec8b3ed9d8ed497086` | `reference/rowplay` |
| `https://github.com/shenghaoc/rowplay-studio` | `main` | `3d406a5b7677372de35fb0817c7133a2589c6564` | `reference/rowplay-studio` |
| `https://github.com/qt/qtbridge-rust-examples` | default | `763bbfbd400791ac09da9d8ba3903310c4313fe2` | `reference/qtbridge-rust-examples` |

Studio's own `docs/source-map.md` (web → Swift) remains the authority for the
Swift column; the rows below extend it with the Rust target. The golden
fixtures in `tests/fixtures/` come from Studio at the commit above
(`tests/fixtures/PROVENANCE.md`).

## Phase 1 — core parity foundation

| Web source | Swift (rowplay-studio) | Rust (rowplay-qt) | Notes |
| --- | --- | --- | --- |
| `src/lib/types.ts` | `Sources/RowPlayCore/Models/{Sport,Workout,DistanceUnit}.swift` | `crates/rowplay-core/src/models.rs` | Full web field set; camelCase JSON; `spm`/`hr` accept Studio's `cadence`/`heartRate` aliases; `Split.index` is 0-based (web). `Workout.date` stays the logbook string. |
| `src/lib/types.ts` (`toSport`) | `Sport.fromConcept2Type` | `Sport::from_concept2_type` | Exact, case-sensitive match (web); Studio lower-cased first. |
| `src/lib/format.ts` | `Support/RowPlayFormatting.swift` | `formatting.rs` (+ `num.rs` for `Math.round` / `toFixed`) | `fmt_time`, `fmt_pace`, `fmt_pace_bare`, `fmt_distance`, `pace_to_watts[_for_sport]`, `watts_to_pace[_for_sport]`, `challenge_distance_metres`, `avg_watts`. Imperial (`fmt_distance_in`) and race margins (`fmt_distance_margin`) come from Studio. |
| `src/lib/datetime.ts` | `Support/RowPlayDateTime.swift` | `datetime.rs` | Logbook parsing, day keys, day/month arithmetic, `parse_instant_millis`, `workout_local_day_key`, `today_key_for_tz`, ISO output, `logbook_date_plus_seconds_iso`. Locale display formatting (`fmtDate`, `fmtLogbookDateTime`, `fmtDateFromEpochMillis`, `monthShortName`, `fmtTimeFromEpochMillis`) is delegated to `QLocale` in Phase 4. |
| `src/lib/paceInput.ts` | `Support/PaceInput.swift` | `pace_input.rs` | Web semantics with Studio's input bounds (raw ≤ 64, trimmed ≤ 20). |
| `src/lib/privacy.ts` | `Support/PrivacyRedaction.swift` | `privacy::is_publicly_shareable` | Fail closed; only `everyone`. |
| `src/lib/server/logger.ts` | `Support/PrivacySafeLogger.swift` | `privacy::{redact, PrivacySafeLogger, LogSink}` | Union of the web and Studio patterns, Studio's 16 KiB bound. The core exposes a sink trait; `rowplay-platform::logging::StderrSink` is the I/O sink. |
| `src/lib/analytics.ts` (`linearTrend`, `distanceBand`, `durationBand`, `summariseBySport`) | `Analytics/WorkoutAnalytics.swift` | `analytics.rs` | Studio's `strokeSummary`, `dashboardSummary`, `dashboardPersonalBests`, `recentPaceWorkouts` are included; `x` is epoch ms and `n` keeps the web name. |
| `src/lib/analytics.ts` (`distancePBs`, `pbWorkoutIds`, `detectNewPBs`) | `Analytics/PersonalBests.swift` | `personal_bests.rs` | Per sport at the seven web standard distances (±2%). `standard_distance_matching` comes from Studio. |
| `src/lib/performancePredictor.ts` | `Analytics/PerformancePredictor.swift` | `performance_predictor.rs` | Golden fixture `performance-predictor-parity.json` (0.5%). |
| `src/lib/workoutQuery.ts` | `Library/WorkoutQuery.swift` | `workout_query.rs` | Parse / serialise (search-param pairs), `list_query_is_filtered`, `avg_power_watts`, `pb_workout_ids(workouts, sport)`, `filter_and_sort_workouts`, chip toggles; Studio's `clear_filters` and chip labels added. |
| `src/lib/workoutTag.ts` | — (Studio did not port; the web still uses `resolveTag` in the list filter) | `workout_tag.rs` | `auto_detect_tag`, `resolve_tag`, `athlete_median_pace`. |
| `src/lib/mockData.ts` | `Fixtures/DemoWorkoutLibrary.swift` | `demo.rs` | Web generator byte for byte (`mock_workouts`, `mock_workout_detail`, `demo_details`). `generateMockWorkout` (live-mode mock) is Phase 8. |
| `tests/unit/fixtures.ts` (`workout()`) | test helpers | `Workout::new` | Same defaults as the web test helper. |
| — | `Tests/RowPlayCoreTests/Fixtures/ParityFixture{,Loader}.swift` | `crates/rowplay-fixtures` | `load_json`, `read_bytes`, SHA-256 manifest check. |
| — | `Library/WorkoutLibraryLoader.swift`, `WorkoutLibrarySnapshot.swift`, `WorkoutLibrarySource.swift` | `rowplay-platform::library` | cache → demo → empty; cache errors propagate. |
| `src/lib/server/session.ts` (token handling) | `Sync/TokenStore.swift`, `Platform/KeychainTokenStore.swift` | `rowplay-platform::token_store` | Trait + `SecretToken` + in-memory mock; the `keyring` store landed in Phase 3 (see below). |
| `src/lib/server/concept2.ts` (client) | `Sync/Concept2Client.swift`, `Concept2/*.swift` | `rowplay-platform::concept2` | Trait + mock; the HTTPS client and the raw-payload mapper landed in Phase 3, enabling the mapper parity test. |
| — (web is stateless) | `Sync/WorkoutCache.swift`, `Storage/SQLiteWorkoutCache.swift` | `rowplay-platform::workout_cache` | Trait + in-memory and failing mocks; the `rusqlite` store landed in Phase 3. |
| — | `Platform/AppPreferences.swift` | `rowplay-platform::preferences` | `Preferences` has no token field by construction; the JSON file store landed in Phase 3. |

## Phase 2 — replay core

Ported into `rowplay-core` as `rowplay_core::replay::*`; unit tests re-express
the web `*.test.ts` and Studio `*Tests.swift` suites, golden fixtures run from
`crates/rowplay-core/tests/parity.rs`.

| Web source | Swift (rowplay-studio) | Rust (rowplay-qt) | Notes |
| --- | --- | --- | --- |
| `src/lib/replay/engine.ts` (`sampleAt`, `sampleIndexAt`) | `Replay/ReplaySample.swift` | `replay::engine::{sample_at, sample_index_at, Frame}` | Web math; Studio's non-finite-`t` zero-frame guard kept; HR/watts stay `f64` lerp (web) rather than Studio's integer rounding. |
| `src/lib/replay/engine.ts` (`ReplayEngine`) | `Replay/ReplayState.swift` | `replay::engine::{ReplayState, ReplaySpeed}` | Tick-driven state machine (Studio): the QML `FrameAnimation` owns the clock, replacing the web's `requestAnimationFrame` loop. |
| `src/lib/replay/motion.ts` | `Replay/ReplayMotion.swift` | `replay::motion` | `meters_per_cycle`, `clamp_dt`, `damp_factor`, `stroke_surge`, `catch_events`, `ParticlePool`, `PerfGovernor`. |
| `src/lib/replay/motion.ts` (`warpStrokePhase`) | `Replay/ReplayMotion.swift` | `replay::motion::{warp_stroke_phase, warp_stroke_phase_rate}` | **C1 divergence, mandated by the roadmap** — see the divergences table. |
| `src/lib/replay/motion.ts` (`PerfGovernor`) | `Replay/ReplayPerformanceGovernor.swift` | `replay::motion::PerfGovernor` | Median calibration capped at 2× floor, EMA sampling, sustained-over window, post-step grace, sticky levels; `reset`/`is_calibrated` accessors from Studio. |
| `src/lib/replay/strokeModel.ts` | `Replay/ReplayStrokePose.swift`, `ReplayStrokePoseAggregates.swift` | `replay::stroke_model` | Web pipeline (`build_stroke_timeline`, `stroke_pose_at`, `fallback_stroke_pose`, `catch_transitions`) plus Studio's frame-based `compute_at_time` for the fixture (see divergences); `reduced_motion` kept from Studio. |
| `src/lib/replay/motionGraph.ts` | `Replay/ReplayMotionGraph.swift` | `replay::motion_graph` | Full channel set, timing and curve algebra with the web's evaluation order; golden corpus `replay-current-main-motion.json` matches within 1e-10. The web's `*Into` scratch samplers (a JS GC optimisation) are not ported. |
| `src/lib/replay/sportKinematics.ts` | `Replay2D` projections | `replay::sport_kinematics` | `solve_rower/skier/bike_kinematics`, `solve_skier_elbow_direction`; corpus `replay-current-main-2d.json` within 1e-10. |
| `src/lib/replay/renderer.ts` (`COLORS_*`, `VENUES_*`) | — (fixture `palettes` block) | `replay::theme` | Exact web palette strings, asserted against the 2D fixture. |
| `src/lib/replay/comparabilityGuard.ts` | `Replay/ComparabilityGuard.swift` | `replay::comparability` | `classify_axis`, `are_comparable` via the Phase 1 band helpers. |
| `src/lib/replay/ghostPick.ts` | `Replay/GhostPick.swift` | `replay::ghost_pick` | Web ranking semantics exactly (see divergences for Studio's hardening). |
| `src/lib/replay/replayGap.ts` | `Replay/ReplayRaceGap.swift` | `replay::race_gap` | Web helpers + Studio's non-zero-origin helpers (`relative_duration`, `absolute_time`, `ghost_frame`, `ghost_distance`). |
| `src/lib/replay/replayGap.ts` (finish behaviour) | `Replay/ReplayRaceResult.swift` | `replay::race_result` | Studio's interpolated target crossing, tie tolerances 0.05 s / 0.5 m, DNF handling. |
| `src/lib/replay/sources.ts` | `Replay/ReplayRival*.swift` | `replay::rivals` | `constant_pace_strokes` (per-sport watts), `parse_rival_file` with CSV/TCX/FIT decoders and Studio's bounds + normalisation (see divergences). |
| `src/lib/replay/replayRenderer.ts` (`RenderQuality`) | `Replay/ReplayRenderQuality.swift` | `replay::quality` | Enum + Studio's portable per-tier budgets and sticky degradation ladder. |

## Phase 3 — platform

Ported into `rowplay-core::concept2` (pure, byte slices in) and the
`rowplay-platform` services behind the Phase 1 traits.

| Web source | Swift (rowplay-studio) | Rust (rowplay-qt) | Notes |
| --- | --- | --- | --- |
| `src/lib/server/concept2.ts` (raw shapes, `mapResult`, `mapStrokes`, `mapSplits`, `mapHeartRate`, `mapTargets`, `mapMetadata`, `mapSplitType`, `synthStrokes`, `getWorkout`) | `Concept2/Concept2Models.swift`, `Concept2/Concept2Mapper.swift` | `rowplay-core::concept2` | Wire units normalised exactly as the web does: `time`/`t`/`rest_time` tenths → seconds, `d` decimetres → metres, `p` and target `pace` tenths → sec/500 m with the BikeErg divisor of 2, interval `t`/`d` resets accumulated into offsets, `raw_t`/`raw_d` kept. Payloads are bounded to 25 MiB and fail with an opaque typed error. |
| `src/lib/server/concept2.ts` (`getWorkout` assembly) | `Concept2/URLSessionConcept2Client.swift` (`fetchWorkoutDetail`) | `rowplay-core::concept2::assemble_detail` | `is_interval` from a non-empty `workout.intervals`, strokes fetched only when `stroke_data` is set, and `synth_strokes` clearing `has_stroke_data` when the timeline is split-derived. |
| `src/lib/server/concept2.ts` (client, `Accept`, auth header) | `Sync/Concept2Client.swift`, `Concept2/Concept2Endpoint.swift`, `Concept2/Concept2Error.swift`, `Concept2/HTTPTransport.swift`, `Concept2/URLSessionConcept2Client.swift` | `rowplay-platform::concept2::{Concept2HttpClient, HttpUri, redirect_target}` | Blocking `ureq` over rustls instead of `URLSession`. HTTPS only (loopback excepted), bearer token via `SecretToken`, no cookies, no HTTP cache, 30 s / 300 s budgets, 25 MiB body cap, `max_redirects(0)` with a hand-written policy, and Studio's status mapping. |
| — (the web app is a server; the native app keeps its own cache) | `Sync/WorkoutCache.swift`, `Storage/SQLiteWorkoutCache.swift`, `Storage/SQLiteWorkoutCacheMigration.swift` | `rowplay-platform::workout_cache` | `SqliteWorkoutCache` on `rusqlite` (`bundled`), Studio's `workouts` table shape, `PRAGMA user_version` migrations, `0700`/`0600` on Unix. |
| — | `Platform/KeychainTokenStore.swift`, `Sync/TokenStore.swift` | `rowplay-platform::token_store` | `KeyringTokenStore` with an explicit native backend per target plus a test that fails if keyring substitutes its mock store. |
| `src/lib/replay/replayRenderer.ts` (`safeStorage`) | `Platform/AppPreferences.swift` (`UserDefaults`) | `rowplay-platform::preferences` | JSON file, atomic temp-file-and-rename writes, tolerant reads. |
| `src/lib/server/data.ts` (`SyncState`) | `Sync/WorkoutSyncCoordinator.swift`, `Sync/SyncStateTracker.swift`, `Sync/WorkoutSyncResult.swift`, `Sync/WorkoutSyncError.swift` | `rowplay-platform::sync` | Synchronous and cancellable (`AtomicBool` + progress callback) so Phase 4 can run it on a worker thread. |
| — | `Platform/Concept2SyncController.defaultCachePath` | `rowplay-platform::paths` | `directories`-based data and config directories with the Unix permission helpers. |
| `src/lib/server/logger.ts` (levels) | `PrivacySafeLogger` | `privacy::LogLevel` / `PrivacySafeLogger::info` | An informational level was added so a normal fallback (no preferences file yet) is not reported as a warning. |

## Phase 4 — QML shell

UI logic lives in the Qt-free `rowplay-viewmodel` crate; the qtbridge objects
in `rowplay-app/src/backend/` are thin adapters (Phase 4 ground rules).

| Web source | Swift (rowplay-studio) | Rust / QML (rowplay-qt) | Notes |
| --- | --- | --- | --- |
| `src/lib/i18n.ts` (`interpolate`, per-key English fallback), `src/lib/locales/*.ts` | — (Studio is not localised) | `tools/convert-locales.mjs`, `i18n/rowplay_*.ts`, `qml/RowPlay/Tr.qml` | ID-based Qt catalogues: message id = the web's dotted key, empty `<source>` (required for `qsTrId` lookup), English in `<oldsource>`, missing keys filled with English. `Tr.t(id, vars)` = `qsTrId` + the web `interpolate` semantics. No numerus forms (the web has no plural rules). Parity tests in `crates/rowplay-viewmodel/tests/i18n_parity.rs`. |
| `src/lib/datetime.ts` (`fmtDate`, `fmtLogbookDateTime`, `fmtTimeFromEpochMillis`, `monthShortName`) | SwiftUI `Date.FormatStyle` | `rowplay_viewmodel::dates` | Six-language pattern and month-name tables, golden-tested against Node `Intl` output from the pinned web checkout. Supersedes the Phase 1 "delegate to `QLocale`" note (see divergences). |
| `src/lib/timezoneOptions.ts` (`TIMEZONE_OPTIONS`) | — | `rowplay_viewmodel::settings::timezone_options` | Verbatim port, including the Unicode minus signs in the labels; group headers stay locale ids resolved through `Tr.t`. |
| `src/lib/i18n.ts` (`SUPPORTED_LANGUAGES`, `LANGUAGES`) | — | `rowplay_viewmodel::settings::Language` | Codes `en zh de es fr ja`, picker endonyms, `from_preference` with the web's English fallback. |
| — | `Views/DesignTokens.swift`, `DESIGN.md` | `qml/RowPlay/Theme.qml` | Palette (light/dark hex pairs), metric colours, spacing/radius/chart scales, `deltaColor` helper, typography scale on the system font; follows `Qt.styleHints.colorScheme`, pinnable via `ROWPLAY_FORCE_COLOR_SCHEME` for tests. |
| — | `Views/ContentView.swift`, `App/RowPlayStudioApp.swift` | `qml/RowPlay/Main.qml`, `rowplay-app/src/backend/{library,detail}.rs` | Split view (sidebar min 260 / ideal 320), toolbar sport filter + reload + settings, empty state, Ctrl/Cmd+1 / Ctrl/Cmd+R / Escape shortcuts, window "rowplay" 1000×680 minimum, dashboard/detail/settings routing. |
| — | `Views/DetailNavigationState.swift` (+ `ReplayNavigationTests.swift`) | `rowplay_viewmodel::nav` | Route stack, replay-unavailability policy and invocation-time selection resolution; the Swift tests are re-expressed against the demo library. |
| `src/routes/settings/+page.svelte`, `src/routes/auth/token/+page.svelte` | `Views/SettingsView.swift` | `qml/RowPlay/SettingsScreen.qml`, `rowplay-app/src/backend/settings.rs` | Token → `SecretToken` → keyring inside Rust; QML sees `hasToken` only. Units / home timezone / language pickers persist through `rowplay-platform::preferences` (new `language` field). Demo-mode toggle; sync disabled in demo mode. |
| `src/lib/server/data.ts` (sync flow) | `Platform/Concept2SyncController.swift` | `rowplay-app/src/backend/sync.rs` | The Phase 3-deferred composition root: `std::thread` worker runs `WorkoutSyncCoordinator::sync_with`, events flow over `mpsc`, the Qt thread is poked through `QmlMethodInvoker` (+ a 50 ms QML timer safety net), cancel flips the `AtomicBool`. `ROWPLAY_SYNC_MOCK=1` swaps in `MockConcept2Client` so CI exercises the whole path token-free. |
| `src/lib/workoutQuery.ts` list UI, `src/components/WorkoutList.svelte` | `Views/SidebarView.swift` | `qml/RowPlay/SidebarPanel.qml` + `rowplay_viewmodel::library` + the `Library` `QListModel` | Filter/sort through the core query engine (web canonical); rows pre-rendered in Rust; PB badges from the unfiltered PB set (Studio); day sections are a desktop addition (neither reference groups the list) using `workout_local_day_key`. Search is debounced 250 ms. |
| `src/routes/dashboard/+page.svelte` (tiles, PB panel, trend charts) | `Views/DashboardView.swift`, `Views/MetricTile.swift` | `qml/RowPlay/{DashboardScreen,MetricTile}.qml` + `rowplay_viewmodel::dashboard` | Tiles via `dashboard_summary` (web), PB cards via `dashboard_personal_bests` with web `distanceBand` labels ("2k", "Half", "Full"), charts via Qt Graphs (`GraphsView` + `LineSeries.replace(list<point>)` bulk loads). The Studio "Challenge" tile is dropped (no web tile, no locale key). |
| `src/routes/replay/[id]/+page.svelte` (metrics, splits table, targets gauges) | `Views/WorkoutDetailView.swift` | `qml/RowPlay/DetailScreen.qml` + `rowplay_viewmodel::detail` | Strip order and `powerText` from Studio; table/section/target strings from the web (`replay.th*`, `replay.splitBreakdown`, `replay.mTarget*`). Split HR falls back from `heartRate.average` to the legacy scalar `hr`. |
| `src/lib/replay/*` chart derivations | `Views/WorkoutStrokeAnalysisView.swift` | `qml/RowPlay/{StrokeAnalysisPanel,StrokeChart}.qml` + `rowplay_viewmodel::strokes` | `downsampleStrokes` (500, endpoints kept), pace domain (12% pad, 3 s floor, −180…−60 placeholder) and split boundaries ported verbatim; the Phase 4 brief adds rate and HR charts (HR draws the first gap-free segment). Synthesised strokes chart like recorded ones (web behaviour). |

## Later phases (mapping only)

| Web source | Swift | Rust target | Phase |
| --- | --- | --- | --- |
| `src/routes/dashboard`, `src/components/*` | `Views/*.swift` | `qml/RowPlay/*.qml` + backend objects in `rowplay-app` | 4 |
| `src/lib/locales/*.ts` | — | `i18n/*.ts` via `tools/` | 4 |
| `src/lib/replay/renderer3d.ts`, `renderer3dAssets.ts`, `renderer3dV4Assets.ts` | `Views/Replay3D/*.swift` | `qml/RowPlay/Replay/*.qml`, `assets/*.glb` | 5 |
| `src/lib/replay/renderer3dEnvironment.ts` | `ReplayEnvironment*.swift` | baked `assets/venues/*.glb` (ADR 0005) | 6 |
| `src/lib/replay/renderer3dV4Motion.ts`, `rigV4.ts`, `handGrip.ts` | `ReplayAthleteContactSolver.swift`, `ReplayHandClosure.swift` | `rowplay-core::replay::motion_graph` driving the V4 athlete | 7 |
| `src/lib/liveMode.ts`, `liveMode.svelte.ts` | `Live/*.swift`, `Connectivity/*.swift` | `rowplay-platform::live`, `btleplug` transport | 8 |

## Divergences recorded by the Rust port

Web wins unless stated. "Kept from Studio" means the web has no equivalent.

| Area | Web | Swift | Rust | Rationale |
| --- | --- | --- | --- | --- |
| Concept2 mapper defaults | absent `workout_type` / `verified` stay `undefined` | `JustRow`, `verified = true` | web | The web keeps the API's `undefined`; Studio's defaults are undocumented. |
| Concept2 `is_interval` | set by `getWorkout` from a non-empty `workout.intervals`; summaries have none | set in `mapWorkout` from `workout_type` containing "interval" **or** non-empty intervals | web | One rule (the intervals array) for summaries and detail alike; the API's `workout_type` values are not documented as an interval signal. |
| Detail assembly without per-stroke rows | synthesises a split-derived timeline and clears `hasStrokeData` | returns empty strokes, keeps `hasStrokeData` | web | `hasStrokeData` must go false when the timeline is synthesised, or the pose model renders one cycle per synthesised point (≈4 catches across a 2K instead of ≈221). |
| Concept2 status mapping | throws a message with the status, no typed cases | `unauthorized` / `forbidden` / `rateLimited` / `httpError(statusCode)` | web's statuses with Studio's typed cases, plus `NotFound(id)` for the in-memory mock only | A 404 from the API maps to `Http { status: 404 }` (the transport's job); `NotFound(id)` stays the mock's domain error. `RateLimited` also carries `Retry-After` seconds when the header is a plain integer (no reference parses it). |
| `total_pages` fallback | the page it just fetched (`?? page`) | `1` | web | Only observable above page 1, which the sync never requests; the web's rule is kept. |
| Sync paging bound | running maximum of `total_pages` | assigns the reported value | web | A monotone bound cannot be cut short by a misbehaving page response. |
| Redirect policy | browser `fetch` follows redirects with cookies/credentials stripped per spec | `URLSession` delegate: HTTPS + host + port must match, everything else blocked | same-origin HTTPS, plus loopback `http` when the request itself is loopback; `resolve` additionally refuses any Location containing a backslash or an at-sign, or whose path has a `..` segment or an interior `//` | Studio's rule is strictly HTTPS, which no local test server can exercise without TLS. The loopback exemption is the same one the initial-request rule uses, and a cross-host or downgrade redirect is still always blocked. The extra character rules are fail-closed: a WHATWG parser folds `\\` to `/`, so `https:\\evil.example` and `\/\/evil.example` are another host to a browser, and `..`/`//` in a path is what a server or proxy may re-read as an authority. A trailing-dot or percent-encoded host is already a different name to the origin comparison. |
| Timeouts | 10 s `AbortSignal.timeout` per request | 30 s per request, 300 s per resource | Studio's budgets: 30 s per request (ureq `timeout_per_call`) and 300 s for the whole fetch, redirect chain included, enforced by the client's own deadline because ureq's global timeout is per call. |
| Transport error text | message includes the failing status | keeps the underlying error but never prints it | a static failure class only | A bearer token is attached to every request; an error string that can echo request detail is a leak vector. The class (`I/O error`, `protocol error`, …) keeps failures diagnosable. |
| Cache summary rows | — (stateless) | rebuilt from the summary columns | decoded from `detail_json`, columns written in full | Studio's column list silently drops `privacy`, `hr_min`/`hr_max`, `timezone`, `date_utc`, `weight_class`, `rest_time`, `rest_distance`, `targets`, `metadata` and heart-rate detail from every summary, which would break e.g. the fail-closed share check. A test asserts the columns and the JSON agree. |
| Cache `date` column | — | SQLite `REAL` (a `Date` epoch) | SQLite `TEXT` (the logbook string) | `Workout.date` is the logbook string in every other layer; parsing it into an instant would invent a time zone. Ordering is unaffected (`YYYY-MM-DD HH:MM:SS` sorts chronologically). |
| Preferences storage | the browser's `safeStorage` | `UserDefaults` keys read one at a time | one JSON file in the config directory | A file is the portable equivalent; unknown fields are ignored and a corrupt file falls back to defaults wholesale (one bad key discards the others, unlike Studio's per-key reads). |
| Token store backends | server-side session | macOS Keychain only | macOS Keychain, Windows Credential Manager, Linux Secret Service | The port is cross-platform; keyring's backends are named explicitly and a test fails if the crate falls back to its in-memory mock. |
| Log levels | debug/info/warn/error | info/warn/error | info/warn/error (`info` added for the preferences fallback) | The core logger had only `warn`/`error`; a first-launch fallback is not a warning. |
| `Sport::from_concept2_type` | exact, case-sensitive | lower-cases first (`"SKI"` → SkiErg) | web | Studio's leniency is undocumented; the API sends lower-case. |
| `fmt_distance` negative values | `metres >= 1000` → km, else metres (`-1500` → `-1500 m`) | absolute threshold (`-1.50 km`) | web for metric; Studio's absolute threshold for imperial (no web equivalent) | Metric follows the canonical formatter. |
| Non-finite formatting | prints `NaN m` / `NaN:NaN` | placeholders `--` / `--:--` | placeholders (Studio) | The web output is an unhandled case, not a feature; Studio's stress spec documents the placeholders. |
| `fmt_time` with huge hours | prints exponent notation | `--:--` | `--:--` beyond 1e15 hours | Same reasoning. |
| `distance_band` labels | en dash (`3k–7k`) | hyphen (`3k-7k`) | web | Character-for-character parity; the fixture policy requires en dashes. |
| `distance_band` lowest band nominal | `(0 + min(750, 0)) / 2 = 0` | 375 | web (0) | Web quirk kept; flagged as a candidate upstream fix (the web fixed the same bug in `durationBand` only). |
| `summarise_by_sport.best_pace` with no positive pace | `Infinity` sentinel | `0` | `None` | Representation only; `Option` is the idiomatic sentinel. Ties in the distance sort keep first-seen order (web, stable sort) rather than sorting by sport name (Studio). |
| Personal bests | per sport, seven distances (`analytics.ts`); list variant across sports unless filtered (`workoutQuery.ts`) | one merged function with a sport filter, fastest across sports when unfiltered, plus 42 195 m | both web variants, seven distances | Studio's marathon distance and cross-sport merge are undocumented deviations. |
| `build_prediction_table` with non-positive inputs | rows with `undefined` predictions | empty table | empty table (Studio) | The web result is unusable; Studio's behaviour is the documented one. |
| List free-text search (`q`) | matches `comments` only | matched type, sport label, comments and source | web | Studio widened search for its sidebar; Phase 4 can revisit in the QML layer. |
| `list_query_is_filtered` | excludes `sport` (a tab) | includes sport | web | |
| Power sort sentinel | `-1` in both directions | `∞` ascending / `-1` descending | web | |
| Sort ties | stable (input order) | pace/power ties fall back to date | web | `Vec::sort_by` is stable. |
| `Split.index` | 0-based | 1-based | web | |
| Demo library | cadence-cycle strokes, 4 or `distance/1000` splits, full-fidelity extras on 1005/1007/1014, `9001` at `2024-01-14 23:30:00` in `America/New_York`, no splits without strokes | distance-step strokes, always splits, 1-based splits, `9001` converted to UTC | web | The web generator is the canonical demo data. |
| Time-zone names | `Intl` (case-insensitive, aliases) | `TimeZone(identifier:)` | `chrono-tz` (exact IANA spelling, aliases included) | Minor; workout time zones come from the API in canonical form. |
| `parse_instant_millis` offsets | regex allows `±HHMM`; `Date.parse` support varies | — | accepts `±HH:MM` and `±HHMM` | Strict superset within the web regex. |
| Redaction patterns | 8 web patterns (`rp_tok`, `session`, `SESSION_SECRET`, `workoutDetail` dumps, …) | 7 Studio patterns (cookie headers, JSON and form credentials, large arrays) + 16 KiB bound | union of both + bound | Privacy invariant: never weaker than either reference. |
| Input bounds | none | pace ≤ 64/20, logbook ≤ 64/30, day key ≤ 64/20 | Studio's bounds (+ 40 for instants) | Privacy invariant. |
| Locale date formatting | `Intl.DateTimeFormat` | `Date.FormatStyle` in SwiftUI | `rowplay_viewmodel::dates` (Phase 4): six-language pattern/month tables, golden-tested against the web's `Intl` output | Supersedes the Phase 1 "delegate to `QLocale`" plan: Phase 4 forbids formatting in QML, and a Qt-free table keeps the output testable and deterministic. ISO instants resolve in the *home* time zone preference (UTC when unset) instead of the browser/system zone. |
| `generateMockWorkout` | `Math.random`-driven live-mode mock | — | deferred to Phase 8 with an injected RNG | Needs the live-mode model first. |
| Workout tags | `resolveTag` used by the list filter | not ported (badge retired) | ported | Required for `filter_and_sort_workouts` parity. |
| `warp_stroke_phase` continuity | piecewise **linear**, C0 at the drive/recovery seam (velocity jump ≈ `(1−f)/f`, ~2× on SkiErg) | same piecewise-linear map | **C1 and periodic** via a cubic Hermite per half (`0.5·h(u/f, 2f)` / `0.5 + 0.5·h((u−f)/(1−f), 2(1−f))`, `h(t, k) = t²(3−2t) + k·t(1−t)(1−2t)`) | Roadmap-mandated divergence: the C0 seam caused visible speed jumps on SkiErg. The contract is unchanged (cycle boundaries fixed, drive end → half cycle, monotonic, drive faster on average) and the Hermite endpoint slopes add `dw/du = 1` at the catch, the finish and both sides of the seam (C1 at the seam, periodic across the cycle boundary), monotonicity for every `f ∈ [0.01, 0.99]` (`k = 2·max(f, 1−f) ≤ 1.98 < 3`, the monotone-cubic ceiling) and the exact identity at `f = 0.5` (`h(t, 1) = t`), like the web. `warp_stroke_phase_rate` exposes the analytic derivative and derivative-continuity tests guard the knots. |
| Playback clock | `ReplayEngine` on `requestAnimationFrame`, emits through a callback | tick-driven `ReplayState` | tick-driven `ReplayState` (Studio) | The QML `FrameAnimation` owns the frame clock; the core stays pure and callback-free (`current_frame()` is polled). |
| `sample_at` with non-finite `t` | smears `NaN` through the interpolation | zero frame | zero frame (Studio) | The web behaviour is an unhandled case; a non-finite clock must not poison render state. |
| Frame watts / HR interpolation | `f64` lerp | rounded to `Int` | `f64` lerp (web) | No web equivalent of the rounding; keep the canonical float math. |
| Stroke pose amplitude / progress | current web: restrained `clamp(0.94 + i·0.12, 0.94, 1.06)`, progress at the bracketing entry's end (`entry.endT/duration`) | livelier `clamp(0.78 + i·0.44 + f·0.08, 0.72, 1.32)`, progress = query time / duration | **both**: web pipeline canonical for renderers; Studio's `compute_at_time` ported alongside | `stroke-pose-parity.json` was exported before the web's 2026-07 rework (#171) and was verified against Studio's frame-based path, so the fixture drives the Studio-semantics entry point. Intensity, fatigue and drive-fraction formulas are identical in both references. |
| Ghost pick hardening | plain ranking, stable ties | adds `hasStrokeData` filter, non-finite sanitizers, id tie-break | web semantics | Studio's additions are undocumented deviations; the app layer can filter stroke-data availability where it builds the candidate list. |
| Race finish on the distance axis | last-sample endpoint timestamps | first **interpolated** crossing of the target | interpolated crossing (Studio) | Studio's source map documents this deliberate deviation (also cited in AGENTS.md); sparse traces otherwise decide wrongly. |
| Rival parsers | browser `DOMParser`/`DataView`, unbounded, no normalisation | 25 MiB / 200 k sample bounds, quoted CSV, DOCTYPE rejection, validated FIT, origin-rebased normalisation | Studio's hardened shape (bounds, quoted CSV streaming, TCX element-stack scanner with DOCTYPE rejection, FIT header/architecture/compressed-timestamp validation, sort + one-per-timestamp + no-backward-distance + zero-based time) | Accepted inputs parse identically; the hardening is a privacy/security invariant (bounded scanning, no entity expansion). The Rust TCX probe reads UTF-8/Latin-1 (not Studio's UTF-16 probe) — a documented simplification, and derived watts stay `f64` (web) instead of Studio's integer rounding. |
| Constant-pace ghost watts | `paceToWatts` (RowErg basis, divisor 2.8) | `paceToWattsForSport` (BikeErg divisor 8) | per-sport via `constant_pace_strokes`; `constant_pace_ghost` keeps the web rower-basis name | The golden fixture expects the bike divisor; the web helper has no sport parameter. |
| Quality budgets | `renderer3d.ts` `QUALITY` mixes portable counts with browser fields (`dprCap`, `antialias`, `shadowMapSize`, `bodySegments`) | portable entity budgets only (48/24/0/0/0/12/30 … 144/96/44/72/6/28/60) | Studio's portable budgets + sticky degradation ladder | Browser/three.js fields have no meaning until Phase 5 maps them onto Qt Quick 3D; renderer/quality *persistence* (web `safeStorage`) arrived in Phase 3 with the preferences store. |
| Gate member check | — | — | `ROWPLAY_GATE_MEMBER_CHECK` probes every `Singleton.member` reference scanned from `qml/`; an unresolved one is a test failure | A missing `qproperty!` registration reads as `undefined` with no QML error or binding warning, so no other check can see it (qt-bridges-notes #15). |
| Gate data directory | — | — | `ROWPLAY_DATA_DIR` redirects the cache/preferences for automated runs | The gate walk syncs into and then clears the cache; without this it would wipe the developer's real logbook data. Unset in normal use. |
| Settings strings | web locale ids (`settings.*`, `token.*`, `auth.*`, `sync.*`, `lang.switch`, `common.demoMode`) | hardcoded English labels | web ids via `Tr.t`; Studio labels map to the nearest web key ("Save Token" → `token.connect`, "Sync Now" → `dashboard.sync`, "Disconnect" → `auth.logout`, "Demo mode" → `common.demoMode`, reload → `pwa.reload`, empty state → `landing.title1`/`landing.lead`/`landing.exploreDemo`/`landing.connect`) | The i18n parity check admits exactly the web `en` key set, so the desktop UI must not invent ids. Each substitution is a wording compromise, recorded here. |
| "Connected" indicator | the header's Log-out button *is* the connected state | "Connected / Not connected" text rows | the web pattern: the Log-out button appears once `hasToken` | No web key for "Connected"; the web pattern also avoids a stale status string. |
| Unit picker labels | metric only (no picker) | "Metric" / "Imperial" segmented control | the unit symbols `km` / `mi` | Symbols are untranslated by design, like the web's chart axis labels; no web key exists for the words. |
| Timezone picker | grouped `<select>` (Americas / Europe-Africa / Asia-Pacific) | system date-picker sheet | flat `ComboBox`; labels carry the UTC offset, group ids and start indices are still exported by the backend | A grouped QML popup is 4b polish; the curated option list itself is a verbatim port. |
| Reduce-motion toggle | OS `prefers-reduced-motion`, no UI | Settings toggle | deferred to Phase 5 with the replay UI; the preference already persists | No web key, and the flag only affects replay rendering. |
| Window title / minimum size | browser tab title | `WindowGroup("RowPlay Studio")`, 1000×680 minimum | `rowplay`, 1000×680 minimum | The desktop app's own name; Studio's product title does not carry over. |
| Hero typography | system-ui + SF Pro Rounded | SF Pro Rounded hero metrics | system font, DESIGN.md sizes/weights kept, `Font.TabularNumbers` for `.monospacedDigit()` | SF Pro is Apple-only; the One Hero Rule survives as the 28px bold hero style. |
| Sport filter widget | tab bar | segmented `Picker`, 280 wide | `ComboBox`, 280 wide, `Tr.t("dashboard.all")` + sport display names | Qt Quick Controls has no segmented control; the ComboBox keeps keyboard access. Names stay untranslated (Concept2 trademarks). |
| Demo selection at startup | n/a (server data) | `@SceneStorage` starts at the default demo workout | the demo library selects `DEFAULT_WORKOUT_ID` on load and the shell routes to the detail screen | Studio behaviour; the gate screenshots depend on it. |
| `fmtLogbookDateTime` patterns | `Intl` numeric y/m/d + clock per locale | SwiftUI abbreviated date + shortened time | per-language `Intl`-golden patterns (en 12-hour with AM/PM, fr zero-padded d/m, de `d.M.yyyy`, …) | Golden outputs captured from the pinned web app under Node; see `rowplay_viewmodel::dates` tests. |
| fr `dashboard.emptyTrend` | drops the `{n}` placeholder ("Une seule séance…") | — | kept verbatim; the parity check only forbids *extra* placeholders in translations | Web parity: `interpolate` tolerates missing placeholders. |
| Sidebar date format | `fmtDate` → "Mar 5, 2026" | `.dateTime.year(.twoDigits).month(.abbreviated).day()` → "5 Mar 26" | web (`fmt_date`) | Web wins; the two-digit-year Studio format has no web counterpart. |
| Sidebar day sections | flat list | flat list with a count header | day-sectioned list (`workout_local_day_key`, home-tz aware) | Phase 4 brief asks for grouping by date; section headers use the locale short date. |
| Sport badge | inline SVG icons (`SportIcon`) | SF Symbols (`figure.rower`, …) | the sport initial on a tonal chip | No SF Symbols or web SVGs on Qt without new assets; the trademark name stays in the accessible text. |
| Dashboard "Challenge" tile | none (challenge distance only feeds goals/badges) | tile with `challengeDistanceMetres` total | dropped | No web tile and no locale key; `DashboardSummary.challenge_distance` stays available for Phase 5+. |
| PB card labels | `distanceBand().label` ("2k", "Half", "Full" — hardcoded English in `analytics.ts`) | `pbLabel` ("2k", "Half", "Marathon") | web band labels | Untranslated in the web too; Studio's "Marathon" loses to the web's "Full". |
| Recent-pace x axis | date scale | date scale | chronological index with locale date tick labels (pre-rendered in Rust) | Qt Graphs `ValueAxis` has no date-scale with custom string ticks; the index axis keeps every label a core-formatted string. |
| Split HR column | `heartRate.average ?? hr` | `heartRate?.average` | `heartRate.average` with a scalar-`hr` fallback | The web mapper keeps both fields; the fallback avoids "-" rows the web would render. |
| HR stroke chart with dropouts | belt gaps split the line | not charted | first gap-free segment charted; later segments omitted | Documented 4b limitation; the full segment list is already exported (`hrSegmentsJson`) for Phase 5. |
| Stroke-chart count | pace/power/rate/HR gauges + charts | pace + power charts | pace, power, rate and HR charts | The Phase 4 brief asks for all four over time. |
| Progress text | web renders counts in components | Studio status strings | rendered in the sync worker (`fmt_time` remaining estimate) | Keeps "no metric formatting in QML" absolute. |
| Sync default mode | incremental, with an explicit full re-sync control | incremental (`syncNow`), full history on demand | incremental (`Sync.start`), full (`Sync.startFull`), both buttons matching `settings.syncIncremental`/`settings.syncFull` | The web's two buttons are the model; the Concept2 list payload has no `updated_at`, so the identity stamp is `date` + optional `date_utc` (a changed stamp refetches that workout). |
| Incremental early stop | unknown (server-side session) | pages to the reported end | stops at the first page the cache holds in full and unchanged | Newest-first results mean a fully cached page implies the rest is cached too; `full` mode always pages to the end. Divergence documented because neither reference exposes the policy. |
| Skip counting | — | — | `WorkoutSyncResult.skipped_count` + `SyncProgress.saved`/`skipped` + `Sync.statusSkipped` | Needed to make "nothing to do" observable (and gate-testable) rather than indistinguishable from a failed fetch. |
| Cache schema | — (stateless) | Studio's column set, no `date_utc` | `SCHEMA_VERSION = 2`: `date_utc` added by `ALTER TABLE` (v1 databases upgrade in place, keeping their rows) | The extra column turns the incremental change check into a flat column read instead of decoding every `detail_json`. Additive, so Studio's columns still round-trip. |
| Sync result strings | `sync.incrementalDone` ("Caught up — {total} workouts cached") for a caught-up run | status message for everything | `sync.incrementalDone` when an incremental run saved nothing, else `sync.done` | Uses the web's existing keys, so no new English enters the catalogue. |
