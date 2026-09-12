// SPDX-License-Identifier: GPL-3.0-or-later
// The replay scene (Phase 5a): procedural-sky IBL (ADR 0004 — no HDRI, ever),
// filmic tonemapping, the web's per-sport key light casting shadows, a ground
// plane tinted from the sport's venue palette, and the vendored V3 equipment
// templates plus the V4 athlete as balsam-generated components (ADR 0008,
// build.rs). The scene is in metres like the web's. Playback (pose, chase
// camera, HUD) is Phase 5b; venues are Phase 6 (ADR 0005 — this is
// deliberately just the ground plane and the sky).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick3D
import QtQuick3D.Helpers
import RowPlay
import RowPlay.Replay
import RowPlay.ReplayAssets

View3D {
    id: scene

    // Static 5a framing per sport (roughly the web's chase offsets without
    // the damping); Phase 5b replaces this with the chase camera. The row
    // shell's 7.8 m axis runs along Z, so the camera must sit well off that
    // axis (yaw ~35°) or the capture reads as a foreshortened blob.
    readonly property vector3d cameraPosition: [
        Qt.vector3d(4.5, 2.2, 6.5),   // rower: stern quarter, hull broadside-ish
        Qt.vector3d(3.0, 2.0, 6.6),   // skierg
        Qt.vector3d(2.8, 1.8, 6.2),   // bike
    ][Replay.sportIndex]
    readonly property vector3d cameraEuler: [
        Qt.vector3d(-15, 35, 0),
        Qt.vector3d(-12, 22, 0),
        Qt.vector3d(-10, 24, 0),
    ][Replay.sportIndex]
    // Which composite templates the current sport owns.
    readonly property string sportPrefix: [
        "equipment:row:", "equipment:ski:", "equipment:bike:",
    ][Replay.sportIndex]

    environment: ExtendedSceneEnvironment {
        backgroundMode: SceneEnvironment.SkyBox
        antialiasingMode: SceneEnvironment.MSAA
        antialiasingQuality: SceneEnvironment.High
        tonemapMode: SceneEnvironment.TonemapModeFilmic
        exposure: 1.0
        // The probe's ProceduralSkyTextureData is rebuilt whenever the palette
        // changes (see rebuildSky): editing its colour properties in place
        // does not refresh the uploaded texture, so the sky box and the IBL
        // would stay on the first sport's palette forever.
        lightProbe: Texture {
            id: skyProbe
        }
    }

    Component {
        id: skyDataFactory
        ProceduralSkyTextureData {
            sunEnergy: 1.0
            skyEnergy: 1.0
            groundEnergy: 1.0
        }
    }
    property var skyData: null
    property string builtSkyKey: ""
    // One string that changes exactly when any sky input changes.
    readonly property string skyKey: Replay.skyZenith + Replay.skyHorizon
                                     + Replay.skyGround + Replay.skySun
                                     + Replay.sunElevation + Replay.sunAzimuth
    onSkyKeyChanged: rebuildSky()
    function rebuildSky() {
        if (scene.builtSkyKey === scene.skyKey)
            return
        scene.builtSkyKey = scene.skyKey
        // Parented to the probe so the object lands in the 3D scene graph
        // (a View3D parent would leave it "not placed in the graphics scene").
        var next = skyDataFactory.createObject(skyProbe, {
            skyTopColor: Replay.skyZenith,
            skyHorizonColor: Replay.skyHorizon,
            groundHorizonColor: Replay.skyGround,
            groundBottomColor: Replay.skyGround,
            sunColor: Replay.skySun,
            sunLatitude: Replay.sunElevation,
            sunLongitude: Replay.sunAzimuth
        })
        skyProbe.textureData = next
        if (scene.skyData !== null)
            scene.skyData.destroy()
        scene.skyData = next
    }

    PerspectiveCamera {
        id: camera
        position: scene.cameraPosition
        eulerRotation: scene.cameraEuler
        // The packs and anchors are in metres (the web scene's unit), so the
        // Qt Quick 3D default near plane of 10 units would sit 10 m out and
        // swallow the SkiErg and BikeErg rigs whole; the web camera's near
        // plane is 0.1 m (renderer3d.ts).
        clipNear: 0.1
    }

    // The key light sits at the web's per-sport sun offset and aims at the
    // shadow target (renderer3d.ts: sunLight.position = SUN_OFFSETS[sport],
    // sunLight.target = (0, SHADOW_TARGET_HEIGHT, 0)); LookAtNode turns its
    // forward (-Z) axis — the direction a DirectionalLight shines — at the
    // target, so the offset needs no angle maths on either side.
    Node {
        id: sunTarget
        y: Replay.shadowTargetHeight
    }
    LookAtNode {
        position: Qt.vector3d(Replay.sunOffset[0], Replay.sunOffset[1],
                              Replay.sunOffset[2])
        target: sunTarget

        DirectionalLight {
            color: Replay.skySun
            brightness: 1.2
            castsShadow: true
            shadowMapQuality: Light.ShadowMapQualityHigh
            shadowFactor: 80
            // Scene units are metres, and Qt's defaults assume a scene a
            // hundred times larger: shadowBias (10) and pcfFactor (2.0) are
            // world-space approximations, so untouched they mean a 10 m depth
            // offset and a 2 m blur that erase every shadow; without cascades
            // one map would also be stretched over the whole 6 km ground
            // plane. Two cascades follow the camera frustum out to 60 m.
            shadowBias: 0.02
            pcfFactor: 0.03
            shadowMapFar: 60
            csmNumSplits: 2
        }
    }

    // The venue is the ground plane plus the sky only (ADR 0005); its tint
    // comes from the sport's venue palette (venue palettes are data mirrored
    // from the web stylesheet, resolved in Rust — not Theme chrome).
    Model {
        source: "#Rectangle"
        y: 0
        scale: Qt.vector3d(60, 60, 1)
        eulerRotation.x: -90
        receivesShadows: true
        castsShadows: false
        materials: PrincipledMaterial {
            baseColor: Replay.groundColor
            roughness: 0.9
        }
    }

    // The V3 pack as a balsam-generated component: every node carries its
    // glTF name as objectName, which is what the material-role walk (and 5b
    // joint posing on the athlete) needs — a RuntimeLoader scene is not
    // addressable from QML at all (ADR 0008, docs/qt-bridges-notes.md).
    //
    // The replay materials are declared statically inside this scene, as
    // children of a node in it — exactly like balsam's own inline placeholder.
    // A dynamically created material that nothing owns (no QML parent, no
    // retained reference) is garbage-collected and the Model it was assigned
    // to drops out of the render; parented or retained dynamic materials
    // render fine (docs/qt-bridges-notes.md). Static declarations need no
    // such care. The values transcribe the Rust spec table
    // `MaterialRole::spec()` and are cross-checked against it at startup so
    // drift fails loudly instead of rendering wrong.
    Rigs {
        id: rigs
        visible: Replay.loadState !== "error"

        PrincipledMaterial {
            id: matAthleteSkin
            baseColor: Theme.replaySkin
            metalness: 0.0
            roughness: 0.55
        }
        PrincipledMaterial {
            id: matAthleteFabric
            baseColor: Theme.replayFabric
            metalness: 0.0
            roughness: 0.8
        }
        PrincipledMaterial {
            id: matAthleteHair
            baseColor: Theme.replayHair
            metalness: 0.0
            roughness: 0.45
        }
        PrincipledMaterial {
            id: matAthleteFootwear
            baseColor: Theme.replayFootwear
            metalness: 0.0
            roughness: 0.6
        }
        PrincipledMaterial {
            id: matAthleteShorts
            baseColor: Theme.replayShorts
            metalness: 0.0
            roughness: 0.8
        }
        PrincipledMaterial {
            id: matAthleteTrim
            baseColor: Theme.replayTrim
            metalness: 0.1
            roughness: 0.4
        }
        PrincipledMaterial {
            id: matAthleteEye
            baseColor: Theme.replayEye
            metalness: 0.0
            roughness: 0.15
        }
        PrincipledMaterial {
            id: matAthleteFaceDetail
            baseColor: Theme.replayFaceDetail
            metalness: 0.0
            roughness: 0.6
        }
        PrincipledMaterial {
            id: matEquipmentPainted
            baseColor: Replay.livePaint
            metalness: 0.05
            roughness: 0.35
        }
        PrincipledMaterial {
            id: matEquipmentDark
            baseColor: Theme.replayEquipmentDark
            metalness: 0.2
            roughness: 0.5
        }
        PrincipledMaterial {
            id: matEquipmentLight
            baseColor: Theme.replayEquipmentLight
            metalness: 0.1
            roughness: 0.4
        }
        PrincipledMaterial {
            id: matEquipmentMetal
            baseColor: Theme.replayEquipmentMetal
            metalness: 0.9
            roughness: 0.25
        }
        PrincipledMaterial {
            id: matEquipmentRubber
            baseColor: Theme.replayEquipmentRubber
            metalness: 0.0
            roughness: 0.9
        }
        PrincipledMaterial {
            id: matEquipmentGrip
            baseColor: Theme.replayEquipmentGrip
            metalness: 0.0
            roughness: 0.85
        }
        PrincipledMaterial {
            id: matEquipmentTrim
            baseColor: Theme.replayEquipmentTrim
            metalness: 0.3
            roughness: 0.4
        }
    }

    // The V4 athlete: skinned, unposed in 5a (Phase 5b drives it from the
    // motion graph). Parked behind the equipment so the placeholder reads
    // clearly; the 5b pose seats it on the rig.
    Athlete {
        id: athlete
        visible: Replay.loadState !== "error"
        position: Qt.vector3d(0, 0, -2.4)
    }

    // The components instantiate synchronously; the backend's "ready" state
    // follows once the scene rules have been applied below (a startup
    // validation failure keeps "error" and the overlay shows it).
    Component.onCompleted: {
        rebuildSky()
        var list = Replay.anchors
        for (var i = 0; i < list.length; ++i)
            anchorsByTemplate[list[i].template] = list[i]
        if (!validateMaterials())
            return
        applySceneRules()
        Replay.reportReady()
    }

    // template slot -> {position, yaw} for the primary clone (README table).
    property var anchorsByTemplate: ({})

    // roleId -> the statically declared material above (wire ids).
    property var byRole: ({
        "athlete-skin": matAthleteSkin,
        "athlete-fabric": matAthleteFabric,
        "athlete-hair": matAthleteHair,
        "athlete-footwear": matAthleteFootwear,
        "athlete-shorts": matAthleteShorts,
        "athlete-trim": matAthleteTrim,
        "athlete-eye": matAthleteEye,
        "athlete-face-detail": matAthleteFaceDetail,
        "equipment-painted": matEquipmentPainted,
        "equipment-dark": matEquipmentDark,
        "equipment-light": matEquipmentLight,
        "equipment-metal": matEquipmentMetal,
        "equipment-rubber": matEquipmentRubber,
        "equipment-grip": matEquipmentGrip,
        "equipment-trim": matEquipmentTrim,
    })

    // The QML table must be exactly the Rust spec table: same roles, same
    // metalness/roughness. A mismatch is a scene-configuration error, so it
    // takes the error path (overlay + gate failure), not a warning.
    function validateMaterials() {
        var specs = Replay.materialSpecs
        for (var i = 0; i < specs.length; ++i) {
            var spec = specs[i]
            var material = byRole[spec.id]
            if (material === undefined) {
                Replay.reportError("no QML material for role " + spec.id)
                return false
            }
            if (material.metalness !== spec.metalness
                    || material.roughness !== spec.roughness) {
                Replay.reportError("material " + spec.id
                                   + " does not match the Rust spec table")
                return false
            }
        }
        return true
    }

    Connections {
        target: Replay
        function onReplayChanged() {
            if (Replay.loadState === "ready") {
                scene.applySceneRules()
            }
        }
    }

    // Walks the loaded pack: re-materials every mesh from its
    // `replayMaterialRole`, shows the current sport's composite templates,
    // and hides the leaf shells (their anchors are driven in Phase 5b; the V4
    // athlete is the visible body).
    onSportPrefixChanged: applySceneRules()
    function applySceneRules() {
        scene.materialsApplied = 0
        scene.templatesPlaced = 0
        scene.leavesHidden = 0
        walkRigs(rigs)
        console.log("replay scene rules:", scene.materialsApplied, "materials,",
                    scene.templatesPlaced, "templates placed,",
                    scene.leavesHidden, "leaves hidden")
    }

    property int materialsApplied: 0
    property int templatesPlaced: 0
    property int leavesHidden: 0

    function walkRigs(node) {
        if (node === null || node === undefined)
            return
        var meta = Replay.meshRoles[node.objectName]
        if (meta !== undefined) {
            if (node.materials !== undefined && byRole[meta.role] !== undefined) {
                node.materials = [byRole[meta.role]]
                scene.materialsApplied += 1
            }
            if (meta.slot !== null && meta.slot !== undefined) {
                node.visible = false      // leaf shell; 5b anchors them
                scene.leavesHidden += 1
            }
        } else if (typeof node.objectName === "string"
                   && node.objectName.lastIndexOf("equipment:", 0) === 0) {
            var belongs = node.objectName.lastIndexOf(scene.sportPrefix, 0) === 0
            node.visible = belongs
            // Statically place the primary clone at its README anchor (yaw
            // comes in radians from Rust; Node euler angles are degrees).
            var anchor = belongs ? anchorsByTemplate[node.objectName] : null
            if (anchor !== undefined && anchor !== null) {
                node.position = Qt.vector3d(anchor.position[0],
                                            anchor.position[1],
                                            anchor.position[2])
                node.eulerRotation.y = anchor.yaw * 180 / Math.PI
                scene.templatesPlaced += 1
            }
        }
        var children = node.children
        for (var i = 0; children !== undefined && i < children.length; ++i)
            walkRigs(children[i])
    }

    // 2D overlay: transport controls arrive in Phase 5b; 5a shows the load
    // state and the way back to the library.
    ColumnLayout {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: Theme.spacingXxLarge
        spacing: Theme.spacingSmall

        Label {
            visible: Replay.loadState !== "ready"
            text: Replay.loadState === "error"
                  ? Tr.t("replay.view3dError") + (Replay.errorText.length > 0
                        ? " — " + Replay.errorText : "")
                  : Tr.t("replay.view3dLoading")
            font: Theme.body
            color: Replay.loadState === "error"
                   ? Theme.alertRed : Theme.textSecondary
            Accessible.name: text
        }

        Button {
            text: Tr.t("replay.back")
            onClicked: Library.closeReplay()
            Accessible.name: text
        }
    }
}
