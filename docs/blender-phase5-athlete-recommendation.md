# Blender Phase 5.3 — athlete route recommendation

**Status: recommendation complete; awaiting owner decision.** This report is
advisory under [ADR 0017](decisions/0017-athlete-contact-truth-and-hero-fit.md)
and the [Phase 5 spec](../.kiro/specs/blender-05-athlete-contact/tasks.md).
It selects nothing by itself: no route is implemented, no asset, QML, Rust,
skeleton, helper, weight, fixture or contract changes, no third-party material
is imported, and no ADR is created. ADR numbering is reserved until the owner
accepts a route. Phase 5.4 and Blender Phases 6–7 remain unstarted.

Synthesized from live `main` `cb26725589d14e5e6c84f6622f5761ca65b4a960`
(the Phase 5.2 merge). The evidence base is the landed
[Phase 5.0–5.1 contact audit](blender-phase5-contact-audit.md) and
[Phase 5.2 athlete audit](blender-phase5-athlete-audit.md), at web pin
`173c6facbcedef419ad39168c5e3e642abb7e57e` and Studio pin
`3d406a5b7677372de35fb0817c7133a2589c6564` (local reference checkouts verified
at those SHAs; no new reference material was consumed). No audit step was
re-run and no new capture corpus was produced: this phase weighs evidence
already on `main`.

**FACT** cites a landed observation; **MEASUREMENT** cites a landed recorded
number and its instrument; **INFERENCE** is this report's interpretation;
**RECOMMENDATION** is advisory; **OWNER DECISION** marks what only the owner
may decide. Where a claim below is a route *prediction* (what a route would
achieve), it is an inference from the measured defect attribution, not a
measurement of an asset that does not exist yet.

## Evidence carried from Phase 5.1 (contact truth)

