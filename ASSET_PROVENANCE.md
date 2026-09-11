# Asset provenance

Every visual asset rowplay-qt ships is either authored inside the rowplay
repositories or a CC0 material map with recorded provenance. No downloaded
human model, third-party character, scan, likeness, avatar-generator output or
user image contributes to any shipped asset (policy inherited from rowplay's
`static/replay-assets/README.md` and rowplay-studio's `ASSET_PROVENANCE.md`).

Runtime 3D assets are glTF 2.0 `.glb` only (ADR 0003). Image-based lighting is
procedural (ADR 0004): no HDRI file, downloaded, imported or scanned, ever
enters this repository.

## Vendored assets

None yet. Phase 5 vendors `rowplay-rigs-v3.glb` and `rowplay-athlete-v4.glb`
from rowplay; Phase 6 vendors the baked venue `.glb` files. Each entry records:

| Asset | Source repository | Source path | Commit | SHA-256 | Licence |
| --- | --- | --- | --- | --- | --- |

Rules for adding a row:

1. Copy the artifact byte for byte; never re-export or "clean up" a vendored
   file without recording the tool, version and reason.
2. Record the upstream commit and the SHA-256 of the committed bytes.
3. Keep the upstream licence: rowplay assets are MIT (`LICENSES/MIT-rowplay.txt`);
   the Human Base Meshes bundle and Poly Haven maps are CC0-1.0
   (`LICENSES/CC0-1.0.txt`).
4. Attribution and lineage that upstream documents (for example the BikeErg
   `source/bike/PROVENANCE.md` and the Poly Haven creators) are copied here.

## Reference sources (not vendored)

| Source | Commit at bootstrap |
| --- | --- |
| `https://github.com/shenghaoc/rowplay` (`main`) | `011e8303b66b4d2265a6f1ec8b3ed9d8ed497086` |
| `https://github.com/shenghaoc/rowplay-studio` (`main`) | `3d406a5b7677372de35fb0817c7133a2589c6564` |

Golden parity fixtures are tracked separately in `tests/fixtures/PROVENANCE.md`.
