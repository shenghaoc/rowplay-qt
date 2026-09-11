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

## Later phases (mapping only)

| Web source | Swift | Rust target | Phase |
| --- | --- | --- | --- |
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
| `warp_stroke_phase` continuity | piecewise **linear**, C0 at the drive/recovery seam (velocity jump ≈ `(1−f)/f`, ~2× on SkiErg) | same piecewise-linear map | **C2** via per-segment quintic smootherstep (`0.5·S(u/f)` / `0.5 + 0.5·S((u−f)/(1−f))`) | Roadmap-mandated divergence: the C0 seam caused visible speed jumps on SkiErg. The contract is unchanged (cycle boundaries fixed, drive end → half cycle, monotonic, drive faster on average); `warp_stroke_phase_rate` exposes the analytic derivative and a derivative-continuity test guards the seam and the cycle boundary. |
| Playback clock | `ReplayEngine` on `requestAnimationFrame`, emits through a callback | tick-driven `ReplayState` | tick-driven `ReplayState` (Studio) | The QML `FrameAnimation` owns the frame clock; the core stays pure and callback-free (`current_frame()` is polled). |
| `sample_at` with non-finite `t` | smears `NaN` through the interpolation | zero frame | zero frame (Studio) | The web behaviour is an unhandled case; a non-finite clock must not poison render state. |
| Frame watts / HR interpolation | `f64` lerp | rounded to `Int` | `f64` lerp (web) | No web equivalent of the rounding; keep the canonical float math. |
| Stroke pose amplitude / progress | current web: restrained `clamp(0.94 + i·0.12, 0.94, 1.06)`, progress at the bracketing entry's end (`entry.endT/duration`) | livelier `clamp(0.78 + i·0.44 + f·0.08, 0.72, 1.32)`, progress = query time / duration | **both**: web pipeline canonical for renderers; Studio's `compute_at_time` ported alongside | `stroke-pose-parity.json` was exported before the web's 2026-07 rework (#171) and was verified against Studio's frame-based path, so the fixture drives the Studio-semantics entry point. Intensity, fatigue and drive-fraction formulas are identical in both references. |
| Ghost pick hardening | plain ranking, stable ties | adds `hasStrokeData` filter, non-finite sanitizers, id tie-break | web semantics | Studio's additions are undocumented deviations; the app layer can filter stroke-data availability where it builds the candidate list. |
| Race finish on the distance axis | last-sample endpoint timestamps | first **interpolated** crossing of the target | interpolated crossing (Studio) | Studio's source map documents this deliberate deviation (also cited in AGENTS.md); sparse traces otherwise decide wrongly. |
| Rival parsers | browser `DOMParser`/`DataView`, unbounded, no normalisation | 25 MiB / 200 k sample bounds, quoted CSV, DOCTYPE rejection, validated FIT, origin-rebased normalisation | Studio's hardened shape (bounds, quoted CSV streaming, TCX element-stack scanner with DOCTYPE rejection, FIT header/architecture/compressed-timestamp validation, sort + one-per-timestamp + no-backward-distance + zero-based time) | Accepted inputs parse identically; the hardening is a privacy/security invariant (bounded scanning, no entity expansion). The Rust TCX probe reads UTF-8/Latin-1 (not Studio's UTF-16 probe) — a documented simplification, and derived watts stay `f64` (web) instead of Studio's integer rounding. |
| Constant-pace ghost watts | `paceToWatts` (RowErg basis, divisor 2.8) | `paceToWattsForSport` (BikeErg divisor 8) | per-sport via `constant_pace_strokes`; `constant_pace_ghost` keeps the web rower-basis name | The golden fixture expects the bike divisor; the web helper has no sport parameter. |
| Quality budgets | `renderer3d.ts` `QUALITY` mixes portable counts with browser fields (`dprCap`, `antialias`, `shadowMapSize`, `bodySegments`) | portable entity budgets only (48/24/0/0/0/12/30 … 144/96/44/72/6/28/60) | Studio's portable budgets + sticky degradation ladder | Browser/three.js fields have no meaning until Phase 5 maps them onto Qt Quick 3D; renderer/quality *persistence* (web `safeStorage`) lands in Phase 3 with the preferences store. |
