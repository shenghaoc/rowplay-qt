# Phase 5.4 candidate evidence — not acceptance

The V5 candidate is blocked before Layer C. See
[the report](../../blender-phase5-athlete-rebuild.md) for the fixed-posture
BikeErg reach proof, remaining deformation defects, and owner decision needed.
No final contact tolerances or Phase 5 acceptance are claimed.

- `light/`: native Wayland/OpenGL, Mesa software rendering, Qt 6.11.2,
  Medium tier, all 11 Row/Ski/Bike phases; 88 captures, 22 hand masks.
- `blue-hour/`: the same renderer, High tier, row catch / ski release /
  bike quarter; 24 captures, six hand masks.
- Each `manifest.json` pins the exact committed capture/analysis tools,
  approved base, modelled source, GLB, contract, equipment, production QML,
  temporary instrumented QML and the current Cargo executable/OUT_DIR outputs.
  The source commit is `5f7f0c6` (full SHA in the manifests); it descends from
  the asset-free ADR 0018 governance commit.
- `frames.json.gz` contains the native joint, equipment, camera and rig
  transforms, viewport, 225-float frame, helper state and replay state.
  `materials.json.gz` contains the eight actual Qt material instances and
  their full recorded state. Neither archive re-derives replay poses.
- Raw native hand-mask PNGs are unchanged. `raw-capture-hashes.json` pins
  all original full-size PNGs, including non-retained production close-ups.
  The labelled JPEG panels are review derivatives; measurements use raw
  matrices and masks, never JPEG pixels.
- `skin-validation.json` records minimum native/CPU hand silhouette IoU
  0.987858 (light) and 0.988303 (blue-hour), above the existing .97 instrument
  requirement. Anatomical hand masks are independent of weight values.
- `deformation.json` uses the same singular-value method and dominant-bone
  region rule as the immutable Phase 5.2 audit. Weight changes move region
  boundaries; compare native contours as well as the numerical table.
- `reach-bound.json` is a conservative triangle-skin bound using actual
  inverse binds, all palm vertices, normalized skin weights and the actual
  V3 hood mesh. The quarter-cycle bound leaves at least 18.821/18.927 mm.
  It permits every descendant rotation and therefore exceeds credible
  anatomical reach. The fixed shoulder posture is an explicit premise.
- `skin.json.gz` records the initial all-phase, bilateral, per-digit actual
  mesh distances and separately labelled signed cylinder diagnostics.
  These are unaccepted candidate measurements; near-zero distance can be
  penetration. The sampled surface-cover bound is ≤1 mm.

To reproduce analysis on this exact source commit, copy one scheme directory
to a fresh scratch directory and decompress `frames.json.gz` there to
`frames.json`. Run `python3 tools/blender/athlete_v5_skin.py <scratch>`
(add `--contacts` for the expensive all-digit triangle distances). The tool
rechecks instrument ancestry/input hashes and the native silhouettes before
interpreting geometry. Raw Cargo JSON/log archives document capture, rather
than requiring that a later build leave those temporary binaries untouched.

The `.blend`, GLB and contract hashes live in both capture manifests and the
authored asset manifest. Repeated export under Blender 5.2.2 is byte-identical
for GLB and contract; `validation.txt` records the commands and test results.
Historical Phase 5.1/5.2 evidence and V4 bytes are unchanged.
