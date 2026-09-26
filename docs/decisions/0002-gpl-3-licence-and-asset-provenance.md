# ADR 0002 — GPL-3.0-or-later with SPDX headers and recorded asset provenance

Status: accepted (2026-09-11); amended 2026-09-26 so that common assets we
author for the RowPlay family are MIT.

## Context

Qt Quick 3D and Qt Graphs are available under GPLv3 only in the open-source
Qt edition (the rest of Qt is LGPLv3). rowplay (MIT) and rowplay-studio are
the sources of truth, and rowplay's 3D assets are MIT with a CC0 base mesh and
CC0 material maps whose provenance is recorded upstream.

## Decision

- The repository is licensed **GPL-3.0-or-later** (`LICENSE`). Every source
  file (Rust, QML, scripts, workflows) starts with an
  `SPDX-License-Identifier: GPL-3.0-or-later` header.
- Third-party licence texts live in `LICENSES/` (GPL-3.0-or-later, MIT for
  material vendored from rowplay, CC0-1.0 for the base mesh and material maps).
- Assets and fixtures vendored from rowplay or rowplay-studio keep their
  original MIT / CC0 terms and are listed in `ASSET_PROVENANCE.md` (assets)
  and `tests/fixtures/PROVENANCE.md` (fixtures) with source repository, path,
  commit and SHA-256.
- **Common RowPlay-family assets are MIT** (amended 2026-09-26). A visual
  asset meant for reuse across rowplay, rowplay-studio and rowplay-qt is MIT
  by default when two things hold. First, the owner (shenghaoc, who owns all
  three repositories) holds its copyright. Second, its provenance is clean:
  every input it takes from elsewhere is recorded, and that input's licence
  allows MIT. That covers models and their `.blend` sources, textures,
  generated skies and probes, placement data, and a pack's manifest. Today
  that is everything under `assets/replay/authored/`. The MIT text is
  `LICENSES/MIT-rowplay.txt` (Copyright (c) 2026 shenghaoc).
  - The rule was added so the MIT projects can reuse these assets without
    depending on GPL-only artwork.
  - Ownership and provenance are the reason, not how an asset was made. Being
    generated does not make an asset MIT, and a tool's licence decides nothing
    about its output: a GPL script makes what it generates neither GPL nor MIT.
  - The application's own source stays GPL-3.0-or-later. That covers the Rust
    and QML, the build scripts, and the Blender generation, export and
    validation tooling, including the scripts that produce MIT assets.
  - An input from elsewhere keeps its own terms. Its source and licence are
    recorded even when it is the owner's work in another repository, such as
    a vendored pack's names and dimensions, a constant from the web app, or a
    mesh the asset is measured against. Inputs from rowplay keep their MIT
    notice (`LICENSES/MIT-rowplay.txt`), and CC0 material stays CC0.
  - Provenance is mandatory. Every asset has an entry in `ASSET_PROVENANCE.md`
    naming its licence, its inputs and the tooling that made it. An asset
    whose ownership or inputs are unclear is not relicensed by assumption.
  - A text asset that can carry a comment carries
    `SPDX-License-Identifier: MIT`. Binary assets and JSON data carry no
    header; their licence is their provenance entry.
- No downloaded human model, third-party character, scan, likeness, avatar
  generator output or user image ever enters the repository (carried over
  from rowplay's provenance policy).

## Consequences

- `qtbridge` itself is `LicenseRef-Qt-Commercial OR LGPL-3.0-only`; linking it
  and Qt into a GPLv3 application is permitted.
- Contributors must add the SPDX header to new files; the PR template checks it.
- A common asset's licence follows its ownership and its inputs, not the
  repository or the tool that made it.
  `crates/rowplay-app/tests/asset_hashes.rs` fails when a file under
  `assets/replay/authored/` has no MIT row in `ASSET_PROVENANCE.md`. Review
  checks the recorded inputs.
