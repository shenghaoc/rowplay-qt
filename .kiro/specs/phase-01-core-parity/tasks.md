# Phase 1 — Core parity foundation: tasks

- [x] Port `models` (web `types.ts`) with Studio aliases and JSON round-trip tests.
- [x] Port `formatting` with JS `toFixed` / `Math.round` semantics (`num`).
- [x] Port `datetime` (UTC civil arithmetic, strict parsers, `chrono-tz` projection).
- [x] Port `pace_input` with Studio input bounds.
- [x] Port `privacy` (share guard, union redaction patterns, sink-based logger).
- [x] Port `analytics` subset and Studio dashboard derivations.
- [x] Port `personal_bests` (per sport, seven distances) and `detect_new_pbs`.
- [x] Port `performance_predictor`.
- [x] Port `workout_query` (parse / serialise / filter / sort / chips) and `workout_tag`.
- [x] Port `demo` (web `mockData.ts` generator) byte for byte.
- [x] Vendor Studio fixtures with manifest + provenance; add `rowplay-fixtures` loader crate and `tools/vendor-fixtures.py`.
- [x] Golden tests: predictor (0.5%), duration bands (exact labels); `#[ignore]` placeholders for Concept2 mapper (Phase 3), replay (Phase 2), grips/equipment (Phase 5/7).
- [x] Platform crate: `TokenStore`, `WorkoutCache`, `Concept2Client`, `PreferencesStore` traits with mocks; `load_library`; stderr log sink.
- [x] Record every divergence in `docs/source-map.md`.
- [x] `cargo test --workspace` green; clippy pedantic clean.
