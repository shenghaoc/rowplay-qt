# ADR 0018 — Use the reviewed CC0 Human Base Meshes snapshot for the replacement athlete

Status: accepted (2026-09-27, explicit owner decision for Blender Phase 5.4).

## Context

The owner accepts Route 3A in the
[Phase 5.3 recommendation](../blender-phase5-athlete-recommendation.md):
replace the V4 adaptation by rebuilding from its already reviewed Blender
Human Base Meshes v1.4.1 anatomical source. The landed
[athlete audit](../blender-phase5-athlete-audit.md) attributes the major
proportion distortions to the adaptation; it does not establish that the
CC0 base is inadequate. Phase 5.0–5.3 evidence remains the before-state.

[ADR 0002](0002-gpl-3-licence-and-asset-provenance.md) says:

> No downloaded human model, third-party character, scan, likeness, avatar
> generator output or user image ever enters the repository.

[ADR 0017](0017-athlete-contact-truth-and-hero-fit.md) requires an explicit
provenance decision before incorporating a human source. This decision
provides that authorization for one exact reviewed snapshot. Accepted ADRs
0002, 0016 and 0017 retain their original bytes.

## Decision

### One source exception

Supersede the quoted prohibition **only for the snapshot identified below**,
as the modelled source input of the replacement RowPlay-family athlete.

| Identity | Pin |
| --- | --- |
| Upstream | Blender Human Base Meshes v1.4.1, Dan Ulrich / Blender Studio |
| Official distribution | [Blender demo files](https://www.blender.org/download/demo-files/) and [v1.4.1 bundle](https://download.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip) |
| Anatomical base licence | CC0-1.0; [local licence text](../../LICENSES/CC0-1.0.txt) |
| Reviewed source repository | [shenghaoc/rowplay](https://github.com/shenghaoc/rowplay) |
| Pinned reference commit | `173c6facbcedef419ad39168c5e3e642abb7e57e` |
| Exact reviewed path | `static/replay-assets/source/rowplay-human-base-male-v1.4.1.blend` |
| Reviewed source bytes | 2,246,454 |
| Reviewed source SHA-256 | `1defdfb22b53ce3bd779acfa96278ccfdff17f0e1178fa94967d600a9e27c457` |
| Extraction source at that commit | `scripts/extract-replay-athlete-base-blender.py` |
| Extraction script SHA-256 | `f7dc3af883f0a340ec2a9a6ac7cfd79cfc8e411dfd39056cf74a1e35e52f8764` |

The snapshot retains the realistic male body and two eyes, applies one
Multires level and removes the library layout. Verify its bytes against both
the Git object at the pinned commit and the Phase 5.2
[source hashes](../evidence/blender052/athlete-metrics.json.gz) before copying.
Use those available bytes; do not download a different copy of the human.
The pinned records identify the official upstream archive but record no
verifiable archive SHA-256. No archive hash or new archive verification is
claimed; the authorized input is the exact reviewed `.blend` above.
The official Blender demo listing was rechecked on 2026-09-27 and still labels
Human Base Meshes v1.4.1 CC0; this is licence-source verification, not a new
anatomical input.

This exception authorizes neither other members of the bundle nor another
version or extraction. Arbitrary downloaded humans, MakeHuman/MPFB output,
community morphs, clothing/hair packs, third-party textures, scans, likenesses,
user images and future replacement humans still need a separate decision.
New hair, clothing, footwear and detail work must be original and generic,
with every input recorded separately. No new third-party texture is approved.

### Licence and source ownership

Record **CC0 anatomical base + MIT RowPlay modifications**, following the
existing V4 provenance expression `MIT (rowplay) + CC0-1.0` (machine-readable
`MIT AND CC0-1.0`). The unmodified anatomical source remains CC0; the owner
does not acquire exclusive MIT ownership of it. New RowPlay modifications
carry [the RowPlay MIT notice](../../LICENSES/MIT-rowplay.txt). Application,
export and validation tooling remains GPL-3.0-or-later. Record each source,
output and input in `ASSET_PROVENANCE.md` with its own path and hash.

Use [ADR 0016](0016-authored-overcast-lighting.md)'s modelled route: the
reviewed, committed `.blend` stores the final anatomy, bone/helper placement
and weights. A deterministic exporter validates and exports it; a Python
human-body generator or V4's segment retargeting is not its sculpt authority.
Ordinary app builds consume committed outputs through Qt Balsam and do not
require Blender. Give the replacement a distinct identity and preserve the
immutable V4 audit inputs; stop converting/packaging V4 when the replacement
becomes runtime-active, unless an explicit runtime dependency remains.

### Approved scope and budget

Phase 5.4 is authorized. The owner approves a **60,000 exported-triangle
target and 75,000 hard ceiling** for the complete runtime athlete GLB:
body, head, hands, eyes, hair, clothing, footwear and every detail part.
Attribute-seam duplicates do not add triangles. The exporter must reject
more than 75,000; 60,001–75,000 requires a measured justification in the PR.
The recommendation's regional allocation is guidance, not independent hard
caps or minimums. Report the actual allocation and inspect deformation loops.

Reuse authoritative Rust replay semantics, stroke timing, the motion graph,
the 225-float frame, sport phases, equipment truth, contact-target ownership,
the 19 semantic bone names/hierarchy/roles, the 32 helper roles/hierarchy,
the three-joint digit-chain concept, the contact solver structure and the
Phase 5.1/5.2 instruments. Re-derive bone transforms, helper pivots/geometry,
digit lengths, closure calibration/limits where needed, all skin weights and
palm/contact calibration against the actual final anatomy. Final weights
belong in the `.blend`; the recommendation's reference to deterministic
scripted weight generation does not authorize carrying over V4's method.

Accept native Qt deformation and material presentation before final contact
calibration, then require actual-skin acceptance for RowErg, SkiErg and
BikeErg. Helper counts alone cannot accept contact. A measured need to change
the protected semantic skeleton, frame or runtime deformation/contact
ownership requires a separate architectural decision, not compensation.

Keep the corrected Phase 5.0 seat/pelvis baseline and equipment fixed. This
decision authorizes no shell reshape, cockpit widening, seat/rail/stretcher
movement, rigger fit, change to the 7.8 m shell, Phase 6 hero fit or Phase 7
water work. Record fit mismatches for Phase 6 after the athlete is frozen.
Route 4 remains available only on a further owner choice; do not invoke it
automatically if an authoring iteration is insufficient.

## Consequences

The Phase 5.3 owner checkpoint is satisfied. Phase 5.4 is authorized, not
accepted or complete. Implement it as a `gh-stack` with governance first,
authored athlete/runtime integration second, final contact acceptance third.
The governance commit must be an ancestor of every commit incorporating the
CC0 source. This layer contains documentation only: no new human asset bytes
or runtime change. The owner retains the decision to merge the stack.

All other source prohibitions and the Phase 5–7 sequence remain in force.
V4 and the old evidence remain reproducible. New measurements must identify
the final asset, source, exact instrument and Cargo artifacts, and renderer
state; neither historical audit success nor this decision accepts a new skin.
