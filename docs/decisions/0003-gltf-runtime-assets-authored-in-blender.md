# ADR 0003 — glTF 2.0 (`.glb`) is the only runtime 3D format; Blender is the authoring tool

Status: accepted (2026-09-11)

## Context

rowplay ships `rowplay-rigs-v3.glb` (equipment templates) and
`rowplay-athlete-v4.glb` (skinned athlete with three sport clips) built from
reviewed Blender and three.js sources, with a machine-readable contract and
hashes. rowplay-studio converted them to USDZ for RealityKit. Qt Quick 3D loads
glTF 2.0 directly (`RuntimeLoader` at development time, `balsam` output for
release builds).

## Decision

- **glTF 2.0 binary (`.glb`)** is the only runtime 3D format in this repository.
- **Blender** is the authoring and clean-up tool for anything we polish
  ourselves (venues, later athlete tweaks); `tools/` scripts drive it.
- **No USDZ** in this repository; Studio's USDZ derivatives are not reused.
- Vendored `.glb` files carry provenance (source repo, path, commit, SHA-256)
  in `ASSET_PROVENANCE.md`, and the rigs' material-role metadata
  (`replayMaterialRole` glTF extras) is mapped to `PrincipledMaterial` at
  runtime following `renderer3dAssets.ts`.

## Consequences

- Development builds load `.glb` with `QtQuick3D.AssetUtils` `RuntimeLoader`;
  release builds pre-process them with `balsam` for load time.
- Any `.glb` that needs geometry generated at runtime is a C++-only path
  (`QQuick3DGeometry`); ADR 0001 requires an ADR before that door is opened,
  which is why venues are baked (ADR 0005).
