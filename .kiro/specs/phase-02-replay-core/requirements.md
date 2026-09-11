# Phase 2 — Replay core: requirements

The web app (`shenghaoc/rowplay`, pinned `011e830`) is canonical; rowplay-studio
(pinned `3d406a5`) is the second reference and supplied the golden fixtures.
When they disagree the web wins unless Studio's source map documents a
deviation; every choice the Rust port makes is recorded in
`docs/source-map.md`.

Everything lands in `rowplay-core` as pure, Qt-free, allocation-predictable
code: no I/O (parsers consume byte slices / strings), inputs bounded before
scanning.

## R1: Replay engine

- **R1.1** `sample_at` / `sample_index_at` interpolate a `Stroke` timeline
  exactly like the web `engine.ts` (binary-search bracketing, clamped ends,
  progress by time, HR carried when only one end has it).
- **R1.2** A tick-driven playback state machine (`ReplayState`, Studio shape:
  origin-relative duration, clamped seek, speed presets, `tick(dt)` advancing
  at `dt × speed` and stopping at the end) replaces the web's rAF loop, since
  the QML `FrameAnimation` owns the clock.

## R2: Motion primitives

- **R2.1** `meters_per_cycle`, `clamp_dt`, `damp_factor`, `stroke_surge`,
  `catch_events` match the web `motion.ts` (Studio's non-finite guards kept).
- **R2.2** `warp_stroke_phase` keeps the web contract — cycle boundaries fixed,
  end of drive maps to half a cycle, monotonic, drive faster than recovery —
  but must be **C1-continuous at the drive/recovery seam and at the cycle
  boundary** (the web/Studio piecewise-linear map is only C0 and the velocity
  jump is visible on SkiErg). A derivative-continuity test proves it.
- **R2.3** `ParticlePool` (fixed capacity, swap-remove expiry, gravity
  integration, fade) and `PerfGovernor` (median calibration capped at 2× the
  floor, EMA sampling, sustained-over-budget window, post-step grace, sticky
  levels, tab-switch deltas ignored) match the web.

## R3: Stroke model

- **R3.1** `build_stroke_timeline` reproduces the web `strokeModel.ts`
  (non-advancing anchor skipping, rate/pace fallbacks, synthetic cycle spans,
  watts / DPS / HR aggregates with web medians).
- **R3.2** `stroke_pose_at`, `fallback_stroke_pose` and `catch_transitions`
  reproduce the web pose (drive-fraction inference, intensity/fatigue blend,
  restrained amplitude `0.94 + i·0.12`).
- **R3.3** Studio's frame-based `compute_at_time` path (progress = query time
  / duration, amplitude `0.78 + i·0.44 + f·0.08`) is also ported because the
  exported `stroke-pose-parity.json` was verified against it; the difference
  from the current web formula is recorded as a divergence.

## R4: Motion graph and 2D kinematics

- **R4.1** `sample_motion_graph` (+ per-sport samplers) reproduces the web
  `motionGraph.ts` channels, timing and curve algebra (quintic ramps, cruise
  ramps, pulses, bumps, sine/cosine, products) bit-comparably: the golden
  corpus `replay-current-main-motion.json` (129 phases × 3 sports, every
  channel) matches within 1e-10.
- **R4.2** The compatibility projections `solve_rower_kinematics`,
  `solve_skier_kinematics`, `solve_bike_kinematics` (web `sportKinematics.ts`,
  including the ski elbow direction) match `replay-current-main-2d.json`
  (64 phases × 3 sports, 1e-10).
- **R4.3** The 2D venue / athlete palettes (`renderer.ts` `COLORS_*`,
  `VENUES_*`) are ported as core constants and asserted against the same
  fixture's `palettes` block.

## R5: Ghost pick and comparability

- **R5.1** `classify_axis` / `are_comparable` match the web
  `comparabilityGuard.ts` (time-axis markers `JustRow` / `FixedTime`, distance
  or duration band equality).
- **R5.2** `pick_default_ghost_candidate` matches the web `ghostPick.ts`
  ranking (comparable pool, time-axis closeness, distance-band preference,
  closest metres, fastest pace, most recent date; stable ties).

## R6: Race gap and result

- **R6.1** `race_gap_metres`, `race_gap_seconds`, `finish_delta_sec`,
  `ghost_dist_at_player_finish`, `player_dist_at_ghost_finish` match the web
  `replayGap.ts`; Studio's `relative_duration` / `absolute_time` /
  `ghost_frame` helpers are included (non-zero timestamp origins).
- **R6.2** The finished-race calculator uses Studio's **interpolated target
  crossing** (documented deviation from the web's endpoint comparison), with
  tie tolerances 0.05 s (distance axis) / 0.5 m (time axis), DNF handling and
  sanitised margins.

## R7: Rival sources

- **R7.1** `constant_pace_strokes` builds the two-point pace trace with
  per-sport watts (BikeErg divisor 8); the web `constant_pace_ghost` name is
  kept for the rower variant.
- **R7.2** `parse_rival_file(data, file_name)` dispatches by content then
  extension between CSV / TCX / FIT, bounded to 25 MiB and 200 000 samples.
- **R7.3** Parsers follow Studio's hardened shape (quoted CSV with strict
  clocks and header tokens; namespace-insensitive TCX with DOCTYPE rejection
  and naive-UTC timestamps; structurally validated FIT with compressed
  timestamps) and normalise traces (sort, one sample per timestamp, no
  backward distance, zero-based time, derived pace / watts).

## R8: Quality budgets and governor wiring

- **R8.1** `RenderQuality` (low/medium/high/ultra, medium default) with the
  portable per-tier entity budgets (course ring, lane markers, wake, spray,
  droplets per catch, buoys, target frame rate) and the sticky degradation
  ladder (no auto-upgrade).
- **R8.2** The browser-only fields of the web `renderer3d.ts` `QUALITY` table
  (dprCap, antialias, shadow map size, …) are deferred to Phase 5 where Qt
  Quick 3D maps them; the divergence is recorded.

## R9: Fixtures and validation

- **R9.1** The six `#[ignore]`d parity tests are enabled and pass:
  `stroke-pose-parity.json`, `replay-race-gap-parity.json`,
  `replay-race-result-parity.json`, `replay-rival-sources-parity.json`,
  `replay-current-main-motion.json`, `replay-current-main-2d.json` — each
  states its tolerance.
- **R9.2** Unit tests re-express the web `engine`, `motion`, `ghostPick`,
  `replayGap`, `strokeModel`, `sources`, `sportKinematics` suites and Studio's
  governor / quality / race-result suites.
- **R9.3** `cargo fmt --all -- --check`, `cargo clippy --all-targets --
  -D warnings`, `cargo test --workspace` (Qt-free members) and
  `git diff --check` pass; no UI changes.
