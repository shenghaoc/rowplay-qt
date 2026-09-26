# Phase 5.0–5.1 evidence

Read the [contact report](../../blender-phase5-contact-audit.md) for scope,
methods, tolerances, classifications and the limits of each claim.

- `main/`: original current-main captures, before any code changes. These do
  not serve as the athlete/contact judgment baseline.
- `seat-baseline/`: #130 corrected, before the two contact fixes.
- `final/`: #130 plus the rower wrist-frame and SkiErg leaf-fit corrections.
- `seat-reference.json`: exact measured before/after carriage/pelvis values.
- `manifest.json`: asset and evidence SHA-256 values.

Each sport JPEG is a labelled contact sheet of held native screenshots:
chase / left / right, one row per phase. JPEGs are for inspection, not pixel
metrics. Each state manifest records hashes of the original held PNG files.
The temporary second asynchronous viewport grabs were discarded.

`frames.json.gz` contains exact native joint/equipment/camera matrices,
frame bundles, sport/tier and installed helper poses. `logical.json.gz`
contains the corresponding Rust probe, including per-digit closure, helper
chain measurements, wrist metrics, residuals and joint locals.
`skin.json` contains the actual-skin measurements. The `*-silhouette.png`
files are lossless native segmentation masks, extracted from the held mask
captures; they are not CPU-generated silhouettes. `skin-validation.json`
records comparison with the CPU reconstruction. Original sources had alpha
255; binary masks preserve their segmentation, not their original alpha data.

With numpy/Pillow installed, from the repository root:

```sh
python3 tools/blender/contact_skin.py docs/evidence/blender05/final --validate-only
```

For complete distance regeneration, copy `final/` to a scratch directory
and run that command without `--validate-only`. It reads the shipped V4
athlete, authored rowing-shell grip, and V3 ski/bike equipment. Their hashes
must match the manifest. No network, Blender, or new asset is involved.

These captures/measurements are generated from this author's existing
repository assets and deterministic demo data under the repository licence.
Asset provenance remains in `ASSET_PROVENANCE.md`; no third-party
human, new mesh, fixture from another pin, or user workout was introduced.
