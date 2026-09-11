# Phase 1 — Core parity foundation: design

## Module map (`crates/rowplay-core/src`)

| Module | Web source | Studio source |
| --- | --- | --- |
| `models` | `types.ts` | `Models/*.swift` |
| `num` | JS semantics (`Math.round`, `toFixed`) | — |
| `formatting` | `format.ts` | `Support/RowPlayFormatting.swift` |
| `datetime` | `datetime.ts` | `Support/RowPlayDateTime.swift` |
| `pace_input` | `paceInput.ts` | `Support/PaceInput.swift` |
| `privacy` | `privacy.ts`, `server/logger.ts` | `Support/PrivacyRedaction.swift`, `PrivacySafeLogger.swift` |
| `analytics` | `analytics.ts` (subset) | `Analytics/WorkoutAnalytics.swift` |
| `personal_bests` | `analytics.ts` (PBs) | `Analytics/PersonalBests.swift` |
| `performance_predictor` | `performancePredictor.ts` | `Analytics/PerformancePredictor.swift` |
| `workout_query` | `workoutQuery.ts` | `Library/WorkoutQuery.swift` |
| `workout_tag` | `workoutTag.ts` | — |
| `demo` | `mockData.ts` | `Fixtures/DemoWorkoutLibrary.swift` |

## Numeric fidelity

TypeScript numbers are `f64`, so measurement fields are `f64` and ids are
integers. `num::js_round` reproduces `Math.round` (ties toward +∞) and
`num::js_to_fixed` reproduces `Number.prototype.toFixed` (ties away from zero
on the exact binary expansion), which keeps formatted strings identical to the
web app. `Workout.date` stays the logbook string so sorting and day keys follow
the web's string semantics.

## Time zones

`chrono-tz` compiles the IANA database into the binary (no runtime zoneinfo
dependency); `workout_local_day_key` reproduces the web's iterative wall-time
to instant resolution rather than `chrono`'s `LocalResult`.

## Tests

Unit tests sit beside each module and re-express the web `*.test.ts` and
Studio `*Tests.swift` cases (with Studio-only expectations dropped where the
web wins). Golden fixtures are exercised from `crates/rowplay-core/tests/parity.rs`
through `rowplay-fixtures`; `crates/rowplay-fixtures/tests/manifest.rs`
verifies every fixture's SHA-256 against `tests/fixtures/manifest.json`.

## Non-goals

No replay engine, networking, keyring, SQLite, QML or locale formatting
(`fmtDate` and friends are delegated to `QLocale` in Phase 4).
