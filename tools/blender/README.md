# Blender asset pipeline

Direction C (overcast championship morning) is canonical. Blue-hour overcast is
the dark companion. Direction A is only a restrained depth reference; B is only
a Low-tier simplification reference. See ADR 0016.

## Build

Source `.envrc` for Qt 6.11.2, then:

```sh
make blender-assets BLENDER="$HOME/opt/blender-5.2.1-linux-x64/blender"
# macOS: BLENDER=/Applications/Blender.app/Contents/MacOS/Blender
```

The target uses factory startup and `--python-exit-code 1`. `build_all.py`
rebuilds the entire new authored pack, including both skies, Qt-prefiltered
probes, the water normal map, buoy mesh, placement records and hash manifest.
The pre-existing vendored athlete/equipment/venues remain governed by their
existing vendor scripts; this target does not silently replace them.

Blender tested here: 5.2.1 LTS, Linux, build `9e2066aef7ef`. Qt/balsam: 6.11.2.
This session has not run on macOS. Randomness uses seed 20260926; CPU Cycles
and one render thread bound preview variability. Cross-version byte identity
is not assumed: changing Blender, Qt or the probe baker's GPU/backend requires
regeneration and review. The reproducibility check here uses llvmpipe for both
independent builds.

Blender 5.2.1 can emit the same opaque mesh with different triangle ordering.
`canonical.py` normalises triangle order and cyclic vertex order without
reversing winding or changing any vertex attribute. It refuses non-opaque
primitives, where draw order could matter. Regression tests check winding,
idempotence and refusal of translucent inputs.

`common.py` owns metre units and the Blender Z-up to glTF Y-up conversion.
One glTF unit is one Qt scene metre; there is no 100x import scale. Qt built-in
Rectangle is 100 units wide, explicitly accounted for by `RowingWater.qml`.

## Outputs and budgets

Committed build outputs live in `assets/replay/authored/`, with their hashes in
`MANIFEST.json`. Previews and Qt's full-resolution intermediate probes go under
gitignored `build/`. No blend file, downloaded texture or third-party logo is
required. Generated source provenance is in `ASSET_PROVENANCE.md`.

| Asset | Limit |
| --- | --- |
| Future hero shell plus oars | 60,000 triangles, 2K textures |
| Each new environment prop | 5,000 triangles, 1K textures |
| Water detail | 512 square, periodic tangent-space normal PNG |
| Sky source | 512 x 256 linear HDR |
| Runtime IBL | 128 square per face, six Qt-prefiltered levels |
| Entire authored pack | 4 MiB |
| Entire replay inventory | 100 MiB (ADR 0011) |

The existing athlete is retained, not rebuilt; the discarded Phase 5's 40k
budget does not apply. Export checks mesh triangles; generation enforces the
pack size; Rust tests verify hashes, inventory, placement radii and budgets.
Course dressing is 256 instances of one shared buoy mesh, not a combined mesh.

Run `python3 tools/blender/audit.py --output docs/blender-asset-inventory.md`
to inventory every model/texture and count exported triangles. The KTX layout
tests run with Blender's bundled Python (which supplies NumPy):
`<blender-dir>/5.2/python/bin/python3.13 -m unittest discover -s tools/blender`.

`probes.py` validates Qt's exact KTX layout before independently box-reducing
every mip. Simply dropping large mips would relabel roughness and is wrong.
Runtime loads the prefiltered KTX directly. PNG normal maps are uncompressed
glTF-compatible inputs; Draco, meshopt and KTX2 are not used.

Low/Medium use key strengths 0.85 (light) / 0.5 (blue hour) with no shadows.
High/Ultra use 1.2 / 1.4 so the shadow signal remains readable above the
unchanged gate floor under the brighter IBL. Exposure and sky assets do not
change across tiers. This is an intentional tier approximation, not a different
time of day; its cross-renderer appearance still needs macOS verification.

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
alpha, nonblank pixels, tier/grip equality and water-band differences. It
creates side-by-side stills and a 4 fps comparison clip. This optional review
tool uses ImageMagick and FFmpeg already installed on this Linux host; neither
is required by generation or the app build. Ask before installing them elsewhere.
