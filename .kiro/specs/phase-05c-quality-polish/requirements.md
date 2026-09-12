# Phase 5c — Quality tiers and polish: requirements

Wires the four `RenderQuality` tiers to real Qt Quick 3D scene settings, lets
the Phase 2 `PerfGovernor` degrade automatically, renders ghosts and rivals
with the race overlays, ships the reduce-motion polish, and records honest
frame-time measurements.

## R1 — Quality tiers

- R1.1 `replay::quality::RenderQuality` (Low/Medium/High/Ultra, default
  Medium) drives: shadow enable + shadow map size (Low/Medium off, High 1024,
  Ultra 2044→2048 as the web config states 2048), antialiasing
  (`View3D.renderSettings` / MSAA quality: Low None, others Medium/High),
  environment texture loading (Low/Medium bind **no** textures at all — the
  environments README's per-tier payload; High binds diffuse+roughness of the
  sport's sets, Ultra adds the OpenGL normal and the SkiErg-only timber
  terrace set), and instance counts from the tier's
  `QualityBudgets` (buoys per ring 12/18/22/28, lane markers, wake/spray
  capacities honoured as caps on the 5b particle/instance lists).
- R1.2 The per-sport High/Ultra texture sets follow the environments README
  table exactly (RowErg 8/8 sets, SkiErg 4/5, BikeErg 4/4); the vendored
  `environments/` JPGs are loaded through `Texture { source: "qrc:…" }` from
  the rcc (no runtime download), Low/Medium never instantiate them.
- R1.3 The tier is user-settable in Settings (`replay.quality*` locale ids)
  and persisted via `Preferences` (new field, tolerant of absent key).

## R2 — PerfGovernor

- R2.1 `replay::motion::PerfGovernor` runs off the frame times measured in
  `tick` (EMA sampling, median calibration capped at 2× floor, sustained-over
  window, post-step grace, sticky levels — all already in core). Degradation
  applies `RenderQuality::degraded(levels)` to the live scene.
- R2.2 A manual override in Settings ("Automatic" + the four tiers) disables
  the governor when a tier is pinned; the sticky ladder never upgrades on its
  own (web behaviour).
- R2.3 The governor's thresholds stay at the Phase 2 values. Frame times
  measured on this machine (Intel UHD 630, hardware GL) are recorded in the
  PR with the GPU string and the exact env; they may motivate a *documented*
  threshold change with evidence, never a silent retune. Software-GL numbers
  are labelled as such and never used for tuning. In particular the CI run
  (llvmpipe under Xvfb, `LIBGL_ALWAYS_SOFTWARE=1`) renders the full 5a
  scene — two-cascade shadows and MSAA High included — inside the strict
  pixel assertions at roughly 2 fps: it is a correctness check only and is
  never a source of timings, and 5c adds no adaptive path for it.

## R3 — Ghosts and race overlays

- R3.1 A ghost athlete + equipment set renders with the ghost material
  variant (cool tint, equipment transparency per 5a; athlete opaque per the
  V4 depth contract), driven by a second `ReplayState` over the rival trace
  (`replay::race_gap::{ghost_frame, ghost_distance}`).
- R3.2 Ghost pick uses `replay::ghost_pick` (past session / constant pace /
  uploaded file), surfaced in the replay route's compare control with the
  web's locale ids (`replay.compareAgainst`, `replay.pastSession`,
  `replay.constantPace`, `replay.uploadedFile`).
- R3.3 The race gap overlay (ahead/behind metres, `replay.ahead` /
  `replay.behind`) and the finish verdict (`replay::race_result`,
  `raceVerdict*` ids) render from Rust strings; the result panel appears at
  the finish like the web.
- R3.4 Rival file import (CSV/TCX/FIT) rides the Phase 2 parsers through a
  file dialog; bounds and normalisation unchanged.

## R4 — Polish

- R4.1 Reduce-motion (5b toggle) also disables camera lag and spray/wake
  animation, matching the web's `prefers-reduced-motion` behaviour plus the
  Settings override.
- R4.2 The quality ladder and governor state are visible in a small
  diagnostics strip (tier, governor level, median frame ms) behind a
  Settings developer toggle, so the measurements in R2.3 are reproducible by
  a reviewer.

## R5 — Measurements and docs

- R5.1 Frame times per sport per tier, median and p95 over ≥ 600 frames
  each, recorded in the PR and in `docs/roadmap.md`'s Phase 5c exit note.
  Every measurement comes from a hardware-GL Wayland run on this machine's
  Intel UHD 630 (`QT_QPA_PLATFORM=wayland`, no `LIBGL_ALWAYS_SOFTWARE`), with
  the GPU/renderer string and the exact environment recorded alongside the
  numbers; software-GL figures, if quoted at all, are labelled as such and
  never feed a threshold (R2.3).
- R5.2 Source map, roadmap, qtbridge notes updated; every divergence
  recorded; CI green on the three OSes + MSRV.