Carried as established, not re-derived. Sources: the
[per-phase skin table and layer verdicts](blender-phase5-contact-audit.md#per-phase-actual-skin-results)
and [skin-versus-helper measurements](blender-phase5-contact-audit.md#skin-versus-helpers).

- **FACT:** logical target/helper parity is insufficient to prove rendered
  contact. Target parity at 1e−9 and green closure counts coexist with
  measured skin gaps and gross intersection.
- **MEASUREMENT:** RowErg reports 10/10 helper closure while the nominal grip
  envelope penetrates the palm skin by 21.3–22.9 mm across the final phases
  and the catch thumb stays 8.7 mm clear (threshold for a clear float: 3 mm).
- **MEASUREMENT:** SkiErg remains 8/10 helper closure (pinky helpers
  +6.210/+6.155 mm). Its pinky *skin* reaches the grip at loaded pull yet gaps
  about 7.8 mm at release: helper shortfall and visible-surface shortfall are
  demonstrably different quantities, phase by phase.
- **FACT:** BikeErg is the aligned, low-residual control (anchor parity,
  10/10, all regions reach the hood) — useful to isolate sport-specific
  failures, not proof the same hand is calibrated.
- **MEASUREMENT:** helper geometry is not calibrated to the rendered skin:
  `HAND_PALM_CONTACT` sits 18.37–18.43 mm from the nearest *entire* skinned
  surface at RowErg finish (30.62–30.73 mm from the palm region), and
  estimated digit tips sit 6.1–8.5 mm from their own digit's skin against a
  0.9 mm pad allowance.
- **FACT:** wrist comfort relief rotates a closure solved around one cylinder
  away from the equipment axis (row 32.9/20.5/≈0/21.2°; ski
  21.0/25.9/≈0/35.2/≈0°); how relief should preserve the shaft channel is an
  open question deliberately deferred.
- **FACT:** the remaining contact-calibration decisions depend on the final
  athlete; the audit's
  [open questions](blender-phase5-contact-audit.md#open-contact-calibration-questions)
  must not be closed around an asset that may be replaced. The corrected
  equipment/wrist-frame/pole-fit/ghost defects are fixed in the pipeline and
  stay fixed regardless of route.

## Evidence carried from Phase 5.2 (V4 athlete audit)

Sources: the audit's
[lineage](blender-phase5-athlete-audit.md#asset-identity-and-three-stage-lineage),
[proportions](blender-phase5-athlete-audit.md#geometry-and-proportions),
[hands](blender-phase5-athlete-audit.md#hands-anatomy-helpers-and-deformed-skin),
[topology](blender-phase5-athlete-audit.md#topology-and-allocation),
[rig/weights](blender-phase5-athlete-audit.md#rig-and-weights),
[deformation](blender-phase5-athlete-audit.md#replay-deformation) and
[experiment](blender-phase5-athlete-audit.md#controlled-native-materialnormal-experiment)
sections and its
[explicit answers](blender-phase5-athlete-audit.md#explicit-answers-and-phase-53-decision-inputs).

- **FACT:** the upstream CC0 Blender Human Base Meshes body (v1.4.1) is *not*
  established as the primary cause of the mannequin appearance. The reviewed
  base has coherent facial and hand surface anatomy; body polygon
  connectivity survives adaptation exactly.
- **MEASUREMENT:** the RowPlay adaptation distorts anatomy before any replay
  pose: the forearm segment maps at 1.454× the scaled source while
  wrist-to-contact maps at 0.628× (forearm chain 242.5 → 374.9 mm; hand
  major extent 192.7 → 148.8 mm); 45.69 % of the source upper-arm surface
  carries a principal stretch above 2×; feet narrow 99.1 → 73.8 mm.
- **FACT:** all 32 hand helpers are derived heuristically from sculpt-region
  means with fixed constants, not independently fitted joint centres; tip
  estimates overshoot the digit's own skin by 3.6–8.4 mm.
- **MEASUREMENT:** every production influence is RowPlay-generated
  (the base snapshot has no weights); 71.6 % of vertices have exactly one
  nonzero influence; weights are deterministic region painting with hard
  transitions (3,144 edges jump by more than L1 = 1); the palm interior is
  100 % Hand, so there is no distributed cupping.
- **MEASUREMENT:** major deformation defects persist across phases: ski
  recovery compresses 26.42 % of shoulder rest area below half, ski release
  14.78 % of upper arm; elbows/wrists crease; hips/knees fold at deep
  flexion. These survive every material variant.
- **MEASUREMENT:** topology is misallocated relative to deformation
  difficulty: head 37,802 and feet 19,616 of 106,256 triangles, against
  shoulders 1,208 and pelvis 2,260 — head density 222,881 tri/m² versus
  shoulder 12,329 tri/m². The hair cap alone is 12,080 triangles and reads
  rigid at replay scale.
- **FACT:** production Qt renders one vertex-coloured PrincipledMaterial with
  no maps; the web's richer role/map treatment is webRuntime and absent. The
  V4 UV layout blocks a faithful per-role port (12,080 zero-UV-area hair
  triangles, overlapping eye/face-detail UVs recovering only 0.9 %/3.3 % of
  intended texels by count in the controlled experiment).
- **MEASUREMENT:** material/normal-only changes improve presentation locally
  (best-variant C reduces broad fabric highlights; masked A–C mean absolute
  RGB difference 18.89 on the row-catch full view) but leave the proportion,
  silhouette, helper-registration and deformation findings intact. What
  presentation cannot fix, per the landed audit: segment proportions, cap
  outline, helper registration, terminal reach, palm crossing, gaps and
  surface collapse.

These findings are carried at full strength. This report does not soften them
into "the mannequin could be improved".

## Route definitions

What each route would concretely mean in this repository. A structural
**FACT** that shapes all four: rowplay-qt vendors only the built artifact
`assets/replay/rowplay-athlete-v4.glb`; the V4 builder, its reviewed base
snapshot (`static/replay-assets/source/rowplay-human-base-male-v1.4.1.blend`,
extracted by `scripts/extract-replay-athlete-base-blender.py`) and the rig
generator live in the **web repository** at the pin. This repository holds no
athlete `.blend` source today, and
[ADR 0016](decisions/0016-authored-overcast-lighting.md) requires a modelled
asset to carry a reviewed `.blend` source plus deterministic, byte-reproducible
export.

### Route 1 — Retain V4 geometry and improve presentation

Keep the current adapted mesh, proportions and topology unchanged. Later work
would be confined to: Qt material/role treatment (bounded by the measured UV
limits above), possibly re-authored weights on the same mesh, and helper/
contact recalibration against the existing hands. Honest scope note: because
the geometry is fixed, "improve presentation" **cannot** include restoring
arm/hand proportions or reallocating topology — those would make it Route 2.

### Route 2 — Substantially refine V4 in Blender

Edit V4's geometry in Blender: restore arm/forearm/hand/feet proportions
toward the reviewed source, rework problem topology, relocate joints and
helpers, repaint weights, repair UVs, and keep some V4 identity (added hair
cap, facial details, shoes, vertex-colour styling). Since no `.blend` exists
here, step one is importing the vendored GLB (or re-running the web builder)
to create one — a fork of the artifact away from its canonical generator.

**INFERENCE — how much V4 would survive:** the body connectivity is the CC0
base's own (preserved exactly), so what is uniquely V4 is precisely the
adaptation deformation field, the added parts, the weights and the helpers —
which is the defect list. Undoing the 1.454×/0.628× arm/hand mapping, the
45.7 % upper-arm stretch, the seat-channel bias and the narrowed feet on the
*adapted* surface is re-deriving the base regions the long way round, on a
mesh whose only in-repo form would be a GLB re-import that has already lost
the source's modifier history. What survives is the hair cap (which the
budget should discard), the painted styling (a measured primary cause of the
mannequin look) and the shoe/face detail parts. Honestly classified, Route 2
is a rebuild wearing the word "refine": nearly all of its real work is
Route 3A's work, plus the extra work of undoing V4 first.

### Route 3 — Replace V4 using a permissively licensed base

**3A — clean rebuild from the existing reviewed CC0 base.** A new RowPlay
athlete authored in this repository as a reviewed `.blend`, starting from the
already reviewed Blender Human Base Meshes v1.4.1 snapshot — the same
anatomical source the shipped athlete already embodies. This is still a
**replacement** of the V4 adaptation, even though it reuses V4's upstream
anatomy; the two must not be conflated. Conceptually:

```text
reviewed CC0 Human Base Meshes v1.4.1 snapshot
        ↓  preserve source anatomy and loop topology; no distorting retarget
new RowPlay athlete .blend in this repository (ADR 0016 modelled route)
        ↓  pose/fit to the frozen Phase 5.0 seat/pelvis reference
19 semantic bones fitted to actual anatomical joint centres
        ↓
32 helpers authored from actual joints and the actual hand skin
        ↓
smooth anatomical weights around fitted joints (within the 4-influence GLB)
        ↓
budgeted retopology/decimation per the triangle budget below
        ↓
clean UVs and authored role separation; in-repo hair/clothing/shoes
        ↓
deterministic GLB export + machine validation (budget, weights, registration)
        ↓
contact recalibration against the final mesh, then Qt native acceptance
```

**3B — a different permissively licensed human source** (MakeHuman/MPFB or
similar). Kept distinct because its governance and diligence differ
materially from 3A (see licensing below). No candidate is selected or
surveyed here.

### Route 4 — Owner-authored sculpt

The owner supplies the sculpt; the agent handles rig, helpers, weights,
export, validation and contact calibration. What the owner would actually
need to supply, at minimum: the body/hands/head surface (either a complete
sculpt or corrections over a base), plus art direction and acceptance at
each milestone. The pipeline, contracts, budget, validation and acceptance
instruments are identical to 3A; only mesh authorship differs. Its real cost
is owner time on the critical path, and that dependency is unbounded from
the repository's side.

## Reuse analysis

Independent of route, and answering ADR 0017's requirement to assess
skeleton, helpers, digit chains and skin weights separately. "Semantic
contract" and "current concrete values" are different claims throughout.

| Element | Reuse decision | Evidence basis |
| --- | --- | --- |
| Authoritative Rust replay state, stroke timing, motion-graph semantics, 225-float frame, sport phase definitions, equipment truth, contact-target ownership | **Reusable, unchanged.** No audit finding implicates them; ADR 0017 ¶7 mandates preserving them. The Phase 5.0/5.1 pipeline corrections (carriage/pelvis, wrist-frame handoff, pole leaf fit, ghost yaw) live here and carry forward under every route. | Contact audit, ADR 0017 |
| 19-bone semantic skeleton **contract** (names, hierarchy, roles) | **Reusable.** Hips→Spine→Chest→Neck→Head plus clavicle/upper-arm/forearm/hand and thigh/shin/foot chains express the replay pose adequately; no defect was attributed to the semantic set itself. | Athlete audit rig section |
| Current bone **transforms** (origins, lengths, inverse binds) | **Not reusable.** They come from fixed source-landmark retargeting, not fitted joint centres, and encode the distortion (upper arm/forearm 390/375 mm versus base 369/242 mm). Re-fit to the final anatomy. | Athlete audit FACT/MEASUREMENT |
| 32 hand helpers — **semantics** (cup + five three-joint digit chains, naming, hierarchy) | **Reusable as a contract.** The role set is sound and the solver consumes it coherently. | Both audits |
| 32 hand helpers — **geometry and closure parameters** (pivots, lengths, `.92 ×` tip estimate, 16 mm closure radius, digit limits, ±4 mm band) | **Not reusable.** Pivots are sculpt-region heuristics 0.5–7.1 mm off boundary centres; tip estimates overshoot the skin by 3.6–8.4 mm; the palm point is 18–31 mm from skin. Re-author from the final mesh's actual joints and skin, per ADR 0017 ¶4 (never blindly inherit V4 helper offsets). | Both audits |
| Digit chains | Same split: three-joint chain **concept reusable**; current lengths/pivots/limits re-derived against the final hand. | Both audits |
| Skin-weight **architecture** (deterministic scripted generation, ≤4 influences, normalization, machine validation) | **Reusable as an approach.** Normalization is numerically clean (max error 4.47e−8) and deterministic generation is what future agents can regenerate and check. | Athlete audit |
| Skin-weight **method and current values** (region-threshold painting; the shipped weights) | **Not reusable.** 71.6 % single-influence, hard transitions, rigid palm, no cupping/twist distribution; anatomically inadequate for the measured shoulder/elbow/palm collapse. Rebuild as smooth joint-centred weights. | Athlete audit |
| Animation clips | **Not applicable — there are none to retarget.** Motion is replay-driven Rust pose synthesis (no authored keyframe clips drive the athlete). The choreography lives in replay semantics, already reusable above; "retargeting" reduces to the pose path consuming the new rig's fitted rest data during Phase 5.4 calibration. | Repo architecture; contact audit clock/pose description |
| Contact solver | **Structure reusable; calibration not.** Composed grip-target parity, hand-local closure and the wrist-relief refinements are sound machinery; palm/pad/tip calibration must be re-derived from the final mesh, and the open wrist-channel/compression questions stay open until then. Deeper architectural change only if final-mesh evidence forces it — decided then, per ADR 0017 ¶7. | Contact audit open questions |
| Phase 5 instruments and references (actual-skin instrument with silhouette validation, native phase capture tooling, stretch/weight/topology metrics, frozen seat/pelvis reference, 128-sample pelvis oracle, equipment parity fixtures, runtime-gate closure checks) | **Reusable — they are the acceptance harness for every route.** Replacing the athlete does not replace the program: most of Phase 5's value carries forward as the instrument that will accept the final athlete. | Both audits; `tools/blender/` |

## Route comparison against the measured defect classes

Classifications: **solves directly** / **likely solves** (mechanism removed,
result still needs acceptance) / **requires substantial rework** /
**does not solve** / **adds risk** / **unresolved**. Route predictions are
INFERENCE from the defect attribution; no numeric scores are invented.

| Defect class / concern | Route 1 retain | Route 2 refine V4 | Route 3A rebuild from reviewed base | Route 4 owner sculpt |
| --- | --- | --- | --- | --- |
| Body proportions | does not solve | requires substantial rework (undo adaptation) | solves directly (distorting retarget never applied) | solves directly (owner-dependent) |
| Upper-arm distortion (45.7 % area >2×) | does not solve | requires substantial rework | solves directly | solves directly |
| Forearm/hand ratio (1.454× / 0.628×) | does not solve | requires substantial rework | solves directly | solves directly |
| Hand anatomy / contact suitability | does not solve | requires substantial rework | likely solves (base hands coherent; authored to grip, accepted by the 5.1 instrument) | likely solves |
| Joint/helper registration | requires substantial rework, against distorted anatomy | requires substantial rework | solves directly (fitted centres) | solves directly (agent-authored rig) |
| Skin weighting | requires substantial rework; ceiling set by distorted rest mesh | requires substantial rework | likely solves (rebuilt around fitted joints; linear-blend limits remain) | likely solves |
| Shoulder deformation | does not solve (rest-surface distortion is a primary contributor) | requires substantial rework | likely solves | likely solves |
| Elbow deformation | unresolved (weights-only ceiling unknown) | requires substantial rework | likely solves | likely solves |
| Wrist/hand deformation | does not solve | requires substantial rework | likely solves | likely solves |
| Hip/knee deformation | unresolved (seat-channel bias persists) | requires substantial rework | likely solves | likely solves |
| Topology allocation | does not solve (geometry frozen) | requires substantial rework | solves directly (budgeted retopo) | likely solves (depends on delivery) |
| UV / material-role limits | does not solve (zero-area hair UVs, overlapping eye/face UVs) | requires substantial rework | solves directly (authored clean) | solves directly |
| Qt presentation quality | limited local gain (measured) | likely solves | likely solves (role separation authored in) | likely solves |
| Contact recalibration burden | required regardless; against distorted hands and heuristic helpers | required regardless | required regardless; cleanest target (calibration authored with the mesh) | required regardless |
| Reuse of replay semantics | solves directly (untouched) | solves directly | solves directly (preserved by design) | solves directly |
| Implementation complexity | lowest, but buys the least | highest honest cost: ≈3A plus undoing V4 first | high but bounded (reviewed base + Phases 2–4 pipeline precedent + existing instruments) | agent cost ≈3A minus sculpt, plus unbounded owner time |
| Deterministic asset pipeline | adds risk: no in-repo source; edits fork the vendored GLB or happen upstream | adds risk: must create a source anyway, from a lossy import | solves directly (ADR 0016 modelled route, byte-reproducible export as Phases 3/4) | solves directly (same pipeline; sculpt committed as `.blend`) |
| Licensing/provenance risk | none new | none new (if no new inputs) | low: base already reviewed, recorded and shipped; needs the narrow provenance decision below | lowest (owner-authored), provenance still recorded |
| Future AI-agent maintainability | adds risk: defect layers stay upstream/opaque | adds risk: hand-edited fork | solves directly: reviewed `.blend`, explicit contracts, machine-verifiable budget/weights/registration/contact checks | mostly solves; sculpt iterations are the one manual step |
| Phase 6 hero-fit risk | high: fits the boat to stylized proportions; if the athlete changes later, the boat is fitted twice | medium–high: moving target while V4 is progressively reshaped | low: single stable, anatomically proportioned target authored to the frozen reference | medium: schedule risk; a late sculpt change moves the target |

## Sunk-cost traps

Which V4 pieces would cost more to repair correctly than to rebuild:

1. **The adapted arm/hand geometry.** Repair means inverting a known
   deformation (1.454× forearm, 0.628× hand, 45.7 % upper-arm stretch) on a
   surface whose canonical source and modifier history live in another
   repository. The inversion target *is the reviewed base*, which is
   available directly.
2. **The heuristic helper package.** Re-deriving 32 pivots, lengths and tip
   estimates against the current distorted hands still calibrates contact to
   hands the evidence says are the wrong shape for it.
3. **The region-threshold weights.** Repainting on the distorted rest mesh
   inherits its creases; the audit could not bound how much weights alone can
   recover (explicitly "unresolved"), so this is open-ended work against a
   ceiling nobody has measured.
4. **The topology allocation.** 62.6 % of all triangles sit in head, neck and
   feet; moving density into shoulders/elbows/hips/knees on the existing mesh
   is a retopology — already rebuild-class work.
5. **The UV layout and single-material path.** The measured layout cannot
   carry the web's role treatment (hair unusable, eyes/face overlapping);
   role-capable UVs on the same mesh are again rebuild-class work.

**INFERENCE:** retaining any of these contracts sets off the compensation
chain — retain contract → mesh compensation → weight compensation → helper
compensation → contact compensation — with each retained layer forcing rework
of every layer above it. Per the task's own criterion, that chain is
architectural debt, not reuse value. The V4 pieces genuinely worth carrying
are the semantic contracts and the instruments (reuse table above), and those
carry forward under every route.

## Licensing and provenance implications

**FACT (upstream, verified 2026-09-27):** blender.org's official demo-files
page lists the Human Base Meshes asset bundle at **version 1.4.1, licence
CC0**, credited to Blender Studio and community contributions, and
`download.blender.org/demo/asset-bundles/human-base-meshes/` serves
`human-base-meshes-bundle-v1.4.1.zip` (2026-01-20) as the current release —
the same version the repository's records pin.
**FACT (repository records):** [ASSET_PROVENANCE.md](../ASSET_PROVENANCE.md)
records the shipped athlete as MIT (rowplay) with the Dan Ulrich / Blender
Studio Human Base Meshes v1.4.1 base under CC0-1.0
(`LICENSES/CC0-1.0.txt`), and the Phase 5.2 lineage table records the
reviewed extraction path and the official bundle URL. The anatomy 3A would
rebuild from is therefore already reviewed, already recorded and already
embodied in the shipped artifact.

Governance per route, against
[ADR 0002](decisions/0002-gpl-3-licence-and-asset-provenance.md)'s rule that
no downloaded human model or generator output enters the repository, and
ADR 0017's note that incorporation requires an explicit
architecture/provenance decision first:

| Route | ADR 0002 path |
| --- | --- |
| 1 | No ADR change. Nothing new enters. |
| 2 | No ADR change for the mesh itself (it is already vendored), but the forked source's provenance entry must record the GLB re-import lineage. |
| 3A | **Narrow provenance decision required before incorporation** (the post-acceptance ADR): authorize the one already reviewed CC0 Human Base Meshes v1.4.1 snapshot as the athlete's modelled-source input, vendored from the owner's rowplay repository with full provenance, recorded as **CC0 base + MIT RowPlay modifications**. This is a scoped carve-out with direct precedent — ADR 0016 superseded ADR 0004's HDRI prohibition narrowly, for reviewed script-generated skies only — not a general opening for downloaded humans or generators. |
| 3B | Requires the same amendment **plus** a fresh terms audit of the new source at decision time: code licence versus output/base-asset licence separately (for MakeHuman/MPFB-class tools these differ), whether generated output carries restrictions, and per-item terms for any bundled clothing/hair/morphs/community assets, which must never be inferred from the base or generator. Not performed here: 3A is sufficient for the decision, so per the spec this report does not expand into a marketplace survey. |
| 4 | No ADR change for an owner-authored sculpt (ADR 0002's ownership/provenance test applies directly; MIT under the common-asset rule if provenance is clean). If the owner sculpts *over* the CC0 base, 3A's recording applies too. |

In every route: hair, clothing, textures, accessories and morphs are authored
in-repository (no third-party accessory assets), export/validation tooling
stays GPL-3.0-or-later, and the asset itself is MIT under ADR 0002's
common-asset test with the CC0 base recorded. **Nothing is imported by
Phase 5.3, and ADR 0002 is not amended by it.** The spec's licensing-
checkpoint recording obligations are import-time obligations and remain open
for Phase 5.4.

## Triangle budget recommendation

Required by ADR 0017 before substantial modelling. The historical 40k
proposal is not resurrected: it is not an active budget
([tooling README](../tools/blender/README.md)), and the measured allocation
shows why it was never evidence-based — V4's body *excluding* head, neck and
feet already consumes 39,788 triangles (106,256 − 37,802 − 9,050 − 19,616)
while under-allocating every difficult joint. A 40k complete athlete would
force the deforming regions below an allocation the audit already found
insufficient.

**MEASUREMENT (baseline, from the landed audit):** V4 exports 106,256
triangles / 57,069 vertices: continuous body 84,680, hair cap 12,080, eyes
2×1,088, facial/footwear details 7,320; head+neck+feet take 66,468 triangles
(62.6 %) while shoulders get 1,208 and pelvis 2,260; density spans 222,881
tri/m² (head) to 8,845 tri/m² (thighs).

**RECOMMENDATION:**

| Budget quantity | Value |
| --- | --- |
| Target | **60,000 exported triangles** |
| Hard ceiling | **75,000 exported triangles** |

**Counting scope (exact, so the number cannot drift):** the budget counts
triangles in the **exported runtime GLB** — after modifiers and export
triangulation — for the **complete athlete**: continuous body, head/face,
eyes, hair, clothing/footwear and every separate detail part in the athlete
asset. That is the same basis as V4's measured 106,256 and as the Phase 2–4
budgets, and it is what the export tooling and asset tests recount
deterministically from the artifact. Attribute-seam duplication affects
exported *vertices*, not triangles (V4: 3,750 duplicated vertices); a
quad-authored source therefore reads as roughly half the budget in faces.
Vertex count is recorded as information, not bounded.

**Regional allocation guidance** (targets; exact per-part budgets are pinned
in the Phase 5.4 manifest once the part list exists, and the export refuses a
part over budget, as Phases 3/4 do):

| Region | V4 measured | Guidance | Rationale |
| --- | ---: | ---: | --- |
| Hands (both; wrist crease to fingertips) | 11,020 | **12,000** | Mechanically the most important surface in the replay; contact validation close-ups magnify them far beyond screen share. Holds V4's density (~172k → ~188k tri/m²) while redistributing into knuckle/webbing loops and a palm that can cup. |
| Arms (deltoid/shoulder through forearms, elbow and wrist loops) | 9,200 | **12,000** | Shoulders roughly triple (1,208 → ≈3,500; ~12.3k → ~36k tri/m²) and elbows gain dedicated loops — the two most consistent measured collapse sites. |
| Torso (chest, waist, pelvis, hip creases, armpits) | 9,668 | **11,000** | Pelvis roughly doubles (2,260 → ≈4,000) for the seated hip fold; armpit/clavicle transitions gain loops. |
| Legs (thighs, knees, shins, ankle blend) | 9,900 | **11,000** | Knee bands gain loops for deep flexion; thighs keep the seat-relationship surface. |
| Feet and footwear | 19,616 | **5,000** | Feet are near-rigid in replay and mostly occluded at stretcher/pedals; V4's allocation is the single largest recoverable block. |
| Head/face and neck (excl. hair, eyes) | ≈32,600 | **7,000** | Visible but not a cinematic portrait; drops from ~223k to ~30k tri/m², still comfortably above torso density so the face reads at chase and close-up scale. |
| Eyes | 2,176 | **600** | Two simple eyeballs suffice at replay scale. |
| Hair | 12,080 | **1,400** | A shaped low-poly hair volume replaces the 12k rigid cap the audit flagged; a rigid cap does not justify disproportionate geometry. |
| **Total** | **106,256** | **60,000** | −43.5 % versus V4 at target; the deforming core (hands+arms+torso+legs) *rises* from 39,788 to 46,000 while head/hair/eyes/feet fall from 66,468 to 14,000. |

The ceiling (75,000, −29.4 % versus V4) is headroom for deformation-driven
density where Phase 5.4's native acceptance demands it — not a second
target. Guidance floor: the deforming core must not fall below its 46,000
share if the total shrinks. Validation is machine-checked as in Phases 3/4:
the manifest records per-part counts and SHA-256, asset tests recount from
the GLB and pin every budget, and the deterministic export refuses a part
over budget. One geometry is budgeted; no per-tier athlete LOD is introduced
by this recommendation (tiers continue to select lighting/effects and
instance counts elsewhere; a future LOD would be its own decision).

**OWNER DECISION:** the target and ceiling are approved, adjusted or rejected
together with the route.

## Phase 6 (hero fit) implications

Fixed inputs regardless of route: the 7.8 m shell contract, the frozen
Phase 5.0 carriage/pelvis reference, the anchor and oarlock/rig contracts,
and the deferred seat-pad/pelvis 28 mm relationship. The athlete is authored
**to** the frozen reference and contracts, never the reverse.

- **Route 1:** Phase 6 calibrates cockpit clearance, rigger span and
  photographic proportion checks against a 1.895 m athlete with 480 mm
  bone-origin shoulder breadth, arms of nearly equal segment length and
  stylized hands. If the owner later rejects that look, the boat is fitted
  twice — the exact failure ADR 0017 exists to prevent. High risk.
- **Route 2:** the athlete changes shape while Phase 6 would want a stable
  target; sequencing forces either a long wait or refitting. Medium–high.
- **Route 3A:** one stable, anatomically proportioned target, seated on the
  frozen reference with a real glute/pelvis surface for the deferred seat-pad
  question, and hands the contact system is calibrated against once. Lowest
  risk. **INFERENCE — expected dimensional shifts Phase 6 should plan for:**
  stature held near V4's (≈1.87–1.90 m, exact value fixed in the 5.4 spec
  against the frozen reference so perceived scale against the 7.8 m shell
  does not shift), while restoring base segment ratios at that stature —
  shoulders narrow by roughly 40–90 mm, forearms shorten by roughly 100 mm,
  hands lengthen by roughly 50–60 mm, feet widen. Rigger span, stretcher and
  cockpit clearances must be checked against those magnitudes, not against
  V4's.
- **Route 4:** same geometry benefits as 3A once delivered, but the delivery
  date and iteration count sit outside the repository; a late sculpt revision
  after fit work starts would also fit the boat twice. Medium.

No shell, cockpit, camera or anchor work is proposed here; Phase 6 owns fit.

## Maintainability for an AI-maintained repository

The repository's proven pattern (Phases 2–4, and Phase 5's instruments) is:
reviewed committed source → deterministic, byte-reproducible export →
machine-verifiable budgets/contracts → native Qt gate acceptance. Route 3A
lands the athlete inside that pattern: a reviewed `.blend` in-repo, explicit
object/bone/helper contracts, budget and weight checks a future agent can
re-run, the 5.1 actual-skin instrument as contact acceptance, and repeatable
native gate evidence. Routes 1 and 2 leave the athlete the one hero asset
whose source of truth is respectively upstream or a hand-edited fork.
Route 4's sculpt is the one step agents cannot regenerate, mitigated by
committing the sculpt as the reviewed source and keeping everything
downstream deterministic. Per ADR 0016, an artistically modelled human uses
the `.blend` route — "AI-maintainable" does not mean procedurally generating
an organic human, and this report does not propose that.

## Recommendation

**RECOMMENDATION — Route 3A: replace the V4 adaptation by rebuilding the
athlete from the already reviewed CC0 Human Base Meshes v1.4.1 snapshot, as a
reviewed `.blend` in this repository, to the budget above.** This is
replacement, stated as such; it is not "refining V4", because the current
adapted geometry is not preserved.

Why the evidence leads here:

1. Phase 5.2 established that the reviewed base is not the proven primary
   problem and that the strongest measured distortions (arms, hands, feet),
   all weights, all helpers and the painted styling enter in the RowPlay
   adaptation. Removing the adaptation removes the mechanism behind the
   dominant defect classes in one step; every other route fights that
   mechanism layer by layer.
2. Phase 5.1 established that contact must be recalibrated against the final
   hand mesh under any route. Calibrating once, against hands authored for
   the purpose, is the only option that does not pay the calibration twice
   or calibrate against hands the evidence says are wrong.
3. The honest cost comparison collapses Route 2 into Route 3A plus undo
   work, and Route 1 buys only the measured "limited and local" presentation
   gain while leaving proportions, deformation, contact and Phase 6 risk in
   place.
4. Route 3A is the only route that ends with the athlete inside the
   repository's deterministic, machine-checkable asset pattern without an
   unbounded owner dependency.

**Fallback — Route 4,** on the same pipeline: if the owner judges the
agent-authored 3A result artistically insufficient at an acceptance
checkpoint (or prefers artistic control from the start), the owner supplies
or corrects the sculpt and the agent keeps rig, helpers, weights, export,
validation and contact calibration. 3A and 4 share every contract, budget
and instrument, so switching between them wastes little work.

**Rejected:**

- **Route 1** — the audits' measured conclusion is that presentation cannot
  fix segment proportions, silhouette, helper registration, terminal reach,
  palm crossing, gaps or surface collapse; the rendered-contact failures and
  deformation defects would be accepted permanently, and Phase 6 would fit
  the boat to them. Rejected as the primary route; its presentation ideas
  (role separation, material response) are subsumed into the rebuild's
  surface authoring.
- **Route 2** — dishonest under its own name: the surviving-geometry
  analysis shows the work is a rebuild wearing "refine", executed on a
  forked artifact with no in-repo source and the base's history lost.
  Strictly more expensive than 3A for the same destination.
- **Route 3B** — no evidence that a new third-party source beats the
  already reviewed, already shipped CC0 base (whose anatomy 5.2 found
  coherent), while adding a fresh licence/diligence surface (code versus
  output terms, bundled-asset terms) and a wider ADR 0002 opening.
  Reconsider only if 3A's base proves anatomically insufficient during
  authoring, which nothing measured suggests.

**Wording note (task-required semantic distinction):** the recommended
implementation begins from the reviewed CC0 base and creates a new athlete;
throughout this report that is called **replacement/rebuild**. No
"refinement of V4" is recommended, and no substantial current V4 geometry is
proposed for preservation beyond the semantic contracts listed in the reuse
table.

## Owner decision required

**OWNER DECISION — the following are the owner's to make; Phase 5.4 stays
gated until they are returned:**

1. **Route.** Accept Route 3A (rebuild from the reviewed CC0 Human Base
   Meshes v1.4.1 snapshot), or select Route 1, 2, 3B or 4 instead. The
   recommendation and its fallback (Route 4) are advisory.
2. **Triangle budget.** Approve or adjust the 60,000-triangle target and
   75,000-triangle hard ceiling with the stated complete-athlete,
   exported-GLB counting scope and regional guidance.
3. **Governance to draft after acceptance (not now).** If 3A (or 3B/4-over-
   base) is selected: authorize drafting the narrow provenance ADR that
   permits incorporating the reviewed CC0 base snapshot as the athlete's
   modelled source (CC0 base + MIT RowPlay modifications), before anything
   is imported. If 3B: additionally authorize the source-specific terms
   audit. No ADR is created by Phase 5.3.

Until the decision is returned: no Phase 5.4 implementation, no asset
authoring, no replacement or refinement work, no contact recalibration, no
ADR 0002 amendment, no third-party import, and no Phase 6 work begins.
V4 remains the production baseline, unchanged.

## Validation

Docs-only synthesis; no production code, asset, fixture or test changed.
`cargo fmt --all -- --check`, `git diff --check` and a relative-link check
over this document pass locally; the associated PR's normal CI carries the
authoritative result. No AI/code-bot review was requested or retriggered.
