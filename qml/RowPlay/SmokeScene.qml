// SPDX-License-Identifier: GPL-3.0-or-later
// Phase 0 stack smoke scene: one PBR sphere and one cube lit by the procedural
// sky light probe, shadow-casting sun, filmic tonemapping. The Rust backend
// (RowPlay.Smoke) owns the per-frame numbers; this file only binds them.
import QtQuick
import QtQuick3D
import QtQuick3D.Helpers
import RowPlay

View3D {
    id: view

    // Same zenith / horizon / nadir / sun palette family as the web app's
    // procedural sky (no downloaded, imported or scanned HDRI — ADR 0004).
    property color skyZenith: "#3b6fb6"
    property color skyHorizon: "#c9dcf2"
    property color groundHorizon: "#7c8a6b"
    property color groundNadir: "#3d4433"
    property color sunColor: "#fff2d6"

    environment: ExtendedSceneEnvironment {
        backgroundMode: SceneEnvironment.SkyBox
        antialiasingMode: SceneEnvironment.MSAA
        antialiasingQuality: SceneEnvironment.High
        tonemapMode: SceneEnvironment.TonemapModeFilmic
        exposure: 1.0
        lightProbe: Texture {
            textureData: ProceduralSkyTextureData {
                skyTopColor: view.skyZenith
                skyHorizonColor: view.skyHorizon
                groundHorizonColor: view.groundHorizon
                groundBottomColor: view.groundNadir
                sunColor: view.sunColor
                sunLatitude: 35
                sunLongitude: 20
                sunEnergy: 1.0
                skyEnergy: 1.0
                groundEnergy: 1.0
            }
        }
    }

    PerspectiveCamera {
        position: Qt.vector3d(0, 150, 500)
        eulerRotation.x: -12
    }

    DirectionalLight {
        eulerRotation.x: -45
        eulerRotation.y: 30
        color: view.sunColor
        brightness: 1.2
        castsShadow: true
        shadowMapQuality: Light.ShadowMapQualityHigh
        shadowFactor: 80
    }

    Model {
        id: sphere
        source: "#Sphere"
        position: Qt.vector3d(-90, Smoke.sphereY, 0)
        materials: PrincipledMaterial {
            baseColor: "#d9432b"
            metalness: 0.1
            roughness: 0.35
        }
    }

    Model {
        id: cube
        source: "#Cube"
        position: Qt.vector3d(90, 50, 0)
        eulerRotation.y: Smoke.cubeAngle
        materials: PrincipledMaterial {
            baseColor: "#2b7fd9"
            metalness: 0.8
            roughness: 0.25
        }
    }

    Model {
        source: "#Rectangle"
        scale: Qt.vector3d(20, 20, 1)
        eulerRotation.x: -90
        materials: PrincipledMaterial {
            baseColor: "#8a8f7a"
            roughness: 0.9
        }
    }
}
