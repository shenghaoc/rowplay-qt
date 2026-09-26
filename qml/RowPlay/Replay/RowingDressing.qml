// SPDX-License-Identifier: GPL-3.0-or-later
// Blender Phase 4: the rowing course dressing. The finish tower, the start
// jetty, the pontoons, the course bridge, the campus buildings, the wetland
// boardwalk and hide, the distance boards and the island come from
// rowing-dressing.blend through build.rs, which generates the declarative
// DressingScene from the exported meshes and placements; this component
// gives them their Direction C materials, one per material class. Each
// mesh's primitives are split by class, and the scene passes the classes'
// materials in the exporter's order. Vertex colours carry the authored
// albedo, the material colour is the scheme's tint, and each furniture
// instance's colour multiplies in. The structures cast and receive the
// key light's shadow as the exporter records (High and Ultra only); the
// furniture casts none.
import QtQuick3D
import RowPlay.Replay
import RowPlay.ReplayAssets

Node {
    id: dressing
    required property int tier

    PrincipledMaterial {
        id: paintMaterial
        baseColor: RowingStyle.dressingTint
        vertexColorsEnabled: true
        roughness: 0.6
        specularAmount: 0.35
    }
    PrincipledMaterial {
        id: timberMaterial
        baseColor: RowingStyle.dressingTint
        vertexColorsEnabled: true
        roughness: 0.85
        specularAmount: 0.2
    }
    PrincipledMaterial {
        id: metalMaterial
        baseColor: RowingStyle.dressingTint
        vertexColorsEnabled: true
        roughness: 0.42
        metalness: 0.45
        specularAmount: 0.6
    }
    PrincipledMaterial {
        id: glassMaterial
        baseColor: RowingStyle.dressingTint
        vertexColorsEnabled: true
        roughness: 0.12
        specularAmount: 1.0
    }
    PrincipledMaterial {
        id: floatMaterial
        baseColor: RowingStyle.dressingTint
        vertexColorsEnabled: true
        roughness: 0.7
        specularAmount: 0.2
    }
    // The island's lawn and beach: the environment's land material.
    PrincipledMaterial {
        id: groundMaterial
        baseColor: RowingStyle.landTint
        vertexColorsEnabled: true
        roughness: 0.95
        specularAmount: 0.2
    }

    DressingScene {
        tier: dressing.tier
        paintMaterial: paintMaterial
        timberMaterial: timberMaterial
        metalMaterial: metalMaterial
        glassMaterial: glassMaterial
        floatMaterial: floatMaterial
        groundMaterial: groundMaterial
    }
}
