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
At this original Linux stage, Mac/Metal and hardware-iGPU performance were
unverified; the subsequent acceptance record below closes those gates.


## Source-first closeout (2026-09-26)

Audited the PR diff against its merge base, including generated QML/resources,
material lifetimes, sport/scheme transitions, tier branches and generator inputs.
No camera, rig, replay-position or per-frame Rust changes are in this PR. Sky
replacement clears the alternate texture source and destroys the old dynamically
owned sky object; rowing materials are scene-owned. Rowing fog/water/course and
key-light overrides are sport-gated; venue colour/overlay overrides match only
rower names. The importer probe's transmission/IOR checks are compatibility
probes, not claims that the runtime uses those effects: runtime clearcoat and
normals are explicit PrincipledMaterial properties.

Two demonstrated generator acceptance holes were corrected, without regenerating
assets or changing runtime QML:

- An index view overlapping a vertex view was accepted and canonicalisation
  changed vertex bytes. The narrow buoy contract now rejects overlapping/shared
  index storage, out-of-view indices, external buffers and multi-primitive or
  extended materials (including transmission with alphaMode OPAQUE). Sorting
  remains within one opaque primitive and only uses cyclic rotations, never
  reversed winding. The committed buoy is byte-identical after canonicalisation.
- KTX headers were checked but baker metadata was copied unchecked. Missing or
  changed metadata was accepted even though Qt uses the marker to select its
  prebaked path. The version-1 metadata and exact payload length now fail closed,
  as do unsupported headers, truncated faces and trailing bytes. Tests preserve
  the destination on rejection and check nonuniform 4x4 box averages.

### KTX/IBL contract

