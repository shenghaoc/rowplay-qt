# ADR 0005 — Bake venues to `.glb`; do not port `renderer3dEnvironment.ts`

Status: accepted (2026-09-11)

## Context

The web app builds its regatta basin, Nordic stadium and velodrome venues
procedurally in `src/lib/replay/renderer3dEnvironment.ts` (about 3,200 lines
of three.js geometry code). Porting that line by line would need runtime
geometry generation, which in Qt Quick 3D means subclassing
`QQuick3DGeometry` in C++ (ADR 0001 forbids quiet C++) or a large custom mesh
pipeline in Rust.

## Decision

- **Bake, don't port.** A `tools/` Node script runs rowplay's environment
  builder per sport and exports the result to `.glb` with three.js
  `GLTFExporter`. The author polishes the result in Blender, and the baked
  `.glb` files are vendored with provenance (ADR 0002, ADR 0003).
- The runtime never generates venue geometry; it loads baked assets and applies
  the procedural sky (ADR 0004), lighting and materials.
- If the exporter turns out to need DOM or canvas APIs that block Node, that is
  a new ADR (options: jsdom shim, headless browser export, Blender rebuild).

## Consequences

- Venue work is Phase 6 and depends on the Node exporter, not on porting
  three.js code.
- Quality tiers (low / medium / high / ultra) select between baked variants or
  toggle features rather than changing generated geometry.
