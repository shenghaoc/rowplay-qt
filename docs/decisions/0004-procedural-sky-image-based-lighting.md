# ADR 0004 — Procedural sky radiance for image-based lighting

Status: accepted (2026-09-11)

## Context

The web app's replay environments are procedural: sky, horizon, ground and sun
are generated from a zenith / horizon / nadir / sun palette per sport and
theme, and its provenance policy forbids downloaded, imported or scanned HDRI
images. Qt Quick 3D's `ExtendedSceneEnvironment` takes a `lightProbe` texture
for image-based lighting, and `QtQuick3D.Helpers` ships
`ProceduralSkyTextureData`, a `QQuick3DTextureData` that generates an
equirectangular sky from colours and sun parameters.

## Decision

- The replay's `lightProbe` is a `Texture` whose `textureData` is
  `ProceduralSkyTextureData`, fed by the same zenith / horizon / nadir / sun
  palette the web app uses per sport and theme (verified against the Qt 6.11
  `QtQuick3D.Helpers` module: `skyTopColor`, `skyHorizonColor`,
  `groundHorizonColor`, `groundBottomColor`, `sunColor`, `sunLatitude`,
  `sunLongitude`, `sunAngleMax`, `sunCurve`, `sunEnergy`, `skyCurve`,
  `skyEnergy`, `groundCurve`, `groundEnergy`, `textureQuality`).
- The same texture doubles as the sky box background, and a shadow-casting
  `DirectionalLight` uses the sun colour.
- **No HDRI file** — downloaded, imported or scanned — is ever added to the
  repository or loaded at runtime.

## Consequences

- The Phase 0 smoke scene already exercises this path (`qml/RowPlay/SmokeScene.qml`).
- Palette values are data in Rust (per sport / theme), so parity with the web
  app's `renderer3dEnvironment.ts` colours is testable without rendering.
