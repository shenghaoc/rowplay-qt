# ADR 0017 — Athlete contact truth and hero fit

Status: accepted (2026-09-27, owner-directed Blender roadmap revision).

## Context

Blender Phases 1–4 are merged and complete. Direction C, its generated skies,
asset-source policy, environment and course dressing remain as delivered.
[ADR 0016](0016-authored-overcast-lighting.md) also recorded this consequence:

> The existing athlete is exempt from the proposed replacement-rower budget;
> the replacement-rower phase was dropped by the owner.

The owner now restores athlete work to the remaining Blender roadmap. The
seat/pelvis reference sits below the web's (tracked in #130), while strong
logical hand-target parity and digit closure do not establish correct visible
contact of the skinned hand with equipment. Judging the human asset against a
wrong anchor or an unexplained contact chain risks replacing the wrong layer
and fitting the boat twice.

Accepted ADRs are immutable; superseding decisions receive a new number and
link back. ADR 0016's earlier in-place amendments remain historical. They do
not create an exception for this new athlete-roadmap decision.

## Decision

1. **Supersede only ADR 0016's dropped-rower and athlete-budget-exemption
   consequence quoted above.** Restore the athlete to Blender Phase 5.
   Replacing or materially refining V4 is an intentional visual divergence
   from the web athlete, just as Direction C diverges from the web
   presentation. V4 remains the baseline until the audit is complete and the
   owner selects the route. Establish an explicit athlete triangle budget
   before substantial modeling; the old exemption does not carry forward.
2. **Phase 5.0 is the correction tracked in #130.** Correct and freeze the
   authoritative seat/pelvis reference before judging contact or the athlete.
   Do not defer it until between Phases 5 and 6.
3. **Audit contact truth before judging/replacing the human asset.** Follow
   equipment geometry → logical contact target → wrist/hand/digit bones and
   helpers → actual deformed/skinned hand mesh → visible Qt contact, across
   RowErg, SkiErg and BikeErg. Attribute each defect to its owning layer and
   correct it there; arbitrary QML offsets as a cosmetic fix are prohibited.
   Success requires geometrically and visibly correct rendered-skin contact,
   with gaps, penetration, palm seating, finger/thumb wrap and wrist alignment
   evaluated against equipment. Phase 5 establishes justified tolerances from
   hand scale, equipment radius, deformation and visual inspection, not from
   existing joint-target tolerances.
4. **Audit V4, experiment, then stop for the owner.** The
   [Phase 5 spec](../../.kiro/specs/blender-05-athlete-contact/tasks.md) retains
   the evidence chronology, required phase captures, geometry/proportion and
   rigging/deformation checklist, and material/normal-only Qt experiment.
   Recommend retain/presentation improvement, substantial Blender refinement,
   permissive-base replacement or owner-authored sculpt; assess skeleton,
   helpers, digit chains and skin-weight reuse separately. **Stop for owner
   decision before refinement/replacement implementation.** Implement only
   the approved route and calibrate contact against the actual final hand
   mesh, never by blindly inheriting V4 helper offsets.
5. **Phase 6 follows the chosen and implemented final athlete.** The
   [hero-fit spec](../../.kiro/specs/blender-06-hero-fit/tasks.md) integrates
   athlete, seat, rails, stretcher, cockpit, riggers, oars/poles and shell/
   machine. Require numerical fit and real single-scull photographic
   calibration at catch, finish and half-slide recovery across several body
   types, with source URLs/provenance. Collision tests alone are insufficient;
   re-verify Phase 5's rendered-skin contact. Correct athlete/contact/anchor
   issues, then re-evaluate the Phase 2 shell, then modify it only if a real
   mismatch remains. Preserve 7.8 m length, grip semantics and the oarlock/rig
   contract unless separately decided; reshape mid-body/cockpit sections
   rather than uniformly widening the hull or losing bow/stern slenderness.
6. **Phase 7 follows accepted hero fit.** The
   [water-polish outline](../../.kiro/specs/blender-07-water-polish/tasks.md)
   owns wake, foam, spray and final water interaction polish. It may consume
   authoritative replay motion, but may not invent independent speed/distance
   models, scroll the world or compensate for circular-course compression.
7. **Preserve contracts unless the audit proves the contract encodes the
   defect.** Preserve authoritative replay state, stroke timing, the 225-float
   frame, replay-driven pose, rig/contact semantics and hand/oar and hand/pole
   relationships. If skeleton semantics, helper placement, digit chains,
   contact geometry or another fixed contract makes correct rendered contact
   impossible, stop and present an architectural decision with evidence and
   options before changing that contract. Do not compensate in the wrong
   layer. This ADR does not authorize a wholesale new animation architecture.

The architectural boundary remains:

- Rust owns meaning and replay truth.
- Qt/QML owns the runtime presentation object graph.
- Blender + Qt tooling owns authored/renderable assets.

ADR 0016 continues to govern procedural versus modelled asset sources. An
artistically modeled human uses a reviewed `.blend` and deterministic export/
validation tooling; Phase 2's procedural `shell.py` remains a valid one-off
engineering choice, not a requirement to author future heroes as Python geometry.

[ADR 0002](0002-gpl-3-licence-and-asset-provenance.md) remains the authority
for licensing and provenance, unchanged here. This ADR selects no human base
or generator. Check current terms at the route decision; its prohibition on
downloaded human models/avatar-generator output requires an explicit
architecture/provenance decision before incorporation. Preserve the distinction
between CC0 base and MIT RowPlay modifications and audit accessories, hair,
clothing, textures, morphs and community assets separately, as detailed in the
Phase 5 spec. Candidate status alone grants no import approval.

## Consequences

The remaining sequence is corrected anchor → contact truth → athlete audit
and owner decision → approved athlete implementation → hero fit → water
polish. Do not fit the boat twice. Existing mathematical parity remains useful
but cannot substitute for actual-skin and Qt acceptance; corrected historical
SkiErg defects must not be represented as current measurements.

ADR 0016 retains its exact pre-PR history, including the now-superseded athlete
consequence. Its Direction C, generated-sky, asset-source, MIT asset/GPL tooling,
Phase 3 environment and Phase 4 dressing decisions are not superseded. ADR 0002
is unchanged. The roadmap, source map, Phase 5–7 specs and agent guide refer to
ADR 0017 for the new sequence, ADR 0016 for asset sources and ADR 0002 for
licensing/provenance. No general in-place amendment exception is introduced.

This documentation decision changes no asset or runtime behavior, begins none
of Phase 5's tasks and leaves Phases 1–4 complete with their historical evidence.
