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
least 16x16), so directional detail is reduced and visual acceptance remains open.

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
app visual acceptance remain external gates; the PR stays draft.


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
