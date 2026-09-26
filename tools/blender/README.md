# Blender asset pipeline

Direction C (overcast championship morning) is canonical. Blue-hour overcast is
the dark companion. Direction A is only a restrained depth reference; B is only
a Low-tier simplification reference. See ADR 0016.

## Build

Source `.envrc` for Qt 6.11.2, then:

```sh
make blender-assets BLENDER="$HOME/opt/blender-5.2.1-linux-x64/blender"
# macOS: BLENDER=/Applications/Blender.app/Contents/MacOS/Blender
make blender-shell BLENDER=...   # the shell and oars alone (Phase 2)
make blender-water BLENDER=...   # the water normal alone (Phase 3)
make blender-environment BLENDER=...   # export the environment from its .blend (Phase 3)
make blender-dressing BLENDER=...   # export the course dressing from its .blend (Phase 4)
```

Each `--only` mode rebuilds one part and re-pins just its files in
`MANIFEST.json`, so the other assets keep the bytes of the machine that made
them.

The target uses factory startup and `--python-exit-code 1`. `build_all.py`
rebuilds the entire new authored pack, including both skies, Qt-prefiltered
probes, the water normal map, buoy mesh, placement records and hash manifest.
The pre-existing vendored athlete/equipment/venues remain governed by their
existing vendor scripts; this target does not silently replace them.

Blender tested here: 5.2.1 LTS, Linux, build `9e2066aef7ef`. Qt/balsam: 6.11.2.
The original generation run was Linux; subsequent native Mac runtime acceptance
is recorded in `docs/blender-audit.md` (no Blender regeneration on the Mac). Randomness uses seed 20260926; CPU Cycles
and one render thread bound preview variability. Cross-version byte identity
is not assumed: changing Blender, Qt or the probe baker's GPU/backend requires
regeneration and review. The reproducibility check here uses llvmpipe for both
independent builds.

Blender 5.2.1 can emit the same opaque mesh with different triangle ordering.
`canonical.py` normalises triangle order and cyclic vertex order without
reversing winding or changing any vertex attribute. It refuses non-opaque
primitives, where draw order could matter. Its supported input is deliberately
one unextended opaque mesh primitive/material with an embedded buffer and
disjoint, tightly packed views. Mixed primitives, transmission/extensions,
index/attribute aliasing and out-of-view indices fail before writing. Regression
tests check winding, idempotence, these refusals and the committed buoy's bytes.
The shell pack uses `canonicalize_pack`, the same contract applied to every
primitive of a multi-mesh pack, then `bound_attributes`. Blender writes one
index accessor for primitives of identical topology (the dressing's boxes and
posts, whatever their vertices), so `canonicalize_pack` sorts a shared index
accessor once and requires every primitive sharing it to hold the vertex
count it addresses. That writes min/max
on every vertex attribute: Blender's exporter bounds only POSITION, and the
V3 rules the app enforces require finite bounds on every attribute.

`common.py` owns metre units and the Blender Z-up to glTF Y-up conversion.
One glTF unit is one Qt scene metre; there is no 100x import scale. Qt built-in
Rectangle is 100 units wide, explicitly accounted for by `RowingWater.qml`.

## Shell and oars (Phase 2)

`shell.py` generates `rowing-shell.glb`, the single scull and its sculls: the
hull, canvases, cockpit, gunwales, riggers, oarlocks, sliding seat and tracks,
foot stretcher, fin, and the oar with its shaft, grip, button, sleeve and
hatchet blade. It is a procedural asset under ADR 0016's source rule, so the
script is its source and no `.blend` is committed. Every dimension that binds
to the rig is named after the Rust constant it must match: the 7.8 m shell,
the oarlock pivots, the grip the hands close on, and the blade leaf's origin.

