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
