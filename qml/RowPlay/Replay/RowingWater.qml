// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick3D
import RowPlay.Replay

Model {
    id: water
    required property int tier
    source: "#Rectangle"
    // Qt primitives are 100 units wide. One scene unit remains one metre.
    scale: Qt.vector3d(60, 60, 1)
    eulerRotation.x: -90
    receivesShadows: true
    castsShadows: false
    materials: PrincipledMaterial {
        baseColor: RowingStyle.water
        metalness: 0.15
        roughness: water.tier === 0 ? 0.38 : 0.32
        specularAmount: 0.65
        normalStrength: water.tier === 0 ? 0.18 : 0.28
        normalMap: Texture {
            source: "qrc:/qt/qml/RowPlay/Environments/authored/water-normal.png"
            tilingModeHorizontal: Texture.Repeat
            tilingModeVertical: Texture.Repeat
            // One 8 m tile of the Phase 3 normal map (6000 m / 750), fixed
            // in the world: the boat moves over it, it never scrolls.
            scaleU: 750
            scaleV: 750
            generateMipmaps: true
            mipFilter: Texture.Linear
        }
    }
}
