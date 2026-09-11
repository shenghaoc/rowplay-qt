# Phase 2 — Replay core: design

## Module map (`crates/rowplay-core/src/replay/`)

| Module | Web source | Studio source |
| --- | --- | --- |
| `engine` | `replay/engine.ts` (`sampleAt`, `sampleIndexAt`, `ReplayEngine`) | `Replay/ReplaySample.swift`, `ReplayState.swift` |
| `motion` | `replay/motion.ts` (`clampDt`, `dampFactor`, `warpStrokePhase`, `strokeSurge`, `catchEvents`, `ParticlePool`, `PerfGovernor`, `METERS_PER_CYCLE`) | `Replay/ReplayMotion.swift`, `ReplayPerformanceGovernor.swift` |
| `stroke_model` | `replay/strokeModel.ts` | `Replay/ReplayStrokePose.swift`, `ReplayStrokePoseAggregates.swift` |
| `motion_graph` | `replay/motionGraph.ts` | `Replay/ReplayMotionGraph.swift` |
| `sport_kinematics` | `replay/sportKinematics.ts` | `Replay/Replay2D` projections (via graph) |
| `theme` | `replay/renderer.ts` (`COLORS_*`, `VENUES_*`) | — (fixture `palettes` block) |
| `comparability` | `replay/comparabilityGuard.ts` | `Replay/ComparabilityGuard.swift` |
| `ghost_pick` | `replay/ghostPick.ts` | `Replay/GhostPick.swift` (web semantics) |
| `race_gap` | `replay/replayGap.ts` | `Replay/ReplayRaceGap.swift` |
| `race_result` | `replay/replayGap.ts` (finish behaviour) | `Replay/ReplayRaceResult.swift` |
| `rivals` | `replay/sources.ts` | `Replay/ReplayRival{Factory,FileParser,CSVParser,TCXParser,FITParser}.swift` |
| `quality` | `replay/replayRenderer.ts` (`RenderQuality`) | `Replay/ReplayRenderQuality.swift` |

Web names are kept in snake_case (`sample_at`, `warp_stroke_phase`,
`stroke_pose_at`, `pick_default_ghost_candidate`) so `docs/source-map.md`
stays greppable.

## Numeric fidelity

All measurement math is `f64` and evaluation order mirrors the web module
(`add` accumulates in argument order, `scale` before `add`, `pulse` is
rise − fall) so the golden corpora compare within 1e-10 rather than a loose
tolerance. `MotionChannel` / `CircularMotion` / `MotionTiming` are plain
`Copy` structs; per-frame allocation is a handful of small structs, which is
acceptable until the QML bridge needs the scratch-sampler variants (deferred
to Phase 5, like the web `*Into` samplers).

## C1 stroke warp

The web/Studio `warpStrokePhase` is piecewise linear (slopes `0.5/f` and
`0.5/(1−f)`), so the athlete's phase velocity jumps by ~2× at the
drive/recovery seam — visible on SkiErg. The Rust port keeps the contract
(`warp(0) = 0`, `warp(f·τ) = π mod τ`, monotonic, drive faster on average)
but composes each half from a quintic smootherstep
`S(x) = 6x⁵ − 15x⁴ + 10x³`:

```
w(u) = 0.5 · S(u / f)                  for u < f        (drive)
     = 0.5 + 0.5 · S((u − f)/(1 − f))  otherwise        (recovery)
```

`S` is C2-flat at both ends, so the warp is C2 at the seam *and* at the cycle
boundary (where `w'` is 0 on both sides), with zero velocity at the catch and
finish — the physical turnarounds. `warp_stroke_phase_rate` exposes the
analytic derivative; the derivative test compares left/right numerical
derivatives at the seam, the cycle boundary and a dense sweep. Recorded as a
deliberate divergence from both references.

## Two pose paths

`stroke_pose_at` is the canonical web pipeline (current amplitude
`0.94 + i·0.12`, progress at the bracketing entry's end). The exported
`stroke-pose-parity.json` predates the web's 2026-07 amplitude/progress
rework and was verified against Studio's frame-based `computeAtTime`
(progress = query time / duration, amplitude `0.78 + i·0.44 + f·0.08`), so
that entry point is ported alongside (`compute_at_time` +
`PoseContext`) to keep the fixture honest. Both share the intensity /
fatigue / drive-fraction formulas, which are identical in the two references.
Phase 4/5 renderers use the web pipeline.

## Rival parsers

`parse_rival_file(data: &[u8], file_name: &str) -> Result<ParsedTrace,
RivalParseError>` owns dispatch (plausible FIT signature → TCX probe →
extension hints → CSV) and normalisation (finite filter, stable sort by
(t asc, d desc, input order), one sample per timestamp, no backward distance,
zero-based origin, pace carried or derived from deltas, watts from
`pace_to_watts_for_sport`). Bounds: 25 MiB input, 200 000 samples. The TCX
decoder is a hand-rolled element-stack scanner (core forbids new
dependencies): namespace-insensitive local names, single root, DOCTYPE /
entity rejection, comment / PI / CDATA skipping, ISO-8601 with fractional
seconds and naive-as-UTC fallback through `datetime::parse_instant_millis`.
The FIT decoder follows Studio's hardened variant (header/architecture
validation, definition bounds, compressed-timestamp rollover, invalid
sentinels). The web's loose `DOMParser`/`DataView` parsers stay canonical
for *accepted* inputs; Studio's rejection behaviour is the documented
hardening.

## Race result

Ported from Studio with the documented deviation: distance-axis finish times
are the first **interpolated crossing** of the target distance (linear
between bracketing samples), not the last sample's timestamp. Time-axis races
compare distances at the target duration; ties within 0.05 s / 0.5 m; player
DNF yields no result; rival DNF is reported with the shortfall.

## Quality

`RenderQuality` carries the portable entity budgets and the sticky
degradation ladder from Studio (the web `renderer3d.ts` `QUALITY` fields that
are browser- or three.js-specific — dprCap, antialias, shadowMapSize,
bodySegments — are Phase 5 material for Qt Quick 3D). Preference persistence
(`safeStorage` on the web) is Phase 3 with the platform preferences store.

## Tests

Unit tests sit beside each module and re-express the web
`*.test.ts` / Studio `*Tests.swift` cases (governor ladder, quality budgets,
catch events, particle pool, timeline normalisation, comparability, ghost
ranking, gap math, crossing interpolation, parser hardening). Golden-fixture
tests live in `crates/rowplay-core/tests/parity.rs` and replace the
`#[ignore]` stubs; tolerances: 1e-10 (motion / 2D corpora), 1e-6 + ranges
(pose), 0.05 s / 0.5 m (race result), 1e-3 (gap, rivals, constant pace).

## Non-goals

No QML, no Qt, no persistence, no networking, no V4 rig / hand-grip solver
(Phase 7), no environment palettes beyond the 2D constants, no live mode.
