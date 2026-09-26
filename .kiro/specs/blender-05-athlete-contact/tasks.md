# Blender Phase 5 — seat/pelvis, contact truth and athlete

Status: planned; not started. Blender Phases 1–4 are merged and complete.
This is the Blender roadmap, separate from the original implementation phases.
[ADR 0016](../../../docs/decisions/0016-authored-overcast-lighting.md) brings
athlete refinement/replacement back into scope as an intentional visual
divergence from the web V4 athlete. V4 remains the baseline until this audit
is complete and the owner chooses a route. This spec authorizes no automatic
replacement and the roadmap PR implements none of its tasks.

Order: correct the authoritative seat/pelvis reference, understand the contact
chain, audit the human asset, obtain the owner's decision, then implement the
chosen athlete. Fit the shell around that final athlete in
[Phase 6](../blender-06-hero-fit/tasks.md). Do not fit the boat twice.

## Phase 5.0 — Correct #130 and freeze the seat/pelvis reference

- [ ] Correct the authoritative anchor and pelvis target tracked in
  [#130](https://github.com/shenghaoc/rowplay-qt/issues/130), with web-derived
  evidence and regression coverage; freeze and record the corrected reference
  before either audit. This is Step 0 of Phase 5, not work between Phases 5
  and 6. Do not implement it in the docs-only roadmap PR.
- [ ] Re-capture the baseline against the corrected reference. The current
  target sits below the web's: auditing it now would contaminate judgments of
  seated height, pelvis position, torso scale, knee path, foot/stretcher
  relationship, cockpit clearance and apparent athlete/boat proportions.
  Record any resulting shell mismatch for Phase 6 rather than pre-fitting it.

## Phase 5.1 — Contact-truth audit

Keep two questions separate: **A. Is the human asset visually/structurally
poor? B. Is the pose/contact machinery putting a good or bad asset in the
wrong place?** Persistent hands/grips and hands/SkiErg poles failures are a
first-class requirement. Neither the asset nor the solver is presumed guilty.

### Existing evidence and its limits

These are existing records and the owner's reported visible defects, not new
measurements performed by this roadmap PR. Reproduce them after Step 0.

| Sport | Evidence to carry into the audit | What it does not establish |
| --- | --- | --- |
| RowErg | `the_composed_oar_grip_target_matches_the_web_hand_target` in `crates/rowplay-viewmodel/src/replay/pose.rs` pins the composed grip target to the web hand target at 1e-9; the closure and runtime gate report 10/10 digit contacts. The rendered hand still visually reads wrong against the oar. | Logical contact parity is not sufficient evidence of correct visible contact. Test hand-bone orientation, helper-joint placement, digit-chain proportions, skin weights, hand mesh proportions, palm thickness, grip-seat geometry and target-point versus deformed-skin location. |
| SkiErg | `crates/rowplay-app/tests/qml_runtime_gate.rs` records 8/10 digit contacts from `replay-current-main-grips.json`: both pinkies stay short of the pole. The source map and parity audit record pole/hand-target motion concerns, including plant/contact behavior; visible hand/pole contact remains wrong. | The digit shortfall, motion behavior and visible skin failure need separate diagnoses. Distinguish pole motion/plant target, hand-target/IK, wrist orientation, digit-helper geometry and skinned hand geometry. Correct or isolate solver-level errors before blaming the mesh. |
| BikeErg | The handlebar-anchor parity test and 10/10 closure provide a control case with simpler contact geometry and stronger existing parity. | Use it to distinguish global V4 hands/skin failures from cylindrical-closure, SkiErg-specific or motion-solve failures; it is not proof of skin contact by itself. |

Read the [source-map divergences](../../../docs/source-map.md#divergences)
and [parity audit](../../../docs/parity-coverage.md) with their chronology.
The old SkiErg retreating-plant defect was corrected in implementation
Phase 7.5; its historical 0.977 m delta is not a current measurement.
Later arm/wrist/contact work is recorded there too. Re-measure the current
plant, approach and contact behavior; do not reopen an old numerical defect
by copying an obsolete bound or equate target parity with skin acceptance.

### Trace the full chain for all three sports

- [ ] Audit each layer independently and locate where each error first enters:

  ```text
  authoritative equipment geometry
          ↓
  logical hand/contact target
          ↓
  wrist / hand / digit bones and helper joints
          ↓
  skinned hand mesh
          ↓
  visible contact in Qt
  ```

- [ ] For each observed defect, record phase, side, evidence, responsible
  layer(s), proposed correction and verification. Classify as one or more of:
  equipment transform; contact target; IK / arm solve; wrist solve;
  digit-helper geometry; skeleton proportions; skin weights; hand mesh
  geometry; material / shading illusion. Prove the attribution by isolating
  layers. Correct defects in their owning layer; arbitrary QML-node offsets
  to make contact look right are prohibited.

### Measure the actual skinned hand

- [ ] Evaluate the actual deformed/skinned hand mesh against the equipment
  surface at representative contact phases. Measurement must correspond to
  the skin rendered in Qt, not joint origins or helper points. Validate any
  offline skinning reconstruction against that Qt frame and record transforms,
  asset/version, replay time, sport, side, renderer, scheme and tier.
- [ ] For cylinders (grips/poles), inspect or calculate minimum skin-surface
  distance, visible gaps, penetration/intersection, palm seating, finger wrap,
  thumb seating, pinky/ring-finger reach and wrist orientation relative to the
  grip axis. A single minimum distance cannot prove whole-hand contact.
- [ ] Establish and justify tolerances during Phase 5 from hand scale,
  grip/pole radius, skin deformation and visual inspection. This roadmap
  invents no skin-contact tolerance; existing target-test tolerances are not
  skin acceptance thresholds.

Success means **the rendered, skinned hand visibly and geometrically contacts
its equipment correctly**, not merely that an abstract hand target equals a
mathematical grip target. Intentional release must remain release; the audit
must not force contact through a phase that calls for separation.

### Required views and phases

| Sport | Minimum phases |
| --- | --- |
| RowErg | Catch; mid-drive; finish; recovery / half slide. |
| SkiErg | Pre-plant approach; plant/contact; loaded pull; pole release; recovery. |
| BikeErg | Representative replay phases as the control, checking both hands. |

- [ ] At each phase capture ordinary chase/replay framing, a hand close-up,
  and the equipment surface; add a skeleton/helper overlay or diagnostic
  render when useful. Resolve floating hands, hand intersection, grip through
  palm, fingers failing to wrap and implausible wrist orientation clearly.
- [ ] Publish the classified contact report before drawing conclusions about
  retaining or replacing the human mesh. Record unresolved causes explicitly.

## Phase 5.2 — Existing V4 athlete audit

Only after the contact chain is understood, inspect the athlete itself against
the corrected Step 0 reference.

- [ ] Geometry/appearance: silhouette, anthropometric proportions, shoulder
  width, hip width, torso length, arm/leg proportions, hands/fingers, feet,
  head, facial treatment, clothing shape, topology, normals and UVs.
- [ ] Rigging/deformation at catch, mid-drive, finish, recovery / half slide
  and representative SkiErg phases: skin weights, shoulders, elbows, wrists,
  fingers, hips, knees, feet and grip deformation.
- [ ] Attribute the mannequin appearance among geometry, proportions,
  materials, normals, Qt lighting/material configuration, rigging, weight
  painting and pose, including combinations. Separate it from contact-chain
  defects rather than calling every problem a bad mannequin.
- [ ] Run a **material/normal-only improvement experiment in Qt on the
  existing V4 mesh** before concluding geometry must be replaced. Hold mesh
  geometry, rig and pose fixed; compare flat material treatment, normals,
  roughness, skin/fabric separation and lighting response under controlled
  Qt views. Diagnose the lighting contribution separately. This experiment
  cannot excuse genuinely bad geometry or deformation.

## Phase 5.3 — Recommendation and owner checkpoint

- [ ] Recommend one route, with evidence and tradeoffs:
  1. Retain V4 geometry and improve presentation.
  2. Substantially refine V4 in Blender.
  3. Replace V4 using a permissively licensed base human.
  4. Owner-authored sculpt, with the agent handling rig/export/validation.
- [ ] State separately whether the existing skeleton, hand helpers, digit
  chains and skin weights are reusable or need replacement/calibration.
- [ ] Establish an explicit athlete triangle budget **before substantial
  modeling**, with counting scope and validation recorded. The old exemption
  from a replacement-rower budget is superseded; this roadmap sets no number.
- [ ] **STOP for owner decision.** Record the selected route and its approved
  scope before implementation. Do not automatically continue into replacement.

### Licensing checkpoint

Apply [ADR 0002](../../../docs/decisions/0002-gpl-3-licence-and-asset-provenance.md)
rigorously. Select no human generator now. MakeHuman / MPFB may be candidates;
verify current terms at the replacement decision, including output terms.
ADR 0002 currently prohibits downloaded human models/generator output: if the
owner chooses that route, explicitly amend that prohibition by an architectural
decision before importing anything. Candidate status and a permissive licence
alone do not waive it; this roadmap does not import or approve a third-party base.

- [ ] Audit clothing, hair, textures, accessories, morphs and community assets
  separately; do not infer their terms from the base or generator.
- [ ] Record source URLs/repository, paths, commits or versions, hashes,
  licence texts and required notices under ADR 0002. Distinguish incorporated
  material, measurements/reference inputs and visual references only.
- [ ] For a CC0 base, record **CC0 base + MIT RowPlay modifications**. Preserve
  third-party terms and attribution; do not imply CC0 geometry became
  exclusively MIT-owned. Owner-authored reusable assets follow ADR 0002's
  ownership/provenance test; export/validation tooling remains GPL.

## Phase 5.4 — Implement the approved athlete and calibrate contact

These tasks remain gated on the owner decision above.

- [ ] Implement only the selected route. An artistically modeled human uses a
  reviewed `.blend` source plus deterministic export/validation tooling.
  Phase 2's procedural `shell.py` was a valid one-off engineering choice;
  future hero assets need not be authored as Python geometry.
- [ ] Calibrate final contact against the actual final hand mesh: palm
  dimensions, finger lengths, joint centres, grip-seat depth, flesh-radius
  assumptions and helper pivots must match that geometry. Do not blindly
  inherit V4 helper offsets or contact geometry for a refined/replacement mesh.
- [ ] Re-run the actual-skin measurements, phase captures and deformation
  checks for the final athlete, with numerical and Qt visual acceptance,
  deterministic export, triangle-budget checks, provenance and parity evidence.
  Publish the final athlete/contact reference for Phase 6.

Preserve authoritative Rust replay state, stroke timing, the 225-float frame
contract, replay-driven pose, grip/contact semantics and hand/oar and hand/pole
relationships unless separately decided. Phase 5 does not authorize a new
animation architecture. If the audit proves a skeleton/helper/contact contract
itself encodes the defect and prevents correct rendered contact, stop and
record an architectural decision with evidence/options; do not compensate in
another layer merely to preserve the contract.

Rust owns meaning and replay truth. Qt/QML owns the runtime presentation object
graph. Blender + Qt tooling owns authored/renderable assets.

Exit: owner decision implemented, corrected anchor frozen, final athlete and
contact system accepted with measured skin and Qt evidence, budget/provenance
recorded, and roadmap/source map/ADR/spec updated. Whole-system proportional
fit belongs to Phase 6; wake, foam, spray and water polish belong to Phase 7.
