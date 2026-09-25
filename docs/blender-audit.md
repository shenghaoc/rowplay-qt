# Direction C audit and acceptance

## Baseline

The reproducible inventory is [blender-asset-inventory.md](blender-asset-inventory.md).
Before this pass: one V4 athlete, one V3 equipment pack, twelve tier/sport
venue GLBs, 39 environment JPEGs and four procedural PNGs; no file-based IBL.
Their provenance and hashes remain unchanged. Qt 6.11.2 with qtbridge 0.2.0
uses offline balsam conversion (ADR 0008), not RuntimeLoader.

The inherited approved reference is demo 1001, fraction 0.50091, elapsed
208.829 s, speed 4.8302 m/s, Medium, mid-drive. The actual Qt baseline is
under `build/blender-scratch/dump-run/caps/`; a fresh same-machine baseline is
under `build/previews/qt-before-light/`. All captures in this session use
Linux Wayland/OpenGL with llvmpipe. They are not macOS verification.

## Critique

The 7.8 m shell, sculls and athlete are proportionate and share a working rig
contract. Replacing the athlete would spend the budget in the wrong place.
The large turquoise water region is almost featureless; it hides forward
translation even when the rig and HUD clearly change. Painted water overlays
read as flat shapes rather than small surface structure. The warm horizon and
violet equipment compete with the application's cool UI. The default Medium
tier lacks shadows, making surface separation depend entirely on colour.

The existing Qt baseline omits tree/marker furniture visible in the Blender
venue reconstruction. This is a separate visibility/instancing audit, tracked
in #121, not repaired by relocating or inventing geometry in this pass.
The distant bank has too little atmospheric separation and too much saturated
green to match the chosen overcast reference. The original dark style frame
reads as night and is not accepted as the blue-hour target.

## Chosen direction

The owner chose C, overcast championship morning. The local self-contained
reference page and all three directions remain in
`build/blender-scratch/rowplay-style-frames.html` and `frames/`. C is canonical,
A only informs restrained depth, and B only informs simplification at Low.
No reference image is being republished or passed off as a runtime result.

The implementation uses generated linear HDR radiance, prefiltered Qt KTX,
silver world-fixed normal-mapped water, restrained material clearcoat and
stationary buoy lines. Blue hour has independent skylight/fog/fill values
with the same water albedo, exposure and material language. No pose, camera,
oar-state or distance-to-loop calculations are changed.

## Acceptance discipline

`tools/blender/capture.py` records the exact pose frame and grip alongside the
captures. The first Direction C run matched every value in the approved
225-float frame except its sequence counter. Quarter-second samples follow
the real workout, approximately 4.80-4.83 m/s over the three-second sequence.
The historical scratch orbit clip used speed/radius instead of the app's
distance/1000 turns and is not valid unchanged-motion evidence.

Opaque alpha and image variance are checked before visual inspection. Early
failed integration runs are not acceptance evidence. Current unchecked work
and platform gaps are listed in
[the Phase 1 task list](../.kiro/specs/blender-01-pipeline/tasks.md).

## Linux results

The light and dark before/after sets each pass exact equality of all 18
captured runtime states, including grips and effective tiers (only the frame
sequence counter is excluded). All compared PNGs have opaque alpha and
nonblank pixels. The approved mid-drive frame also matches the inherited
runtime dump exactly apart from that counter. Light SkiErg and BikeErg captures
remain within the repository's noise bounds (9 pixels / delta 2 and 13 pixels /
delta 2 respectively).

| Water band, first versus last frame | Before | Direction C |
| --- | ---: | ---: |
| Light, mean absolute RGB difference | 0.00271741 | 0.0176015 |
| Dark, mean absolute RGB difference | 0.00390547 | 0.0205024 |

Values are normalised to 0-1, alpha excluded, over the fixed 250 x 230 region
at (0,420), away from the athlete and HUD. This measures visual change, not
perceived speed. In the sequence, stationary buoys pass the hull and the
venue shifts behind it. The workout advances three seconds at approximately
4.8 m/s without a water-offset animation or any change to the loop mapping.

Blue-hour sky and water grayscale means in the sampled regions are 0.7868
and 0.6034 respectively: the sky is still luminous, water distinctly darker,
and the boat/athlete are readable. Exposure remains 1.0 in both schemes.
Low reduces normal strength and clearcoat; Low/Medium have no dynamic
shadows. High/Ultra retain the existing shadow and venue-detail tiers.

Request (gate step 52) to the first settled row capture measured 47.937 s
before / 10.075 s after in light, and 49.321 s / 10.040 s in dark. These
were measured during the first integrated Medium iteration, before the final
shared-water-albedo adjustment. They include capture settling and are single-run
software-renderer observations, not an iGPU frame-rate benchmark. The gate's
first-presented-frame metric is
not interchangeable: it reported 0.198 s for the dark baseline, well before
the actual row capture. The final uninstrumented full light gate reports 3.7 s
to its first frame. Ski/bike procedural-sky delays remain; #67 stays open.

Evidence (all under gitignored `build/previews/`):

- `comparison-light/` and `comparison-dark/`: five before/after tier stills,
  `motion.mp4` at four frames per second, and pixel/frame `report.json`.
- `c-reference-comparison.png`: current Qt versus approved Blender C light.
- `blue-hour-reference-comparison.png`: original too-dark Blender frame versus
  the new Qt blue-hour companion. It is not a claim that the old frame was
  the accepted dark target.
- `buoy-contact-sheet.png`: four Blender views of the exported 224-triangle prop.
- `full-gate-light-retry/` and `full-gate-dark-final/`: uninstrumented app
  gate captures and timing logs.

The first light pass cleared the shadow floor narrowly (1.01% area / 5.33%
darkening), but the first two dark passes failed it. The final shared water
albedo and High/Ultra key strengths preserve measurable shadows: the dark
full gate passes at 12,384 pixels (1.29%) / 6.68%, and the final light gate at
11,810 pixels (1.23%) / 5.88%. No test bound was widened.
Low/Medium keep the softer key and no shadows; High/Ultra use a stronger key,
an intentional tier approximation that still needs other-renderer inspection.

Qt-free tests pass in an isolated run. A concurrent-render run measured 53.09
ms against the unchanged 50 ms library-filter budget and failed; the complete
suite passed when the renderer was stopped, without a code or threshold change.
Competing render work is suspected interference, not an established cause.
One final light app-test attempt ended with signal 15 during the gate, without
a test failure report; no child process remained. Its cause is unexplained.
The unchanged isolated retry completed the full gate successfully in 166.39 s;
all 24 app tests, including smoke capture, passed. The uninstrumented app builds.
Fmt, workspace clippy, asset hash/budget tests, four KTX/canonicalisation tests
and the material importer probe pass locally. Independent Blender builds
initially differed only in opaque triangle ordering; canonicalising order
without reversing winding makes all seven assets byte-identical, including
both HDRs, both KTX probes, the PNG, GLB and placement JSON.

Remaining compromises: the bank is still geometrically sparse (#121), water
detail remains too regular/directional and has no physical wake/foam, and the
shell/oars are retained meshes with new materials rather than a Phase 2 rebuild.
These are visible limitations, not a claim to have reproduced Blender photorealism.
Mac/Metal and a hardware iGPU performance run remain unverified.