- **Contract.** The pack carries the V3 pack's three rowing template roots and
  the blade leaf. They use the same node names, material roles and composite
  rules. `build.rs` checks this against the vendored pack on the exact bytes
  balsam converts (`rowing_shell::validate_rowing_shell`), and recounts the
  triangles as drawn: the boat and seat once, the oar rig and blade once per
  side. The app hides the V3 pack's rowing nodes and walks these instead.
  The V3 pack stays vendored and pinned.
- **Fit.** The seat, rails, floor and stretcher are fitted to the athlete the
  app draws. The V4 athlete was skinned with the replay's own pose frames over
  one stroke, and the fit is recorded in `docs/blender-audit.md`, "Phase 2".
  The generator refuses a cockpit part that pokes through the hull at any seat
  position, and a fin that shows above the water at the top of the bob.
- **Left blade.** The blade has a handedness. The app reflects the left leaf
  in its own z and gives it a front-face-culled copy of the paint material.
  `docs/qt-bridges-notes.md` explains why double-sided would be wrong.
- **Wet band.** The hull's vertex colours scale the carbon material's roughness
  (red) and clearcoat roughness (green) toward the waterline. There is no
  texture and no UV set; parts without the colour attribute are unaffected.
- **Budget.** `build_all.py` refuses more than 60,000 triangles as drawn
  before export, and `build.rs` refuses it again from the GLB.

`make blender-shell` rebuilds only this pack and re-pins it in `MANIFEST.json`
(its `shell` section records the Blender version and the triangle counts). The
other authored assets keep their Linux build. The shell was generated on macOS
with Blender 5.2.2 LTS; two independent builds there were byte-identical. The
previews are `build/previews/shell-contact-sheet.png` (top, side, front and
three-quarter views, posed as the app clones the oars) and
`build/previews/shell-details.png` (close-ups).

## Environment and water (Phase 3)

**Water** (`water.py`, procedural). A periodic height field on an 8 m tile:
360 integer wave vectors from a wind-sea spectrum, 6 cm to 2.8 m, spread about
a 28-degree wind with a second direction 74 degrees away, and an 18 % seeded
amplitude variation. The slopes are scaled to an RMS of 0.11, chosen in Qt
(`docs/blender-audit.md`, "Phase 3"). `RowingWater.qml` repeats the tile every
8 m: it is fixed in the world and never scrolls. `build_all.py` writes the
normals through Blender's image API. `test_water.py` checks that the field
tiles, its RMS, its spectrum, and that no component or direction dominates.

**Environment** (a modelled asset: `assets/replay/authored/rowing-environment.blend`
is its source, MIT). `export_environment.py` does not model; it opens the
file, validates it and writes `rowing-environment.glb` (one mesh per part,
vertex colours, no materials) and `vegetation.json` (one instance per line,
sorted by variant and tier). The file's contract:

- a collection `rowplay-environment` holding `land` (`environment:row:terrain`,
  `far-bank`, `woodland`), `vegetation-variants` (the six variants) and
  `vegetation-low` to `vegetation-ultra`;
- every mesh named like its object, unparented, with an identity world
  transform (delta transforms and constraints count) and no modifier, and
  carrying a point colour attribute `Col` (the albedo, linear);
- every instance a linked duplicate of a variant, turned about +Z only, scaled
  uniformly, tinted by its object colour, and in exactly one tier collection.
  An instance shown at Medium sits in `vegetation-medium` and also shows at
  High and Ultra. The placements are read from its location, Z rotation and
  scale, so its world transform must agree with them to 1 mm.

It refuses:

- a part over its budget, or a tier over its instance or triangle count;
- a variant with no Low instance;
- land at or above the water inside 36.2 m, checked on every triangle, not
  only at its vertices (the outer buoy ring is at 33.3 m);
- anything planted inside a retained web venue structure, or ground at a
  structure's middle outside its range. The footprints are annular sectors
  read from the vendored Ultra venue.

