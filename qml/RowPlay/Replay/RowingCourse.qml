// SPDX-License-Identifier: GPL-3.0-or-later
import QtQuick
import QtQuick3D
import RowPlay.ReplayAssets

Node {
    id: course
    CourseInstances { id: placements }
    PrincipledMaterial { id: buoyMaterial; baseColor: "white"; roughness: 0.48 }
    CourseBuoy { id: buoy }

    function applyInstances(node) {
        if (node.instancing !== undefined && node.source !== undefined) {
            node.instancing = placements
            node.materials = [buoyMaterial]
            node.castsShadows = false
            node.receivesShadows = false
        }
        var children = node.children
        for (var i = 0; children && i < children.length; ++i)
            applyInstances(children[i])
    }
    Component.onCompleted: applyInstances(buoy)
}
