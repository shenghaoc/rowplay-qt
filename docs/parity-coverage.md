# Parity coverage map

Which ported surface has been checked against the web, by what evidence, and
what could still be wrong in it. Written by the pre-Phase-8 parity audit;
**re-verified against `origin/main` @ `8732cb5`** (post-Phase 7) after the
original pass was cut from a stale base. Every row reflects the code and the
pinned references at that commit (`reference/rowplay` @ `011e8303`,
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
- **Web-generated in this repository** (the audit's stage-2 pattern, both
  recording `sourceCommit` and per-file SHA-256s inside the JSON):
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

### Partial

| Surface | Rust | Web | Via | Evidence | Gap and risk |
| --- | --- | --- | --- | --- | --- |
| **Rig phase calibration, composed layer** | `rig_pose::{solve_rower, solve_skierg, solve_bike}` | `renderer3d{Row,Ski,Bike}Avatar.ts` composed `animate` state | web | `replay-rig-phase-parity.json` (this audit, 384 samples × 2 timing sweeps). Per-sport tests: **bike enabled and green**; rower and skierg `#[ignore]`d with recorded verdicts. Post-Phase-7 re-run: **rower `seat_z` passes** — independent confirmation that this fixture agrees with the shipped fix — and crank passes. | Rower outstanding: `oar_sweep` — the port now rides `armDraw` (corrected channel) but still omits the arm-authority reach solve the web applies on top (**an inverted chain, not a layering indifference** — ranking 1 has the full finding and the fix plan); Studio handle/torso channels are dead surface. Skierg outstanding: torso base, hand-path frame composition (0.7–0.9 m), course-anchored plant (ranking 2). |
| **Rower oar channel** | `rig_pose::solve_rower` (`armDraw` in) | `renderer3dRowAvatar.ts` `placeOars(equipmentHandleTravel = graph.body.armDraw.value)` | web | `rower_rig_phase_parity` (corrected fixture, 33 samples at 1e-10, green). The audit found the port (and Phase 7's own generator) keyed the oar sweep and the roll's handle-rise on `handleTravel`; the web's comment warns that channel "would include its leg contribution and pull the grip through the knees and torso too early". The channel error is up to 1.05 rad at mid-drive, and its handle-rise half exactly explained the previously-unexplained +0.029 rad roll residual. |
| Motion-graph timing parameters | `motion_graph::timing_into` etc. | `motionGraph.ts` `timingInto` | web | Corpus sweeps phase at one fixed timing per sport; drive-fraction clamps and rate-dependent timing unit-pinned exactly (motion_graph.rs tests, re-expressing web tests). | No corpus varies `driveFrac`/`secondsPerCycle`/rate inputs. A regressed clamp would fail only the unit pins, which were written from the port. **Phase-source choice unpinned**: the graph reads `pose.phase` (motion_graph.rs, matching the web), but because the corpus pins `warpedPhase = phase`, a regression to `warped_phase` would pass every existing test. Sign error unlikely; input-selection error possible. |
| Stroke pose, web pipeline (production path) | `stroke_model::{build_stroke_timeline, stroke_pose_at}` | `strokeModel.ts` `buildStrokeTimeline`, `strokePoseAt` | web | Unit tests pin timeline arithmetic, the web amplitude law `clamp(0.94 + i·0.12, 0.94, 1.06)` and drive-fraction law exactly; the app renderer uses this path (backend `replay.rs`, not `compute_at_time`). Verified: the web 3D renderer itself never calls `strokePoseAt` per frame — the Svelte page does, then hands `pose.phase` to the avatar — so the Rust production chain (page-equivalent path) matches the web's shape. | No web-generated corpus of `stroke_pose_at` outputs over varied rate/intensity/fatigue/duration inputs; the existing fixture drives the Studio path with 3 range cases predating the web's #171 rework. Intensity/fatigue composition could drift undetected. Inversion unlikely (progress/amplitude laws pinned). |
| Warp stroke phase | `motion::warp_stroke_phase[_rate]` | `motion.ts` `warpStrokePhase` | web (deliberate divergence) | Boundary mapping (0.4·τ → π), identity at f = 0.5, monotonicity and C1/periodicity guard tests. | The port is intentionally C1 where the web is C0 — a web-generated fixture would fail by design; the divergence row is the contract. Residual risk: the documented contract itself is only enforced at the pins, not swept. |
| PerfGovernor | `motion::PerfGovernor` | `motion.ts` `PerfGovernor` | web (deliberate divergence) | Scenario tests pin the calibrated budget caps (44.0 = 2×22, 25.6 = 1.6×16) and the rollback ladder. | Documented divergence (66 ms outlier clamp, ≥10% payoff check, rollback+lock vs the web's monotone levels). Web defaults (22 ms / 60 / 90 / 30, 250 ms ignore) are the floor; nothing regenerates if the web retunes. |
| `meters_per_cycle` | `motion::meters_per_cycle` | `motion.ts` `METERS_PER_CYCLE` | web | Constants 11/8/5 exercised transitively by the motion corpus (`generator_pose`); unit test asserts only `> 0`. | No direct pin of the table; low risk (values confirmed against the web source in this audit). |
| Two-bone IK | `two_bone::*` | `figurePose.ts` `solveTwoBone3D` / `solveRigidContactPoint3D` | Studio naming, web geometry | Exact analytic unit tests; since Phase 7 also exercised transitively at 1e-12 through `row_equipment::solve_rower_arm` in the equipment corpus. | No direct web-generated sweep of the two-bone solver's own input space (folded/overextended/hint-flip); risk low — the geometry is exact and now consumed under fixture load. |
| Wrist orientation & budgets | `wrist` | `handGrip.ts` `orientHandToGripChannel`, `refineGrip{Spin,Tilt}ForWrist`; `renderer3dV4Motion.ts` `constrainWristFrame` + SkiErg keep/share rules | web | 12 unit tests re-express the web `handGrip.test.ts` orientation suite; the budget constants ride the equipment corpus (per the module doc). | No dedicated web-generated sweep of the budget pass (swing–twist redistribution); a mis-signed budget would deform wrists subtly. Phase 7 slice 2. |

### None

| Surface | Rust | Web (actual) | Via | Evidence | Risk |
| --- | --- | --- | --- | --- | --- |
| **Skierg rig stroke-phase calibration** | `rig_pose::solve_skierg` | `renderer3dSkiAvatar.ts` (`skiPreferredHand`, pole carry `degToRad(80 − poleSweep·57)`, pelvis `0.735 − kneeFlex·0.11 + rebound·0.045`, torso `0.055 + hipHinge·0.56` with −0.14/−0.38 counter-tilts) | Studio, copied op-for-op | Range tests + reach-annulus bound (self-referential); the audit's composed-layer fixture has caught it (torso base 0.18 vs 0.055; preferred hand 0.7–0.9 m off; plant basket rig-local vs course-anchored) — `rig_phase_parity_skierg` is `#[ignore]`d and the fix is outstanding (ranking 1). | **Inversion-capable with subtle symptoms** (pole motion is symmetric). Pole rotation conventions differ outright: web 1.396 − 0.995·sweep vs Rust `−0.20 − 0.92·sweep`. Rust drops the web's hip/head counter-tilts and its hard reach clamp. A phase-inverted pole plant passes every enabled test. |
| Engine sampling | `engine::{sample_at, sample_index_at, ReplayState}` | `engine.ts` | web | Exact unit re-expressions of the web's `engine.test.ts` (progress pins, interpolation, non-zero origins, speed presets). | Not fixture-backed, but every value is arithmetic on the web's own test data; inversion would fail immediately. Not worth closing with a fixture (recorded reason). |
| Comparability guard | `comparability::*` | `comparabilityGuard.ts` | web | Unit re-expression over workout-type → axis mapping and band rules. | Discrete classification; a fixture would restate the same table. Not worth closing (recorded reason). |
| Ghost pick | `ghost_pick::*` | `ghostPick.ts` | web | Unit re-expression of the ranking semantics with exact winner ids. | Same as above — total ordering pinned by units; Studio's hardening intentionally not ported (documented). Not worth closing (recorded reason). |
| Quality budgets | `quality::RenderQuality::budgets` | `renderer3d.ts` `QUALITY` (lines 65–138) | Studio | Budget table pinned exactly — **against Studio's documented tiers**. | Not an inversion risk, but the portable fields that are comparable **numerically disagree with the web**: wake 0/16/28/44 vs web 0/20/32/52; spray 0/40/48/72 vs 0/64/80/112; per-catch 0/4/4/6 vs 0/7/8/10; ring segments 48/72/96/144 vs lane segments 48/80/112/160 (only `buoysPerRing` 12/18/22/28 matches). Needs either a per-field divergence record or adoption of the web values. |

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
| **Chase camera** | `camera::{rig, chase}` | `renderer3d.ts` camera block | web | The repo's only source quote-check (`constants_match_the_web_source_…`): 11 verbatim web lines — rig table, damping rates, snap thresholds, `aspect < 1.25`, `5.4 + ghostPullback`, FOV constants. Behavioural tests pin the composed dynamics to Rust's own constants. | The quote-check pins constants, not composition: nothing numeric verifies the chase assembly's sign conventions against the web, and the check silently skips when `reference/` is absent (CI). **The web's ghost-comparison framing is not ported**: `ghostPullback 1.05`, midpoint focus, `comparisonPullback` from the horizontal FOV, and the with-ghost narrow scales (2.12/1.38) — yet Phase 5c shipped ghost racing, so the ghost can sit outside the framed lane. The module doc still says "no ghost lane (Phase 5c)". |
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

1. **Rower arm/oar authority — inverted chain. FIXED.**
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
2. **`solve_skierg` phase calibration** — torso base, hand-path frame
   composition, course-anchored plant; fixture-confirmed, structural (a
   0.7–0.9 m frame-composition disagreement, not a constant).
3. **Chase camera ghost framing** — web comparison framing unported while
   ghosts ship; camera sign conventions unverifiable by quote-check alone.
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
5. **`stroke_pose_at` web-pipeline corpus** — production path has no
   web-generated sweep over varied inputs.
6. **Wrist budget sweep** — covered only through the equipment corpus and
   web-test re-expression.
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

## Stage-2 generator contract

**Record what the web renders, not how it computes it** (AGENTS.md). Phase 7's
row-phase generator re-implemented the web's formula — including the porter's
`handleTravel` channel choice — and asserted the port matched itself; this
audit's rig-phase generator builds the real avatar, samples its animated
transforms, and re-solves from its own recording to prove the recording is
faithful, which is what exposed the 1.05 rad channel error. Where a quantity is
only reachable as a pure function, record that function's inputs *and* output
per sample, and self-check the reconstruction against the rendered value.

Otherwise follow the in-repo pattern (`tools/gen-row-phase-parity.mjs`, Phase 7,
and `tools/gen-rig-phase-parity.mjs`, this audit): evaluate the **web source** at
a pinned commit under Node ≥ 23.6 (Studio's exporter reads via `git show`;
the rig-phase generator asserts the checkout's HEAD equals the pin and
records per-file SHA-256s), sweep the input space the surface spans (a phase
calibration needs samples across the full cycle at varied `driveFrac`/rate,
not one fixed timing), and record provenance in the fixture so a reference
change surfaces as a diff. Commit the generator under `tools/`, register the
fixture in `tests/fixtures/manifest.json` and `PROVENANCE.md`, and add it to
`GENERATED_FIXTURES` in `tools/vendor-fixtures.py`. **Run the test before
fixing anything** and state in the PR whether the fixture passed first run or
caught a defect — never adjust a fixture to match the port.

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
  green across all 128 bike samples). Outstanding: the skierg calibration
  (ranking 1), the rower composed layer (ranking 3 — the reach solve is dead
  at runtime, confirmed by trace), and the remaining stage-2 groups.

## Capture caveat (affects the phase-shot baseline)

The `ROWPLAY_PHASE_SHOTS=1` baseline **cannot be produced on macOS**: on this
host `grabToImage` returns a black Quick 3D viewport while the live window
renders the scene correctly (docs/qt-bridges-notes.md #17), and the
whole-window `assert_rendered` did not notice because the sidebar supplies the
colours. Phase 7's own baseline was captured on the RHEL machine (Wayland,
Intel UHD 630) and is valid; this caveat is about hosts where `grabToImage`
returns black. The baseline is captured in the Linux CI gate (Xvfb + Mesa,
which does render it) and uploaded as `artifacts/phase-*.png`;
`common::assert_viewport_rendered` now samples the viewport region under a real
GL backend so a blank 3D area fails instead of riding on the chrome. Verify 3D
changes on macOS against the live window, not a local capture.

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
