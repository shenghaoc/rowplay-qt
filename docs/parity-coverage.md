# Parity coverage map

Which ported surface has been checked against the web, by what evidence, and
what could still be wrong in it. Written by the pre-Phase-8 parity audit
(2026-09); every row was verified against the pinned references
(`reference/rowplay` @ `011e8303`, `reference/rowplay-studio` @ `3d406a5b`)
rather than trusted from `docs/source-map.md`.

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
  `docs/source-map.md` claimed (see the rig-pose row).
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

All fixtures live in `tests/fixtures/`, vendored from Studio
(`tools/vendor-fixtures.py`). Provenance splits in two:

- **Web-generated** (`replay-current-main-{motion,2d,equipment,grips}.json`):
  Studio's `script/export_rowplay_native_parity.mjs` evaluates the **web
  TypeScript** at pinned commit `4d96480` via `git show` (never the working
  copy), under Node ≥ 23.6, and records per-file SHA-256s. The web's
  `src/lib/replay/` is byte-identical between `4d96480` and the pinned
  reference HEAD `011e830` (verified: `git diff 4d96480..011e830 --
  src/lib/replay/` is empty), so these fixtures evaluate exactly the code the
  reference pins.
  - The generator's pose scheme (`poseFor`) sweeps `phase` over the full cycle
    at fixed per-sport timing (rower 60/28 s, drive 0.38; skierg 1.875 s,
    0.34; bike 0.75 s, 0.5), with `intensity` varying by a stride-37
    permutation and **`warpedPhase = phase`** — so nothing the corpus pins can
    distinguish a consumer that keys on warped phase from one that keys on
    raw phase. `watts`, `amplitude`, `fatigue` stay fixed.
