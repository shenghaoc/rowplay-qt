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
| `src/lib/server/session.ts` (token handling) | `Sync/TokenStore.swift`, `Platform/KeychainTokenStore.swift` | `rowplay-platform::token_store` | Trait + `SecretToken` + in-memory mock; `keyring` in Phase 3. |
| `src/lib/server/concept2.ts` (client) | `Sync/Concept2Client.swift`, `Concept2/*.swift` | `rowplay-platform::concept2` | Trait + mock; HTTPS client and raw-payload mapper in Phase 3 (Concept2 fixtures vendored now, test ignored). |
| — (web is stateless) | `Sync/WorkoutCache.swift`, `Storage/SQLiteWorkoutCache.swift` | `rowplay-platform::workout_cache` | Trait + in-memory and failing mocks; `rusqlite` in Phase 3. |
| — | `Platform/AppPreferences.swift` | `rowplay-platform::preferences` | `Preferences` has no token field by construction. |

## Later phases (mapping only)

| Web source | Swift | Rust target | Phase |
| --- | --- | --- | --- |
| `src/lib/replay/engine.ts`, `motion.ts`, `motionGraph.ts`, `ghostPick.ts`, `replayGap.ts`, `sources.ts`, `strokeModel.ts`, `replayRenderer.ts` (`QUALITY`, `PerfGovernor`) | `Replay/ReplaySample.swift`, `ReplayState.swift`, `ReplayMotion.swift`, `ReplayMotionGraph.swift`, `GhostPick.swift`, `ReplayRaceGap.swift`, `ReplayRaceResult.swift`, `ReplayRival*.swift`, `ReplayStrokePose.swift`, `ReplayRenderQuality.swift`, `ReplayPerformanceGovernor.swift` | `rowplay-core::replay::*` | 2 |
| `src/lib/server/concept2.ts` (mapping, transport) | `Concept2/Concept2Mapper.swift`, `HTTPTransport.swift`, `URLSessionConcept2Client.swift` | `rowplay-platform::concept2` | 3 |
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
| Locale date formatting | `Intl.DateTimeFormat` | `Date.FormatStyle` in SwiftUI | not in core; `QLocale` in QML (Phase 4) | Locale formatting belongs to the UI toolkit. |
| `generateMockWorkout` | `Math.random`-driven live-mode mock | — | deferred to Phase 8 with an injected RNG | Needs the live-mode model first. |
| Workout tags | `resolveTag` used by the list filter | not ported (badge retired) | ported | Required for `filter_and_sort_workouts` parity. |
