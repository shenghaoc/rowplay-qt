# ADR 0002 — GPL-3.0-or-later with SPDX headers and recorded asset provenance

Status: accepted (2026-09-11)

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
- No downloaded human model, third-party character, scan, likeness, avatar
  generator output or user image ever enters the repository (carried over
  from rowplay's provenance policy).

## Consequences

- `qtbridge` itself is `LicenseRef-Qt-Commercial OR LGPL-3.0-only`; linking it
  and Qt into a GPLv3 application is permitted.
- Contributors must add the SPDX header to new files; the PR template checks it.
