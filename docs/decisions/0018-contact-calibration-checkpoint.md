# ADR 0018 — Contact calibration checkpoint after Phase 5.1

Status: proposed evidence checkpoint; no new contact architecture selected.

Context: [Phase 5.1](../blender-phase5-contact-audit.md) reproduced the logical
10/10 RowErg and BikeErg counts and 8/10 SkiErg count, validated actual V4 skin
against native Qt silhouettes, and found errors above the mesh (a stale rowing
wrist frame and incorrectly fitted pole leaves). Those narrow port defects
are corrected. Remaining grip-axis tilt and helper-to-surface discrepancies
mean that preserving a green helper count is insufficient to accept skin contact.

[ADR 0017](0017-athlete-contact-truth-and-hero-fit.md) requires this checkpoint
when a contact contract itself blocks correct contact. The present decision
is to preserve and publish the evidence and defer contract/anatomy changes.
Do not move QML nodes, inflate radii, increase finger limits or edit skin
weights merely to recover a numerical count.

Questions for the later authorized contact-calibration decision:

- Can the existing wrist refinements preserve the actual shaft channel while
  distributing comfort motion through the arm, within measured reach limits?
- Which palm/pad points and compression allowance describe the actual chosen
  skin, rather than the current abstract helper envelope?
- Which helper placements, digit lengths or skin weights would need changes
  to make the visible ring/pinky/thumb surfaces obey that contract?

Options to investigate at that checkpoint include a calibrated contact
surface on the chosen mesh, constrained wrist relief that preserves the
shaft channel, and changes to helper placement/weights if their measured
surfaces cannot satisfy it. None is selected here; none authorizes a new
athlete, a new skeleton, a triangle budget, or the Phase 5.2 material experiment.
This is not a retain/refine/replace recommendation (Phase 5.3).

Consequences: Phase 5.0 and the Phase 5.1 investigation may finish while
rendered contact remains explicitly unaccepted. Phase 5.2+ and Phase 6 remain
separate owner-controlled work. Future acceptance must rerun the actual-skin
instrument and native frames, not only helper parity tests.
