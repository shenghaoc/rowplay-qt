# Phase 1 — Core parity foundation: requirements

The web app (`shenghaoc/rowplay`) is canonical; rowplay-studio is the second
reference. When they disagree the web wins unless Studio's source map documents
a deliberate deviation, and every divergence is recorded in `docs/source-map.md`.

## R1: Domain models and Sport mapping

- **R1.1** `Sport` (`rower`, `skierg`, `bike`) with the web `toSport` mapping and
  Studio's display / short / cadence-unit labels.
- **R1.2** `Workout`, `Stroke`, `Split`, `WorkoutDetail` and the nested detail
  types carry every field of the web `types.ts`, serialise to the web JSON
  shape, and accept Studio's `cadence` / `heartRate` aliases.

## R2: Formatting

- **R2.1** `fmt_time`, `fmt_pace`, `fmt_pace_bare`, `fmt_distance` reproduce the
  web strings character for character (including `toFixed` rounding).
- **R2.2** `pace_to_watts`, `pace_to_watts_for_sport`, `watts_to_pace`,
  `watts_to_pace_for_sport`, `challenge_distance_metres`, `avg_watts` match the
  web, with the BikeErg divisor of 8.
- **R2.3** Imperial distance and race-margin formatting match Studio.

## R3: Datetime, pace input, privacy

- **R3.1** Logbook parsing, day keys, day arithmetic, instant parsing, time-zone
  resolution and ISO output match the web; parsers bound input length.
- **R3.2** `parse_pace_input` / `format_pace_input` match the web with Studio's bounds.
- **R3.3** `is_publicly_shareable` matches the web; `redact` covers the union of
  the web and Studio patterns with a 16 KiB bound; a sink-based
  `PrivacySafeLogger` redacts every message and argument.

## R4: Analytics, personal bests, predictor, query

- **R4.1** `linear_trend`, `distance_band`, `duration_band`, `summarise_by_sport`
  match the web; Studio's dashboard derivations are included.
- **R4.2** Personal bests at the seven standard distances with ±2% tolerance,
  per sport (web `analytics.ts`), plus the list variant across sports
  (web `workoutQuery.ts`).
- **R4.3** Paul's Law predictions and the prediction table match the golden
  fixture within 0.5%.
- **R4.4** Workout list query parse / serialise / filter / sort / chips and
  workout tags match the web tests.

## R5: Demo library

- **R5.1** `mock_workouts` / `mock_workout_detail` reproduce the web generator
  (seeded LCG, cadence-cycle strokes, splits, full-fidelity extras).

## R6: Fixtures

- **R6.1** Studio's fixtures are vendored under `tests/fixtures/` with a
  SHA-256 manifest and provenance note; a dev-only crate loads them.
- **R6.2** Golden tests cover the predictor and duration bands; Concept2 mapper
  and replay fixtures land with `#[ignore]` tests naming their phase.

## R7: Validation

- **R7.1** Every ported helper has tests re-expressed from the web and Studio suites.
- **R7.2** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, `git diff --check` pass.
