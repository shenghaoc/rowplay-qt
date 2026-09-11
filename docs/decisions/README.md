# Architecture decision records

Numbered, immutable once accepted. A superseding decision gets a new number and
links back. Format: context, decision, consequences.

| ADR | Decision |
| --- | --- |
| [0001](0001-rust-core-with-qt-bridges-and-qt-quick-3d.md) | Rust for all logic; Qt Bridges for Rust to QML; Qt Quick 3D and Qt Graphs; Qt 6.11; no C++ |
| [0002](0002-gpl-3-licence-and-asset-provenance.md) | GPL-3.0-or-later, SPDX headers, asset provenance |
| [0003](0003-gltf-runtime-assets-authored-in-blender.md) | glTF 2.0 `.glb` is the only runtime 3D format; Blender authors; no USDZ |
| [0004](0004-procedural-sky-image-based-lighting.md) | Procedural sky radiance for IBL; no downloaded, imported or scanned HDRI |
| [0005](0005-bake-venues-do-not-port-the-environment-builder.md) | Bake venues to `.glb` with the web exporter; do not port `renderer3dEnvironment.ts` |
| [0006](0006-rust-core-first-layering.md) | Rust core first: app → platform → core, nothing testable depends on Qt |
