# ADR 0011 — Venues stay in plain Git; the ADR 0009 ceiling is raised to 100 MB with a documented basis

Status: accepted (2026-09-13)

## Context

ADR 0009 kept vendored assets in plain Git with a 50 MB tripwire and said
Phase 6 venues tripping it would force this follow-up ADR "with real numbers".
The venues are now measured.

| Asset group | Files | Bytes |
| --- | --- | --- |
| Phase 5a vendored (rigs, athlete, environments) | 42 | 9,050,376 (8.63 MiB) |
| Phase 6a baked venues + contracts + procedural maps | 28 | 10,127,568 (9.66 MiB) |
| **Total under `assets/replay/`** | **70** | **19,177,944 (18.29 MiB)** |

The largest single file is `rowplay-venue-bike-ultra.glb` at 1,452,864 B
(1.39 MiB). Neither the total nor any file is near the 50 MB tripwire's
per-file or total reading — the venues did **not** trip it, contrary to the
ADR 0009 expectation that they would "grow the tree several-fold" (they grew
it by 2.1×). The naive four-variant, instanced-archetype bake is the reason:
contract-driven instancing keeps one geometry per scatter group instead of
hundreds of expanded copies.

## Decision

Vendored assets **stay as ordinary Git blobs**. No Git LFS.

- The measured total (18.29 MiB) is 2.7× below the existing 50 MB tripwire,
  and the largest file is 34× below GitHub's 50 MB per-file warning.
- ADR 0009's provenance argument still holds and is now exercised by 28 more
  files: the SHA-256 pins want byte-exact files in the checkout, and an LFS
  smudge filter converts "bytes changed" into "filter not installed", a worse
  failure surface for a public repo.
- The tripwire itself is raised from 50 MB to 100 MB, because 50 MB was set
  when the next asset family's size was unknown and it now sits close to the
  order of magnitude venues will reach if a future phase adds Ultra-only
  variants or a fourth sport. 100 MB keeps the "deliberate act" property —
  the next doubling still forces an ADR — without a false trip on a
  legitimate ~20 MB tree. The per-file ceiling stays at 50 MB.

## Consequences

- The ADR 0009 tripwire test (`crates/rowplay-app/tests/asset_hashes.rs`)
  carries the new 100 MB constant and a comment pointing here.
- If a future phase pushes the total past 100 MB, that is again a decision
  point (LFS, or splitting venue packs into a separate repository), taken on
  fresh measurements.
- `venues/MANIFEST.json` plus `tools/vendor-venues.py` make each venue change
  a reviewed, hashed diff in the same spirit as the upstream artifact pins.