To edit the environment, open the file in Blender 5.2, change it (sculpt the
bank, move or re-tier a tree, retint an instance), save it, and run `make
blender-environment`. The manifest's `environment` section then records the
file's SHA-256, the triangle counts, the instances per variant and tier, and the
budgets; the asset tests check all of them, recount the triangles from the
GLB and pin every budget. `build.rs` checks the GLB itself
(`replay::environment::validate_environment` in rowplay-viewmodel): the nine
meshes on flat, untransformed nodes, triangle lists with vertex colours, and
no materials. Two things trip the exporter:

- **Shared topology.** Blender's exporter shares identical index buffers
  between meshes, which `canonical.py` refuses, so each variant needs its own
  topology.
- **Icosphere subdivisions.** `bmesh.ops.create_icosphere` counts them from
  one: 3 is 320 faces.

**Review renders** (`review_environment.py`, Blender only, not acceptance
evidence). They put the committed `.blend` in the context the replay draws it
in: the Phase 2 shell at the approved moment, the buoys, the web venue's
retained structures, the water, the authored sky, and Qt's depth fog
reproduced in every material. Views: `chase`, `lap` (eight chase views round
the loop), `wide`, `top`, `bank` and `water`.

```sh
blender -b --factory-startup -P tools/blender/review_environment.py -- \
    --output build/previews/environment-light --scheme light
```

## Course dressing (Phase 4)

**Dressing** (a modelled asset: `assets/replay/authored/rowing-dressing.blend`
is its source, MIT). `export_dressing.py` does not model; it opens the file,
validates it and writes `rowing-dressing.glb` (one mesh per structure and
per furniture variant, its primitives split by material class, vertex
colours, no materials) and `dressing.json` (the structures' classes, shadow
roles and footprints, the variants' classes, and every furniture instance,
one per line, sorted by variant and tier). The file's contract:

- a collection `rowplay-dressing` holding `structures` (the fourteen in
  `STRUCTURES`: the finish tower, the start jetty, the launch and coaching
  pontoons, the course bridge, the clubhouse, boathouse and regatta office,
  the wetland boardwalk and hide, the three distance boards, the island),
  `furniture-variants` (the six in `VARIANTS`: bollard, bench, life-ring,
  flagpole, trestle-hull, finish-buoy) and `furniture-low` to
  `furniture-ultra`;
- every structure a mesh named like its object, modelled where it stands,
  unparented at the identity world transform, without modifiers, with a
  point colour attribute `Col` and material slots named `paint`, `timber`,
  `metal`, `glass`, `float` or `ground` (each used by a face); every variant
  the same, modelled at the origin;
- every furniture instance a linked duplicate of a variant, turned about +Z
  only, scaled uniformly (0.5 to 2), tinted by its object colour, in exactly
  one tier collection, and where its own transform channels put it.

It refuses: a structure or variant over its budget, or a tier over its
instance or triangle count; anything solid inside the lane band (r 22.8 to
33.6 m, the buoy rings and the blades' reach) below the bridge's 4.2 m
clearance; a land structure whose base does not meet the Phase 3 terrain
(read from `rowing-environment.blend`) within -0.5 and +0.45 m, or that
reaches deeper than a driven pile; a water structure whose floats are not
0.2 to 0.6 m under the water or that is grounded; a bridge that does not
land on the bank; an island outside 16.5 m; a plant (`vegetation.json`)
inside a structure's footprint, or on the island's beach. The footprints
are annular sectors from the structures' vertices, and the environment's
export reads them back from `dressing.json`, so the two sources agree.

After export it reads the GLB back and matches every primitive to its
material slot by the sorted centroids of its triangles, which survive the
export and the canonicalisation unchanged, and records the classes in
primitive order: the scene passes one material per primitive in that order.
An unmatched or ambiguous primitive refuses the export.

To edit the dressing, open the file in Blender 5.2, change it (move a
bollard to another tier, re-plank a deck, reshape the tower), save it, and
run `make blender-dressing`. The manifest's `dressing` section then records
the file's SHA-256, the triangle counts, the instances per variant and tier,
and the budgets; the asset tests check all of them, recount the triangles
from the GLB against the placements and pin every budget. `build.rs` checks
the GLB against the placements (`replay::dressing::validate_dressing` in
rowplay-viewmodel), converts it with balsam for its mesh files alone, and
generates `DressingScene.qml`: one Model per structure with its shadow
roles and one material per primitive, one instanced Model per variant
whose `InstanceList` is sorted by tier so `instanceCountOverride` selects a
prefix; a variant with no instance at a tier is hidden by a binding.
`RowingDressing.qml` declares the six class materials.

**Review renders** (`review_environment.py`) draw the dressing in place of
the web venue's structures since Phase 4, with a `zones` view set (launch,
finish, bridge, far bank, coaching pontoon, wetland) beside the Phase 3 ones.

