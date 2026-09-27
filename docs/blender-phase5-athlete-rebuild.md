# Blender Phase 5.4 — replacement athlete

Status: Layer B implementation under native acceptance. Phase 5.4 is **not
complete**. Layer C has not started. ADR 0018's approved Route 3A is used.

## Authoritative source

`assets/replay/authored/rowplay-athlete-v5.blend` is the modelled source.
`sources/rowplay-human-base-male-v1.4.1.blend` preserves the exact reviewed
CC0 snapshot. ASSET_PROVENANCE.md and MANIFEST.json pin both; the anatomical
base remains CC0, with MIT RowPlay modifications. Export/validation tooling
is GPL-3.0-or-later. No human regeneration script is retained.

Bounded one-off Blender editing was explicitly authorized by the owner.
The source anatomy received a uniform scale of 1.1064884850315604, to 1.87 m
before hair. The modelling operations reduced head/feet density, kept hand
loops, cut deliberate jersey/shorts/cuff regions, shaped a short integrated
scalp, and fitted closed shoes from the base's foot dimensions. Eye iris
colour is albedo; no vertex lighting or downloaded maps are used.

The saved mesh has 48,632 triangles against the 60,000 target and 75,000
hard ceiling. The initial allocation is hands 10,992; arms 9,608; torso
7,236; legs 11,392; feet/shoes 1,058; head/neck 6,984; eyes 596; hair 766.
Full height is 1.875918525 m including hair. Shoulder joint breadth is
0.440382421 m. The contract records every fitted joint centre and bone
length in metres. Final native anatomical/deformation measurements follow
in the acceptance evidence; these counts alone do not establish quality.

New anatomical bone centres, helper pivots and heat-solved skin weights are
stored in the .blend. Four normalized influences at most are exported.
The source distribution is 7,029 vertices with one influence, 8,921 with
two, 4,156 with three, and 4,220 with four (28.895% single-influence).
It is a diagnostic, not a quality target. All 19 semantic and 32 helper
names/hierarchies survive. Final palm/flesh/closure calibration is pending.

## Export and runtime

`tools/blender/export_athlete.py` validates the saved source and exports
eight material primitives in Blender 5.2.2. It creates no artistic
geometry. The repeat export is byte-identical for both GLB and contract.
The source SHA is required explicitly, and an incorrect SHA fails before
the source is opened. Ordinary Cargo builds need only committed outputs.

The 19 semantic animation channels retain the immutable V4 key times and
motion graph, expressed in the fitted bones' bases. V4's node TRS contains
an initial animated pose; its actual semantic inverse-bind rotations are
identity. The exporter verifies that premise. The Rust test compares the
animated world skin rotation deltas against those actual binds, including
between keys. Using V4's node TRS as its bind pose is an instrument error.

Qt Balsam converts only V5. Build-time static material properties bind the
eight explicit roles to the existing Theme colours while preserving the
source's PBR values and albedo. No runtime discovery or new frame crossing
is introduced. V4 bytes remain immutable audit inputs and are neither
converted nor packaged. Historical schema/type identifiers remain for
compatibility; the source/output asset identity is explicitly V5.

## Native evidence in progress

`capture_athlete_v5.py` reuses the Phase 5.1 transform recorder and Phase
5.2 material inventory without changing either historical instrument.
It requires committed inputs, verifies instrument ancestry, selects only
the current Cargo invocation's exact executable/OUT_DIR, pins their bytes,
and verifies all eight actual Qt materials. Captures cover all established
Row/Ski/Bike phases, two schemes and Medium/High tiers.

Initial Qt loading succeeded. The existing quick gate rejects the new
RowErg helper count (8/10 against the legacy 10/10 assertion). This is an
open calibration result, not a reason to change a threshold or claim skin
contact. Native geometry/material acceptance must precede Layer C.

The corrected seat/pelvis baseline and every equipment asset/transform
remain fixed. Phase 6 fit and Phase 7 water work have not started.
