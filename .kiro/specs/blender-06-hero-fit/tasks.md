# Blender Phase 6 — hero-fit integration

Authority: [ADR 0017](../../../docs/decisions/0017-athlete-contact-truth-and-hero-fit.md)
restores the athlete to Phase 5 and defines the contact/hero-fit/water sequence.
[ADR 0016](../../../docs/decisions/0016-authored-overcast-lighting.md) governs
procedural versus modelled Blender source of truth;
[ADR 0002](../../../docs/decisions/0002-gpl-3-licence-and-asset-provenance.md)
governs licensing and provenance.

Status: planned; not started. Begins only after the
[Phase 5](../blender-05-athlete-contact/tasks.md) athlete is chosen and
implemented, including the corrected seat/pelvis reference and final hand
contact calibration. The boat is fitted once, around that final athlete.

Make the complete hero system coherent:

```text
athlete ↔ seat ↔ rails ↔ stretcher ↔ cockpit ↔ riggers
        ↔ oars / poles ↔ shell / machine
```

## Numerical acceptance

- [ ] Measure shoulder/hip width versus cockpit opening; thigh/knee clearance;
  seat width; rail spacing; stretcher width; heel-cup placement; rigger span
  relative to torso and arms; gunwale height; cockpit beam; bow/stern
  transition; hand/grip and hand/pole contact; apparent scale.
- [ ] Record values, coordinate spaces, reference basis and justified bounds
  over the relevant replay phases. Re-verify the final rendered-skin contact
  acceptance established in Phase 5, including SkiErg and BikeErg controls.
  Passing collision tests is necessary but not sufficient.

## Real-world visual acceptance

- [ ] Compare the Qt chase camera against real single-scull reference
  photography at catch, finish and recovery at half slide. Use several
  athletes/body types, record source URLs/provenance and account for camera
  perspective rather than tuning anatomy to a single photograph.
- [ ] Treat photographs as calibration evidence only: do not vendor them as
  runtime assets, trace copyrighted silhouettes, reproduce branding, clone
  one athlete or model one manufacturer's shell exactly.
- [ ] Answer with measured and visual evidence: **Does the athlete/equipment
  relationship plausibly read as a real single scull?** Retain ordinary replay
  views alongside close-ups and numerical checks; establish hero-system
  visual acceptance in Qt, not just a Blender render or collision pass.

## Correction order and shell constraints

1. Correct remaining athlete geometry/contact/pose-anchor issues in their
   owning layer, with Phase 5's architectural decision boundary respected.
2. Re-evaluate the Phase 2 shell against the final athlete.
3. Modify the shell only if a real mismatch remains.

- [ ] If the shell needs adjustment, retain 7.8 m overall length, the
  oarlock/rig contract unless separately decided, and grip semantics. Do not
  uniformly widen the complete hull: reshape the mid-body/cockpit sections
  while preserving bow/stern slenderness. Record the actual mismatch first.
- [ ] Recheck the complete athlete/seat/rails/stretcher/cockpit/riggers/
  equipment relationship after any change, especially hand/grip and hand/pole
  contact. Preserve replay state, timing and frame truth.
- [ ] Record numerical tables, real-photo provenance, same-state Qt captures,
  visual acceptance, applicable local/runtime checks and updated project
  records. Carry any unresolved finding explicitly; do not hide it in water
  effects.

Exit: the final hero system passes both numerical and real-world visual
acceptance, with Phase 5 contact acceptance re-verified. Wake, foam, spray and
final water polish start only afterward in [Phase 7](../blender-07-water-polish/tasks.md).
