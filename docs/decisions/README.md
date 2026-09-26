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
| [0007](0007-platform-libraries.md) | Platform libraries: `ureq` (rustls, blocking), `keyring` 3 with explicit native backends, `rusqlite` (bundled), `directories`; no `url`, no tokio |
| [0008](0008-balsam-components-not-runtimeloader.md) | Replay packs converted by `balsam` into QML components in every build; `RuntimeLoader` is not used (supersedes the development-time loader consequence of 0003) |
| [0009](0009-vendored-assets-stay-in-plain-git.md) | Vendored assets stay as plain Git blobs; a 50 MB tripwire under `assets/replay/` forces a deliberate revisit before Phase 6 venues |
| [0010](0010-venue-baking-route.md) | Venue baking runs the web builder headless in Node; instances live in a contract, not the GLB |
| [0011](0011-venue-assets-stay-in-plain-git.md) | Venues stay in plain Git; the ADR 0009 ceiling is raised to 100 MB on measured sizes |
| [0012](0012-packaging-route.md) | Packaging: `macdeployqt` bundle + DMG, `windeployqt` + Inno Setup, `linuxdeploy` AppImage; launch-checked release bundles; ad-hoc signing by default; draft releases on `v*` tags; Flatpak deferred |
| [0013](0013-cross-platform-design-system.md) | One design system of our own on every OS, on Qt Quick Controls Basic and `Theme.qml` tokens (neutral AA ramps, system-font scale, system accent, high-contrast variant), plus a thin platform-behaviour layer (StandardKey shortcuts, native macOS menu bar, platform dialog order) |
| [0014](0014-distribution-on-linux-macos-and-windows.md) | Distribution on Linux, macOS and Windows (supersedes the Linux-only policy recorded after 0012); Windows ships CI-verified only; signing and notarisation are open decisions |
| [0015](0015-native-qt-styles.md) | Native Qt styles for standard controls (macOS, Windows, Fusion; no forced style), our identity in the content (charts, metric colours, tiles, replay and HUD); surfaces and text from the system palette, fitted to the contrast floors (supersedes 0013's control layer) |
| [0016](0016-authored-overcast-lighting.md) | Script-generated, prefiltered skies permitted; Direction C rowing and blue-hour companion; stationary course cues, unchanged replay mechanics; procedural assets from reviewed scripts, modelled assets from a reviewed `.blend` plus a deterministic export and validation script (amended) |