## Outputs and budgets

Committed build outputs live in `assets/replay/authored/`, with their hashes in
`MANIFEST.json`; `course.json` holds one buoy per line, `vegetation.json` one
plant per line and `dressing.json` one furniture instance per line. The
environment's and the dressing's `.blend` sources are pinned there too. The outputs are MIT
common assets and these scripts are GPL-3.0-or-later tooling (ADR 0002). Previews and Qt's full-resolution intermediate probes go under
gitignored `build/`. No downloaded texture or third-party logo is required;
the two `.blend` files are the environment's and the dressing's own sources. Generated source provenance is in `ASSET_PROVENANCE.md`.

| Asset | Limit |
| --- | --- |
| Shell plus oars | 60,000 triangles as drawn; 1K PNG textures (none used) |
| Each new environment prop | 5,000 triangles, 1K textures |
| Environment terrain / far bank / woodland (Phase 3) | 16,000 / 8,000 / 6,000 triangles |
| Each vegetation variant (Phase 3) | 800 triangles; no textures |
| Vegetation instances, Low / Medium / High / Ultra | 100 / 200 / 300 / 400 |
| Vegetation triangles drawn, Low / Medium / High / Ultra | 60k / 90k / 120k / 150k |
| Each dressing structure / all structures (Phase 4) | 5,000 / 30,000 triangles; no textures |
| Each furniture variant (Phase 4) | 600 triangles |
| Furniture instances, Low / Medium / High / Ultra | 10 / 40 / 80 / 120 |
| Furniture triangles drawn, Low / Medium / High / Ultra | 4k / 10k / 20k / 30k |
| Water detail | 512 square, periodic tangent-space normal PNG |
| Sky source | 512 x 256 linear HDR |
| Runtime IBL | 128 square per face, six Qt-prefiltered levels |
| Entire authored pack | 8 MiB (4 MiB before Phase 3, 6 MiB before Phase 4) |
| Entire replay inventory | 100 MiB (ADR 0011) |

The existing athlete is retained, not rebuilt; the discarded Phase 5's 40k
budget does not apply. Export checks mesh triangles; generation enforces the
pack size; Rust tests verify hashes, inventory, placement radii and budgets.
The course buoys are 256 instances of one shared mesh, not a combined mesh,
and the furniture 24 instances of six.

