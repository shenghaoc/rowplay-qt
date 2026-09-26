# Blender Phase 5.0–5.1: seat reference and contact truth

Scope: [ADR 0017](decisions/0017-athlete-contact-truth-and-hero-fit.md),
[Phase 5 tasks](../.kiro/specs/blender-05-athlete-contact/tasks.md), and issue
[#130](https://github.com/shenghaoc/rowplay-qt/issues/130). This report stops
before Phase 5.2. It is a contact investigation, not acceptance of V4 or a
retain/refine/replace recommendation.

## Authority and reproduction

Started from live `main` `2b13e718f4102add40b461d8d581fd5a87cd2da5` on
2026-09-27. Both reference checkouts were refreshed and their current main
pins verified: rowplay `173c6facbcedef419ad39168c5e3e642abb7e57e`;
rowplay-studio `3d406a5b7677372de35fb0817c7133a2589c6564`.
No deliberate divergence authorized the low seat/pelvis.

**FACT** means a source/contract observation; **MEASUREMENT** means a recorded
numeric or native-frame result; **INFERENCE** is an attribution not uniquely
established by those observations; **OPEN** names work still needed. Logical
parity, contact count and visible skin contact are distinct claims.

The capture uses the existing debug guard clock, `fallback_stroke_pose` at
30 spm, cycle = step/2000 and distance = step × 3 m. HUD time/distance still
belong to the loaded demo workout and must not be used as the diagnostic
clock. Row drive fraction is .38; SkiErg .34; bike .5. Each phase has a normal
chase view and separate left/right hand views, with equipment visible.

| Sport | Samples (step; cycle) |
| --- | --- |
| RowErg | catch 0; 0 — mid-drive 380; .19 — finish 760; .38 — half-slide 1380; .69 |
| SkiErg | approach 1900; .95 — plant 0; 0 — loaded pull 300; .15 — pole release 540; .27 — recovery 1100; .55 |
| BikeErg | top 0; 0 — quarter crank 500; .25, both hands |

SkiErg release is pole/course release, not a requirement for the hand to drop
the pole. The diagnostic does not force the pole onto the ground in recovery.

```sh
source .envrc
LANG=C.UTF-8 QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl \
  LIBGL_ALWAYS_SOFTWARE=1 python3 tools/blender/capture_contact.py \
  --output build/phase5/final --masks
cargo run -p rowplay-viewmodel --example contact_probe > build/phase5/final/logical.json
# Python needs numpy and Pillow; no Blender or new asset is needed.
python3 tools/blender/contact_skin.py build/phase5/final
python3 -m unittest discover -s tools/blender -p test_contact_skin.py
```

Run the temporary QML capture alone, without concurrent app builds or edits.
It restores both instrumented QML files in `finally`. The mask pass is an
unlit segmentation check with the equipment hidden, not the Phase 5.2
material/normal improvement experiment. All ordinary captures use the normal
materials. No athlete, shell, skeleton, skin weight or Blender asset changed.

## Phase 5.0: authoritative seat and pelvis

**FACT:** the web's moving `rower-athlete` group translates by
`[0, graph.accents.vertical × .03, .26 − graph.body.pelvisTravel × .44]`.
`rower-seat-carriage` attaches at `[0,.29,−.14]`; hips at `[0,.38,−.14]`.
The old port dropped the moving group's vertical term, set the carriage
anchor to zero, overwrote only its Z in QML, and used hips `[0,.30,seat_z−.10]`.
These are separate missing transform compositions, not an art-direction choice.

| Quantity (metres) | Web | Old port | Old minus web |
| --- | --- | --- | --- |
| carriage attachment Y | .29 | 0 | −.29 |
| carriage attachment Z | −.14 | 0 | +.14 |
| pelvis attachment Y | .38 | .30 | −.08, plus omitted vertical cue |
| pelvis attachment Z relative to moving group | −.14 | −.10 | +.04 |
| pelvis relative to carriage | `[0,.09,0]` | `[0,.30,−.10]` | `[0,.21,−.10]` |

**MEASUREMENT:** the existing rig oracle's representative first row sample
has pelvis `[0,.375908, .12]` (rounded here), versus the old `[0,.30,.16]`:
Y −75.908 mm and Z +40 mm. Its composed carriage is `[0,.285908,.12]`,
versus old `[0,0,.26]`. Exact doubles remain in the original fixture.

**FIXED:** core now carries `seat_y`; the view-model composes carriage and
pelvis from the same moving group. The app packs the complete carriage XYZ
for both player and ghost; QML assigns it. `seatPosition` aliases the unused
row-sport pole slot, retaining the 225-float contract and existing `seatZ`
meaning. A new test consumes all 128 existing web pelvis samples; it failed
on the old values and passes without changing that oracle. A separate new
web recorder samples actual carriage nodes and fitted pole meshes, with
source hashes. No old fixture value was rewritten to accommodate the fix.

**DEFERRED, Phase 6:** the Phase 2 shell source still has `SEAT_TOP=.118`,
`SEAT_CHANNEL=.097`, `SEAT_Z=−.09`, `FLOOR_Y=.034`. The seat mesh now rides the
correct carriage, while these authored local offsets remain unchanged.
Seat-pad/cockpit fit is therefore not accepted. Moving the carriage .29 m
while raising the pelvis by only about .08 m exposes their prior compensation.
No cockpit widening, boat scaling, floor/seat remodelling or camera compensation
was performed. The ordinary chase camera still follows the actual athlete.

## Measurement instrument and its limits

**FACT:** `contact_skin.py` reads the shipped GLB's POSITION, JOINTS_0,
WEIGHTS_0 and inverse bind matrices. The palette comes from each named joint's
final Qt `mapPositionToScene` basis, captured in the same settled frame as the
image. It evaluates linear blend skinning as `Σ weight × jointWorld × inverseBind
× vertex`. It neither reconstructs the pose solver nor substitutes helper points
for skin. Camera projection uses the recorded Qt camera, vertical FOV and viewport.

The independently rendered segmentation silhouette is compared with CPU
triangles in each hand's projected bounding rectangle. The instrument rejects
an overlap below .97. Opaque images and both hands are checked. This validates
position/deformation/projection, not lighting, normals, occluded anatomy or
arbitrary future assets. Wire overlays are corroboration, not the independent
validation; the native segmentation is that validation.

Equipment distance is exact point-to-triangle distance against the shipped authored rowing-shell grip or V3 SkiErg/BikeErg
mesh transformed by its actual Qt node. A nearest-centroid candidate provides
only an upper bound; conservative AABB pruning retains every triangle that
could beat it. Tests cover triangle interiors, edges, degeneracy and a large
triangle whose centroid is distant. Equipment is not reduced to vertex distances.

Hand triangles are partitioned by dominant aggregate skin influence (palm or
one digit, at least .5); all three vertices must belong to the region. Seam
triangles crossing region boundaries are excluded, explicitly. Every retained
triangle is sampled with subtriangle edges ≤1 mm. Distance is 1-Lipschitz, so
the sampled minimum overestimates the true retained-surface minimum by at most
1 mm. Percentiles/fractions describe diagnostic samples, not anatomical surface
area. No single minimum establishes whole-hand acceptance.

Signed values use a **nominal capped-cylinder envelope**, separately labelled
from actual mesh distances. This is a useful penetration diagnostic for the
RowErg rubber (23 mm radius, actual grooves approximately .3 mm larger), but
is not an exact signed distance to the lobed SkiErg grip or BikeErg hood.
Unsigned actual-mesh distances remain authoritative for those shapes. Wrist
channel angle and palm/digit results are reported separately.

## Contact tolerances

Tolerances are for diagnosis, not a claim that an arbitrary pose is approved:

| Metric | Diagnostic threshold | Evidence/rationale |
| --- | --- | --- |
| CPU/native silhouette agreement | IoU ≥.97 in hand ROI | Baseline observed .984–.996; accommodates raster edge/MSAA differences while rejecting wrong transforms/skin palettes. |
| Sampled skin distance uncertainty | ≤1 mm, plus native float/projection uncertainty | Adaptive subdivision bound; native hand close-ups independently validate the palette. |
| Clear floating digit | actual minimum >3 mm | More than the 1 mm sampling bound plus a 2 mm seating band; inspect all digits and palm independently. |
| Tentative surface seating | actual distance ≤2 mm | Approximately half the hand's 2.5–4 mm median triangle edge and a small fraction of the 16/23 mm grip radius. This is necessary, not sufficient. |
| Small compression | approximately 0–2 mm into a meaningful solid envelope | Small relative to grip radius and hand scale; visually plausible allowance, not a measured flesh constitutive model. |
| Gross intersection | >5 mm into the RowErg envelope | Exceeds sampling uncertainty and rubber groove relief; cannot be excused as a minor shading seam or 2 mm compression. |
| Helper parity/contact | retain existing 1e−9 parity and 4 mm contact band | These test logical helper geometry only; never substitute them for skin acceptance. |

**OPEN:** the 2 mm visual seating/compression band is a conservative diagnostic
choice from this mesh resolution and native close-up scale, not an owner-approved
final-human calibration. A future asset needs its own measured contact surfaces.

## First incorrect layers and bounded fixes

| Finding | Observed / expected | First responsible layer | Action in this PR |
| --- | --- | --- | --- |
| #130 carriage/pelvis | omitted .29 m/−.14 m attachment and vertical cue; hips .30 instead of .38 m | equipment transform and pelvis target | Corrected; existing 128-sample pelvis oracle plus rendered carriage fixture. |
| RowErg stale wrist frame | equipment/hand target use the reach-solved yaw; wrist uses the earlier authored sweep | wrist-frame handoff | Corrected. Regression supplies either initial or already-solved frames and requires identical final pose; failed at catch before the fix. |
| SkiErg grip and shaft fit | V3 long axis stretched along Y, centre 75 mm below hand; web uses +Z in its upper frame and grip centre 42 mm toward tip | equipment transform/leaf fit | Corrected for shaft/grip, including upper-frame composition. Basket placement is outside this hand-contact correction and retained. |
| RowErg comfort tilt | corrected channel still differs from physical shaft by 32.88° catch, 20.54° mid-drive, ≈0° finish, 21.20° recovery | wrist refinement/contact contract | Recorded; no arbitrary orientation or helper compensation. |
| SkiErg comfort tilt | 21.0° approach, 25.94° plant, ≈0° pull, 35.15° release, ≈0° recovery | wrist refinement/contact contract | Recorded separately from pole placement and skin. |
| SkiErg two pinkies | +6.210/+6.155 mm helper clearance versus existing ±4 mm logical contact band | digit-helper geometry and bounded closure | Reproduced; not forced to 10/10. |
| Hand-surface calibration | abstract pad/palm values do not coincide with measured deformed skin | helper/contact-to-skin calibration; exact mesh/weight contribution remains open | Recorded at the contact-contract checkpoint; no remodel or weight edit. |

### RowErg

**FACT:** `the_composed_oar_grip_target_matches_the_web_hand_target` still
passes at 1e−9. This covers the logical composition, not the complete web V4
refinement loop or skin. Both hands still report 10/10 aggregate helper closure.
The stale wrist frame was a port defect even though those tests passed.

**MEASUREMENT:** after the fix, hand-target residual is at floating-point noise
at catch, mid-drive and finish; half-slide is 1.982 mm on each hand. No wrist
swing clamp fires at these samples. A signed thumbward-direction check against the physical handle end/grip top
(and the bike hood forward axis) gives the same angles, with no 180° hand
reversal in the final samples. The nonzero grip-axis angles are the
explicit comfort/tilt refinements, not evidence of a clamp firing. They rotate
a closure solved around one cylinder away from the actual equipment axis.

**MEASUREMENT:** the corrected seat exposes reach-circle tangencies near
cycles .398 and .433. The 2000-sample guard's elbow step reaches 29.77 mm;
8000 samples still exceed its 20 mm bound, while 32000 pass unchanged position
and orientation bounds. The rower guard now uses 32000 samples; SkiErg's
2000-sample web-oracle comparison is unchanged. This convergence establishes
a sharp continuous solve, not smooth-looking motion or complete V4 parity.
No production motion timing or animation architecture was changed.

The ghost's equipment path also discarded the solver's yaw. It now packs
that yaw, like the player. A same-workout player/ghost test failed at the start
line before the correction and checks matching equipment bundles for all
three sports at four seek positions afterwards. This is the same ownership
rule: the rendered equipment must consume the contact solve's result.

### SkiErg

**FACT:** closure is computed once in hand-local space. Its radius is .016 m
and it does not read global pole placement or the fitted mesh. Therefore the
75→42 mm placement correction cannot turn the pinky helper counts into 10/10.
Both pinkies reach MCP/PIP/DIP limits `[1.57,1.92,1.40]` radians. The right/left
estimated tip lengths are 15.719/15.714 mm, computed from the distal helper
spacing × .92 (floor 12 mm). Both miss the current contact band; the ring
helper clearances are +1.958/+1.961 mm and count as contact. Index, middle and
thumb helper distances are essentially zero.

**MEASUREMENT:** all five selected phases close the hand targets at floating-
point noise; the dense guard still records a worst 9 mm residual near step
529. The inherited wrist snap and that dense extreme remain documented by
the existing guard; neither the retired .977 m defect nor a claim of exact
full-cycle skin contact is appropriate here.

**INFERENCE:** bounded helper geometry/closure limits explain the numerical
pinky shortfall; this is not proof that every plausible flexion/opposition
combination is anatomically unreachable. Nor does a +6.2 mm helper miss imply
the visible pinky has a +6.2 mm gap: its skin is a different surface. A global
pole offset cannot repair that local calibration. Exhaustive anatomical
feasibility and the relative effects of helper pivots, lengths and weights
remain open; changing anatomy or expanding limits here would conceal the
question the audit is meant to answer.

### BikeErg control

**FACT:** the static handlebar-anchor parity and 10/10 helper closure pass.
Both selected phases have negligible target residual and the hand channel
aligns with the hood axis. That rules out a universal bridge/palette offset
and shows that the large row/ski channel angles are sport-specific.
It does not establish acceptable skin contact. The actual hood is a rounded,
multipart V3 mesh, not the logical .018 m cylinder. The report keeps its actual
triangle distance separate from the cylinder diagnostic.

**OPEN:** surface calibration across all three sports must be resolved before
accepting the grip as physically seated. Skin weights versus source hand
geometry are not uniquely distinguishable from a single deformed surface;
this audit does not label one of them the sole cause.

## Per-phase actual-skin results

**MEASUREMENTS:** distances below are millimetres, **left / right**. They are
minimum distances from each retained deformed-skin region to the actual
rendered equipment triangles (1 mm sampling bound). A printed 0.0 means
touch/intersection at this precision, not a passing grip. Use the penetration,
orientation, helper and native-image columns alongside these distances.
Full per-region sample distributions, mesh resolution and calibration values
are in [final skin data](evidence/blender05/final/skin.json) and the
[seat-corrected baseline](evidence/blender05/seat-baseline/skin.json).

| Sport / phase | Palm | Index | Middle | Ring | Pinky | Thumb | Channel angle ° L / R |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| row / catch | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 8.7 / 8.7 | 32.9 / 32.9 |
| row / middrive | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 20.5 / 20.5 |
| row / finish | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 |
| row / halfslide | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.3 / 0.3 | 21.2 / 21.2 |
| ski / approach | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 21.0 / 21.1 |
| ski / plant | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 25.9 / 25.9 |
| ski / pull | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 |
| ski / release | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 7.9 / 7.8 | 0.0 / 0.0 | 35.1 / 35.1 |
| ski / recovery | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 |
| bike / top | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 |
| bike / quarter | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 | 0.0 / 0.0 |

**MEASUREMENT:** the RowErg catch thumb gap falls from approximately 12 mm
to 8.7 mm after the stale-frame fix, but clearly remains outside the 3 mm
clear-gap threshold. The nominal rubber penetrates the palm skin envelope
by 21.3–22.9 mm across the final phases; groove relief and the 1 mm sampling
bound cannot explain that. The native images corroborate a grip cutting
through the hand. Thus row target parity and 10/10 helper closure coexist
with a measurable surface gap and gross intersection.

**MEASUREMENT:** before the pole-fit correction, SkiErg's index skin sits
16.8–18.5 mm from the actual grip across the phases. Afterwards it reaches
the actual grip in all five. At release, both pinky skin regions still
float about 7.8 mm away. At loaded pull, however, pinky skin reaches the
grip even though its helper model says +6.2 mm: the helper shortfall and
visible-surface shortfall are demonstrably different quantities. Ring skin
also reaches the grip; a +1.96 mm ring helper distance is not its skin gap.

**MEASUREMENT:** BikeErg reaches the actual hood in all six regions, both
hands and both samples. Its cylinder-envelope overlap is not an exact
penetration depth for the rounded multipart hood. No gross hood penetration
claim or whole-hand acceptance is inferred from those unsigned minima. It
remains the aligned, low-residual control, with helper-to-skin calibration
discrepancies still visible in the independent measurements.

### Layer verdict by phase

`Target` below refers to the tested logical contract, not full end-to-end web
V4 pose parity. `Calibration` means helper/contact geometry versus the actual
deformed skin; it does not uniquely blame mesh topology or weights.

| Sport / phase | Equipment | Target / IK | Wrist | Helpers | Skin / native result | First remaining defect or open layer |
| --- | --- | --- | --- | --- | --- | --- |
| Row catch | corrected carriage; rubber axis/radius checked | parity; residual ≈0 | stale frame fixed; 32.9° relief remains | 10/10 | thumb gap 8.7 mm, palm crossing | wrist relief + calibration |
| Row mid-drive | checked | parity; residual ≈0 | 20.5° relief | 10/10 | palm crossing despite all minima ≈0 | wrist relief + calibration |
| Row finish | checked | parity; residual ≈0 | aligned | 10/10 | palm crossing remains | calibration independently exposed without channel tilt |
| Row half-slide | checked | 1.982 mm reach residual | 21.2° relief | 10/10 | palm crossing, thumb minimum .3 mm | arm reach limit; wrist relief + calibration |
| Ski approach | Z fit / .042 m shift fixed | residual ≈0 | 21.0° relief | 8/10 | index gap removed; contact/intersection minima | wrist relief + calibration |
| Ski plant | fixed | residual ≈0 | 25.9° relief | 8/10 | index gap removed; visible grip fit changed | wrist relief + calibration |
| Ski loaded pull | fixed | residual ≈0 | aligned | 8/10 | all regions reach, including pinky skin | helper-to-skin calibration; credible wrap not established |
| Ski release | fixed; pole free of course as intended | residual ≈0 | 35.2° relief | 8/10 | pinky skin gap ≈7.8 mm | wrist relief + calibration |
| Ski recovery | fixed | residual ≈0 | aligned | 8/10 | all skin regions reach | helper-to-skin calibration; credible wrap not established |
| Bike top | V3 hood control | anchor parity; residual ≈0 | aligned | 10/10 | all regions reach | calibration discrepancies; exact compression open |
| Bike quarter | V3 hood control | anchor parity; residual ≈0 | aligned | 10/10 | all regions reach | calibration discrepancies; exact compression open |

### Skin versus helpers

**MEASUREMENT:** the point labelled `HAND_PALM_CONTACT` is 18.37–18.43 mm
from the nearest **entire skinned surface** at RowErg finish, and 30.62–30.73 mm
from the palm-weighted region. This is an exact point/triangle query, without
the 1 mm hand-surface sampling bound or region-seam exclusion for the first
number. The value does not lie on the rendered skin. The construction's palm
seat allowance is a separate abstract calibration, not measured soft-tissue
compression; moving this one point alone would also change the grip channel.

The RowErg estimated index/middle/ring/pinky tip points lie approximately
6.1–8.5 mm from their own digit's nearest skin, versus the solver's .9 mm
pad allowance. Helper intermediate/distal/tip distances are included per
region in the JSON. These differences are stable through the phases because
closure is installed once and the hand is then moved as a unit.

**INFERENCE:** a calibration contract that uses these helpers as a skin proxy
cannot establish the claimed visible seating. **OPEN:** which changes belong
to helper pivots/estimated tip length, weights, mesh shape or coupled wrist
constraints requires an explicit later decision. The evidence does not prove
that a replacement human is necessary. See [ADR 0018](decisions/0018-contact-calibration-checkpoint.md).

## Frozen baseline and native evidence

The [exact native seat reference](evidence/blender05/seat-reference.json)
records before/after carriage and hip matrices in rig space. The capture's
30 spm clock differs from the original fixture's representative timing:

| Corrected native sample | Carriage XYZ (m, rounded) | Pelvis XYZ (m, rounded) |
| --- | --- | --- |
| catch | `[0,.285489,.120000]` | `[0,.375490,.120000]` |
| mid-drive | `[0,.292265,−.315420]` | `[0,.382265,−.315420]` |
| finish | `[0,.294511,−.320000]` | `[0,.384511,−.320000]` |
| half-slide | `[0,.290938,−.277898]` | `[0,.380938,−.277898]` |

The authored seat pad spans local Y .090–.118 m; the corrected pelvis is
.090 m above the carriage. The pad top therefore stands 28 mm above the hip
origin. This is a measured attachment/mesh mismatch, not a claim that the
hip origin itself is a skin seating surface. It is explicitly reserved for
Phase 6; it does not justify reverting authoritative pelvis truth.

Each sheet contains every listed phase, with chase / left / right columns:

| State | RowErg | SkiErg | BikeErg |
| --- | --- | --- | --- |
| original current main, before code changes | [row](evidence/blender05/main/row.jpg) | [ski](evidence/blender05/main/ski.jpg) | [bike](evidence/blender05/main/bike.jpg) |
| #130 corrected, frozen contact baseline | [row](evidence/blender05/seat-baseline/row.jpg) | [ski](evidence/blender05/seat-baseline/ski.jpg) | [bike](evidence/blender05/seat-baseline/bike.jpg) |
| final narrow contact fixes | [row](evidence/blender05/final/row.jpg) | [ski](evidence/blender05/final/ski.jpg) | [bike](evidence/blender05/final/bike.jpg) |

Native evidence is Linux Wayland, Qt 6.11.2, OpenGL software rendering,
light scheme, Medium tier. This is not macOS/Windows visual verification.
The full capture manifests pin original native PNG hashes; compressed frame
JSON preserves the exact Qt palettes/cameras/225-float frames and helper
poses. Lossless binary silhouettes extracted from the held native images
preserve the independent validation input. All 22 final views and 22 frozen
baseline views have IoU .9843–.9958 and opaque source frames. Native close-ups
corroborate the numerical findings; they are not the measurement oracle.

**Instrument correction:** a secondary asynchronous `View3D.grabToImage`
could be overtaken by the next camera step. It has been removed. Evidence
sheets and silhouettes use the existing gate's held full-window PNG, cropped
to its viewport (the original 1200×800 replay has a 57 px top bar).
Revalidation against those held pixels passed; no geometry result depends
on the raced secondary images. Row distance queries use the actual Phase 2
`authored/rowing-shell.glb`, not the older V3 row grip. Ski/bike use V3.

To revalidate the committed evidence without running Qt (numpy and Pillow):

```sh
python3 tools/blender/contact_skin.py docs/evidence/blender05/final --validate-only
```

For a fresh distance run, copy the evidence directory to a scratch directory
and run the same tool without `--validate-only`. The shipped assets must
match the SHA-256 values in `skin.json` and the evidence manifest.

## Validation and stop point

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass, including Qt.
- `cargo test`: pass (core/platform/viewmodel/fixtures). Mock HTTP tests need
  loopback permission; the sandbox-denied run was rerun with it enabled.
- `cargo test -p rowplay-app -- --nocapture`: pass with Wayland/OpenGL,
  `ROWPLAY_QT_SMOKE=1`, phase shots and close-ups. Full runtime walk 248.1 s
  of app log (254.2 s test), first replay frame 2.1 s. The shadow assertion
  measured 2.43% darkened area / 8.53% darkening and passed. Runtime/contact,
  teardown and ghost/tier paths were exercised.
- After the ghost handoff fix, workspace Clippy and the complete app suite
  passed again; the final full gate is recorded in
  [validation output](evidence/blender05/validation.txt).
- Five instrument unit tests and 44 native silhouette checks: pass.
- `git diff --check`: pass. Rust 1.98.1, Qt 6.11.2, qtbridge 0.3.0,
  Node 24.21.0; no new Qt Bridges API or friction.

Required remote CI is recorded on the associated PR; local success does not
stand in for its three operating-system jobs. No AI/code-bot review was
requested or invoked; no draft/ready review retrigger was used. Nothing was
merged. No athlete replacement/refinement, broad material experiment,
triangle-budget choice, Phase 6 shell fitting or Phase 7 water work started.
Phase 5.0 and the Phase 5.1 **investigation** are complete; rendered hand
contact is not accepted as solved. Return to the owner before Phase 5.2+.
