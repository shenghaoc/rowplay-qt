# Phase 5c — Quality tiers and polish: tasks

- [ ] T1 `replay::quality` tier-settings resolver in the view-model + unit
  tests against the environments README table and `budgets()` (R1.1, R1.2).
- [ ] T2 Scene wiring: MSAA, shadows, conditional texture sets, instance
  caps; Settings quality picker + persisted preference (R1.1–R1.3).
- [ ] T3 Governor integration: sampling in `tick`, sticky degradation,
  manual override, diagnostics strip (R2.1–R2.3, R4.2).
- [ ] T4 Ghost group + race gap/verdict overlays + compare control + rival
  file import (R3.1–R3.4).
- [ ] T5 Reduce-motion polish for camera and particles (R4.1).
- [ ] T6 Bench mode + per-sport per-tier measurements on hardware GL, PR
  table with GPU string and env, roadmap exit note (R5.1).
- [ ] T7 Docs: source map, roadmap, qtbridge notes; CI validation record
  (R5.2).

## Follow-ups

- [x] T8 The race gap is worded with the web's `replay.ahead` /
  `replay.behind`, as R3.3 asks (R3.3 and the "Ghosts" design paragraph said
  "render from Rust strings"; they now say what both overlays do, the verdict
  having moved to QML in the design system). It was English built in Rust
  ("20 m (0:04 ahead)") in every language, with a "—" under half a metre that
  the web never shows. Rust now passes `ahead|metres|seconds` or
  `behind|metres|seconds` (`hud::race_gap_bundle`, rounded as the web rounds,
  with `Math.round` and `toFixed(1)`), and QML words it. A level race counts
  as ahead, as on the web. Two differences from the web remain (the source
  map's "Race gap" divergence): the gap stays beside the verdict at the
  finish, and the web's coloured pill is not ported. The native gate (Apple
  M5, light) shows the change where a ghost is loaded and nowhere else:
  `replay-ghost` and the 24 phase shots and close-ups differ in the HUD's
  lower row only, "1 m (0:00 behind)" becoming "▼ behind by 1m (0.1s)".

- [x] T9 Compact localized gaps: at the compact width class the gap has
  its own full-width wrapping row below the metric chips. The full runtime
  gate visits a 480 px ghost replay in all six locales, asserts the visible
  gap and all four chips stay inside the HUD without overlap, and captures
  each locale. This covers the longer Spanish wording found in PR review.