Checked Qt **v6.11.2** sources:
[qssgiblbaker.cpp](https://github.com/qt/qtquick3d/blob/v6.11.2/src/iblbaker/qssgiblbaker.cpp),
[qssgrenderbuffermanager.cpp](https://github.com/qt/qtquick3d/blob/v6.11.2/src/runtimerender/resourcemanager/qssgrenderbuffermanager.cpp)
and [material shader generator](https://github.com/qt/qtquick3d/blob/v6.11.2/src/runtimerender/qssgrenderdefaultmaterialshadergenerator.cpp),
against the [KTX1 specification](https://registry.khronos.org/KTX/specs/1.0/ktxspec.v1.html).
Qt bakes levels 0–4 at roughness 0, .25, .5, .75, 1 and level 5 as diffuse
irradiance. Each is already filtered linear radiance, so spatial box reduction
within each level preserves its roughness identity; it is a quality reduction,
not an equivalent re-bake. The diffuse face becomes 4x4 (Qt comments prefer at
least 16x16), so directional detail is reduced. Visual acceptance was still open
at this source-audit stage; it subsequently passed on Apple M5/Metal below.

512/256/128/64/32/16 faces become 128/64/32/16/8/4. For RGBA16F each pixel is
eight bytes; imageSize is one face, not six faces, and divides by 16 after 4x4
reduction. Every row, face and mip payload is four-byte aligned, requiring zero
cube/mip padding. The 64-byte header is followed by exactly 28 metadata bytes,
including its size field and padding. Little-endian half floats and header words
are intentional; big-endian or other formats are rejected. Qt loads all six
levels, records the file's level count and passes max level 5 to its shaders;
it does not infer roughness count from the new base size. Thus the unused 2x2
and 1x1 storage levels do not replace the diffuse level. There is no identified
Metal-specific layout error. Both source HDRs successfully baked and compacted
with local Qt 6.11.2 / Apple M5 Metal, producing 1,048,436-byte files; these
scratch outputs were not committed and do not establish app visual acceptance.
The initial sandboxed bake could not access MTLDevice; the unsandboxed bake passed.

### Buoy and resource cost

Local balsam output contains exactly one Model, sourcing one
`meshes/sphere_mesh.mesh`. `RowingCourse.qml` assigns its `instancing` to the
single `CourseInstances` InstanceList once at Component.onCompleted. That list
has 256 InstanceListEntry records, not 256 Models or geometry copies.
`build.rs` parses course.json at build time and emits this static component;
there is no runtime JSON parsing or per-frame placement work. Materials and
instance data have static QML ownership. Only authored KTX/PNG files enter the
environment resource pack; source HDRs, GLB, placement JSON and manifest are
excluded. The separately converted buoy mesh/QML and instance QML are embedded.

### 53.09 ms investigation

The gate is `rowplay-viewmodel/tests/perf_5k.rs`'s
`filtering_a_5000_workout_library_stays_under_50ms`: each of five queries measures
the median of seven `filter_and_sort_workouts` calls over 5,000 synthetic records.
It measures existing CPU filtering/sorting, not this PR's rendering, asset loading
or build-time generation. The original Linux log naming the failing query was
not available in this checkout; 53.09 ms is the earlier recorded observation,
not a newly recovered raw measurement. The test and query implementation are
identical on the PR and current main.

On this Mac (Apple M5, arm64, Rust 1.98.1 debug), 30 isolated processes per branch,
one test thread, without audit render work, all passed. One validation worktree
was switched between PR and main, with a rebuild before each batch. Main was
`23fdee4`; PR was the handed-off head. Values below are min/median/max of the
30 reported seven-sample medians, in milliseconds; they are not raw-call tails.

| Query | PR | main |
| --- | --- | --- |
| Default/date descending | 2.393 / 2.440 / 4.639 | 2.420 / 2.503 / 4.702 |
| Sport | 0.744 / 0.762 / 1.108 | 0.754 / 0.785 / 1.028 |
| Free text | 1.259 / 1.297 / 1.717 | 1.284 / 1.315 / 1.718 |
| Pace ascending | 2.328 / 2.367 / 3.038 | 2.343 / 2.447 / 3.194 |
| Date range/distance | 1.189 / 1.222 / 1.336 | 1.222 / 1.257 / 2.025 |

No material regression was observed. Neither branch reproduced a 50 ms tail;
this does **not** establish a shared noisy timing gate or prove interference on
the original Linux host. That historical miss remains unexplained, not a flake.
The threshold is unchanged. Hardware-iGPU interactive performance and macOS/Metal
app visual acceptance were still external gates at this stage; see the
subsequent acceptance record below.


Closeout validation on macOS: six Python tests pass; fmt and diff whitespace
checks pass; workspace/all-target clippy with warnings denied passes; app builds;
Qt-free tests report 573 passed / two existing ignored; app tests report 24 passed,
including the full offscreen runtime-error walk (24.8 s). The opt-in screenshot
test returns early without ROWPLAY_QT_SMOKE, so that count is not 24 rendered
checks and no app visual assertion result is claimed. Initial sandboxed HTTP
mock tests failed on denied loopback binds; the unsandboxed suite passed. The
material importer probe also passes. No Blender full regeneration was performed
in closeout; assets remain exactly those reviewed in the original Linux evidence.
`issue-44-local` and the primary checkout were not modified. No Phase 2 work,
ready-for-review transition or merge was performed.


## Apple M5 acceptance before integration (2026-09-26)

Accepted head: `2528da10bc894a771296eea89e33c719347d9e74`.
MacBook Pro (Mac17,2), Apple M5, 10-core integrated GPU, 16 GB RAM;
macOS 27.0 (26A428), Qt 6.11.2, Rust 1.98.1. Native Cocoa/QRhi logs identify
Metal and Apple M5. No code or asset changed during this acceptance pass.

Demo 1001 at 208.829 s, mid-drive, was captured in both schemes and all four
tiers. Light retained subdued silver water and restrained clearcoat; blue hour
had its own cool environment while retaining readable water, sky and equipment.
The 4x4 diffuse faces caused no objectionable seams, lighting banding or broad
specular/diffuse mismatch in the inspected views. Low/Medium remained intentionally
shadowless; High/Ultra shadows and materials behaved normally. All 18 controlled
pose/grip/tier states per scheme matched the predecessor scene after excluding
only the sequence counter. The predecessor comparison substituted its ReplayScene
in the otherwise identical app; it was not a complete historical executable.

Actual interactive Medium light/dark and High/Ultra light playback exercised
several strokes and changing buoy positions. Water stayed world-fixed and buoys
provided translation cues. Five 24-second steady windows (including predecessor
Medium light) showed 8 ms median / 9 ms p95 GUI intervals, approximately 2880
intervals each. Current maxima were 11–15 ms, predecessor 10 ms, with none over
50 ms inside those windows. These are GUI cadence samples, not GPU completion
timings or utilization measurements. Initial renderer preparation reached
0.97 s Medium light, 0.21 s Medium dark, 1.59 s High and 0.48 s Ultra; predecessor
Medium light was 0.70 s. Differing cache histories prevent a cold-start regression
claim. One isolated 70 ms interval occurred outside the steady windows; no
recurring hitch was observed. Short ps samples showed no pathological memory
growth or practical CPU issue from the 256 instances; they are not a leak test.
Both gates are accepted on this M5, not on every GPU.

All 24 app tests passed in each native scheme with screenshot smoke, full gate,
phase shots and closeups enabled. Shadow assertions passed unchanged: light
1.30% area / 5.93% darkening; dark 1.35% / 6.75%. The exact accepted head's
[direct CI](https://github.com/shenghaoc/rowplay-qt/actions/runs/36227418359) passed.
Local evidence is retained under
`/private/tmp/rowplay-pr123-audit/build/mac-acceptance/` (ACCEPTANCE.md, light/dark
and predecessor captures, gate logs, interactive timing/resource logs); it has
not been uploaded. Historical Linux evidence and its limitations remain above.

## Current-main integration (2026-09-26)

Rebased the two Phase 1 commits onto current main
`23fdee4962adf6772ae97a74382e0820369f9828`. Only two conflicts occurred:
`docs/decisions/README.md` and `docs/source-map.md`. Main's native-style ADR remains
0015; Blender lighting is renumbered to 0016, with its index and cross-references
updated. Main's source-map structure and bridge-maintenance section are retained,
with the Blender entries reapplied alongside them. Both reference repositories
were refreshed at main and still match the recorded pins.

Cargo manifests/lockfile, the complete Rust app/core/viewmodel sources and CI
workflow match main. Thus qtbridge 0.3.0, MSRV 1.88, QmlElement, public QmlObject
worker APIs, local attachment guards and macOS CXX/cxx-gen alignment protections
are preserved. Main's native HUD and ghost-clock fixes are intentionally retained;
the integration does not claim to revert those behaviors to the old branch.
The 225-float contract and solo rowing truth remain the acceptance comparison.
No new compatibility shim or scene walker was added. Authored assets and the
Phase 1 build integration, RowingCourse, RowingStyle and RowingWater are unchanged
from the accepted head. Normal builds consume committed assets without Blender.

### Integration verification

Both scheme captures use the existing fixed-state harness on Cocoa/Metal.
All 18 controlled states per scheme match the accepted head exactly for the
225-float payload (excluding its sequence counter), grip and tier. That includes
the packed camera, course, athlete and oar data. In all four tiers and both
schemes, the common unobscured scene rectangle (2400x1600 captures, rows 110–1299,
all columns) is pixel-identical to the accepted captures. Main's newer HUD lies
outside that rectangle and is intentionally different. Medium light, Medium
blue hour and High were also inspected: IBL, normals, clearcoat and materials
remain intact. The approved art direction was not changed.

Generated balsam output contains one buoy Model and one mesh source; generated
CourseInstances contains exactly 256 entries, matching the placement JSON.
RowingCourse assigns them only in Component.onCompleted, unchanged. The placement
JSON remains build-time input. All seven authored assets and the build integration
are byte-identical to the accepted head.

Main's exact CXX alignment script passed locally: CXX 1.0.198, cxx-gen 0.7.198
(ABI revision 198), CXX-Qt 0.10.0 and all qtbridge packages 0.3.0. Formatting,
workspace/all-target clippy with warnings denied, app build, six Python pipeline
tests and Qt-free tests (585 passed, two existing ignores) passed.
Both native Cocoa/Metal full app runs passed all 29 tests (zero failures or
ignores), with screenshot smoke, phase shots and closeups enabled. Light/dark
walks logged 107.0/106.9 s; first replay frame was 0.2 s in both. Unchanged
High shadow assertions passed: light 1.23% area / 5.97% darkening, dark 1.29% /
6.78%. The count includes main's new ghost-clock and race-gap tests.

A short real interactive Medium replay ran from about 3:30 to 4:19, through
several strokes and changing world-relative buoy positions. Water remained
world-fixed; no new recurring hitch appeared. A 24-second steady excerpt of
Qt's existing timing log had 2881 GUI intervals, median 8 ms, p95 9 ms, maximum
12 ms and none over 50 ms. This is a smoke check, not a repeated hardware
benchmark or GPU completion measurement. The app exited normally.
Local post-integration evidence is under
`/private/tmp/rowplay-pr123-audit/build/integration/`: light/dark captures,
comparison.json, instancing.json, invariants.json, full app logs and captures,
CXX alignment output and interactive logs. No real-user data was captured.

### Source-policy follow-up before shell/oar work

ADR 0016 says “The scripts, not a hand-edited blend file, are the source of truth.”
That is broader than the intended future distinction: a procedural asset should
have a reviewed generation script, while an artistically modeled asset should
have a reviewed `.blend` source plus deterministic export and validation scripts.
Refine that scope before shell/oar work. This integration preserves the accepted
Phase 1 decision and does not design a Python CAD framework or begin Phase 2.
Phase 2's opening commit amends ADR 0016 accordingly.

## Review follow-up (2026-09-26)

`build_all.py` now writes `course.json` with one buoy per line, so a moved
buoy is a one-line diff; the asset test fails if the file goes back to one
value per line. Only the layout changed. The regenerated file parses to the
same 256 placements with the same number literals, in order, and
`MANIFEST.json` re-pins it at 16,145 bytes. The other six assets are not
touched.

Two independent full generations (`build_all.py` under Blender 5.2.2 LTS,
build d13f752e3b9c, on the Apple M5 above with Qt 6.11.2 balsam), each into
a fresh directory, were byte-identical across all eight outputs. Against the
committed Linux pack (Blender 5.2.1), `water-normal.png` matched byte for
byte. The skies, probes and buoy did not match, which the pipeline README
anticipates for another Blender build and baker backend. The buoy's positions,
normals and indices were identical; 34 of its 302 texture-coordinate floats
differed by at most 6e-8. None of those outputs was committed.

## Phase 2: shell and oars (2026-09-26)

`rowing-shell.glb` rebuilds the single scull and its sculls inside the V3
contract. It is a procedural asset under the amended ADR 0016: its script is
its source. The shell is 7.8 m long, the oarlock pins stand on the animated
pivots, and the grip spans the V3 grip the hands close on (0.82 to 0.50 m
inboard, 23 mm radius, 4 cm below the pivot line). Each of these is pinned by
a test against the Rust constant (`rowing_shell.rs`). As drawn, the boat,
seat and both oars total 46,164 triangles against the 60,000 budget. The
authored pack totals 3,226,748 bytes, under its 4 MiB budget.

### Fitting the cockpit to the athlete the app draws

The V3 seat carriage never showed in the port. The port's anchor puts the
template at the moving group's origin, where the web adds `(0, 0.29, −0.14)`,
so the seat rendered below the cockpit floor. The port's pelvis target is also
0.30 against the web's 0.376. Both are tracked in #130; Phase 2 changes
neither.

To fit the new cockpit, the V4 athlete was skinned with the replay's own pose
frames: 40 samples over one stroke of demo 1001, logged by an instrumented
gate. The skinning was validated two ways:

- Projected through the logged camera, the skinned vertices land on the
  rendered athlete in a native capture of the same frame.
- The hand contacts land on the oar grip axis, exactly at one sample and
  within 14.5 mm at the catch.

The fit follows from the measurements:

- **Seat.** The pelvis rests at y 0.121–0.126 under the ischia and 0.099 on the
  centre line, 0.16 aft of the seat origin. The pad top is at 0.118, with a
  centre channel.
- **Floor.** The floor sits at 0.034 on the centre line, rising in a shallow U.
  At the largest bob (±1.94 cm) and roll (4.5°) it stays above the water plane.
- **Feet.** The soles lie on a 43° plane. At the catch the heels lift 7–8 cm;
  at the finish they rest 2 cm past the board plane, on the floor. The board
  therefore starts above the heels, and rubber heel cups on the floor take
  them.

Athlete vertices inside each part, at the worst of the 40 samples (V3 as the
port places it, then Phase 2):

| Part | V3 | Phase 2 |
| --- | ---: | ---: |
| Heel cups | 5,319 | 95 |
| Footboard | 3,983 | 8 |
| Stretcher hardware | 192 | 0 |
| Riggers | 76 | 0 |
| Slide rails | 30 | 0 |
| Seat pad | 0 | 6 |
| Grip | 3,054 | 3,053 |
| Handle cap | 168 | 170 |

The grip and handle cap are the hand closing on the same 23 mm rubber, unchanged
by design. The generator refuses a cockpit part that pokes through the hull at
any seat position, and a fin that shows above the water at the top of the bob.

### Qt comparisons

The Qt captures were taken with `tools/blender/capture.py` natively on the
Apple M5 (Cocoa/Metal): demo 1001 at 208.829 s, Low through Ultra, light and
blue hour, before (Phase 1 head) and after.

- All 18 runtime states per scheme match exactly (pose frame without its
  sequence counter, grip table and tier).
- Every capture is opaque and non-blank.
- 4.1–4.2 % of pixels change at Low and Medium, and 5.2 % at High and
  Ultra. Every change lies in the boat, oar and shadow region.

A probe with the hull's vertex-colour masks switched off changed about 2,700
pixels by at most delta 26 (light, Medium). They form a band a median 6 px
wide along the projected waterline, which confirms the wet band renders on
Metal and stays subtle.

The phase close-ups (catch, mid-drive, finish, mid-recovery) show the hands
closing on the grips through the stroke. The blades in the recovery frames
render solid and mirrored. At mid-drive the boat's roll accent sinks the port
blade and lifts the starboard one clear of the water, in the Phase 1 capture
as in this one. The blades never square, because the port does not apply the
frame's blade roll (tracked in #129).

Two independent `make blender-shell` builds on this Mac were byte-identical to
each other and to the committed file.

### Validation (Apple M5, macOS 27.0, Qt 6.11.2, Rust 1.98.1)

Both native Cocoa/Metal app suites passed all 29 tests in light and in dark,
with screenshot smoke, phase shots and close-ups enabled. The High shadow
check passed unchanged limits:

| Scheme | Shadow area | Darkening |
| --- | ---: | ---: |
| Light | 1.07 % | 5.42 % |
| Dark | 1.13 % | 6.10 % |

These figures are recomputed from the saved twin captures and truncated. The
check prints them rounded, so its log reads 1.08 % for the light area (41,342
of 3,840,000 pixels). That is less margin than Phase 1's 1.23 % and 5.97 %
(light): the old tall stretcher board and oarlock posts cast more shadow than
the new parts.

The walks took about 160 s, against 106.5 s for a Phase 1 walk earlier the
same day. Back-to-back full light walks show that gap is machine state, not
the change: the Phase 1 head walked in 161.2 s and the Phase 2 head in
159.3 s. The replay's first frame stayed at 0.2–0.3 s.

## Phase 3: environment and water (2026-09-26)

Phase 3 finishes the water and gives the basin depth: a near bank, a
mid-distance mass and a far-bank silhouette. Phase 1's lighting (overcast
light, the independent blue hour, the probes, the key light, the fog) is not
redone, and the replay truth is untouched. The branch is stacked on #132, with
#132's two commits restacked onto #131's head: #132 did not contain #131's
two newest commits.

### Baseline and the three deficiencies

The Phase 2 scene was captured with `tools/blender/capture.py` natively on the
Apple M5 (Cocoa/Metal): demo 1001 at 208.829 s, Low to Ultra, light and blue
hour. At the approved moment the three most visible deficiencies were:

1. **The water is a lattice.** Parallel ripple bands at one spacing cover the
   whole surface. The Phase 1 map has 16 wave components on a 4 m tile, with an
   RMS slope of 0.070 along the tile and 0.029 across it.
2. **There is no shoreline.** The water runs flat into lawn slabs at water
   level. There is no bank relief, no waterline and no cue to the basin's
   scale, and the pontoons sit on an edgeless plane.
3. **The depth is empty.** Nothing stands between the bank and a two-band ridge
   with a faceted, even top. The tower stands alone, and the web's woodland
   does not render in Qt (#121).

### Budgets

Set before the environment was built, from the scene's measured totals:
athlete 106,256 triangles (twice with a ghost), shell 46,164 as drawn, 256
buoys at 224 each, rower venue 7,304 (Low) to 17,516 (Ultra). The authored pack
was 3,226,748 bytes against a 4 MiB budget.

| Item | Budget | Final |
| --- | ---: | ---: |
| Terrain, near bank to the rim (one mesh, every tier) | 16,000 triangles | 13,350 |
| Far bank: forest belt and distant hills | 8,000 triangles | 6,480 |
| Woodland masses | 6,000 triangles | 2,436 |
| Each vegetation variant | 800 triangles | 16-599 |
| Vegetation instances: Low / Medium / High / Ultra | 100 / 200 / 300 / 400 | 73 / 147 / 221 / 294 |
| Vegetation triangles drawn per tier | 60k / 90k / 120k / 150k | 21,269 / 42,554 / 64,047 / 85,316 |
| New textures | the water normal only, 512 square | 512 square |
| Authored pack | 6 MiB (was 4 MiB) | 4,677,393 bytes |

The pack budget was the Phase 1 brief's guess. It rises to 6 MiB for the
`.blend` source (648,848 bytes) and its GLB (595,504), far below ADR 0011's
100 MB tripwire. Every environment mesh is one draw: three land Models and six
instanced variants. At Medium, Phase 3 adds 64,820 triangles over 9 draws. The
largest cost stays the athlete, and Low and Medium draw no shadows.

### Concept, in Blender and then in Qt

The concept was built in Blender 5.2.2 LTS around the scene the replay draws:
the web venue's retained structures, the Phase 2 shell at the approved moment,
the buoys, the committed water normal, the authored skies as the world, and
Qt's depth fog reproduced in every material. The render was calibrated against
the Qt baseline first: sky, water and tower tones matched. Iterations, judged
from eight chase views round the lap as well as the approved one:

1. **Scattered trees.** Round crowns on long trunks read as lollipops, the
   shrubs as rocks, the campus paving as a road.
2. **Crowns as unions of lumps, woodland as stands.** The woodland filled the
   upper third of the frame and competed with the athlete.
3. **Stands pushed back to 118-150 m, fewer accents.** More sky, and the
   layering read.
4. **In Qt.** The grass read lime-bright and the woodland as rows of separate
   trees. The palette went darker and greyer, crowns lower on thicker
   trunks. A continuous canopy mass went behind each stand's edge, with
   understory shrubs in front, and the masses taper to the ground at their
   ends.

Qt then exposed two integration faults, both fixed in the source and both now
caught by the export:

- **The launch dock.** The web's launch dock (High and Ultra; deck top 0.21 m)
  was buried by a 0.28 m quay, and its front face poked out of the quay as a
  dark wedge. The quay is now 0.18 m and follows the dock's front edge.
- **The boardwalk.** Reeds stood on the web's wetland boardwalk. The export's
  footprint check had used bounding boxes, which cannot describe the curved
  deck. It now uses annular sectors read from the venue GLB.

### Sources of truth

| Asset | Kind | Source |
| --- | --- | --- |
| `water-normal.png` | procedural | `tools/blender/water.py`, written by `build_all.py` |
| `rowing-environment.blend` | modelled source (MIT) | itself: authored in Blender and judged by eye |
| `rowing-environment.glb` | derived | exported from the `.blend` by `export_environment.py` |
| `vegetation.json` | derived placement data | exported from the `.blend` by `export_environment.py` |

The environment is the first modelled asset under ADR 0016's source rule. Its
first version came from a scripted Blender session: a polar height field with
seeded noise, crowns made as unions of lumps, and placement by land use. The
session was not committed; the `.blend` is the source. It holds collections
for the land (terrain, far bank, woodland), the six variants and one per tier.
The instances are linked duplicates, tinted by their object colour. Moving a
tree or reshaping a bank is an edit in Blender, then `make
blender-environment`. The export refuses a file that breaks the contract:
names, identity world transforms (delta transforms and constraints count),
yaw-only instances written where Blender shows them, budgets, no land at or
above the water inside 36.2 m on any triangle, nothing planted in a retained
structure, and a Low instance for every variant.

The land uses follow the web's sectors: the campus near 8-56 degrees (tower,
dock, pavilion, boathouse), the open vista near 300-358 degrees (wetland,
boardwalk) and woodland between them. The terrain meets the retained
structures at their base heights, and the export checks those heights.

### Water

The tile is 8 m (6000 m / 750). It carries 360 integer wave vectors, from 6 cm
to 2.8 m, spread about a 28-degree wind with 28 % of them about a second
direction 74 degrees away, plus an 18 % seeded amplitude variation. The texture
is tileable and fixed in the world. Nothing scrolls, and no speed or presentation term exists.

The RMS slope was settled in Qt. Phase 1's map had an RMS slope of 0.0757,
nearly all of it along one axis. Spread over every direction, the same RMS
read nearly flat near the boat, where the passing texture is the water's
motion cue. At the approved frame, 0.08, 0.11 and 0.14 were compared: 0.11
keeps visible, irregular ripples near the boat, and 0.14 reads choppy for flat
water. The material's normal strengths are unchanged.

`test_water.py` guards the spectrum. No component may carry 2 % or more of the
slope variance (each of Phase 1's carried at least 6.25 %), and the RMS slopes
along and across the tile may not differ by 1.5x or more (Phase 1's: 2.4x).
Seen from above, the texture repeats every 8 m, and its distinctive points
repeat with it (`water-normal-16m-before-after.png` on the evidence branch).
From the chase camera, at grazing angles and with mipmapping, the Qt captures
show neither that repeat nor a lattice.

### In Qt

`build.rs` runs balsam on `rowing-environment.glb` for its meshes only, so it
first checks the GLB (`replay::environment::validate_environment`): the nine
meshes, each drawn by one flat node with no transform, which balsam's mesh
files would drop, triangle lists with vertex colours, and no materials. It
checks balsam's mesh file names and the placement invariants, then generates
`EnvironmentScene.qml`: the three land Models and six instanced Models, whose
`InstanceList`s come from `vegetation.json`. The file is sorted by variant and
then tier, so a tier's instances are a prefix of each list, and
`instanceCountOverride` selects the prefix. `RowingEnvironment.qml` declares
the materials. Vertex colours carry the albedo, blue hour tints them cooler,
and each instance's colour multiplies in. No runtime walker touches the
environment.

The web rower venue's land and vegetation (banks, horizon and ridge bands,
woodland, reeds, the campus path) are hidden through the venue walk that
already hid Phase 1's painted overlays: the name list grew, and no walker was
added. Its structures stay until Phase 4: the finish tower, pontoons, launch
dock, distance posts, pavilion, boathouse, timing tower, course bridge,
boardwalk and hide. So does the island at the course's centre, with its lawn,
trees and shrubs. Bucket clones now inherit their archetype's visibility. The
island shows only when the comparison camera pulls back with a ghost.
`RowingStyle` gives grass, lawn, canopy, shrubs and reeds one green, which now
reaches only the island. Its lawn, trees and shrubs, which have shared that
green since Phase 1, take the authored lawn tone and no longer read as a
bright green disc beside the new banks.

**Shadows.** The High shadow check first dropped from Phase 2's 41,342 px
(1.07 %) / 5.42 % to 38,877 px (1.01 %) / 5.16 %, still passing. Bounds
unchanged, an A/B isolated the cause:

- with the old water normal it read 38,748 px: the water is not the cause;
- hiding the environment restored 40,893 px;
- the 2,016 px lost lay in rows 640-673 only: the finish tower's shadow, which
  had fallen on the water and now fell on the new, non-receiving quay. The
  rower's own shadow was pixel-identical.

The web flags every bank arc it draws as a shadow receiver, and its trees and
horizon as neither, so the terrain now receives and nothing in the environment
casts. The tower shadows the quay again. The check reads light 40,672 px (1.05
%) / 7.83 % and dark 43,048 px (1.12 %) / 8.13 % (truncated, from the saved
twins).

### Runtime invariants

`compare.py` checks all 18 controlled states per scheme against the Phase 2
parent: pose frame without its sequence counter, grip table and tier. They
match exactly in light and in blue hour. The same replay state, grabbed at two
wall-clock times (`style-medium` and `motion-000`), renders 0 differing pixels
before and after, in both schemes. Nothing in the water or the environment
moves on its own clock. The only motion is the replay's own camera, whose
frames are unchanged. `compare.py` now asserts this on either pixel path:
Pillow and numpy where they are installed, FFmpeg otherwise.

Phase 1's "water band" patch at (0, 420) lies across the far bank and the sky
in the current layout, so it now sits on near water at (0, 1150). Its
first-to-last change over the 3 s sequence is 0.0120 before and 0.0098 after
in light, and 0.0158 and 0.0129 in blue hour. That is visual change in a fixed
patch, not a measure of perceived speed.

### Motion

A 12 s real-replay sequence (demo 1001 from 208.829 s, every 0.5 s, Medium; the
same 25 states on both sides) shows:

- the near bank and the trees shifting against the far bank as the boat moves;
- the stationary buoys passing the hull;
- the water texture staying put in the world.

`qt-motion-before-after.gif`, `.mp4` and `qt-motion-strip.jpg` are on the
evidence branch. A phase-correlation measure of motion by depth band was
tried and dropped: in this framing the left half of the frame looks across the
basin, so its bands do not separate depths, and its correlation peaks were
weak (0.06-0.24).

### Metal results (Apple M5, macOS 27.0, Qt 6.11.2, Rust 1.98.1)

All four tiers in light and blue hour were inspected at the approved moment
and at eight positions round the lap. The athlete and the data stay primary.
The environment reads as layered and restrained, and blue hour reads as its
own cool scene: dark silhouettes against a lighter sky, with the water the
brightest plane. Low draws the smallest vegetation subset, and Ultra the
fullest.

Both native Cocoa/Metal app suites passed all 32 tests, with screenshot smoke,
phase shots and close-ups. The gate walks took 108.1 s (light) and 107.2 s
(blue hour), and the replay's first frame came 0.3 s after step 52.

### Performance

Measured with Phases 1 and 2's method: real playback of demo 1001, Qt's
`QSG_RENDER_TIMING` GUI intervals over 24 s steady windows, `ps` samples every
2 s, and the Phase 2 parent run back to back from its own worktree. This is GUI
cadence at the display's 120 Hz, not GPU timing. GPU time was not measured:
the Metal HUD's log mode wrote no metrics here. The repository's bench
(`ROWPLAY_REPLAY_BENCH`) stayed at 8.33 ms on both sides, because
`QSG_NO_VSYNC` does not unthrottle Cocoa/Metal, so it shows no headroom.

| Window | Samples | Median | p95 | Max | > 50 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Parent, Medium, light | 2,880 | 8 ms | 10 ms | 13 ms | 0 |
| Phase 3, Medium, light | 2,878 | 8 ms | 10 ms | 21 ms | 0 |
| Parent, High, light | 2,879 | 8 ms | 10 ms | 13 ms | 0 |
| Phase 3, High, light | 2,880 | 8 ms | 10 ms | 14 ms | 0 |
| Parent, Ultra, light | 2,879 | 8 ms | 10 ms | 12 ms | 0 |
| Phase 3, Ultra, light | 2,880 | 8 ms | 10 ms | 14 ms | 0 |
| Parent, Medium, blue hour | 2,878 | 8 ms | 10 ms | 30 ms | 0 |
| Phase 3, Medium, blue hour | 2,880 | 8 ms | 10 ms | 12 ms | 0 |

No window shows a recurring stall. CPU ranged over 42-71 % of one core on
both sides. Resident memory is flat within every window: no growth. Between
runs it is not stable. Three alternating Medium runs per side read 344.2,
366.1 and 365.0 MiB (parent) and 293.5, 370.6 and 373.4 MiB (Phase 3): a spread
of up to 80 MiB for one tree, pairwise differences of -50.7, +4.5 and +8.4 MiB,
and medians 5.6 MiB apart. No memory regression is demonstrated. For a laptop
iGPU the added cost is the table above's triangles and 9 draws, plus
shadow sampling on the terrain at High and Ultra. That is an estimate, not a
measurement on such a GPU.

### Determinism

Two generations of the water normal and three exports from the unchanged
`.blend` were byte-identical (Blender 5.2.2 LTS on this Mac). The water PNG
also matches the numpy prototype judged in Qt to within one 8-bit level. The
`.blend` is not regenerated from a script: it is the source.

### Validation

- 21 pipeline tests with Blender's Python (`test_canonical`, `test_probes`,
  `test_water`, `test_export_environment`).
- `cargo fmt --all -- --check`.
- `cargo clippy --workspace --all-targets -- -D warnings`.
- Qt-free tests: 594 passed, 2 existing ignores.
- The asset tests: the new ones check the manifest's source hash, its
  triangle counts against a recount from the GLB, every budget against a
  pinned value, the placement invariants, and the water tile against
  `RowingWater.qml`. The tile check fails, as it should, with the old 1500
  repeat put back.
- The export's refusals, on the committed `.blend` mutated in memory: a delta
  transform, a quaternion turn and a constraint on a land part or variant; a
  delta location, a delta rotation and a constraint on an instance; a terrain
  face that surfaces at 35.75 m between vertices; an Empty in a variant's
  place. The first exporter accepted the first seven and crashed on the
  eighth. This one refuses all eight by name, and the committed file still
  exports byte for byte.
- `build.rs` refuses a GLB with a moved node, or a truncated one, naming the
  file; `validate_environment`'s unit tests refuse ten drifts by name.
- Both native app suites; `git diff --check`.

### Limitations

- **Style.** The environment is stylised rather than photographic: crowns are
  lumpy low-poly masses, and conifers are stacked cones. Distance and fog hide
  most of it, but the nearest parkland trees show their facets at 2400 px.
- **The web venue.** Its structures and island stay unchanged until Phase 4.
  #121 (the web venue's instanced groups in Qt) is untouched. For rowing, the
  woodland it concerned is now hidden and replaced.
- **The water tile.** It repeats every 8 m when looked at from above.
- **Wake and foam.** There is none; that is its own phase.
- **Performance.** GPU time and iGPU hardware were not measured.

## Phase 4: course dressing and venue furniture (2026-09-27)

Phase 3 made the basin believable as an environment. Phase 4 makes it read
as a rowing venue: where the course is, where crews launch, where the timing
and finish infrastructure stands, what gives the scene human scale, and what
marks the lap. The athlete and the shell stay the hero; the replay truth
(the 225-float frame, the camera, the pose, the seat, the oars, the shell
contract, the course mapping, the circular loop) is untouched, and every
structure is stationary world geometry.

### The retained structures, audited

Every structure the web venue bake (`rowplay-venue-rower-*.glb`) still
rendered after Phase 3 was captured natively (Cocoa/Metal, demo 1001) at the
approved moment, at eight loop positions and from zone views (launch,
finish, bridge, far bank, wetland, the island with a ghost), then classified.
"Source" is `renderer3dEnvironment.ts` @ `173c6fa`, baked (ADR 0005).

| Structure | Web source, tiers | As rendered in Qt before Phase 4 | Role | Decision |
| --- | --- | --- | --- | --- |
| Finish tower | `addRowerFinishTower`: a 1.5 m box shaft, a rounded cabin, a glass band, a box wing, a mast; 768 triangles; Medium and up (Low had none) | A flat white monolith, the one vertical; visible at the approved moment, at 0 m and 875 m of the lap and from the wetland | The finish and timing landmark | **REBUILD**: a judges' tower on a lattice frame (plinth, four columns, bracing, an open stair, a mid landing, a glazed cabin with a gallery, a finish-line boom over the water with a camera housing and a plain finish board, a mast with a plain flag); every tier |
| Start pontoons | `addRowerIslandCenter`: two rounded blocks, 700 triangles each, at r 18.8 and 21.4 m, 27°; Medium and up | Two grey slabs on the water, left of the boat at the approved moment | The start | **REBUILD** as the start jetty: a floating jetty from the island's beach to r 21.6 m on the finish line (52°), with mooring piles, the aligner's seat and the finish post; every tier |
| Launch dock | `addRowerCampus`: a rounded deck on the quay (top 0.21 m) with four rail posts; High and up | A slab on the quay with four white posts, at the left edge of the approved view | Where crews launch | **REBUILD** as a floating launch pontoon (8 m, freeboard 0.15 m, cleats, fenders, piles, a gangway from the quay); every tier |
| Distance posts | `addTrackEdgePosts`: 4 (High) or 6 (Ultra) tapered posts with blank boards, campus arc only | Thin white poles along the quay, like lamp posts | Lap distance | **REPLACE** with three numbered distance boards, 250, 500 and 750 m from the finish line (142°, 232°, 322°), block numerals in geometry; every tier |
| Regatta pavilion | `addPavilions`: rounded block, cone roof, glass strip; 3,644 triangles; every tier | A white villa with a terracotta cone roof at r 72 m, visible from the wetland and at 750-875 m | The campus skyline | **REBUILD** as the clubhouse: a glazed front, a terrace with a railing, a clerestory; 456 triangles |
| Boathouse | as the pavilion, scaled; Medium and up | The same villa, smaller | The campus | **REBUILD** as a boathouse: a gabled shed with three bay doors facing the water and an apron; 132 triangles |
| Timing tower | as the pavilion, stretched; High and up | A tall villa | The campus | **REBUILD** as the regatta office: two storeys, a balcony toward the water, a mono-pitch roof; 192 triangles |
| Course bridge | `addOverheadSpan`: two cylinder legs and a box deck at 4.7 m, 148°; High and up | Two white cylinders and a brown slab; the only overhead landmark | The lap's one crossing | **REBUILD** as a footbridge from the island to the bank: a swept deck with handrails, A-frame piers outside the lanes, stairs down to the island's beach; every tier |
| Wetland boardwalk | `addRowerWetlandBoardwalk`: an arc deck at 0.32 m with seven posts; Ultra only | Lost among the reeds | The quiet bank | **REBUILD**: a deck on piles with a handrail on the water side, ramps at both ends and a spur to the hide; every tier |
| Wetland hide | a rounded timber block with a dark slot; Ultra only | Hidden by the reeds and the deck | The quiet bank's landmark | **REBUILD**: a timber hide on short piles with a slot window and a mono-pitch roof; every tier |
| Central island | `addRowerIslandCenter`: three discs, a scaled foothill mound, instanced trees and shrubs (the web's instance path, #121); every tier | A flat green disc with a dark rim; its trees and shrubs do not render; visible only when a ghost pulls the camera back | The centre of the loop | **REPLACE**: an authored low island (a lawn dome, a beach, a sunken skirt) in the dressing pack, planted from the environment's own vegetation variants, the start jetty and the bridge landing on it |
| Reed beds, ripples, reflections, glints, mist, campus path | painted overlays and instanced reeds | Hidden since Phases 1 and 3 | none | **REMOVE** (already hidden; nothing changes) |

Judged from the chase camera and round the lap, the island earns its place
once it has a function: the start jetty and the bridge land on it, its trees
stand between the bank and the far side of the loop, and with a ghost the
pulled-back camera sees a lagoon course rather than a buoy circle on a lake.
Kept as the web drew it, it was a bare disc.

**New for Phase 4**, each answering a composition or readability need:

| Asset | Need | Tiers |
| --- | --- | --- |
| Finish line: the tower's boom, the jetty's finish post and two finish buoys (r 22.25 and 34.2 m, 52°) | where the lap is timed | every tier |
| Coaching pontoon with a moored coaching launch (264°) | a landmark on the otherwise empty far bank; the safety boat | every tier |
| Bollards, benches, life-ring posts, flagpoles with plain flags, an upturned single on slings | human scale at the quay, the jetty, the pontoons and the boardwalk | instanced, by tier |

Nothing is branded: no maker's shapes, logos, sponsor boards or club colours;
the flags and the finish board are plain.

### Layout

The web's zones stay, since the Phase 3 terrain was authored round them:

```
campus (8-56 deg): clubhouse, boathouse, regatta office, flagpoles
  launch zone (31 deg): the launch pontoon at the quay, the slings, bollards
  finish line (52 deg): the tower and its boom, the jetty from the island, two buoys
open water and woodland banks (56-300 deg): the bridge at 148 deg,
  the 250 m and 500 m boards, the coaching pontoon at 264 deg
wetland (300-358 deg): the boardwalk, the hide, the 750 m board
```

The start and the finish are one line, as they must be on a loop; the boards
count from it. The finish line stays at 52° and not at the loop's 0° because
the approved moment (355°) looks straight at it from 33 m: at 0° the tower
would stand 7 m off the camera's right shoulder.

### Budgets

Set before the structures were built, from the Phase 3 scene: athlete
106,256 triangles, shell 46,164, 256 buoys at 224, the environment 64,820 at
Medium over 9 draws, and the retained web structures 14,000 to 17,500. The
authored pack was 4,677,393 bytes against 6 MiB.

| Item | Budget | Final |
| --- | --- | --- |
| Each structure | 5,000 triangles | 120-2,184 |
| All structures together (every tier) | 30,000 triangles | 8,816 |
| Each furniture variant | 600 triangles | 88-440 |
| Furniture instances: Low / Medium / High / Ultra | 10 / 40 / 80 / 120 | 2 / 10 / 18 / 24 |
| Furniture triangles drawn per tier | 4k / 10k / 20k / 30k | 288 / 2,092 / 3,848 / 4,780 |
| New textures | none: vertex colours, numerals in geometry | none |
| Draw calls | 20 Models: 14 structures with 1-5 material subsets each, 6 instanced variants | 42 subsets at Ultra |
| Authored pack | 8 MiB (was 6 MiB) | 8,092,112 bytes (96 % of it) |

The pack budget rises for the dressing's `.blend` source (1.29 MB) and its
GLB (0.64 MB), still far below ADR 0011's 100 MB tripwire; the next
modelled asset will need the budget raised again, deliberately. The furniture
draws no shadow; the structures cast and receive at High and Ultra, the
island and the boards' backs receive only, as the exporter records.

### Sources of truth

| Asset | Kind | Source |
| --- | --- | --- |
| `rowing-dressing.blend` | modelled source (MIT) | itself: authored in Blender and judged by eye in Blender and in Qt |
| `rowing-dressing.glb` | derived | exported from the `.blend` by `export_dressing.py` |
| `dressing.json` | derived placement data | exported from the `.blend` by `export_dressing.py`: the structures' classes, shadow roles and footprints, the variants' classes, every furniture instance by tier |
| `rowing-environment.blend` | modelled source (MIT, Phase 3) | edited in place: nine plants on the island's lawn, in the tier collections |
| `vegetation.json` | derived (Phase 3) | re-exported; the environment GLB is byte-identical |

The dressing is the second modelled asset under ADR 0016's source rule, on
the Phase 3 precedent: its first version came from a scripted Blender
session (boxes, bars, lofts and sweeps placed on the loop from the Phase 3
terrain's heights, block numerals from seven strokes), which is not
committed and is not the source; the `.blend` is. Repeated furniture is
instanced: one mesh per variant, placements in tier collections, exported
as a table. No Python CAD framework was written; `export_dressing.py`
models nothing.

### Concept, in Blender and then in Qt

The concept was built in Blender 5.2.2 LTS in the context the replay draws
(`review_environment.py` now draws the dressing in place of the web
structures, with zone views and close-ups), from the approved chase view,
the zone views and the structure close-ups, then judged in Qt on the Apple
M5 (Cocoa/Metal). Three rounds:

1. **The first concept.** Every structure landed where intended, and the
   zones read from the chase camera: the tower and the finish line at the
   approved moment, the numbered boards passing on the bank, the bridge
   overhead, the campus and its flags from across the water. The island's
   lawn had radial colour stripes, the tower's stair was a solid wedge, the
   upturned single floated over its slings, and the bridge deck was a brown
   slab: all four were fixed in the source.
2. **What the export refused.** The committed exporter caught what the
   renders had not: reeds under the coaching pontoon and the boardwalk's
   ramps, the launch pontoon's ends inside the quay face (a straight 12 m
   pontoon on a 35 m arc has a 0.51 m sagitta; it is 8 m now), a finish
   buoy touching the lane band (r 22.7 m; now 22.25, with the jetty
   shortened to 21.6), the coaching pontoon grounded on the shore (moved to
   264 degrees, the one reed-free stretch of the far bank, and its floats
   made shallower), and the bridge landing 0.7 m above the bank (its deck
   now descends to the bank at r 41.2 m).
3. **In Qt.** The upturned single read as a white plank on legs at Medium
   and above; its hull is now a light warm grey. Nothing else moved.

**Validation of the exporter's rules**, by mutating the committed source in
memory and expecting each defect to be refused by name (a scratch script,
run with Blender's Python): a structure moved into the lane band; a pontoon
sunk, and one moved onto the shore; a building floated 1.2 m over the
ground, and a board buried; a delta transform, a modifier and an unknown
material slot; a variant modelled off the origin; an instance in two tiers,
one tilted, one in the lane band, one floating over its ground, one moved
by a constraint; a plant in the tower's footprint; the island raised 1 m;
the bridge lowered into the lane band; a structure subdivided over its
budget. All eighteen refused, and the unchanged file still exports byte
for byte. The Rust gate refuses eleven drifts of the GLB and placements by
name (`dressing.rs`), and the canonicaliser's shared-index rule and the
export helpers (sectors, the lane band, primitive matching, the placement
file's layout) have their own tests.

### In Qt

`build.rs` gates the GLB through `replay::dressing::validate_dressing`
against `dressing.json`, runs balsam for the mesh files alone (twenty, one
per structure and variant, named as the Phase 3 note predicts), and
generates `DressingScene.qml`: one Model per structure with one material
per primitive in the exporter's class order and the shadow roles it
records; one instanced Model per furniture variant, its `InstanceList`
sorted by tier so `instanceCountOverride` selects a prefix, and hidden by a
binding at a tier with no instance. `RowingDressing.qml` declares the six
class materials (paint, timber, metal, glass, float, and the island's
ground, which is the environment's land material). Nothing walks the
dressing at runtime. The web rower venue's structures and island are hidden
by the existing walk's name list, which grew by eleven names; no walker was
added or broadened, and the walk still builds the hidden nodes' materials,
so the gate's venue texture counts hold.

### Runtime invariants

`compare.py` checks all 18 controlled states per scheme against the Phase 3
parent: pose frame without its sequence counter, grip table and tier. They
match exactly in light and in blue hour, and so do the 33 lap and zone
states of the audit probe (51 states per scheme in all) and the 25 states
of the 12 s sequence. The same replay state, grabbed at two wall-clock
times (`style-medium` and `motion-000`), renders 0 differing pixels before
and after in both schemes: nothing in the dressing moves on its own clock.
The near-water patch's first-to-last change over the 3 s sequence is
0.0098 before and after in light: the dressing does not touch it.

### Shadows

The structures cast at High and Ultra. The gate's shadow check (the
replay's first frame at High, looking across the campus at the tower, the
launch pontoon and the jetty, against its twin with the light's shadow off)
reads light **2.44 % / 8.68 %** (93,552 px) and blue hour **2.39 % /
10.46 %** (91,711 px), from Phase 3's 1.05 % / 7.83 % and 1.12 % / 8.13 %.
The mask shows why: the tower's frame and cabin shade themselves and the
quay behind them, the pontoon and the jetty shade the water, and the
rower's own shadow is what it was. The 1 % floor and the 5 % darkening
stand, untouched.

### Determinism

Two exports from the unchanged `.blend` produced the same GLB, placements
and manifest byte for byte (Blender 5.2.2 LTS on this Mac). The environment
re-exported byte-identically after its exporter changed to read the
dressing's footprints; planting the island then changed `vegetation.json`
by nine lines and the environment GLB not at all.

### Metal results (Apple M5, macOS 27.0, Qt 6.11.2, Rust 1.98.1)

All four tiers in light and blue hour were inspected at the approved
moment, in the zones and round the lap, before and after, on the same
replay states. From the chase camera the venue now answers the five
questions the phase set out to: the buoy rings and the finish line say
where the course is; the pontoon, the slings and the boathouse say where
crews launch; the tower, its boom and the jetty say where the lap is timed;
the bollards, benches, railings, flags and the single on its slings give
the quay and the island human scale; the boards, the bridge, the coaching
pontoon and the hide give each stretch of the lap its own landmark. The
athlete and the shell stay primary: nothing stands in the lane band, the
tower is 33 m from the approved camera where the web's was, and the
furniture sits at the water's edge rather than filling the frame. Blue
hour keeps the tinted structures as dark silhouettes against the lighter
sky, the flags and the finish board readable. Low is the venue's
silhouettes without furniture, not a broken scene; Ultra adds a few
bollards and benches, not clutter.

The web's island is gone with its bare disc; the authored one, with its
trees, jetty and bridge landing, shows when a ghost pulls the camera back,
and the pulled-back view now reads as a lagoon course rather than a buoy
circle on a lake.

### #121

The gate's SkiErg capture on this branch shows the snow field, the apron
and the horizon band and none of the venue's instanced pines, berms or
foothills, so the discrepancy the issue found is the instanced groups as a
class, for every sport, not the rower venue's woodland. For rowing it is
superseded by Phase 3 (the woodland it saw missing is hidden and replaced)
and completed by Phase 4 (the island's trees and shrubs and the distance
posts, the last two instanced groups the rower scene relied on, are
replaced, and nothing of the rower bake is drawn). It stays open for the
SkiErg and BikeErg venues, whose instanced groups go through the same
`applyInstanceGroup` path; that path was not instrumented here, and the
cause is not established. Recorded on the issue without a closing keyword.

### Limitations

- **Style.** Stylised hard-surface geometry with vertex colours: no
  textures, no signage beyond the numerals, flags that do not move.
- **Only the rower bake is replaced.** SkiErg and BikeErg keep their baked
  venues, and #121's instancing path with them.
- **The athlete** looks weaker beside a proper venue: the seat still sits
  low and the blades never square (#130, #129). That is evidence for Phase
  5, not something fixed here.
- **GPU time** was not measured (the Metal HUD logs nothing, the bench is
  vsync-capped on Cocoa), and no laptop iGPU was tested.
- **The web venue GLBs still load** for rowing, wholly hidden, and the walk
  still builds their materials; unloading them is a ReplayScene refactor
  item, out of scope here.

### Motion

The 12 s real-replay sequence (demo 1001 from 208.829 s, every 0.5 s, Medium;
the same 25 states on both sides, checked frame by frame) shows the tower
and its boom growing as the boat approaches the line, the launch pontoon
and the quay furniture sweeping past faster than the tower behind them,
and the jetty drifting across the left of the frame: the near structures
give the parallax the web's builder wanted from its span, the far ones
move slowly, and nothing animates. That is a visual reading of the frames,
not a perceived-speed number; the sequence, its clip and a five-frame strip
are on the evidence branch.

### Performance (Apple M5)

Measured with Phase 3's method: real playback of demo 1001 from 0.45 of the
workout, Qt's `QSG_RENDER_TIMING` GUI-thread intervals (`polishAndSync`,
"elapsed since last call") over a 24 s steady window starting 8 s after
play, `ps` every 2 s, and the Phase 3 parent run back to back from its own
worktree. This is GUI cadence at the display's 120 Hz, not GPU timing. GPU
time was not measured: the Metal HUD's log mode writes nothing here, and
the repository's bench stays vsync-capped on Cocoa.

| Window | Samples | Median | p95 | Max | > 50 ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Parent, Medium, light | 2,875 | 8 ms | 10 ms | 17 ms | 0 |
| Phase 4, Medium, light (first run) | 2,528 | 8 ms | 9 ms | 2,684 ms | 2 |
| Parent, Medium, light (reruns) | 2,879 / 2,868 | 8 / 8 ms | 10 / 10 ms | 20 / 56 ms | 0 / 1 |
| Phase 4, Medium, light (reruns) | 2,876 / 2,876 | 8 / 8 ms | 10 / 10 ms | 15 / 14 ms | 0 / 0 |
| Parent, High, light | 2,877 | 8 ms | 9 ms | 17 ms | 0 |
| Phase 4, High, light | 2,880 | 8 ms | 9 ms | 12 ms | 0 |
| Parent, Ultra, light | 2,880 | 8 ms | 9 ms | 16 ms | 0 |
| Phase 4, Ultra, light | 2,881 | 8 ms | 9 ms | 11 ms | 0 |
| Parent, Medium, blue hour | 2,880 | 8 ms | 10 ms | 14 ms | 0 |
| Phase 4, Medium, blue hour | 2,878 | 8 ms | 10 ms | 34 ms | 0 |

The first Phase 4 Medium window held one 2.7 s interval and lost 350
samples to it. Two back-to-back reruns per side did not reproduce it (the
Phase 4 maxima were 15 and 14 ms; one parent rerun had a single 56 ms
interval), so it is recorded as unexplained, not as a cost of the dressing
and not as a flake: nothing in the log names its cause, and this Mac was in
use for other work during the chain. No other window shows a stall. CPU in
the steady windows ranged over 15-91 % of one core on both sides. Resident
memory is flat within every window and not stable between runs (Phase 3
measured the same spread): 332-432 MiB for Phase 4 against 351-441 MiB for
the parent across the light windows, in the same range. For a laptop iGPU
the added cost is the budget table's 8,816 structure triangles and up to
4,780 furniture triangles over 20 Models (42 material subsets at Ultra),
plus the structures' shadow casting at High and Ultra. That is an
estimate, not a measurement on such a GPU.

### Validation (Apple M5, macOS 27.0, Qt 6.11.2, Rust 1.98.1, Blender 5.2.2 LTS)

- 28 pipeline tests with Blender's Python (`test_canonical` with the
  shared-index case, `test_probes`, `test_water`, `test_export_environment`,
  `test_export_dressing`).
- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets
  -- -D warnings`; `git diff --check`.
- Qt-free tests: 597 passed, 2 existing ignores.
- The asset tests: the dressing's manifest section against the source's
  SHA-256, every budget pinned, the triangles recounted from the GLB against
  the placements, the placements' layout and lane clearance; the vegetation
  test admits the island's lawn; the provenance rows for the three new
  files.
- The exporter's eighteen refusals and the validator's eleven (above); the
  environment's export, re-run, reproducing its GLB byte for byte.
- The full native gate walk, light and blue hour, with phase shots and
  close-ups: all three gate tests, 108.3 s and 113.5 s of app log, the
  replay's first frame 0.3 s after step 52, no hold starved.
- Both native Cocoa/Metal app suites (`cargo test -p rowplay-app` with the
  smoke screenshot, phase shots and close-ups): 33 passed each in light and
  in blue hour, the gate walks at 112.2 s and 111.7 s, no hold starved.