- **Studio-semantics** (`stroke-pose-parity`, `replay-race-gap/result/rival-*`,
  `Concept2/*`, predictor/duration-band): hand-verified or Studio-computed
  expectations. `stroke-pose-parity.json` predates the web's 2026-07 stroke
  rework (#171) and drives Studio's frame-based path, not the web pipeline.

Two web-generated fixtures are not yet consumed: the equipment and grips
corpora are shape-checked only (`grip_and_equipment_parity` is `#[ignore]`d
for Phase 7's V4 contact solver).

## rowplay-core — `crates/rowplay-core/src/replay/`

### Covered

| Surface | Rust | Web | Via | Evidence |
| --- | --- | --- | --- | --- |
| Motion-graph channel algebra | `motion_graph::sample_*_motion_graph` | `motionGraph.ts` `sample{Rower,Skier,Bike}` | web | `replay-current-main-motion.json`: 129 phases × 3 sports × every channel (value/velocity/acceleration) at 1e-10. Pins the full curve algebra over one cycle at the generator's fixed timing. |
| 2D kinematics projections | `sport_kinematics::solve_{rower,skier,bike}_kinematics` | `sportKinematics.ts` `solve*` | web | `replay-current-main-2d.json`: 64 phases × 3 sports, all output channels at 1e-10, over the same synthetic pose. |
| Canvas palettes | `theme::{COLORS_*, venues_*}` | `renderer.ts` `COLORS_*`, `VENUES_*` | web | Same fixture's `palettes` block: character-exact strings, all sports, light + dark. |
| Race gap helpers | `race_gap::*` | `replayGap.ts` | web+Studio | `replay-race-gap-parity.json` at 1e-6 incl. Infinity/NaN cases; Studio's non-zero-origin helpers included. |
| Race result | `race_result::race_result` | `replayGap.ts` (finish) via Studio's interpolated crossing | Studio | `replay-race-result-parity.json`; the interpolated crossing is a documented deliberate deviation (source-map divergences). |
| Rival sources | `rivals::{constant_pace_strokes, parse_rival_file}` | `sources.ts` + Studio hardening | web+Studio | `replay-rival-sources-parity.json`: endpoints 1e-3, bike divisor exact, CSV/TCX/FIT normalisation. Hardening documented. |
| Stroke pose, Studio path | `stroke_model::compute_at_time` | Studio `ReplayStrokePose.computeAtTime` | Studio | `stroke-pose-parity.json`: 3 cases (one per sport), index/drive exact, scalars as hand-verified **ranges**. Kept only for this fixture; not the production path. |

### Partial

| Surface | Rust | Web | Via | Evidence | Gap and risk |
| --- | --- | --- | --- | --- | --- |
| Motion-graph timing parameters | `motion_graph::timing_into` etc. | `motionGraph.ts` `timingInto` | web | Corpus sweeps phase at one fixed timing per sport; drive-fraction clamps and rate-dependent timing unit-pinned exactly (motion_graph.rs tests, re-expressing web tests). | No corpus varies `driveFrac`/`secondsPerCycle`/rate inputs. A regressed clamp would fail only the unit pins, which were written from the port. **Phase-source choice unpinned**: the graph reads `pose.phase` (motion_graph.rs:530, matching the web), but because the corpus pins `warpedPhase = phase`, a regression to `warped_phase` would pass every existing test. Sign error unlikely; input-selection error possible. |
| Stroke pose, web pipeline (production path) | `stroke_model::{build_stroke_timeline, stroke_pose_at}` | `strokeModel.ts` `buildStrokeTimeline`, `strokePoseAt` | web | Unit tests pin timeline arithmetic, the web amplitude law `clamp(0.94 + i·0.12, 0.94, 1.06)` and drive-fraction law exactly; the app renderer uses this path (backend `replay.rs`, not `compute_at_time`). Verified: the web 3D renderer itself never calls `strokePoseAt` per frame — the Svelte page does, then hands `pose.phase` to the avatar — so the Rust production chain (page-equivalent path) matches the web's shape. | No web-generated corpus of `stroke_pose_at` outputs over varied rate/intensity/fatigue/duration inputs; the existing fixture drives the Studio path with 3 range cases predating the web's #171 rework. Intensity/fatigue composition could drift undetected. Inversion unlikely (progress/amplitude laws pinned). |
| Warp stroke phase | `motion::warp_stroke_phase[_rate]` | `motion.ts` `warpStrokePhase` | web (deliberate divergence) | Boundary mapping (0.4·τ → π), identity at f = 0.5, monotonicity and C1/periodicity guard tests. | The port is intentionally C1 where the web is C0 — a web-generated fixture would fail by design; the divergence row is the contract. Residual risk: the documented contract itself is only enforced at the pins, not swept. |
| PerfGovernor | `motion::PerfGovernor` | `motion.ts` `PerfGovernor` | web (deliberate divergence) | Scenario tests pin the calibrated budget caps (44.0 = 2×22, 25.6 = 1.6×16) and the rollback ladder. | Documented divergence (66 ms outlier clamp, ≥10% payoff check, rollback+lock vs the web's monotone levels). Web defaults (22 ms / 60 / 90 / 30, 250 ms ignore) are the floor; nothing regenerates if the web retunes. |
| `meters_per_cycle` | `motion::meters_per_cycle` | `motion.ts` `METERS_PER_CYCLE` | web | Constants 11/8/5 exercised transitively by the motion corpus (`generator_pose`); unit test asserts only `> 0`. | No direct pin of the table; low risk (values confirmed against the web source in this audit). |
| Skier elbow direction | `sport_kinematics::solve_skier_elbow_direction` | `sportKinematics.ts` | web | Unit sign tests (plant: vertical < 0, fore-aft > 0; pole-off flips; continuity 1e-6). | The web-generated sweep **exists** (32 samples inside `replay-current-main-equipment.json`) but is `#[ignore]`d with the Phase 7 gate. Enable rather than regenerate. |

### None

| Surface | Rust | Web (actual) | Via | Evidence | Risk |
| --- | --- | --- | --- | --- | --- |
| **Rower rig stroke-phase calibration** | `rig_pose::solve_rower` (seat/handle/oars/joints vs cycle) | `renderer3dRowAvatar.ts` (`placeAvatar` seat block, `placeOars`, `placeTorso`: `SEAT_CATCH_Z 0.26 + pelvisTravel·(−0.44)`, `OAR_YAW_CATCH 0.68 → OAR_DRAW_YAW −0.8`, roll `−(bladeDepth·0.28 + handleProgress·0.04)`, `BODY_PITCH 0.56 → −0.3`) | Studio (`ReplayRigPose.swift`), copied op-for-op | Range/boundedness tests only — they would pass for any calibration inside the envelope, including an inverted one. | **Inversion-capable, and concrete disagreements exist.** The web drives the seat with `pelvisTravel` (asymmetric +0.26 → −0.18); Rust/Studio drive it with `legExtension` (symmetric −0.20 → +0.20, mirrored sign). Web oar yaw spans +0.68 → −0.8 asymmetric; Rust `−0.58 + handle·1.16` symmetric, mirrored. Web keys oar roll on **`bladeDepth`**; Rust keys `handle_y`/`oar_feather` on the **feather** channel. Every Rust range is the web's mirror — coherent only if the whole downstream scene honours the same flipped frame, which nothing tests (the vendored GLBs are authored in the web's frame). The source-map row claiming agreement cites `rowrig.ts`/`skiEquipment.ts`/`bikeRig.js` — modules that contain contact landmarks and IK, **not** this calibration; the web equivalent lives in the avatar files the row never names. |
| **Skierg rig stroke-phase calibration** | `rig_pose::solve_skierg` | `renderer3dSkiAvatar.ts` (`skiPreferredHand`, pole carry `degToRad(80 − poleSweep·57)`, pelvis `0.735 − kneeFlex·0.11 + rebound·0.045`, torso `0.055 + hipHinge·0.56` with −0.14/−0.38 counter-tilts) | Studio, copied op-for-op | Range tests + reach-annulus bound (self-referential). The preferred-hand path (polar arc 0.72 − elbow·0.28 + armExt·0.08, angle 0.56 − sweep·2.56, Bezier controls (0.1, 0.48)/(0.2, 0.3), pelvis/torso frame constants) matches the web exactly — the one sub-surface with genuine web provenance, though still fixture-less. | **Inversion-capable with subtler symptoms** (pole motion is more symmetric, as the audit brief notes). Pole rotation conventions differ outright: web 1.396 − 0.995·sweep vs Rust `−0.20 − 0.92·sweep`. Rust drops the web's hip/head counter-tilts (`−0.14`, `−0.38`) and its hard reach clamp toward the shoulder. A phase-inverted pole plant passes every existing test. |
| **Bike rig stroke-phase calibration** | `rig_pose::solve_bike` | `renderer3dBikeAvatar.ts` (`placePedalLegs` `pedalY = −r·cos`, `pedalZ = −r·sin`; wheel `meters / 0.31`; torso `0.8 + spineLean·0.9`) | Studio, copied op-for-op | Circle-membership and roll tests (self-referential; pass for any phase offset). | **Sign mirror found**: web negates both pedal components; Rust doesn't — the pedals sit π apart from the web's at the same crank angle. Wheel divisor differs (Rust 0.335 outer-tyre radius vs web 0.31 `wheelRadius`, ≈7.5% under-rotation). Torso lean base and scale differ (0.74 + lean vs 0.8 + 0.9·lean); ankle neutral (−0.05) and clamp dropped. A π-shifted crank is exactly the symmetric error the audit predicted would survive range tests. |
| Two-bone IK | `two_bone::*` | `figurePose.ts` `solveTwoBone3D` / `solveRigidContactPoint3D` | Studio naming, web geometry | Exact analytic unit tests (reachable/folded/overextended, hint flip, contact sphere) — strong, but written from the port. | Sign/hint inversion would fail the unit pins; risk low. Web-generated sweeps for the contact geometry exist inside the ignored equipment corpus (`rowRig.solveRowerOarYaw`, flexion/reach laws) whose Rust counterparts are structured differently (pose.rs targets) — Phase 7 mapping work, not a quick enable. |
| Engine sampling | `engine::{sample_at, sample_index_at, ReplayState}` | `engine.ts` | web | Exact unit re-expressions of the web's `engine.test.ts` (progress pins, interpolation, non-zero origins, speed presets). | Not fixture-backed, but every value is arithmetic on the web's own test data; inversion would fail immediately. Not worth closing with a fixture (recorded reason). |
| Comparability guard | `comparability::*` | `comparabilityGuard.ts` | web | Unit re-expression over workout-type → axis mapping and band rules. | Discrete classification; a fixture would restate the same table. Not worth closing (recorded reason). |
| Ghost pick | `ghost_pick::*` | `ghostPick.ts` | web | Unit re-expression of the ranking semantics with exact winner ids. | Same as above — total ordering pinned by units; Studio's hardening intentionally not ported (documented). Not worth closing (recorded reason). |
| Quality budgets | `quality::RenderQuality::budgets` | `renderer3d.ts` `QUALITY` (lines 65–138) | Studio | Budget table pinned exactly — **against Studio's documented tiers**. | Not an inversion risk, but the portable fields that are comparable **numerically disagree with the web**: wake 0/16/28/44 vs web 0/20/32/52; spray 0/40/48/72 vs 0/64/80/112; per-catch 0/4/4/6 vs 0/7/8/10; ring segments 48/72/96/144 vs lane segments 48/80/112/160 (only `buoysPerRing` 12/18/22/28 matches). The divergence row says "Studio's portable budgets" without enumerating that the web's numbers differ where comparable. Needs either a per-field divergence record or adoption of the web values. |

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
| **Chase camera** | `camera::{rig, chase}` | `renderer3d.ts` camera block (lines 2956–3215) | web | The repo's only source quote-check (`constants_match_the_web_source_…`): 11 verbatim web lines — rig table, damping rates, snap thresholds, `aspect < 1.25`, `5.4 + ghostPullback`, FOV constants. Behavioural tests pin the composed dynamics to Rust's own constants. | The quote-check pins constants, not composition: nothing numeric verifies the chase assembly's sign conventions (back along −tangent, lateral along +radial) against the web — a mirrored lateral would read as an aesthetic choice, and the check silently skips when `reference/` is absent (CI). **The web's ghost-comparison framing is not ported**: `ghostPullback 1.05`, midpoint focus, `comparisonPullback` from the horizontal FOV, and the with-ghost narrow scales (2.12/1.38) — yet Phase 5c shipped ghost racing, so the ghost can sit outside the framed lane exactly the way the audit brief warns. The module doc still says "no ghost lane (Phase 5c)". |
| **Course placement & loop radii** | `course::{place, accents, profile}` | `renderer3d.ts` `placeAvatar`/`SPORT_PROFILES`, `renderer3dAvatarKit.ts` `COURSE_LOOP_METERS` | web | Exact unit tests of the circle math and accents. **Verified in this audit against the web source**: `LIVE_LOOP_RADIUS 30` = web `loopRadius` (renderer3d.ts:825), `GHOST_LOOP_RADIUS 26` = web `ghostRadius` (:826), loop 1000 ✓, profiles (0.13/0.48, 0.08/1.45, 0.03/0) ✓, rower surge negation ✓, roll law ✓, `animPhase` rate ✓. The audit brief's suspicion ("set in 5b without a web check") is true of the process but the values agree. | No quote-check or fixture — the agreement is verified once, here, and a web change would not surface. `GHOST_LOOP_RADIUS` is not referenced by any test at all. Cheap to close: a `course` quote-check in the camera-test pattern plus one placement assertion. |
| Athlete posing composition | `pose::{rig_targets, PoseSolver}`, `equipment::{oar_rotations, blade_roll_degrees, …}` | V4 clip mapping + `rowRig.ts`/`skiEquipment.ts`/`bikeRig.js` constants; avatar assembly conventions | web+Studio | `clip_fraction` pinned to the web mapping exactly; geometry constants cited and pinned (oarlock 0.88/0.51/0.28 ✓, pole 1.37 ✓, axle 0.335 ✓, blade offset 1.82/−0.06 ✓, head angle 73° ✓); quaternion assembly tests exact-analytic. | **Keying risk**: `rig_pose` drives the oar **roll** from the feather channel (`oar_feather = −0.06 + feather·0.34`, and `handle_y` likewise) while the web buries the blade from **`bladeDepth`** (`−(bladeDepth·0.28 + handleProgress·0.04)`) — the wrong-channel pattern the audit brief predicted, so rig solver and scene compose the error. (`equipment::blade_roll_degrees`, the 90°→0° spoon square from feather, matches the web's squaring convention; it is the burial roll that is keyed wrong.) Pose targets also consume `rig_pose` outputs, inheriting that row's calibration risk. No web-generated fixture exists for any of it. |
| V4 athlete reader & clip sampling | `athlete::*` | `rowplay-athlete-v4.glb` + contract (web asset) | web asset | Contract validation pins the vendored web asset exactly (51 joints, 3 clips, drive ends 0.38/0.34/0.5); slerp/spline maths unit-pinned analytically. | Sampling correctness is against the vendored binary, not web *outputs*; acceptable — the asset is the web's. Kept partial with reason: no behaviour beyond the contract to pin. |
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

## Risk ranking (drives stage 2)

Ordered by (inversion likelihood × symptom subtlety × how much of the scene
inherits the error). 1–3 are the same defect class as the rower rig: a
Studio-authored calibration, copied op-for-op, never compared to the web, and
self-referentially tested. Concrete sign/range disagreements already identified
by this audit are listed in each row above — the stage-2 fixtures exist to
adjudicate them numerically, and the web wins unless a divergence row says
otherwise.

1. **`solve_rower` phase calibration** — mirror-symmetric ranges, wrong drive
   channel (legs vs pelvisTravel), feather-for-bladeDepth keying; the
   source-map row that claims agreement cites the wrong web modules.
2. **`solve_bike` phase calibration** — pedals sit π off the web's (found);
   wheel divisor and torso law differ.
3. **`solve_skierg` phase calibration** — pole-rotation convention differs;
   counter-tilts dropped; hand path is the only sub-surface with web
   provenance.
4. **Chase camera ghost framing** — web comparison framing unported while
   ghosts ship; camera signs unverifiable by quote-check alone.
5. **Equipment oar roll keying** (with 1) — feather channel vs `bladeDepth`;
   composes with the rig solver.
6. **`stroke_pose_at` web-pipeline corpus** — production path has no
   web-generated sweep over varied inputs.
7. **Quality budgets** — per-field numeric disagreement with the web `QUALITY`
   table needs a decision (adopt or record), not a fixture per se.
8. **Motion-graph phase-source gap** — extend the generator to sweep
   `warpedPhase ≠ phase` (and varied timing) so the input selection is pinned.
9. **Course quote-check + `GHOST_LOOP_RADIUS` pin** — values verified correct
   in this audit; make that permanent in the camera-test pattern.
10. **Skier elbow direction** — enable the existing 32-sample sweep from the
    equipment corpus (Phase 7 gate permitting).

## Stage-2 generator contract

Follow `export_rowplay_native_parity.mjs` (the pattern the audit brief calls
the row-phase generator pattern): read each web file with
`git show <pinned-commit>:<path>` from the reference checkout (never the
working copy), evaluate the TypeScript under Node ≥ 23.6 with the reference
`node_modules`, sweep the input space the surface spans (a phase calibration
needs samples across the full cycle at varied `driveFrac`/rate, not one fixed
timing), and record `sourceCommit` plus per-file SHA-256s in the fixture so a
reference change surfaces as a diff. Commit the generator under `tools/`,
register the fixture in `tests/fixtures/manifest.json` and `PROVENANCE.md`
and, if it stays Studio-vendored, in `tools/vendor-fixtures.py`. **Run the
test before fixing anything** and state in the PR whether the fixture passed
first run or caught a defect — never adjust a fixture to match the port.

## State of this audit

- Stage 1 (this document): complete. Headline: of 35 surfaces, **10 covered,
  14 partial, 11 none** (of which 3 are none-by-design with recorded reasons).
- Stage 2 (generate fixtures down the ranking): pending.
- Stage 3 (fix what the fixtures catch): pending. Known candidates already
  visible from verification alone: the rig-pose calibration family (rows 1–3)
  and the camera's missing ghost framing. When fixed, correct
  `docs/source-map.md` — in particular the rig-pose divergence row, whose web
  column must point at `renderer3d{Row,Ski,Bike}Avatar.ts`, and whose
  "agree on the geometry constants" claim is unsupported for phase
  calibration.
