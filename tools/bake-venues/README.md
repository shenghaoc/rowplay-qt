# Venue baker

Bakes the web app's procedural replay venues (`renderer3dEnvironment.ts`) to
vendored `.glb` assets. See ADR 0005 (bake, don't port) and ADR 0010 (this
route, the seed, the instancing decision).

## Requirements

- Node ≥ 24 (the baker uses `--experimental-transform-types` and a module
  resolve hook; see `resolve-hooks.mjs`).
- `pnpm install` in `reference/rowplay` — the baker imports three.js from
  there. The checkout must be at the pinned commit in `docs/source-map.md`
  (`bake.mjs` records the actual `git rev-parse HEAD` in every contract and
  flags a mismatch with `AUTHORED_AGAINST` for review).
- Blender 4.x/5.x. Set `BLENDER_BIN` if it is not on `PATH`.

## Bake

```bash
tools/bake-venues/bake.sh            # bake, Blender pass, finalize
tools/bake-venues/bake.sh --verify   # also re-bake one variant byte-for-byte
```

`bake.sh` runs three stages:

1. **Bake (Node).** Each of the 12 variants (3 sports × 4 quality tiers) is
   built in its own process: a three.js venue build retains a lot of geometry,
   so one process per variant keeps memory flat. Output goes to
   `build/venues-staging/`.
2. **Blender hygiene pass.** `cleanup.py` over the staged GLBs, single-threaded
   with `PYTHONHASHSEED=0`. Output in `build/venues-cleaned/`, with a JSON
   report.
3. **Finalize.** The cleaned GLBs replace the staged ones and the contract
   inventory and bounds are re-derived from the artifacts (Blender triangulates
   and dissolves degenerate geometry, so the pre-Blender inventory is not the
   source of truth).

To bake one variant in-process (useful when iterating):

```bash
node --experimental-transform-types --import ./tools/bake-venues/register.mjs \
  tools/bake-venues/bake.mjs --only skierg:high --out /tmp/venue
```

## Vendor

```bash
tools/vendor-venues.py            # verify the staging matches the committed manifest
tools/vendor-venues.py --accept   # review a fresh bake's hashes and install
```

The manifest (`assets/replay/venues/MANIFEST.json`) pins every vendored file by
size and SHA-256, mirroring the provenance system for the Phase 5a assets. A
re-bake that changes any byte must be re-reviewed with `--accept`, which prints
the added/removed/modified list. Install refreshes `ASSET_PROVENANCE.md`; the
`asset_hashes.rs` rows are printed by `--emit-rust`.

## Determinism

The bake is seeded (`SEED` in `bake.mjs`, `20260913`): `Math.random` is pinned
before the builder module is imported, so its module-scope `SimplexNoise`
permutation is fixed; all scatter jitter in the web source is already
index-hashed. `bake.sh --verify` re-bakes one variant in a fresh process and
compares bytes. The Blender pass is byte-deterministic with `--threads 1` and a
fixed hash seed. Re-baking the same reference commit reproduces the vendored
files exactly.

## Where the copies live

The baker carries verified copies of two module-private web tables,
`QUALITY` and `ENVIRONMENTS` (each with its source line range in a comment).
Re-review them against the reference when bumping the pin; the contract records
both the pin the baker was authored against and the checkout's actual HEAD.
