# Fixture provenance

Every file in this directory (except this note and `manifest.json`) was copied
verbatim from **shenghaoc/rowplay-studio** at commit `3d406a5b7677372de35fb0817c7133a2589c6564`
(`Tests/RowPlayCoreTests/Fixtures/`). Studio in turn derived them from the
rowplay web app's verified helpers; the `replay-current-main-*` files record the
web commit and source-file hashes they were exported from inside the JSON.

They are the parity oracle for `rowplay-core`: a Rust port is accepted only when
it reproduces these expected values (floats within the per-fixture tolerance the
test states). Never edit a fixture by hand. To refresh, run
`python3 tools/vendor-fixtures.py <path-to-rowplay-studio-checkout>` and record
the new commit here. `manifest.json` lists the SHA-256 of every file and is
checked by `crates/rowplay-fixtures` tests.

Redaction policy: see `Concept2/REDACTION.md` (copied from Studio). No fixture
contains real athlete data, tokens, cookies, or hardware identifiers.

Two fixtures are generated locally rather than vendored:

- `replay-row-phase-parity.json` pins the web avatar's rower stroke-phase
  calibration (seat slide, oar sweep yaw, oar dip roll per cycle) directly
  from the **rowplay web repo** at commit `4d96480e7c6fb382f800555bd3aa463d9fe5b1a6`,
  because the Studio-derived corpus has no phase coverage — the hole that let
  Studio's inverted rower phase port cleanly (the seat driven farthest from the
  feet at the catch). Regenerate with
  `node tools/gen-row-phase-parity.mjs --rowplay-repo reference/rowplay`
  (Node ≥ 23.6, web node_modules present).
- `replay-rig-phase-parity.json` (parity coverage audit stage 2,
  `docs/parity-coverage.md`) sweeps the **composed** web avatar calibration for
  all three sports over the full cycle — rig-local transforms and the V4
  contact landmarks from `renderer3d{Row,Ski,Bike}Avatar.ts` at the pinned
  rowplay commit `011e8303` (384 samples × 2 timing sweeps). Regenerate with
  `node --experimental-transform-types tools/gen-rig-phase-parity.mjs`.

Both record the web source-file SHA-256s inside the JSON, and
`tools/vendor-fixtures.py` preserves their manifest entries. Never hand-edit
either.

| Fixture | Bytes | SHA-256 |
| --- | ---: | --- |
| `Concept2/REDACTION.md` | 1765 | `53d8eca59e87720aabc797b840658c342813ef7d47bf8b30b4e23e49682d7e91` |
| `Concept2/bike-steady.fixture.json` | 855 | `f1dba05f05bc41273d2625c604bffb7be1fb27ecba065c6c8b87029aaf24232b` |
| `Concept2/rower-interval.fixture.json` | 1632 | `c74f5eac23949cb18c3c2b5cd7468183e41f5948f767ed29f2db0b0ec8ec8ef9` |
| `Concept2/rower-steady.fixture.json` | 1059 | `ddcc58c3f1796a17b8b289b3e7c2581c61c7d49ec6973e9836a84a08aebd9d41` |
| `Concept2/ski-steady.fixture.json` | 863 | `07f2a8170f7570e7b84b4722b97f5e96dc2d41c76fe9355006341d5dadb28db0` |
| `duration-band-parity.json` | 6120 | `b25819daecb6458eb1a32d226965aeeb66335d2166028dfdbfaca8cb4d918ee4` |
| `performance-predictor-parity.json` | 1304 | `46cdec99696325feacdd1bd1df89409fdc5704b2fefc2bf34f995ecc2343712d` |
| `replay-current-main-2d.json` | 112327 | `407a4a4db8b4c96dcf0121fec5018f820be5485e6d3111f084c1a98baeea2693` |
| `replay-rig-phase-parity.json` | 861062 | see `manifest.json` — generated locally by `tools/gen-rig-phase-parity.mjs` |
| `replay-current-main-equipment.json` | 61418 | `d403eb47c835dc2c8766ac13f235c7a6313962f2157bf5854e37932921f9d865` |
| `replay-current-main-grips.json` | 55216 | `50940759fe637d35267a3b20eb60f92fb8bc5debff6ce7f43c087b39563b962b` |
| `replay-current-main-motion.json` | 482320 | `e47ffdf5e2332ce86911bda2b8b963f5b3ec559f252feee8aa8aaf9bbfdee230` |
| `replay-race-gap-parity.json` | 3283 | `e526bd38de15812967d573645f6177945a91b7f4ab048cd3378355c7b90987b8` |
| `replay-race-result-parity.json` | 6401 | `98558fd0dbaa9c73f1467e3e7479a23495d226734d1ed01fc38d52b9e2df5ff7` |
| `replay-rival-sources-parity.json` | 5404 | `35e252dcd69a3df8d40fbaa4855ba059f2e1571d25495bf525a256c34f05e04d` |
| `stroke-pose-parity.json` | 2385 | `b72484dfdbc335ae2652a0f86f245c98be6db250424edc60c2805655953d57ae` |
| `replay-row-phase-parity.json` | 29459 | `ec426b6890bce64e0854837b569df42175198cc75e991f6255c2ea2bb446853d` |

## Licence

rowplay-studio is (c) shenghaoc; the fixtures are reused here by the same
author under this repository's GPL-3.0-or-later licence.
