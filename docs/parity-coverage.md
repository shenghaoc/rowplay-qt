# Parity coverage map

Which ported surface has been checked against the web, by what evidence, and
what could still be wrong in it. Written by the pre-Phase-8 parity audit;
**re-verified against `origin/main` @ `8732cb5`** (post-Phase 7) after the
original pass was cut from a stale base. Every row reflects the code and the
pinned references at that commit (`reference/rowplay` @ `173c6fa`,
`reference/rowplay-studio` @ `3d406a5b`), verified directly rather than
trusted from `docs/source-map.md`.

**Process rule (it cost a full stage to learn): before any analysis,
`git fetch origin` and record the base SHA in the PR description.** The
original stage-1 pass was based on a week-old `origin/main` and so reported
Phase 7's rower fix (`cf85cdb`), its fixtures and its modules as absent — the
rower inversion was described as unfixed when it had already landed.

Two rules govern this document (see AGENTS.md, "Parity is the test oracle"):

1. **Fixture-first.** A surface is *covered* only when a web-generated fixture
   sweeps the input space the surface actually spans. A fixture that pins
   static geometry does not cover phase; a corpus that pins
   `warpedPhase = phase` does not cover the warp.
2. **A green parity suite means the covered surface agrees** — not that the
   port is correct. Everything below with a verdict of `partial` or `none`
   passes CI today.

This document outlives the audit: when you add a ported surface, add its row
and its fixture in the same PR.

## How to read a row

- **Rust / web** — the function-level pairing. The web column is what the code
  was actually compared against in this audit, which is not always what
  `docs/source-map.md` claimed (see the rig-pose rows).
- **Via** — `web` (ported from the web source), `Studio` (ported from
  rowplay-studio's Swift, op-for-op), or `web+Studio` (web behaviour with
  Studio hardening/naming recorded in the divergences table).
- **Evidence** — what exists today and, for fixtures, **which part of the
  surface it pins** (sample count, swept inputs, and whether the samples vary
  the thing that would catch an error).
- **Verdict** — `covered` / `partial` (evidence exists, named gap remains) /
  `none`.
- **Risk** (partial/none only) — the defect class that could hide, and whether
  an inversion or sign error is possible.

Unit tests that re-express the web's `*.test.ts` with exact expected values
count as evidence in the table but **not** as `covered` — they pin values the
port's author believed at port time; only a web-generated fixture pins values
the web believes now.

## What the existing fixtures actually sweep

All fixtures live in `tests/fixtures/`. Provenance splits in three:

- **Web-generated via Studio's exporter** (`replay-current-main-{motion,2d,
  equipment,grips}.json`): Studio's `script/export_rowplay_native_parity.mjs`
  evaluates the **web TypeScript** at pinned commit `4d96480` via `git show`
  (never the working copy), under Node ≥ 23.6, and records per-file SHA-256s.
  The web's `src/lib/replay/` is byte-identical between `4d96480` and the
  pinned reference HEAD `011e830` (verified: `git diff 4d96480..011e830 --
  src/lib/replay/` is empty), so these fixtures evaluate exactly the code the
  reference pins. Since Phase 7 the equipment and grips corpora are consumed
  (`equipment_contact_parity` at 1e-12, `grip_closure_parity`).
  - The exporter's pose scheme (`poseFor`) sweeps `phase` over the full cycle
    at fixed per-sport timing (rower 60/28 s, drive 0.38; skierg 1.875 s,
    0.34; bike 0.75 s, 0.5), with `intensity` varying by a stride-37
    permutation and **`warpedPhase = phase`** — so nothing the corpus pins can
    distinguish a consumer that keys on warped phase from one that keys on
    raw phase. `watts`, `amplitude`, `fatigue` stay fixed.
- **Web-generated in this repository** (the audit's stage-2 pattern, all
  three recording `sourceCommit` and per-file SHA-256s inside the JSON):
  - `replay-row-phase-parity.json` (Phase 7, `tools/gen-row-phase-parity.mjs`,
    web commit `4d96480`): 33 cycle samples of the **authored** rower
    calibration formulas — seat slide, oar sweep yaw, dip roll — evaluated
    from the web constants and motion graph.
  - `replay-rig-phase-parity.json` (audit stage 2,
    `tools/gen-rig-phase-parity.mjs`, web commit `011e830`): 384 samples of
    the **composed** avatar state — the three avatar factories built headless
    and animated over the full cycle at two timing inputs per sport, recording
    rig-local transforms (seat, oars, torso, poles, cranks, wheels, pedals)
    and the V4 contact landmarks after `animate` + `resolveWorldContacts`.
    Pins a strictly deeper layer than the row-phase fixture: it also sees the
    analytic oar-yaw reach solve, the torso pitch, and the contact rig.
  - `replay-stroke-model-parity.json` (audit ranking 5,
    `tools/gen-stroke-model-parity.mjs`, web commit `011e830`): 129 pose
    samples of the **production stroke-pose pipeline** — the web's real
    `strokeModel.ts` imported and called (`buildStrokeTimeline`,
    `strokePoseAt`, `fallbackStrokePose`), not re-derived in the generator.
    Nine timelines sweep the input space the surface spans: real and
    synthetic, three sports, interval rests, a non-advancing anchor,
    degenerate rows (spm 0 / spm past the rate ceiling / no distance / no hr)
    and an empty timeline, each queried at every row's start, midpoint and
    end plus outside the timeline. `driveFrac` spans its whole clamp range
    (0.28–0.5) and `intensity`/`fatigue`/`amplitude` each take dozens of
    distinct values across the sweep.
    These three fixtures serialise doubles at `toPrecision(17)`, which
    serde_json's default fast parser mis-rounds by 1 ULP for a fraction of
    such literals (the Studio-exported corpora hit the same class through
    V8's shortest round-trip form); exact parsing — serde_json's
    `float_roundtrip` feature, landed as its own PR ahead of this stack — is
    pinned by `crates/rowplay-fixtures/tests/float_roundtrip.rs` as a
    property over every fixture: each float literal reads back bit-identical
    to its nearest double (see ranking 5 — a 1 ULP shift is what made that
    ranking's first run fail).
- **Studio-semantics** (`stroke-pose-parity`, `replay-race-gap/result/rival-*`,
  `Concept2/*`, predictor/duration-band): hand-verified or Studio-computed
  expectations. `stroke-pose-parity.json` predates the web's 2026-07 stroke
  rework (#171) and drives Studio's frame-based path, not the web pipeline.

## rowplay-core — `crates/rowplay-core/src/replay/`

### Covered

| Surface | Rust | Web | Via | Evidence |
| --- | --- | --- | --- | --- |
| Motion-graph channel algebra | `motion_graph::sample_*_motion_graph` | `motionGraph.ts` `sample{Rower,Skier,Bike}` | web | `replay-current-main-motion.json`: 129 phases × 3 sports × every channel (value/velocity/acceleration) at 1e-10. Pins the full curve algebra over one cycle at the generator's fixed timing. |
| 2D kinematics projections | `sport_kinematics::solve_{rower,skier,bike}_kinematics` | `sportKinematics.ts` `solve*` | web | `replay-current-main-2d.json`: 64 phases × 3 sports, all output channels at 1e-10, over the same synthetic pose. |
| Skier elbow direction | `sport_kinematics::solve_skier_elbow_direction` | `sportKinematics.ts` `solveSkierElbowDirection` | web | 32 web-generated samples inside `replay-current-main-equipment.json`, consumed by `equipment_contact_parity` at 1e-12 since Phase 7 (was `#[ignore]`d at stage-1 time). |
| Canvas palettes | `theme::{COLORS_*, venues_*}` | `renderer.ts` `COLORS_*`, `VENUES_*` | web | Same fixture's `palettes` block: character-exact strings, all sports, light + dark. |
| **Sport equipment contracts** | `row_equipment`, `ski_equipment`, `bike_equipment`, `bike_saddle` | `rowRig.ts`, `skiEquipment.ts`, `bikeRig.js`, `bikeSaddle.js` | web | `equipment_contact_parity` over `replay-current-main-equipment.json` (253 web-generated samples at 1e-12): `solveRowerOarYaw`, elbow flexion/reach laws, `bikeKneeFlexion`, `bikeSaddleDropAt`, plus the contact-landmark and grip constants. Enabled in Phase 7. Read the green for what it is: the `oarYaw` block is a **synthetic domain sweep** of the pure function (its reaches, 0.55–0.95, run past the 0.758 maximum the draw schedule can produce — it predates the record-what-renders rule; the web's own code evaluated standalone, so a legitimate oracle, but a domain sweep, not rendered behaviour — and it leaves [0.256, 0.55), the drawn half of the stroke, untouched here; that half is covered one layer deeper by the rig-phase `oarSolve` recording, see ranking 10). |
| **Hand-grip closure** | `hand_grip` | `handGrip.ts` `collectHandDigitChains`, `solveHandGripClosure` | web | `grip_closure_parity` over `replay-current-main-grips.json`: 6 closure samples (3 sports × 2 sides) at 1e-12 plus the hand-channel constants, built from the web athlete contract's rest transforms. Phase 7 slice 1. |
| **Rower rig phase calibration (authored layer)** | `rig_pose::solve_rower` (seat/sweep/dip) | `renderer3dRowAvatar.ts` `animate` constants | web | `replay-row-phase-parity.json` (Phase 7): 33 cycle samples at 1e-10 — seat `0.26 + pelvisTravel·(−0.44)`, sweep `0.68 + handleTravel·(−1.48)`, dip `bladeWater·0.28 + handleTravel·0.04`, with an inversion guard. This is the fix for the inverted Studio port (`cf85cdb`); the audit's own composed-layer fixture independently confirms the seat now matches (see the partial row below for what it still catches). |
| **Bike rig phase calibration** | `rig_pose::solve_bike` (pedals, wheel roll, crank) | `renderer3dBikeAvatar.ts` `placePedalLegs` (`−(r·cos, r·sin)`), wheel roll (`meters / WHEEL_RADIUS 0.31`) | web | `rig_phase_parity_bike` (enabled, green) over the audit's composed-avatar fixture: crank, wheel and both pedal components at 1e-6 across 128 samples. The fix adopted the web's pedal negation and the 0.31 rim rotation radius (Studio's chain sat π around the crank circle and divided by the 0.335 axle height, under-rotating the wheels ≈7.5% since Phase 5b). |
| Race gap helpers | `race_gap::*` | `replayGap.ts` | web+Studio | `replay-race-gap-parity.json` at 1e-6 incl. Infinity/NaN cases; Studio's non-zero-origin helpers included. |
| Race result | `race_result::race_result` | `replayGap.ts` (finish) via Studio's interpolated crossing | Studio | `replay-race-result-parity.json`; the interpolated crossing is a documented deliberate deviation (source-map divergences). |
| Rival sources | `rivals::{constant_pace_strokes, parse_rival_file}` | `sources.ts` + Studio hardening | web+Studio | `replay-rival-sources-parity.json`: endpoints 1e-3, bike divisor exact, CSV/TCX/FIT normalisation. Hardening documented. |
| Stroke pose, Studio path | `stroke_model::compute_at_time` | Studio `ReplayStrokePose.computeAtTime` | Studio | `stroke-pose-parity.json`: 3 cases (one per sport), index/drive exact, scalars as hand-verified **ranges**. Kept only for this fixture; not the production path. |
| **Stroke pose, web pipeline (production path)** | `stroke_model::{build_stroke_timeline, stroke_pose_at, fallback_stroke_pose}` | `strokeModel.ts` `buildStrokeTimeline`, `strokePoseAt`, `fallbackStrokePose` | web | `stroke_model_web_pipeline_parity` over `replay-stroke-model-parity.json` (audit ranking 5; the web module imported at the pinned commit, outputs recorded verbatim): 9 timelines × boundary-inclusive query sweep + 8 fallback poses, every timeline entry field, aggregate and pose field at 1e-10 — except `warped_phase`, pinned as cycle + seam-side only (plus exact at `driveFrac = 0.5`, where both warp laws are the identity) because the port's C1 warp is a documented deliberate divergence. The app renderer uses this path (backend `replay.rs`, not `compute_at_time`). |

### Partial

| Surface | Rust | Web | Via | Evidence | Gap and risk |
| --- | --- | --- | --- | --- | --- |
| **Rig phase calibration, composed layer** | `rig_pose::{solve_rower, solve_skierg, solve_bike}` | `renderer3d{Row,Ski,Bike}Avatar.ts` composed `animate` state | web | `replay-rig-phase-parity.json` (this audit, 384 samples × 2 timing sweeps). Per-sport tests: **bike and skierg enabled and green** after their fixes; rower `#[ignore]`d with recorded verdict. Post-Phase-7 re-run: **rower `seat_z` passes** — independent confirmation that this fixture agrees with the shipped fix — and crank passes. | Rower outstanding: `oar_sweep` — the port now rides `armDraw` (corrected channel) but still omits the arm-authority reach solve the web applies on top (**an inverted chain, not a layering indifference** — ranking 1 has the full finding and the fix plan); Studio handle/torso channels are dead surface. Skierg outstanding: torso base, hand-path frame composition (0.7–0.9 m), course-anchored plant (ranking 2). | Skierg: torso base and head local counter-tilt fixed against the web (`0.055 + hipHinge·0.56`, `-hipHinge·0.38`), pelvis carry now exposed as `pelvis_y`/`pelvis_z` and pinned against `upper.position`. The pre-composition `preferred_hand_*` and `plant_basket_z` fields are no longer compared against the fixture's `targets.leftHand` / `poleTipLeft`: the web `placePoleArms` (called via `resolveWorldContacts`) needs the Rust runtime's V4 shoulder data through `refineV4Targets`, and Node has none — so `arm.hand` and `skierg-pole-tip-*` came back at the pelvis origin for every sample and the old deltas measured "port-target minus pelvis", not any deviation from the web's placement (the parity test now records the rationale in full). Extending the generator with `hipsRotation` (`-hipHinge · 0.14`) is deferred — a byte-identical regeneration environment is not to hand. At reference `173c6fa` the fixture also carries `handTargets` (`v4HandTargets`, rowplay#199), pinned by the viewmodel hand-target tests (rower exact after the Euler-XYZ order fix, skierg recovery window exact, bike exact). |
| **Rower oar channel** | `rig_pose::solve_rower` (`armDraw` in) | `renderer3dRowAvatar.ts` `placeOars(equipmentHandleTravel = graph.body.armDraw.value)` | web | `rower_rig_phase_parity` (corrected fixture, 33 samples at 1e-10, green). The audit found the port (and Phase 7's own generator) keyed the oar sweep and the roll's handle-rise on `handleTravel`; the web's comment warns that channel "would include its leg contribution and pull the grip through the knees and torso too early". The channel error is up to 1.05 rad at mid-drive, and its handle-rise half exactly explained the previously-unexplained +0.029 rad roll residual. |
| Motion-graph timing parameters | `motion_graph::timing_into` etc. | `motionGraph.ts` `timingInto` | web | Corpus sweeps phase at one fixed timing per sport; drive-fraction clamps and rate-dependent timing unit-pinned exactly (motion_graph.rs tests, re-expressing web tests). | No corpus varies `driveFrac`/`secondsPerCycle`/rate inputs. A regressed clamp would fail only the unit pins, which were written from the port. **Phase-source choice unpinned**: the graph reads `pose.phase` (motion_graph.rs, matching the web), but because the corpus pins `warpedPhase = phase`, a regression to `warped_phase` would pass every existing test. Sign error unlikely; input-selection error possible. |
| Warp stroke phase | `motion::warp_stroke_phase[_rate]` | `motion.ts` `warpStrokePhase` | web (deliberate divergence) | Boundary mapping (0.4·τ → π), identity at f = 0.5, monotonicity and C1/periodicity guard tests. The stroke-model corpus (`replay-stroke-model-parity.json`) now sweeps the divergence contract itself: every recorded pose's `warped_phase` must share the web's cycle and seam side, and match exactly at `driveFrac = 0.5`. | The raw web `warpedPhase` values cannot be pinned — the port is intentionally C1 where the web is C0, and a value-level comparison would fail by design; the divergence row is the contract. The band-structure sweep covers that contract at 129 points, but not the curve shape between the pins (no oracle exists by definition). |
| PerfGovernor | `motion::PerfGovernor` | `motion.ts` `PerfGovernor` | web (deliberate divergence) | Scenario tests pin the calibrated budget caps (44.0 = 2×22, 25.6 = 1.6×16) and the rollback ladder. | Documented divergence (66 ms outlier clamp, ≥10% payoff check, rollback+lock vs the web's monotone levels). Web defaults (22 ms / 60 / 90 / 30, 250 ms ignore) are the floor; nothing regenerates if the web retunes. |
| `meters_per_cycle` | `motion::meters_per_cycle` | `motion.ts` `METERS_PER_CYCLE` | web | Constants 11/8/5 exercised transitively by the motion corpus (`generator_pose`); unit test asserts only `> 0`. | No direct pin of the table; low risk (values confirmed against the web source in this audit). |
| Two-bone IK | `two_bone::*` | `figurePose.ts` `solveTwoBone3D` / `solveRigidContactPoint3D` | Studio naming, web geometry | Exact analytic unit tests; since Phase 7 also exercised transitively at 1e-12 through `row_equipment::solve_rower_arm` in the equipment corpus. | No direct web-generated sweep of the two-bone solver's own input space (folded/overextended/hint-flip); risk low — the geometry is exact and now consumed under fixture load. |
| Wrist orientation & budgets | `wrist` | `handGrip.ts` `orientHandToGripChannel`, `refineGrip{Spin,Tilt}ForWrist`; `renderer3dV4Motion.ts` `constrainWristFrame` + SkiErg keep/share rules | web | 12 unit tests re-express the web `handGrip.test.ts` orientation suite; the budget constants ride the equipment corpus (per the module doc). | No dedicated web-generated sweep of the budget pass (swing–twist redistribution); a mis-signed budget would deform wrists subtly. Phase 7 slice 2. |

### None

| Surface | Rust | Web (actual) | Via | Evidence | Risk |
| --- | --- | --- | --- | --- | --- |
| Engine sampling | `engine::{sample_at, sample_index_at, ReplayState}` | `engine.ts` | web | Exact unit re-expressions of the web's `engine.test.ts` (progress pins, interpolation, non-zero origins, speed presets). | Not fixture-backed, but every value is arithmetic on the web's own test data; inversion would fail immediately. Not worth closing with a fixture (recorded reason). |
| Comparability guard | `comparability::*` | `comparabilityGuard.ts` | web | Unit re-expression over workout-type → axis mapping and band rules. | Discrete classification; a fixture would restate the same table. Not worth closing (recorded reason). |
| Ghost pick | `ghost_pick::*` | `ghostPick.ts` | web | Unit re-expression of the ranking semantics with exact winner ids. | Same as above — total ordering pinned by units; Studio's hardening intentionally not ported (documented). Not worth closing (recorded reason). |
| Quality budgets | `quality::RenderQuality::budgets` | `renderer3d.ts` `QUALITY` (lines 65–138) | Studio | Budget table pinned exactly — **against Studio's documented tiers**. | Not an inversion risk, but the portable fields that are comparable **numerically disagree with the web**: wake 0/16/28/44 vs web 0/20/32/52; spray 0/40/48/72 vs 0/64/80/112; per-catch 0/4/4/6 vs 0/7/8/10; ring segments 48/72/96/144 vs lane segments 48/80/112/160 (only `buoysPerRing` 12/18/22/28 matches). Needs either a per-field divergence record or adoption of the web values. |
| **SkiErg `preferred_hand_*` — contact and next-plant-approach windows** — **CLOSED at reference `173c6fa`** | `pose::skierg_targets`'s solved contact (`rig_targets().contacts`, the port's `solve_rigid_contact3d` output) | `v4HandTargets` (rowplay#199: each avatar exposes the pre-IK `handTarget`; for skierg `placePoleArms`'s contact pass overwrites it with the solved hand, so the surface is the post-pole-solve target) | web | `the_skierg_solved_hand_matches_the_web_post_pole_solve_hand_target` (viewmodel) over the regenerated `replay-rig-phase-parity.json`'s new `handTargets` field: **recovery window machine epsilon (6.6e-16)** — the port's solve composes the same Bezier law — while **contact worst 0.977 m** and **approach worst 0.033 m** are measured, bounded by regression guards, and attributed to the documented `plant_basket_z` model divergence (row below): the web pins the tip at rig-local (−0.46, 0.081, 0.24) through contact while the port's plant retreats with travel. First run before any port change; bike (`the_bike_bar_contact_matches_the_web_handlebar_anchor`, exact) and rower (`the_composed_oar_grip_target_matches_the_web_hand_target`, exact at 1e-9 after the order fix) are pinned in the same change. | The 0.977 m contact / 0.033 m approach deltas are the `plant_basket_z` defect seen through a second observable, not new surface: the source-map's scoped follow-up (per-frame IK + blend rewrite + visual verification) closes both this row's windows and that row together. Once landed, tighten the bounds to 1e-6. |
| **SkiErg `plant_basket_z`** — known-wrong rendering | `rig_pose::solve_skierg` (`POLE_PLANT_FORWARD_OFFSET − cycle_frac · stroke_meters`, retreating course-anchored plant) | `renderer3dSkiAvatar.placePoleArms` keeps the plant stationary in rig-local — `poleTipLeft.z ≈ 0.24` across the whole contact (SkiErg athlete does not physically translate; the web treats "distance" as effort, not travel) | port | The rig-phase fixture is a clean oracle: web ≈ 0.24 through contact vs port at −1.76 m at cyc=0.25 mid-drive. **This is a rendering defect, not a design choice**: the port's pole basket drifts ~2 m rearward through every SkiErg stroke — visible on screen, the last unfixed defect the audit has found in the replay scene. Collapsing `plant_basket_z` to a constant matched the fixture but broke `requested_twist_stays_continuous_and_engages_the_budgets` in the viewmodel (~106° jump at step 8) — the existing `pose::skierg_targets` blend assumes `plant_basket` tracks `free_basket`, an assumption that only holds because the port's plant moves. That guard is information about the blend, not a reason to keep the plant moving. The collapse was reverted; the follow-up ships both changes together. | **One `pose::skierg_targets` rewrite closes it, not two.** Replace the current `plant_basket` / `free_basket` mix-blend with a per-frame IK against the fixed rig-local plant at `POLE_PLANT_FORWARD_OFFSET` (mirror what `placePoleArms` does): solve the pole shaft direction from `arm.handTarget` and the plant, drop the retreat term in `plant_basket_z` in the same edit. Requires visual verification on a host that captures Quick 3D correctly (any Linux session with Xvfb + Mesa, the Linux CI leg, or the author's RHEL box — see docs/qt-bridges-notes.md #17 for the narrow macOS/Metal exception), and a re-shot ski phase baseline (wide and close-up, keep the before set alongside). **Watch items (author-set): the failure to look for is what the rewrite introduces at the extremes, not the drift returning** — basket below the snow, rendered pole length changing (direction and length must be solved together), a visible snap at the blend handover. Shoot the before-captures first and keep both sets side by side. |

## rowplay-viewmodel — `crates/rowplay-viewmodel/src/replay/`

### Covered / by-construction

| Surface | Rust | Web | Via | Evidence |
| --- | --- | --- | --- | --- |
| Equipment anchor table | `anchors` | `static/replay-assets/README.md` + `renderer3dAssets.ts` | web | Exact table pins (oarlocks ±0.88/0.51/0.28 yaw π, skis ±0.15/0/0.16) cross-checked against the V3 template roots. |
| Venue geometry (baked) | `venue::validate_venue` + contracts | `renderer3dEnvironment.ts` `EnvironmentBuilder` | web (bake, ADR 0005/0010) | The baker drives the web builder directly at the pinned commit; contracts re-validated at build time; nightly bake check pins reproducibility. Not a fixture in `tests/fixtures/` but a full web evaluation. |
| V3 pack reader | `glb::validate_v3` | `renderer3dAssets.ts` contract | web asset | Validates the vendored web pack byte-exactly (733,864-byte pin) against the web's own contract. |

### Partial

| Surface | Rust | Web | Via | Evidence | Gap and risk |
| --- | --- | --- | --- | --- | --- |
| **Chase camera** | `camera::{rig, chase}` | `renderer3d.ts` camera block | web | The repo's source quote-check (`constants_match_the_web_source_…`): 16 verbatim web lines — the original 11 (rig table, damping rates, snap thresholds, `aspect < 1.25`, `5.4 + ghostPullback`, FOV constants) plus the 5 ghost-framing lines (`GHOST_PULLBACK 1.05`, the with-ghost narrow-scale ternary `2.12`/`2.1`/`1.38`/`1.2`, `comparisonMargin` per sport, `Math.tan(horizontalHalfFov) * 0.9`, `Math.min(2.5, comparisonSpan * 0.16)`). Three behavioural tests pin the composition: `ghost_pullback_and_midpoint_focus_engage_when_ghost_is_placed` (BikeErg, midpoint focus + height addend + `GHOST_PULLBACK` on top of `rig.back`), `narrow_ghost_widens_the_pullback_scale_per_sport` (narrow rower `2.12` vs skierg `1.38`), `a_wide_pair_forces_comparison_pullback_from_the_horizontal_fov` (200 m pair forces back derived from horizontal half-tangent, height caps at `2.5`). | The quote-check still silently skips when `reference/` is absent (CI); the behavioural tests pin composition against Rust-side computed values rather than a web-sampled fixture. A full parity fixture sampling the web `render`'s camera output per (sport, aspect, focus, ghost placement, speed) state would upgrade this to `covered` and catch damping-rate drift — same pattern as the rig-phase fixture. |
| **Course placement & loop radii** | `course::{place, accents, profile}` | `renderer3d.ts` `placeAvatar`/`SPORT_PROFILES`, `renderer3dAvatarKit.ts` `COURSE_LOOP_METERS` | web | Exact unit tests of the circle math and accents. **Verified against the web source at both audit passes** (`011e830` reference): `LIVE_LOOP_RADIUS 30` = web `loopRadius` (renderer3d.ts:825), `GHOST_LOOP_RADIUS 26` = web `ghostRadius` (:826), loop 1000 ✓, profiles (0.13/0.48, 0.08/1.45, 0.03/0) ✓, rower surge negation ✓, roll law ✓, `animPhase` rate ✓. The audit brief's suspicion ("set in 5b without a web check") is true of the process but the values agree. | No quote-check or fixture — the agreement is verified here, and a web change would not surface. `GHOST_LOOP_RADIUS` is not referenced by any test at all. Cheap to close: a `course` quote-check in the camera-test pattern plus one placement assertion. |
| Athlete posing composition | `pose::{rig_targets, PoseSolver}`, `equipment::{oar_rotations, blade_roll_degrees, …}` | V4 clip mapping + `rowRig.ts`/`skiEquipment.ts`/`bikeRig.js` constants; avatar assembly conventions | web+Studio | `clip_fraction` pinned to the web mapping exactly; geometry constants cited and pinned (oarlock 0.88/0.51/0.28 ✓, pole 1.37 ✓, axle 0.335 ✓, blade offset 1.82/−0.06 ✓, head angle 73° ✓); quaternion assembly tests exact-analytic. **Phase 7 fixed the burial-roll keying** the stage-1 pass flagged: `solve_rower` now keys the dip on `blade_water`, not the feather channel. | The rower's remaining Studio leftovers (`handle_y`/`handle_z`/`torso_lean`, feather-keyed) are documented as unconsumed — the V4 clip plus contact targets pose the athlete — so their divergence from the web avatar (fixture: handle off by 0.18/0.79, torso −0.28 vs +0.56) is dead-surface debt, not a live defect. `equipment::blade_roll_degrees`' 90°→0° spoon square matches the web's squaring convention. No web-generated fixture for the composed posing layer beyond the rig-phase corpus. |
| Runtime hand layer | `grip` (per-sport grip frames, `gripContractFor`, digit-chain collector, pose table), `PoseSolver` wrist orientation | `renderer3d{Row,Ski,Bike}Avatar.ts` grip frames + `handGrip.ts` runtime closure | web | Per-sport frames mirror the avatars (source-map Phase 7 row); unit tests + the gate's 10/10 grip-contact assertions; wrist orientation consumes the covered `wrist` budgets. | Runtime composition (frames + closure + budgets per frame) has no web-generated fixture — the web's runtime is a renderer; the closest oracle is the phase-shot baseline (visual). Phase 7 slice 3. |
| V4 athlete reader & clip sampling | `athlete::*` | `rowplay-athlete-v4.glb` + contract (web asset) | web asset | Contract validation pins the vendored web asset exactly (51 joints, 3 clips, drive ends 0.38/0.34/0.5); slerp/spline maths unit-pinned analytically. | Sampling correctness is against the vendored binary, not web *outputs*; acceptable — the asset is the web's. Kept partial with reason. |
| Material roles | `materials` | `renderer3dAssets.ts` roles + `renderer3dV4Assets.ts` surface roles | web | Role list pinned in V3 order; `validateMaterials` cross-checks QML against the Rust table; Theme-key scan. | The cross-check compares two transcriptions of the same table (self-referential); metalness/roughness values are Studio/web-mixed. Low risk — roles are enumerable strings. |
| Venue palette / sky / sun | `palette` | `renderer3d.ts` `SUN_OFFSETS`, venue `themed()` pairs | web | Colours flow from `theme` (golden-covered); `SUN_OFFSETS` [−22,18,14]/[16,28,10]/[12,20,−10] verified verbatim in this audit; elevation/azimuth derivation unit-pinned and settled in 5b. | No fixture; drift-invisible but values verified. Low. |
| Tier → Qt mapping | `tier` | web `QUALITY` browser fields → Qt | Studio | Shadow/MSAA tables and texture-set lists pinned per tier; inherits the quality-budgets verdict for the counts themselves. | Qt-specific mapping has no web counterpart (documented divergence); see quality row for the numeric disagreement. |
| HUD strings/numbers | `hud` | replay page formatting | web (via core) | All formatting delegates to core formatters (golden-covered transitively); rounding law pinned exactly. | Field order and the numeric block are desktop design (no web oracle). Low. |

### None — by design (recorded reasons)

| Surface | Rust | Why no fixture is planned |
| --- | --- | --- |
| Frame bundle layout | `frame` | Qt-internal pack contract (225 floats); no web counterpart. Layout consistency is enforced structurally; QML indexing is exercised by the pose pack test and the gate. |
| Venue runtime instancing | `venue_runtime` | Qt-specific `InstanceList` bucketing; the web's `scatterTint` per-instance colours are a documented divergence (2×2 quantisation). Determinism and completeness are unit-pinned; nothing web-comparable to pin. |
| Venue contract validation | `venue` (reader) | Covered by-construction (see above); listed here because its tests are contract-shape, not parity. |

## Risk ranking (drives the remaining stages)

Ordered by what to act on next (inversion likelihood × symptom subtlety × how
much of the scene inherits the error). The original 1–3 family was the
Studio-authored rig calibration: Phase 7 fixed the rower (`cf85cdb`), the
audit's stage 3 fixed the bike (two constants, `rig_phase_parity_bike` green).

1. **Rower arm/oar authority — inverted chain. FIXED** — and the same
   audit stage found the **composition-order sibling** at `173c6fa`: the
   port composed the oar's yaw and roll as `qz(roll) ⊗ qy(yaw)` (Studio's
   order) where the web's three.js Euler-XYZ is `qy(yaw) ⊗ qz(roll)` —
   breaking the roll's yaw-independent handle lift by a `cos(yaw)` factor
   and placing the port's grip target **0.181 m off the web's rendered
   grip at the catch** (measured against `v4HandTargets`, rowplay#199;
   `the_composed_oar_grip_target_matches_the_web_hand_target` now pins it
   at 1e-9). Same defect family as the channel bug: invisible to contact
   residuals. Fixed in `equipment::{oar_rotations, oar_rotations_from_yaws}`
   and both `pose.rs` grip-rebuild sites.
   The web makes the **arm** the authority and solves the oar yaw to meet it.
   `renderer3dRowAvatar.ts` `placeArms`: the `armDraw` channel (the "only
   velocity profile in this chain") schedules elbow flexion from the soft
   long-arm unlock (`ROWER_DRAW_SOFT_FLEXION` 0.32) to the finish fold
   (`ROWER_DRAW_FINISH_FLEXION` 2.46 rad), `rowerReachForFlexion` converts
   that to a shoulder→wrist reach via the law of cosines (≈0.758 → 0.258 m),
   and `solveRowerOarYaw(..., requestedReach, preferredYaw,
   forceReachBoundary = true)` returns the reach-circle root — the rendered
   yaw. The authored arc is **only** `preferredYaw`, a branch-selection
   fallback. The port had inverted this (`oar_sweep` authoritative, arms bent
   to reach it), *and* keyed the oar on the wrong channel: `handleTravel`
   where the web passes `armDraw`, which its own comment warns "would include
   its leg contribution and pull the grip through the knees and torso too
   early". Both are now fixed: the sweep and the roll's handle-rise ride
   `armDraw` (`rig_pose::solve_rower`), and the reach solve is composed in
   `PoseSolver::pose` (requested reach from the shoulder positions after the
   pelvis/torso pass → `solve_rower_oar_yaw` per side → grip targets
   recomputed → `Posed::oar_yaw` packed as the oar rotations instead of
   `oar_rotations(oar_sweep, …)`). Evidence: the corrected fixture
   (`rower_rig_phase_parity`, schedule at 1e-10 — it failed 1.05 rad before);
   `the_rower_pose_composes_the_arm_authority_oar_solve` (reach identity,
   branch continuity); and the two contact-pass tests that failed by 0.127 m
   when the channel fix landed alone, which is what proved the composition was
   required rather than optional. **Pinned against the web's rendered output**:
   the fixture now records, per sample and side, the arguments the web's
   `solveRowerOarYaw` received and the yaw the avatar rendered, and refuses to
   write unless it reproduces the rendered value —
   `the_composed_oar_yaw_matches_the_web_rendered_yaw` feeds that recording
   through the port's solver and matches the rendered yaw across all 256
   samples at 1e-9. Visual re-verification on the next CI phase-shot pass.
2. **`solve_skierg` phase calibration — CLOSED.** Ported to the web
   avatar's `animate` constants (torso hinge, head counter-tilt, pelvis
   carry, shoulder recording) and pinned by `rig_phase_parity_skierg` —
   eight comparisons at 1e-6 across all 128 skierg samples. Full account
   below.
3. **Chase camera ghost framing — CLOSED.** `GHOST_PULLBACK 1.05`,
   midpoint focus, `comparisonPullback` from the horizontal FOV,
   with-ghost narrow scales `2.12`/`1.38`, and the height addend from
   `comparisonSpan` are composed in `camera::chase` and driven from the
   backend via `CameraInput.ghost_placement`; 5 added quote-check
   web-lines and 3 behavioural tests pin the constants and composition.
   Remaining (see the chase row): a web-sampled parity fixture would
   upgrade it partial → covered.
4. **QA close-up camera framing** — **measured on `bf8d77f`**: the camera sat
   ~29–31 m from the athlete (one loop radius). The offset was rotated into
   the rig frame but never translated to the athlete's placement, so the twins
   framed venue geometry (row: a flat wall; bike: a spectator pillar). Fixed
   by translating the offset (`bf8d77f`, #25), with a test pinning 0.5–3.0 m
   from the athlete on all three sports. **Open question, not settled:** the
   camera code is unchanged since Phase 7 introduced it (`cf85cdb` — after the
   Phase 6 venues), yet Phase 7's T8 notes record close-up judgement ("the
   held-phase close-ups show flat wrists with no snap at either window edge";
   "the close-ups show no stiffness, so the budget stands as ported") and the
   feather read went through `Replay.setCloseupCamera`. So *something*
   produced athlete-framed close-ups then, and this measurement does not
   explain what. Either the feather/close-up work used a path other than the
   phase-shot grab, or those judgements were made on the wide set. Resolve
   against the RHEL `artifacts/phase-baseline/` before relying on either
   reading — and do not write it up as "the close-up never worked".
5. **`stroke_pose_at` web-pipeline corpus. CLOSED.**
   The production path had no web-generated sweep over varied inputs — the
   only fixture drove the Studio path with 3 range cases predating the web's
   #171 rework. `tools/gen-stroke-model-parity.mjs` now imports the web's
   real `strokeModel.ts` at the pinned commit (the stage-2 contract: record
   what the web returns, never re-derive it) and
   `replay-stroke-model-parity.json` pins `buildStrokeTimeline` +
   `strokePoseAt` + `fallbackStrokePose` over 9 timelines and 129 pose
   samples at 1e-10 (`stroke_model_web_pipeline_parity`; row moved to
   Covered). `warped_phase` is exempt by design — the port's C1 warp is the
   documented divergence — and is pinned as cycle + seam-side, plus exact at
   the `driveFrac = 0.5` identity. **The first run failed on one field, and
   the fault was in the harness, not the port**: serde_json's default fast
   float parser lands 1 ULP off a fraction of the fixtures' long literals —
   4283 of them across every float-bearing corpus, generated and
   Studio-exported alike, so every parity comparison had been running on
   perturbed inputs — which pushed one bike-midpoint `cycleFrac` from
   `0.4999999999999992` to exactly `0.5` and flipped the `drive` flag. That
   fix landed first, as its own PR ahead of this stack: exact float parsing
   workspace-wide (serde_json's `float_roundtrip` feature) plus the property
   pin `crates/rowplay-fixtures/tests/float_roundtrip.rs` — every float
   literal in every fixture must read back bit-identical to its nearest
   double — and the AGENTS.md rule it motivates (the harness is a suspect
   equal to the port). After it, every
   recorded timeline field, aggregate and pose field agreed at 1e-10 with no
   port change — the production intensity/fatigue/drive-fraction composition
   is confirmed against the web pipeline, not just its own unit pins.
6. **Wrist composition order + budgets against a rendered-orientation
   oracle.** Reframed (was "wrist budget sweep") after the oar
   composition-order defect: `qz(roll) ⊗ qy(yaw)` vs the web's Euler-XYZ
   `qy(yaw) ⊗ qz(roll)` placed the rower grip 0.181 m off the rendered
   target at the catch (ranking 1's sibling finding at `173c6fa`), and it
   survived four fixtures and every unit test because each pinned the
   *components* (yaw alone, roll alone) and never the composed result —
   re-expressed tests inherit whatever composition order the port made,
   so a swapped order passes silently. That is a **defect class, not a
   one-off**: any site stacking two or more rotations whose tests pin the
   parts rather than the product is exposed to it. The mechanism is
   derivable, not fixture-attested: three.js Euler `XYZ` builds
   `qx ⊗ qy ⊗ qz`, so the roll tilts the shaft out of plane first and the
   yaw about world-Y preserves the vertical lift (`L·sin(roll)` at any
   yaw); composing in the other order makes the roll act on an
   already-yawed shaft and the lift becomes `L·sin(roll)·cos(yaw)` — the
   measured ~6× shortfall at the finish's solved yaws.
   **The wrist layer is the most quaternion-dense surface in the port**
   (channel alignment, swing–twist decomposition, budget clamping, the
   forearm/shoulder share, the rower flat-wrist roll, the ski seam
   split) and it came down the same Studio path with the same
   verification mode: re-expressed web orientation unit tests. A
   composition-order inspection pass (this audit, post-find) re-derived
   every site op-for-op against the web (`constrain_wrist_frame`,
   `orient_hand_to_grip_channel`, `refine_grip_spin_for_wrist`,
   `refine_grip_tilt_for_wrist`, `flat_wrist_roll`, `split_seam_excess`,
   the wrist-rest build) and found **no order disagreement** — but that
   is inspection, i.e. re-expression, exactly the mode rule 3 distrusts;
   it triages, it does not close. What closes is an oracle for the
   *rendered orientation*: no fixture in the corpus records a hand/world
   quaternion today (`replay-current-main-grips.json` pins scalar stage
   angles + contact/tip positions — positions survive an order swap that
   keeps tips on the surface; the rig-phase fixture records positions
   and node Euler rotations of rig groups, not the solved hand frame).
   **Plan**: extend `gen-rig-phase-parity.mjs` (or a sibling generator)
   to record the avatars' rendered hand world quaternions — the
   procedural avatars expose `v4Targets.leftHand` whose `quaternion`
   after `animate()`+`resolveWorldContacts()` is the composed frame; for
   the V4 path the comparable is the hand bone's world quaternion — then
   drive `PoseSolver::pose` per sample and compare the port's solved
   hand local rotation at a stated tolerance (the skinning pipeline is
   f32 at the frame pack, so ~1e-6 rad is the right bar, not 1e-9).
   Self-check the generator as with `handTargets` (RowErg's visible hand
   sits exactly on the grip channel; assert the recorded quaternion maps
   the hand-local long axis onto the recorded shaft direction). Then,
   with the composed frame pinned, run the budget sweep the old title
   asked for (twist/flexion/deviation against the web's budget
   constants across all phases — the metrics are exposed in
   `WristMetrics`).
   **The multi-axis sweep across the port** (same audit pass), by
   category with the pose solver's passes named explicitly — not folded
   into an "FK" bucket:
   - **Single-axis, no order choice exists**: wheel, crank, pole-shaft
     `pole_rotation`, `blade.rotation.x`.
   - **Forced-FK accumulation** (parent ⊗ child, the only order that
     walks a rest chain): `grip.rs` rest-chain walk, `hand_grip` digit
     FK, `Workspace` skeleton FK.
   - **Pose-solver passes — inspected individually, each order-free by
     construction, not by inspection of a *choice***: the pelvis
     alignment is **translation-only** (root-local shift + subtree
     recompute; no quaternion written); the two-bone IK contact pass
     (`solve_limb` → `aim_joint`) aims each joint with a single minimal
     swing (`rotation_between`) and its only composition is the forced
     `delta ⊗ world` then `conjugate(parent) ⊗ desired` world→local
     conversion — the sole valid order, since converting local-first
     would compute the delta in the wrong frame; the post-orient
     re-close is a re-run of the same `solve_limb` machinery (a
     sequence, not an order choice). The pass *sequence* (arm-authority
     → pelvis → IK → orient → re-close → seam-split) is pinned end-to-
     end by the rig-phase fixture's rendered `handLeft/Right` and
     `poleTipLeft/Right` positions and the `handTargets` tests.
   - **Genuine a⊗b choice sites, all accounted for**:
     `equipment::oar_rotations{,_from_yaws}` — fixed and pinned by the
     combined-order unit test plus `handTargets` parity; the rower
     arm-authority recompose (`pose::pose` rower branch) — same fix,
     same pins; the ski seam split (`split_seam_excess`) — named in the
     exposure above with the wrist chain.
   One residual worth naming: `frame::pack` writes rotations as f32, so
   a composition error below ~1e-7 rad is unobservable on screen by
   construction — a property of the pipeline, not coverage. And that
   tolerance reasoning generalises as a triage rule (same distinction
   as ranking 10): **any replay-geometry comparison whose value ends up
   in the frame pack is asserting precision the renderer discards below
   ~1e-7. A tight test is not wrong — catching a systematic difference
   early is worth it — but a 1e-12 miss on a frame-packed quantity is
   parity hygiene, a 1e-3 miss is a rendering defect. Do not treat all
   failures as equal.**
7. **Quality budgets** — per-field numeric disagreement with the web `QUALITY`
   table needs a decision (adopt or record), not a fixture per se.
8. **Motion-graph phase-source gap** — extend a generator to sweep
   `warpedPhase ≠ phase` (and varied timing) so the input selection is pinned.
9. **Course quote-check + `GHOST_LOOP_RADIUS` pin** — values verified correct
   twice; make that permanent in the camera-test pattern.
10. **Oar-yaw reach-boundary approach — an attainable, ill-conditioned
    interval the corpus never samples.**
    **Read before implementing, twice over.** First, this is parity
    hygiene, not a rendering concern: the worst case below is ~1e-8 rad ≈
    6×10⁻⁷ degrees — invisible on screen and below anything downstream
    reacts to — which is why the item sits below 6–9. Second, **tail
    samples need a conditioning-scaled tolerance**: above argument 0.999
    the amplification runs from ~20× to ~5×10⁷ at the last double below
    1, so even with exact parsing the port/web op-order difference
    (~1 ULP) can legitimately move a tail sample's yaw by up to ~1e-8 rad
    from benign arithmetic alone — above the 1e-9 and 1e-12 tolerances
    these families use. A flat tolerance in the tail reports a defect
    that isn't there; scale it with the derivative
    (k·ε/√(1−x²), ε the argument's ULP, k a small multiple for op-order
    slack).
    `row_equipment::solve_rower_oar_yaw`'s root solve clamps its `acos`
    argument to ±1; the +1 clamp is the reach boundary — the web's
    `forceReachBoundary` fallback for a requested reach the inboard circle
    cannot achieve — and a boundary branch is exactly where two
    implementations part company. Measured directly (argument probe over
    the equipment corpus's 20 `oarYaw` samples, one amplitude-degenerate;
    found while verifying the `float_roundtrip` audit's conditioning claim
    instead of inferring it from absent deltas): the clamp **is**
    exercised — one recorded sample sits past +1 at argument 1.024, four
    past −1 (min −1.607) — but the largest recorded argument in the
    achievable regime is **0.867** (~30° of root offset), so (0.867, 1.0)
    is empty in the corpus. That interval is where the solve's conditioning
    degrades: `|acos′| = 1/√(1−x²)` grows from ~2 at 0.87 through ~70 at
    0.9999 to ~5×10⁷ at the last double below 1, where a 1 ULP argument
    shift moves the yaw ~1e-8, four orders above the corpus's 1e-12
    tolerance — also why the pre-`float_roundtrip` runs never showed
    amplification, and why the past-boundary sample cannot (the clamped
    branch collapses the root offset to 0 and is perfectly
    well-conditioned). **The gap is sample coarseness, not system
    behaviour**: sweeping `armDraw` finely through the web's own schedule
    (`requestedRowerWristReach` at the pin — flexion `0.32 +
    draw·(2.46−0.32)` over the 0.39/0.38 arms, reach 0.758 → 0.256)
    against each recorded geometry, 9 of the 19 non-degenerate geometries
    cross the boundary **continuously** — their arguments walk the whole
    interval and graze ≥ 0.9999 before clamping. (The recorded reaches,
    0.55–0.95, already exceed the schedule's 0.758 maximum, so the
    exporter was driving synthetic reaches rather than the schedule.)
    **The drawn half is not an operational gap**: the composed layer
    covers it — #27's `oarSolve` recording sweeps real cycle phases
    (armDraw 0 → 1, 256 solves, reach 0.2555–0.7582) with 36 solves in
    [0.256, 0.538). The same recording also walks the interval (10
    solves in (0.867, 1.0), max 0.9946) and crosses the boundary (11 at
    or above 1, min 1.0098), so the coarse crossing is already pinned
    there; what no corpus samples is the approach tail above 0.999
    (0 of 256), where the amplification is largest. Remedy — record the
    sweep through the schedule, not around it: extend
    the `oarYaw` corpus (and the rig-phase generator's `oarSolve`
    recording, PR #27) with a fine `armDraw` grid per geometry,
    densified around each geometry's crossing rather than uniform, which
    crosses arguments 0.99 / 0.999 / 0.9999 and the boundary naturally,
    pinning the approach, the exact-boundary root collapse and the
    fallback together. Driving `armDraw` also keeps the
    record-what-renders rule: `requestedReach` is a length in metres, so
    it cannot be set to the argument targets directly (a literal 0.99
    reach lands deep in the clamped branch) — the reaches that produce
    each argument come out of the flexion schedule. Runtime shoulders
    move with the clip, which only widens the attainable set beyond these
    recorded geometries.

11. **Skierg wrist-refinement ±π wraps — inherited web behaviour the port
    amplifies into position.**
    Both `refineGripTiltForWrist` and `refineGripSpinForWrist` derive their
    correction angle from a bare `atan2`, so when the measured geometry
    walks past ±π the angle — and with it the sign of the applied
    correction — snaps between frames. This is **web behaviour, not a port
    defect**: the pinned web avatar snaps too (tilt at cyc 0.2635, qjump
    1.299 rad; spin at cyc 0.7080, qjump 1.096 rad; `tools/web-tilt-probe.mjs`
    is the oracle). The architectural difference is that the web's
    procedural path sets `arm.hand.position` directly, so its snap is
    orientation-only (hand stays within 0.011 m), while the port's
    V4-style chain solves the wrist as `target − R_hand·offset`, coupling
    the oriented offset into position — the same snap moves the rendered
    hand **~0.11 m** (2·|offset⊥|·sin(Δθ/2) with Δθ ≈ 1.68 rad).
    **One bounded unwrap attempt was made and reverted** (Phase 7.5):
    unwrapping the angle to the nearest equivalent of the previous value
    before the budget clamp fails because the refinement is called
    several times per frame (4 alternating passes × 2 hands), so a
    scalar "previous" is not the previous *frame* — it re-binds to
    whichever call preceded it (often the other hand's mid-pass value)
    and introduces a *new* discontinuity at step 443 (0.040 m) while
    treating the original. Correcting the state keying would be a second
    attempt, out of scope by the one-attempt rule. Any future fix must
    (a) key the unwrap state per (hand, pass, call-site) or carry it in
    the solve state instead of a thread-local, and (b) accept that
    unwrapping diverges from the web's rendered orientation at the wrap
    — it makes the port smoother than its oracle, a deliberate deviation
    to record in `docs/source-map.md`, not silent parity. Until then the
    wraps are carved out of the dense continuity guard with this note
    (`requested_twist_stays_continuous_and_engages_the_budgets`).

### Guard reachability sweep (post–Phase 7.5 cleanup)

Class, not incident: assertions after a mid-loop panic / early skip have
never necessarily executed. Sweep of the long-running guards (dense
continuity, governor-after-break, warp 20k, three 401-step rig sweeps):

- **N = 12** post-hazard assertion sites inspected (7 in
  `requested_twist_stays_continuous…` after the first-step `Option` gate
  or post-loop; 4 post-loop after `break` in the governor degrade tests;
  1 post-loop on the warp dense sweep).
- **M = 12** confirmed reached on a passing run (dense sites by
  counters: elbow/hand_pos/hand_ori/twist_cont = 1999, post = 1 per
  sport; governor/warp by green completion — `break` cannot skip
  post-loop asserts).
- **K = 0** never reached; nothing to fix.

The historical "step-1377" abort is the skierg spin-wrap window
(cyc ≈ 0.6885 = 1377/2000); with the ranking-11 carve-out the
orientation assert no longer kills the loop, so the post-loop
saturation / engagement asserts now run. Rig-pose mid-loop asserts
after `else { panic!("sport mismatch") }` are structurally reached
whenever the let-else does not fire (confirmed by the three green
401-step sweeps; not double-counted in N).

### Skierg twist demand reaches the keep budget independently

Three-level trace of `requested_twist` (the quantity the saturation assert
measures): L1 `requested_twist: twist_angle` (`wrist.rs:349`); L2
`let mut twist_angle = 2.0 * dot_twist.atan2(delta[3])` (`wrist.rs:266`);
L3 `dot_twist = delta · bone_axis_local` and
`delta = quat_mul(effector, quat_inverse(rest.hand_rest))`
(`wrist.rs:260–263`). None of those levels contain `SKI_WRIST_TWIST_KEEP`
/ π/6; the budget only clamps `kept_twist` after the demand is recorded.

### Spin-wrap carve-out converted to wrap-aware — guard fails

The cyc-window budget widen (0.678–0.698 / 0.255–0.275) was a SKIP of the
tight TOL. Replaced with `wrap_signed` + TOL 0.02/0.35 everywhere. Guard
fails at Skierg step 529 (cyc=0.2645, tilt-wrap window): dh=0.0961 >
POSITION_TOL=0.02; prev=[-0.3194, 0.8947, -0.4476] curr=[-0.2895, 0.9817,
-0.4756]. Not widened; skip not restored.

The former ranking 2 (`solve_skierg` phase calibration — torso base, head
counter-tilt) is closed: the fix ports the web
`renderer3dSkiAvatar.animate` constants (`SKI_NEUTRAL_TORSO_PITCH 0.055 +
hipHinge · SKI_TORSO_HINGE_RANGE 0.56` for the torso, `-hipHinge ·
SKI_HEAD_GAZE_COUNTER_TILT 0.38` for the head), exposes the pelvis carry
as `pelvis_y`/`pelvis_z` and pins all three against the rig-phase fixture's
`upperRotation` / `headRotation` / `upper.position` (`rig_phase_parity_skierg`
no longer `#[ignore]`d).

The former ranking 3 (chase camera ghost framing) is closed: `GHOST_PULLBACK`,
midpoint focus, `comparisonPullback` from the horizontal FOV, with-ghost narrow
scales `2.12`/`1.38`, and the height addend from `comparisonSpan` are all
composed in `camera::chase` and driven from the backend via
`CameraInput.ghost_placement`. Behavioural tests plus 5 additional quote-check
web-lines pin the constants and composition; the module doc lost its "no
ghost lane (Phase 5c)" caveat. A full web-sampled parity fixture for the
whole chase camera (all constants, damping, aspect / ghost combinations)
remains as the "upgrade `partial` → `covered`" follow-up, same pattern as
the rig-phase fixture.

Pole rotation was investigated too and left as Studio's `-0.20 - poleSweep
· 0.92`. Reading the port's composition through
(`pose::skierg_targets`: `carried = normalize([side·0.12, -cos(θ).max(0.18),
sin(θ)])`, axes X=lateral, Y=up, Z=forward) shows Studio's `θ` is the pole's
angle-from-vertical, and `-0.20 - poleSweep · 0.92` approximates `poleAngle
- π/2` (where the web's `poleAngle = degToRad(80 - poleSweep · 57)` is the
angle from horizontal) within 1–3° across the cycle. The resulting carry
direction is `(0.12, -0.98, -0.20)` at the reach (mostly down) and
`(0.12, -0.44, -0.90)` at the finish (mostly back) — the same phase and
direction the web produces via its vertical/horizontal decomposition
`(0.04, -0.985, -0.170)` and `(0.20, -0.39, -0.90)`. An earlier reading of
the constants alone (Studio `-0.20 - poleSweep · 0.92` vs web
`degToRad(80 - poleSweep · 57)`) read as a phase inversion in the same
family as the rower and bike; the composition shows it is not one. The
port's frame convention is different from the web's but the rendered pole
direction agrees within a few degrees. Real oracle values from the fixture
will confirm this once `placePoleArms` runs headless (the audit's None
entries below).

The generator now parents the avatar to a throwaway `THREE.Scene` before
each sport walk, so `renderer3dSkiAvatar.placePoleArms` (which
early-returns on `!group.parent` at line 828) runs and the fixture
records real oracle values for `arm.hand`, `skierg-pole-tip-*`, and the
pole shaft rotations. That surfaced a real distinction the previous
pelvis-collapse hid: the port's `preferred_hand_y/z` is the *pre*-pole-
solve target it feeds to `pose::solve_rigid_contact3d`, but the fixture
records `arm.hand.getWorldPosition()` **after** `placePoleArms` runs its
pole-blend IK / grip-reach solve — different quantities. The two agree
exactly (`d=(0, 0)` at 1e-6) within the pure recovery window between
`SKI_POLE_OFF_CYCLE = 0.29` and `SKI_POLE_APPROACH_START_CYCLE = 0.88`;
outside that window the pole solve pulls the hand off the authored polar
arc / Bezier by up to 0.58 m (contact) or 0.23 m (next-plant approach).
`rig_phase_parity_skierg` now pins the recovery window at 1e-6 and the
two out-of-window ranges were recorded in this document's None table
with the fix path (the comparable web quantity is `arm.handTarget`,
which lives on the arm object closed over inside `makeSkierAvatar`).
**Closed at reference `173c6fa`**: rowplay#199 exposed `v4HandTargets`
on every avatar, the generator records it per sample (post-pole-solve
for skierg — `placePoleArms`'s contact pass overwrites `handTarget`
with the solved hand before the arm IK), and the viewmodel test pins
the recovery window at machine epsilon while measuring the contact and
approach windows (0.977 m / 0.033 m) against the documented
`plant_basket_z` divergence.

The hip local counter-tilt (`-hipHinge · SKI_PELVIS_COUNTER_TILT 0.14`)
and the shoulder position are still not pinned — `sample()` doesn't read
`hips.rotation` and no scene-graph node exposes the shoulder. Both are
small extensions to the generator's `sample()` function.

**Stage-2 fixture reproducibility is now byte-stable across Node
versions**, achieved by serialising every finite number through
`Number.prototype.toPrecision(17)` (the IEEE-754 double round-trip
length). V8's default `JSON.stringify` uses the shortest string that
round-trips and that "shortest" length has shifted across V8 releases,
so the same double serialises as `0.1249269234996172` on one Node and
`0.12492692349961732` on another — differing by ULPs of text without
differing in the double. `tools/gen-rig-phase-parity.mjs` now ships a
`stableStringify` helper that pipes every number through
`toPrecision(17)` before writing; regenerating under Node 22.22.2 and
Node 24.5.0 produces byte-identical fixtures. All parity tests tolerated
the previous ULP drift so nothing changes in behaviour; the win is that
a CI regeneration check would now pass. Fixed precision beats pinning
the Node version (which the venue bake does) because it makes the
output independent of V8's number-to-string algorithm rather than
tracking it. The next stage-2 generator should adopt `stableStringify`
from the start.

## Stage-2 generator contract

**Record what the web renders, not how it computes it** (AGENTS.md). Phase 7's
row-phase generator re-implemented the web's formula — including the porter's
`handleTravel` channel choice — and asserted the port matched itself; this
audit's rig-phase generator builds the real avatar, samples its animated
transforms, and re-solves from its own recording to prove the recording is
faithful, which is what exposed the 1.05 rad channel error. Where a quantity is
only reachable as a pure function, record that function's inputs *and* output
per sample, and self-check the reconstruction against the rendered value.

Follow the in-repo pattern (`tools/gen-row-phase-parity.mjs`, Phase 7,
`tools/gen-rig-phase-parity.mjs`, this audit, and
`tools/gen-stroke-model-parity.mjs`, ranking 5): evaluate the **web source**
at a pinned commit under Node ≥ 23.6 (Studio's exporter reads via `git show`;
the rig-phase and stroke-model generators assert the checkout's HEAD equals
the pin and record per-file SHA-256s), sweep the input space the surface
spans (a phase calibration needs samples across the full cycle at varied
`driveFrac`/rate, not one fixed timing), and record provenance in the
fixture so a reference change surfaces as a diff. Where the quantity is
reachable as a module, **import the web's module and call it** (the
stroke-model generator imports the real `strokeModel.ts`); re-deriving its
formula in the generator records the porter's reading of the source instead
of the web's behaviour. Commit the generator under `tools/`, register the
fixture in `tests/fixtures/manifest.json` and `PROVENANCE.md`, and add it to
`GENERATED_FIXTURES` in `tools/vendor-fixtures.py`. Serialise doubles at
`toPrecision(17)` (`stableStringify`) for byte-stable output — and note the
Rust side must parse them with serde_json's `float_roundtrip` feature (see
ranking 5); the default fast parser is 1 ULP off on ~9% of such literals.
**Run the test before fixing anything** and state in the PR whether the
fixture passed first run or caught a defect — never adjust a fixture to
match the port.

## State of this audit

- Stage 1: complete, re-verified against `8732cb5` (see the process rule at
  the top). Headline: of 40 surfaces, **15 covered, 16 partial, 8 none**
  (3 none-by-design).
- Stage 2, group 1 (composed rig-phase fixture): delivered and re-run
  post-Phase-7. `tools/gen-rig-phase-parity.mjs` evaluates the web avatar
  factories at the pinned commit over the full cycle at two timing inputs per
  sport (`tests/fixtures/replay-rig-phase-parity.json`, 384 samples, 16
  hashed sources, deterministic). **First run (stale base, pre-fix): 13 of 13
  fields failed. Re-run against `8732cb5`: the rower seat — the exact field
  Phase 7 fixed — now passes, independently confirming the generator agrees
  with the shipped fix.** The parity test is split per sport: bike enabled,
  rower/skierg `#[ignore]`d naming their outstanding findings.
- Stage 3: rower landed as Phase 7's `cf85cdb`; **bike landed next** (pedal
  π-inversion and the 0.31 wheel rotation radius — `rig_phase_parity_bike`
  green across all 128 bike samples). **Skierg landed after that**, against
  a rebuilt rig-phase fixture (avatar parented to a throwaway
  `THREE.Scene` so `placePoleArms` runs; skierg samples now record
  `hipsRotation` and `shoulderLeft`; numbers pass through
  `toPrecision(17)` for byte-identity across Node/V8 versions). Fixed in
  `solve_skierg`: `torso_lean` `0.18 + hipHinge · 0.55` → `0.055 +
  hipHinge · 0.56`; `head_pitch` `torso_lean · 0.2` → `-hipHinge · 0.38`;
  new `hip_counter_tilt = -hipHinge · 0.14` for the pelvis local
  counter-tilt. Pelvis carry exposed as `pelvis_y`/`pelvis_z`.
  `plant_basket_z` was investigated with the new oracle: web ~0.24
  through contact vs port −1.76 m at cyc=0.25 — a rendering defect
  where the port's pole basket drifts ~2 m rearward every stroke
  (visible on screen, the last unfixed replay-scene defect the audit
  has found). Collapsing the port's plant to a constant matched the
  fixture but broke
  `requested_twist_stays_continuous_and_engages_the_budgets` in the
  viewmodel (~106° jump at step 8) because `pose::skierg_targets`'s
  mix-blend of `plant_basket` and `free_basket` assumes the two track
  each other — an assumption that only holds because the port's plant
  moves. That guard is information about the blend, not a reason to
  keep the plant moving. The fix is one edit scoped to
  `pose::skierg_targets`: rewrite the blend as a per-frame IK against
  the fixed rig-local plant at 0.24, drop the retreat term in the same
  change. Requires visual verification on a host that captures Quick 3D
  correctly (any Linux + Xvfb + Mesa session, the Linux CI leg, or the
  author's RHEL box — the black-viewport bug is macOS/Metal-specific;
  see docs/qt-bridges-notes.md #17) and re-shot ski phase baselines
  (wide + close-up, keep the before set alongside). Deferred as a
  follow-up.
  **Watch items beyond the drift itself** (author-set): fixing the plant
  means the tip stays put while the athlete travels, so the
  hand-to-plant direction sweeps far wider than the blend has ever
  seen. The failure to look for is **not the drift returning — it is
  what the rewrite introduces at the extremes**: the basket dropping
  below the snow surface, the pole's rendered length changing if shaft
  direction and pole length are not solved together, or a visible snap
  where the blend hands over from plant to free. Shoot the ski wide
  captures **before** the change and keep them; a metre of drift is
  unmistakable side by side, and so is anything the fix adds.
  `rig_phase_parity_skierg` pins six fields at 1e-6 (`torso_lean`,
  `head_pitch`, `hip_counter_tilt`, `pelvis_y/z`, `shoulder_y/z`) and
  `preferred_hand_*` in the pure recovery window at 1e-6; the earlier
  `#[ignore]` is gone.
  The `preferred_hand_*` contact / next-plant-approach windows closed at
  reference `173c6fa` (rowplay#199's `v4HandTargets`; the viewmodel test
  pins recovery at machine epsilon and bounds contact 0.977 m /
  approach 0.033 m against the documented `plant_basket_z` divergence —
  the source-map's per-frame-IK follow-up closes both);
  `plant_basket_z` still needs the pose-solver rewrite described above.
  The rower composed layer landed with the armDraw channel +
  reach-solve composition (PR #27; the composed yaw is pinned by the
  rig-phase fixture's `oarSolve` recording and the viewmodel pose test).
  Remaining: the stage-2 follow-ups and rankings 6–10.
- **Queue order (author-set, Sept 2026): the skierg per-frame IK rewrite
  preceded Phase 8 — DONE.** `plant_basket_z` was the largest known rendered
  defect, confirmed by two independent observables — the fixture's plant
  position (web ~0.24 pinned through contact vs port retreating to
  −1.76 m) and the exposed hand target (0.977 m at contact, first
  consumption of `v4HandTargets`). It landed as Phase 7.5 (PR #31):
  the plant is fixed at catch in course space, the carried tip blends
  in direction space, contact is 0.02 m, and the two web-inherited
  wrist-refinement wraps are recorded as ranking 11 below. Rankings
  6–10 follow as fill-in around
  Phase 8 (live mode) and Phase 9 (packaging).
  (ranking 1), the rower composed layer (ranking 3 — the reach solve is dead
  at runtime, confirmed by trace), and the remaining stage-2 groups.
- Ranking 5 (stroke-model web pipeline): closed against `bf8d77f`. The
  fixture caught a **harness** defect, not a port defect — serde_json's
  default float parser is off by 1 ULP on a fraction of the fixtures' long
  literals (4283 of them across every float-bearing corpus, generated and
  Studio-exported alike, silently perturbing the parsed inputs of every
  parity comparison). The fix landed first, as its own PR: exact float
  parsing workspace-wide (serde_json's `float_roundtrip`) plus the
  property pin `float_roundtrip.rs`, plus the AGENTS.md rule that the
  harness is a suspect equal to the port. The port itself agreed at 1e-10
  once it was fed the web's exact doubles. Rankings 1–4 are
  in flight as PRs #25–#28; check those before starting any of them.

## Capture caveat (narrower than the earlier wording suggested)

The `ROWPLAY_PHASE_SHOTS=1` baseline **cannot be produced on the one
macOS/Metal host that hit `grabToImage`'s black-viewport bug**
(docs/qt-bridges-notes.md #17). That was one previous session's host;
it is not a general property. Verified paths that DO capture the 3D
scene:

- The Linux CI gate (Xvfb + Mesa, `QSG_RHI_BACKEND=opengl`) — uploads
  `artifacts/phase-*.png`.
- A local Linux session with the same env vars
  (`xvfb-run -a cargo test -p rowplay-app --test qml_runtime_gate`
  under `QT_QPA_PLATFORM=xcb QSG_RHI_BACKEND=opengl
  LIBGL_ALWAYS_SOFTWARE=1`). Verified 2026-09 in this session: the
  gate walk produced `replay-{row,ski,bike}.ppm` with the venue
  palettes (row teal water, ski near-white snow, bike cream terrace)
  and zero black samples across a 96-point grid over the `View3D`
  region.
- The author's RHEL box (Wayland, Intel UHD 630), used for the Phase 7
  T8 baseline.

`common::assert_viewport_rendered` samples the viewport region under a
real GL backend so a blank 3D area fails instead of riding on the
chrome; that's the guard for the black-viewport case regardless of
host.

**Do not read the earlier "verify on the RHEL box" phrasing as a
requirement.** On the affected macOS host, yes — verify against the
live window, not a local capture. Everywhere else (any Linux session
with Xvfb + Mesa), local visual verification is a valid path.

A second capture gap: the QA close-up camera (`closeup_camera_view`, used only
under `ROWPLAY_PHASE_CLOSEUPS`) measured ~29–31 m from the athlete on `bf8d77f`
(one loop radius) — the offset was rotated into the rig frame but never
translated to the athlete, so the twins framed venue geometry (row: a flat
wall; bike: a spectator pillar). Fixed in `bf8d77f` (#25) with a framing test, and
re-enabled in CI so the fixed framing is exercised and reviewed. **Reviewed on
the row twins**: the athlete is framed at close range in every phase, and the
aim follows the contact midpoint (pelvis + hands), so the lens tracks the
0.44 m seat slide and the layback instead of cropping the near body at the
catch and the finish — the two phases the instrument exists to judge. A fixed
rig offset framed mid-drive well and clipped the extremes; the aim now moves
1.56 m between catch and finish (`the_closeup_camera_tracks_the_athlete_through_the_stroke`). The measurement is of `bf8d77f` only — see ranking 4 for the
unresolved history (Phase 7's notes do record close-up judgement).
