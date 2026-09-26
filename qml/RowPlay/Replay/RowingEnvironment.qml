// SPDX-License-Identifier: GPL-3.0-or-later
// Blender Phase 3: the rowing environment around the basin. The terrain (near
// bank to the rim), the far bank and the instanced vegetation come from
// rowing-environment.blend through build.rs, which generates the declarative
// EnvironmentScene from the exported meshes and placements; this component
// gives them their Direction C materials. Vertex colours carry the authored
// albedo, the material colour is the scheme's tint, and each plant's instance
// colour multiplies in as well (Qt 6.11: qt_vertColor = attr_color *
// qt_instanceColor). Nothing here casts the key light's shadow; the terrain
// receives it, as the web's banks it replaces do (High and Ultra only).
import QtQuick3D
import RowPlay.Replay
import RowPlay.ReplayAssets

Node {
    id: environment
    required property int tier

    PrincipledMaterial {
        id: landMaterial
        baseColor: RowingStyle.landTint
        vertexColorsEnabled: true
        roughness: 0.95
        specularAmount: 0.2
    }
    PrincipledMaterial {
        id: farMaterial
        baseColor: RowingStyle.farTint
        vertexColorsEnabled: true
        roughness: 1.0
        specularAmount: 0.1
    }
    PrincipledMaterial {
        id: canopyMaterial
        baseColor: RowingStyle.foliageTint
        vertexColorsEnabled: true
        roughness: 0.85
        specularAmount: 0.25
    }
    // Reed blades are single triangles: drawn from both sides.
    PrincipledMaterial {
        id: reedMaterial
        baseColor: RowingStyle.foliageTint
        vertexColorsEnabled: true
        roughness: 0.8
        specularAmount: 0.25
        cullMode: Material.NoCulling
    }

    EnvironmentScene {
        tier: environment.tier
        land: landMaterial
        far: farMaterial
        canopy: canopyMaterial
        reeds: reedMaterial
    }
}
