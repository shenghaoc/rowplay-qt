# ADR 0009 — Vendored assets stay in plain Git, with a size tripwire before Phase 6 venues

Status: accepted (2026-09-12)

## Context

Phase 5a vendored 42 files (8.8 MB) under `assets/replay/` — the V3 rig pack
(733,864 B), the V4 athlete (4,584,320 B) with its contract JSON and 13
Poly Haven texture families — each pinned by path, byte count and SHA-256 in
`crates/rowplay-app/tests/asset_hashes.rs` and tabled in
[ASSET_PROVENANCE.md](../../ASSET_PROVENANCE.md) (ADR 0002). Phase 6 adds
baked venue `.glb`s (ADR 0005) to the same directory and will grow it
several-fold, so the storage mechanism has to be chosen before that lands:
moving to Git LFS later rewrites history.

Git LFS changes what a clone contains. Without `git-lfs` installed and
fetched, a checkout holds pointer text files, not bytes — the byte-count
half of the hash test fails immediately, and CI needs `lfs: true` on
`actions/checkout` (and on every contributor's clone) for the pins to hold
at all.

## Decision

Vendored assets stay as ordinary Git blobs.

- 8.8 MB is two orders of magnitude below GitHub's 50 MB per-file warning
  and 100 MB hard limit, and cloning the repo stays trivial without a
  third-party tool. The venue budgets (ADR 0005's baked venues, a few MB
  each) do not change that order of magnitude.
- The provenance system depends on byte-exact files in the checkout. Plain
  Git serves the SHA-256 pins directly; LFS interposes a smudge filter that
  turns "bytes changed" into "filter not installed" — a strictly worse
  failure surface for a public repo whose tests must fail loudly on a
  silent asset swap.
- A tripwire keeps the decision deliberate: `asset_hashes.rs` fails when the
  total size under `assets/replay/` exceeds 50 MB. Phase 6 venues tripping
  it is the signal to write a follow-up ADR (LFS, or splitting venue packs
  out) with real numbers — not to grow the tree silently.

## Consequences

- CI needs no `lfs: true`; contributors need no `git-lfs`.
- The repository carries the full history of every asset bump; large asset
  updates are commits to be scoped deliberately (one asset-family per
  commit, provenance table refreshed by `tools/vendor-replay-assets.py`).
- The 50 MB tripwire is a ceiling on "plain Git", not a target; exceeding it
  reopens this ADR rather than switching automatically.