Run `python3 tools/blender/audit.py --output docs/blender-asset-inventory.md`
to inventory every model/texture and count exported triangles. The pipeline
tests (canonicalisation, KTX layout, the water field, the environment and
dressing exports' helpers) run with Blender's bundled Python, which supplies
NumPy:
`<blender-dir>/5.2/python/bin/python3.13 -m unittest discover -s tools/blender`.

`probes.py` validates Qt's exact KTX layout before independently box-reducing
every mip. The little-endian RGBA16F 512-square, six-face/six-level layout,
version-1 Qt baker metadata and complete payload length are pinned explicitly;
new baker output requires review. All supported face sizes are four-byte aligned,
so KTX1 face/mip padding is zero. Simply dropping large mips would relabel
roughness and is wrong. See the source-first closeout in `docs/blender-audit.md`.
Runtime loads the prefiltered KTX directly. PNG normal maps are uncompressed
glTF-compatible inputs; Draco, meshopt and KTX2 are not used.

Low/Medium use key strengths 0.85 (light) / 0.5 (blue hour) with no shadows.
High/Ultra use 1.2 / 1.4 so the shadow signal remains readable above the
unchanged gate floor under the brighter IBL. Exposure and sky assets do not
change across tiers. This is an intentional tier approximation, not a different
time of day; subsequent Apple M5/Metal acceptance is recorded in
`docs/blender-audit.md`.

## Importer compatibility

`python3 tools/blender/probe_import.py` converts a generated GLB with clearcoat,
transmission, IOR and a PNG normal map, then fails if their Qt material
properties disappear. Passed on Linux balsam 6.11.2. The shipped athlete also
converts to a Qt Skin with joints and is exercised by the replay captures.
The runtime uses PrincipledMaterial for base colour, roughness, metalness,
normal strength and clearcoat; no transmission is needed by this pack.

The previous-machine report says morph targets survive; emissive strength,
sheen, KHR_materials_specular and WebP are dropped; Draco and meshopt fail.
Those optional paths have not been revalidated here and are not permitted
inputs to this pipeline. KTX1 is used only as Qt's own prefiltered light probe,
not as a glTF texture extension; KTX2 is not used or claimed supported.
The generated material explicitly enables back-face culling (Blender defaults
to double-sided materials). No macOS compatibility claim is made by this run.

Qt's [IBL documentation](https://doc.qt.io/qt-6/quick3d-ibl.html) describes
offline probe baking. The [instance list documentation](https://doc.qt.io/qt-6/qml-qtquick3d-instancelist.html)
requires InstanceListEntry, used by the generated course component. Blender's
5.2 documentation endpoint was unavailable during this session; API names
were checked against runtime RNA and the shipped glTF exporter instead.

## Review

The build renders a four-view buoy contact sheet to
`build/previews/buoy-contact-sheet.png`. The approved local reference page is
`build/blender-scratch/rowplay-style-frames.html`; its generated files are not
source assets and are not committed.

```sh
source .envrc
QT_QPA_PLATFORM=wayland QSG_RHI_BACKEND=opengl LIBGL_ALWAYS_SOFTWARE=1 \
  python3 tools/blender/capture.py --output build/previews/c-light --scheme light
```

Use `--scheme dark` for blue hour. `--baseline` uses the scene at `main`;
`--baseline-ref <ref>` selects another comparison base. Each output directory
must be new. Do not edit Main.qml or ReplayScene.qml while a capture
is running: the script instruments their in-memory snapshots and restores them
in `finally`. Rebuild the app afterwards to remove the instrumented binary.

The capture uses the existing gate to load demo 1001, dismiss the ghost and
seek to 0.50091 (208.829 s, approximately 4.83 m/s), matching the approved
mid-drive frame. It captures Low through Ultra and 13 quarter-second samples
of real replay time. ART_FRAME records pose, grip and tier for comparison.
Check pixel variance, opaque alpha and inter-frame differences before viewing.
Do not infer visibility from successful file creation or from a green test.

The historical scratch speed proof advanced its orbit at speed/radius,
whereas the app uses distance/1000 turns. It is not evidence for unchanged app
motion. Only same-renderer, runtime-driven sequences qualify for acceptance.

`python3 tools/blender/compare.py <before-dir> <after-dir> --output <comparison-dir>`
checks all 18 runtime states (excluding only the sequence counter), opaque
alpha, nonblank pixels, tier/grip equality and water-band differences. It also
checks that the same replay state renders the same pixels at two wall-clock
times (`style-medium` and `motion-000`), so nothing moves on its own clock. It
creates side-by-side stills and a 4 fps comparison clip. It reads pixels with
Pillow and numpy where they are installed and with ImageMagick otherwise
(the same-state pair is then decoded through FFmpeg), and needs FFmpeg for
the clip; none of these is required by generation or the app build. Ask
before installing them. The water band is a fixed 250 x 230 patch of near
water at (0, 1150); Phase 1's (0, 420) lies across the far bank in the
current layout.
