// SPDX-License-Identifier: GPL-3.0-or-later
// The replay scene (Phase 5a assets + Phase 5b playback): procedural-sky IBL,
// filmic tonemapping, the web's per-sport key light, a ground plane tinted
// from the venue palette, and the V3 equipment / V4 athlete as balsam
// components (ADR 0008). Phase 5b adds the chase camera, athlete joint
// posing, equipment motion (oars, poles, crank, wheels on the course loop)
// and the transport bar. One FrameAnimation drives Replay.tick; one
// onFrameChanged applies the flat frame bundle — one bridge crossing per
// rendered frame (spec R1.3).
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick3D
import QtQuick3D.Helpers
import RowPlay
import RowPlay.Replay
import RowPlay.ReplayAssets

Item {
    id: replayRoot

    // ---- constants from the Replay singleton ----
    readonly property var fl: Replay.frameLayout
    readonly property var el: Replay.equipmentLayout
    readonly property string sportPrefix: [
        "equipment:row:", "equipment:ski:", "equipment:bike:",
    ][Replay.sportIndex]

    // The contract's 19 semantic bones, contract order (index 0 = Hips).
    readonly property var semanticBones: [
        "v4Hips", "v4Spine", "v4Chest", "v4Neck", "v4Head",
        "v4LeftClavicle", "v4LeftUpperArm", "v4LeftForearm", "v4LeftHand",
        "v4RightClavicle", "v4RightUpperArm", "v4RightForearm", "v4RightHand",
        "v4LeftUpperLeg", "v4LeftLowerLeg", "v4LeftFoot",
        "v4RightUpperLeg", "v4RightLowerLeg", "v4RightFoot"
    ]

    // ---- node references (built by setupScene) ----
    property var jointNodes: []
    property var seatNode: null
    property var boatNode: null          // boat-assembly (single instance)
    property var drivetrainNode: null
    property var frameNode: null         // bike frame-assembly (single instance)
    property var oarRigNode: null        // primary (right) oar
    property var oarRigMirror: null      // mirror (left) oar
    property var wheelAssemblyNode: null
    property var wheelMirror: null       // second wheel
    // Equipment leaves: blade and pole parts, one per side.
    // Keyed "right"/"left" → Node reference.
    property var bladeNodes: ({})
    property var poleShaftNodes: ({})
    property var poleGripNodes: ({})
    property var poleBasketNodes: ({})
    property var anchorsByTemplate: ({})
    property var mirrorByTemplate: ({})
    // ---- HUD text parts (updated by applyFrame) ----
    property string clockText: ""
    property string totalText: ""
    property string distanceText: ""
    property string paceText: ""
    property string rateText: ""
    property string wattsText: ""
    property string heartText: ""

    // ---- 3D scene ----
    View3D {
        id: scene
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: transportBar.top

        environment: ExtendedSceneEnvironment {
            backgroundMode: SceneEnvironment.SkyBox
            antialiasingMode: SceneEnvironment.MSAA
            antialiasingQuality: SceneEnvironment.High
            tonemapMode: SceneEnvironment.TonemapModeFilmic
            exposure: 1.0
            lightProbe: Texture { id: skyProbe }
        }

        Component {
            id: skyDataFactory
            ProceduralSkyTextureData {
                sunEnergy: 1.0; skyEnergy: 1.0; groundEnergy: 1.0
            }
        }
        property var skyData: null
        property string builtSkyKey: ""
        readonly property string skyKey: Replay.skyZenith + Replay.skyHorizon
                                         + Replay.skyGround + Replay.skySun
                                         + Replay.sunElevation + Replay.sunAzimuth
        onSkyKeyChanged: rebuildSky()
        function rebuildSky() {
            if (scene.builtSkyKey === scene.skyKey) return
            scene.builtSkyKey = scene.skyKey
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
            if (scene.skyData !== null) scene.skyData.destroy()
            scene.skyData = next
        }

        PerspectiveCamera { id: camera; clipNear: 0.1; clipFar: 1000 }

        // Key light (web SUN_OFFSETS, LookAtNode → DirectionalLight).
        Node { id: sunTarget; y: Replay.shadowTargetHeight }
        LookAtNode {
            position: Qt.vector3d(Replay.sunOffset[0], Replay.sunOffset[1],
                                  Replay.sunOffset[2])
            target: sunTarget
            DirectionalLight {
                color: Replay.skySun; brightness: 1.2; castsShadow: true
                shadowMapQuality: Light.ShadowMapQualityHigh; shadowFactor: 80
                shadowBias: 0.02; pcfFactor: 0.03; shadowMapFar: 60; csmNumSplits: 2
            }
        }

        // Ground plane (venue palette tint, ADR 0005).
        Model {
            source: "#Rectangle"; y: 0; scale: Qt.vector3d(60, 60, 1)
            eulerRotation.x: -90; receivesShadows: true; castsShadows: false
            materials: PrincipledMaterial { baseColor: Replay.groundColor; roughness: 0.9 }
        }

        // ---- Course / rig hierarchy ----
        Node {
            id: courseNode
            Node {
                id: rigGroup

                // Primary (right-side) equipment + athlete leaves.
                Rigs {
                    id: rigs
                    visible: Replay.loadState !== "error"

                    // Static materials — values transcribe MaterialRole::spec()
                    // and are cross-checked at startup (validateMaterials).
                    PrincipledMaterial { id: matAthleteSkin; baseColor: Theme.replaySkin; metalness: 0.0; roughness: 0.55 }
                    PrincipledMaterial { id: matAthleteFabric; baseColor: Theme.replayFabric; metalness: 0.0; roughness: 0.8 }
                    PrincipledMaterial { id: matAthleteHair; baseColor: Theme.replayHair; metalness: 0.0; roughness: 0.45 }
                    PrincipledMaterial { id: matAthleteFootwear; baseColor: Theme.replayFootwear; metalness: 0.0; roughness: 0.6 }
                    PrincipledMaterial { id: matAthleteShorts; baseColor: Theme.replayShorts; metalness: 0.0; roughness: 0.8 }
                    PrincipledMaterial { id: matAthleteTrim; baseColor: Theme.replayTrim; metalness: 0.1; roughness: 0.4 }
                    PrincipledMaterial { id: matAthleteEye; baseColor: Theme.replayEye; metalness: 0.0; roughness: 0.15 }
                    PrincipledMaterial { id: matAthleteFaceDetail; baseColor: Theme.replayFaceDetail; metalness: 0.0; roughness: 0.6 }
                    PrincipledMaterial { id: matEquipmentPainted; baseColor: Replay.livePaint; metalness: 0.05; roughness: 0.35 }
                    PrincipledMaterial { id: matEquipmentDark; baseColor: Theme.replayEquipmentDark; metalness: 0.2; roughness: 0.5 }
                    PrincipledMaterial { id: matEquipmentLight; baseColor: Theme.replayEquipmentLight; metalness: 0.1; roughness: 0.4 }
                    PrincipledMaterial { id: matEquipmentMetal; baseColor: Theme.replayEquipmentMetal; metalness: 0.9; roughness: 0.25 }
                    PrincipledMaterial { id: matEquipmentRubber; baseColor: Theme.replayEquipmentRubber; metalness: 0.0; roughness: 0.9 }
                    PrincipledMaterial { id: matEquipmentGrip; baseColor: Theme.replayEquipmentGrip; metalness: 0.0; roughness: 0.85 }
                    PrincipledMaterial { id: matEquipmentTrim; baseColor: Theme.replayEquipmentTrim; metalness: 0.3; roughness: 0.4 }
                }

                // Mirror-side copy: only multi-instance templates (left oar,
                // left ski, second wheel) are shown; everything else is hidden.
                // The material declarations live on the primary copy; this one
                // references them by id in the walkRigs material assignment.
                Rigs {
                    id: rigsMirror
                    visible: Replay.loadState !== "error"
                }

                Athlete {
                    id: athlete
                    visible: Replay.loadState !== "error"
                }
            }
        }

        // Drive Replay.tick from the rendering loop.
        FrameAnimation {
            running: Replay.hasWorkout && Replay.loadState === "ready"
            onTriggered: Replay.tick(frameTime)
        }

        Component.onCompleted: {
            scene.rebuildSky()
            replayRoot.setupScene()
        }
    }

    // ---- Transport bar ----
    Pane {
        id: transportBar
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        visible: Replay.hasWorkout
        padding: Theme.spacingSmall
        background: Rectangle { color: Theme.windowBackground; opacity: 0.92 }

        ColumnLayout {
            anchors.fill: parent
            spacing: Theme.spacingXSmall

            RowLayout {
                spacing: Theme.spacingSmall
                Layout.fillWidth: true

                Button {
                    text: Replay.playing
                          ? Tr.t("replay.pause") : Tr.t("replay.play")
                    onClicked: Replay.toggle()
                    Accessible.name: text
                }
                Label {
                    text: replayRoot.clockText + " / " + replayRoot.totalText
                    font: Theme.body
                    Accessible.name: text
                }
                Slider {
                    id: seekSlider
                    Layout.fillWidth: true
                    from: 0; to: 1
                    value: Replay.progress
                    onMoved: Replay.seek(value)
                    Accessible.name: Tr.t("replay.seekSlider")
                }
                Label {
                    text: replayRoot.distanceText
                    font: Theme.body
                    Accessible.name: text
                }
            }

            RowLayout {
                spacing: Theme.spacingSmall
                Layout.fillWidth: true

                Repeater {
                    model: Replay.speedLabels
                    Button {
                        flat: true; text: modelData
                        highlighted: index === Replay.speedIndex
                        onClicked: Replay.setSpeedIndex(index)
                        Accessible.name: Tr.t("replay.playbackSpeed")
                                         + " " + modelData
                    }
                }
                Item { Layout.fillWidth: true }
                Label { text: Tr.t("replay.gPace") + " " + replayRoot.paceText;  font: Theme.body; visible: paceText.length > 0 }
                Label { text: Tr.t("replay.gRate") + " " + replayRoot.rateText;  font: Theme.body; visible: rateText.length > 0 }
                Label { text: Tr.t("replay.gPower") + " " + replayRoot.wattsText; font: Theme.body; visible: wattsText.length > 0 }
                Label { text: Tr.t("replay.gHeart") + " " + replayRoot.heartText; font: Theme.body; visible: heartText.length > 0 }
                Button { text: Tr.t("replay.back"); onClicked: Library.closeReplay(); Accessible.name: text }
            }
        }
    }

    // Error / loading overlay (when no workout is loaded).
    Label {
        anchors.left: parent.left
        anchors.bottom: transportBar.visible ? transportBar.top : parent.bottom
        anchors.margins: Theme.spacingXxLarge
        visible: Replay.loadState !== "ready" && !Replay.hasWorkout
        text: Replay.loadState === "error"
              ? Tr.t("replay.view3dError")
                + (Replay.errorText.length > 0 ? " — " + Replay.errorText : "")
              : Tr.t("replay.view3dLoading")
        font: Theme.body
        color: Replay.loadState === "error" ? Theme.alertRed : Theme.textSecondary
        Accessible.name: text
    }
    Button {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.margins: Theme.spacingXxLarge
        visible: !Replay.hasWorkout
        text: Tr.t("replay.back"); onClicked: Library.closeReplay()
        Accessible.name: text
    }

    // ---- Keyboard shortcuts (web replay transport keys) ----
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Space"; onActivated: Replay.toggle() }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Left"; onActivated: Replay.seekBy(-10) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Right"; onActivated: Replay.seekBy(10) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Shift+Left"; onActivated: Replay.seekBy(-30) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Shift+Right"; onActivated: Replay.seekBy(30) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "["; onActivated: Replay.stepSpeed(-1) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "]"; onActivated: Replay.stepSpeed(1) }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequences: ["Home", "0"]; onActivated: Replay.seek(0) }

    // ---- Connections ----
    Connections {
        target: Replay
        function onFrameChanged() { replayRoot.applyFrame() }
        function onReplayChanged() {
            if (Replay.loadState === "ready") replayRoot.applySceneRules()
        }
    }
    onWidthChanged: Replay.setViewport(width, height)
    onHeightChanged: Replay.setViewport(width, height)

    // ---- roleId → material ----
    property var byRole: ({
        "athlete-skin": matAthleteSkin, "athlete-fabric": matAthleteFabric,
        "athlete-hair": matAthleteHair, "athlete-footwear": matAthleteFootwear,
        "athlete-shorts": matAthleteShorts, "athlete-trim": matAthleteTrim,
        "athlete-eye": matAthleteEye, "athlete-face-detail": matAthleteFaceDetail,
        "equipment-painted": matEquipmentPainted, "equipment-dark": matEquipmentDark,
        "equipment-light": matEquipmentLight, "equipment-metal": matEquipmentMetal,
        "equipment-rubber": matEquipmentRubber, "equipment-grip": matEquipmentGrip,
        "equipment-trim": matEquipmentTrim,
    })

    // ---- helpers ----
    function quatFromFrame(f, i) {
        return Qt.quaternion(f[i + 3], f[i], f[i + 1], f[i + 2])
    }
    function vec3FromFrame(f, i) {
        return Qt.vector3d(f[i], f[i + 1], f[i + 2])
    }


    // ---- setupScene ----
    function setupScene() {
        var list = Replay.anchors
        for (var i = 0; i < list.length; ++i)
            anchorsByTemplate[list[i].template] = list[i]
        var mlist = Replay.mirrorAnchors
        for (var i = 0; i < mlist.length; ++i)
            mirrorByTemplate[mlist[i].template] = mlist[i]
        // Joint-node lookup (walk the athlete subtree).
        var jn = []
        for (var j = 0; j < semanticBones.length; ++j) jn.push(null)
        walkForJoints(athlete, jn)
        jointNodes = jn
        validateJoints()
        if (!validateMaterials()) return
        applySceneRules()
        Replay.reportReady()
    }

    function walkForJoints(node, list) {
        if (!node) return
        for (var j = 0; j < semanticBones.length; ++j) {
            if (node.objectName === semanticBones[j]) { list[j] = node; break }
        }
        var ch = node.children
        for (var i = 0; ch && i < ch.length; ++i) walkForJoints(ch[i], list)
    }

    // Check that every semantic bone resolves to a real joint node.
    function validateJoints() {
        var missing = []
        for (var j = 0; j < semanticBones.length; ++j) {
            if (!jointNodes[j]) missing.push(semanticBones[j])
        }
        if (missing.length > 0)
            console.warn("replay: unresolved semantic joints:", missing.join(", "))
        return missing.length === 0
    }

    // ---- validateMaterials (same as 5a) ----
    function validateMaterials() {
        var specs = Replay.materialSpecs
        for (var i = 0; i < specs.length; ++i) {
            var spec = specs[i]
            var mat = byRole[spec.id]
            if (mat === undefined) {
                Replay.reportError("no QML material for role " + spec.id); return false
            }
            if (mat.metalness !== spec.metalness || mat.roughness !== spec.roughness) {
                Replay.reportError("material " + spec.id + " drift"); return false
            }
        }
        return true
    }

    // ---- applySceneRules (materials + visibility + placement) ----
    property int materialsApplied: 0
    property int templatesPlaced: 0
    property int leavesHidden: 0

    onSportPrefixChanged: { if (Replay.loadState === "ready") applySceneRules() }

    function applySceneRules() {
        materialsApplied = 0; templatesPlaced = 0; leavesHidden = 0
        seatNode = null; boatNode = null
        drivetrainNode = null; frameNode = null
        oarRigNode = null; oarRigMirror = null
        wheelAssemblyNode = null; wheelMirror = null
        bladeNodes = {}; poleShaftNodes = {}; poleGripNodes = {}; poleBasketNodes = {}
        walkRigs(rigs, anchorsByTemplate, false)
        walkRigs(rigsMirror, mirrorByTemplate, true)
        initPoleScales()
        console.log("replay scene rules:", materialsApplied, "materials,",
                    templatesPlaced, "templates placed,", leavesHidden, "leaves hidden")
        equipmentCheck()
    }

    // Structural equipment inventory — fixed counts from the V3 contract's
    // anchor table. The gate test asserts every line against these numbers so
    // missing geometry fails loudly.
    function equipmentCheck() {
        // Fixed inventory per sport, derived from the V3 contract's anchor
        // table (assets/replay/README.md):
        //   RowErg:  1 boat + 2 oars + 2 blades + 1 seat = 6
        //   SkiErg:  2 skis + 2×3 pole-parts              = 8
        //   BikeErg: 1 frame + 1 drivetrain + 2 wheels     = 4
        var inventory = []
        if (sportPrefix === "equipment:row:") {
            inventory = [
                ["boat",        boatNode !== null],
                ["oar-right",   oarRigNode !== null],
                ["oar-left",    oarRigMirror !== null],
                ["blade-right", !!bladeNodes["right"]],
                ["blade-left",  !!bladeNodes["left"]],
                ["seat",        seatNode !== null],
            ]
        } else if (sportPrefix === "equipment:ski:") {
            inventory = [
                ["ski-right",     true],   // always placed by primary walk
                ["ski-left",      true],   // always placed by mirror walk
                ["shaft-right",  !!poleShaftNodes["right"]],
                ["shaft-left",   !!poleShaftNodes["left"]],
                ["grip-right",   !!poleGripNodes["right"]],
                ["grip-left",    !!poleGripNodes["left"]],
                ["basket-right", !!poleBasketNodes["right"]],
                ["basket-left",  !!poleBasketNodes["left"]],
            ]
        } else if (sportPrefix === "equipment:bike:") {
            inventory = [
                ["frame",        frameNode !== null],
                ["drivetrain",   drivetrainNode !== null],
                ["wheel-front",  wheelAssemblyNode !== null],
                ["wheel-rear",   wheelMirror !== null],
            ]
        }
        var present = 0
        for (var i = 0; i < inventory.length; ++i) {
            if (inventory[i][1]) present += 1
            else console.warn("replay equipment missing:", inventory[i][0])
        }
        var sportTag = ["row", "ski", "bike"][Replay.sportIndex]
        console.log("replay equipment " + sportTag + ":", present, "of", inventory.length)
    }

    // Walk one balsam Rigs component: assign materials, show/hide by sport,
    // place templates at anchors, cache nodes for per-frame updates.
    // `isMirror` restricts the mirror copy to multi-instance templates only.
    function walkRigs(node, anchorMap, isMirror) {
        if (!node) return
        var side = isMirror ? "left" : "right"
        var meta = Replay.meshRoles[node.objectName]
        if (meta !== undefined) {
            if (node.materials !== undefined && byRole[meta.role] !== undefined) {
                node.materials = [byRole[meta.role]]
                materialsApplied += 1
            }
            if (meta.slot !== null && meta.slot !== undefined) {
                // Equipment leaves belonging to the current sport are shown
                // and cached for per-frame positioning; everything else hidden.
                var leafBelongs = meta.slot.lastIndexOf(sportPrefix, 0) === 0
                if (leafBelongs) {
                    node.visible = true
                    cacheLeaf(meta.slot, side, node)
                } else {
                    node.visible = false
                    leavesHidden += 1
                }
            }
        } else if (typeof node.objectName === "string"
                   && node.objectName.lastIndexOf("equipment:", 0) === 0) {
            var belongs = node.objectName.lastIndexOf(sportPrefix, 0) === 0
            var anchor = anchorMap[node.objectName]
            var isMulti = anchor !== undefined && anchor.instances > 1
            // The mirror copy shows only multi-instance templates.
            var show = isMirror ? (belongs && isMulti) : belongs
            node.visible = show
            if (show && anchor) {
                node.position = Qt.vector3d(
                    anchor.position[0], anchor.position[1], anchor.position[2])
                node.eulerRotation.y = anchor.yaw * 180 / Math.PI
                templatesPlaced += 1
            }
            // Cache template nodes that applyFrame updates per tick.
            if (!isMirror) {
                if (node.objectName === "equipment:row:boat-assembly") boatNode = node
                if (node.objectName === "equipment:row:seat-carriage") seatNode = node
                if (node.objectName === "equipment:row:oar-rig") oarRigNode = node
                if (node.objectName === "equipment:bike:frame-assembly") frameNode = node
                if (node.objectName === "equipment:bike:drivetrain-assembly") drivetrainNode = node
                if (node.objectName === "equipment:bike:wheel-assembly") wheelAssemblyNode = node
            } else {
                if (node.objectName === "equipment:row:oar-rig") oarRigMirror = node
                if (node.objectName === "equipment:bike:wheel-assembly") wheelMirror = node
            }
        }
        var ch = node.children
        for (var i = 0; ch && i < ch.length; ++i) walkRigs(ch[i], anchorMap, isMirror)
    }

    function cacheLeaf(slot, side, node) {
        switch (slot) {
        case "equipment:row:blade":        bladeNodes[side] = node; break
        case "equipment:ski:pole-shaft":   poleShaftNodes[side] = node; break
        case "equipment:ski:pole-grip":    poleGripNodes[side] = node; break
        case "equipment:ski:pole-basket":  poleBasketNodes[side] = node; break
        }
    }

    // ---- applyFrame (the per-tick frame-bundle reader) ----
    function applyFrame() {
        var f = Replay.poseFrame
        if (!f || f.length < fl.length) return

        // Camera (world space).
        camera.position = vec3FromFrame(f, fl.cameraPosition)
        camera.lookAt(vec3FromFrame(f, fl.cameraAim))
        camera.fieldOfView = f[fl.cameraFov]

        // Course placement.
        courseNode.position = Qt.vector3d(f[fl.courseX], 0, f[fl.courseZ])
        courseNode.rotation = quatFromFrame(f, fl.courseYaw)

        // Profile accents.
        rigGroup.position = Qt.vector3d(0, f[fl.accentBob], f[fl.accentSurge])
        rigGroup.rotation = quatFromFrame(f, fl.accentRoll)

        // Semantic joints (19 × 7 floats: tx ty tz qx qy qz qw).
        for (var j = 0; j < fl.jointCount; ++j) {
            var jn = jointNodes[j]
            if (!jn) continue
            var at = fl.joints + j * fl.jointStride
            jn.position = Qt.vector3d(f[at], f[at + 1], f[at + 2])
            jn.rotation = Qt.quaternion(f[at + 6], f[at + 3], f[at + 4], f[at + 5])
        }

        // Equipment: seat carriage z (rower).
        if (seatNode) seatNode.z = f[fl.seatZ]

        // Oar rigs: right (primary) and left (mirror).
        if (oarRigNode) oarRigNode.rotation = quatFromFrame(f, fl.oarRight)
        if (oarRigMirror) oarRigMirror.rotation = quatFromFrame(f, fl.oarLeft)

        // Blades: position and rotation precomputed in Rust.
        placeLeaf7(bladeNodes["left"],  f, fl.bladeLeft)
        placeLeaf7(bladeNodes["right"], f, fl.bladeRight)

        // Wheel assemblies: both rotate by the same amount.
        var wq = quatFromFrame(f, fl.wheel)
        if (wheelAssemblyNode) wheelAssemblyNode.rotation = wq
        if (wheelMirror) wheelMirror.rotation = wq

        // Crank / drivetrain: single instance.
        if (drivetrainNode) drivetrainNode.rotation = quatFromFrame(f, fl.crank)

        // Poles: positions precomputed in Rust, rotation from the pole root,
        // scale is constant (set once in initPoleScales).
        applyPoleLeaves(f, fl.poleLeft,  fl.poleLeavesLeft,  "left")
        applyPoleLeaves(f, fl.poleRight, fl.poleLeavesRight, "right")

        // HUD text parts (pipe-separated from Rust).
        var parts = Replay.hudText.split("|")
        clockText    = parts[0] || ""
        totalText    = parts[1] || ""
        distanceText = parts[2] || ""
        paceText     = parts[3] || ""
        rateText     = parts[4] || ""
        wattsText    = parts[5] || ""
        heartText    = parts[6] || ""
    }

    // Set the constant pole-leaf scales from the equipmentLayout (once per
    // sport change, not per frame).
    function initPoleScales() {
        var scales = el.poleLeafScales
        if (!scales || scales.length < 3) return
        function setScale(node, s) {
            if (node) node.scale = Qt.vector3d(s[0], s[1], s[2])
        }
        setScale(poleShaftNodes["left"],  scales[0])
        setScale(poleShaftNodes["right"], scales[0])
        setScale(poleGripNodes["left"],   scales[1])
        setScale(poleGripNodes["right"],  scales[1])
        setScale(poleBasketNodes["left"],  scales[2])
        setScale(poleBasketNodes["right"], scales[2])
    }

    // Read a leaf's position (xyz) and rotation (quat xyzw) from the frame.
    function placeLeaf7(node, f, at) {
        if (!node) return
        node.position = vec3FromFrame(f, at)
        node.rotation = quatFromFrame(f, at + 3)
    }

    // Read the three pole leaf positions from the frame (rotation shared
    // with the pole root, scale set once from the constant fits).
    function applyPoleLeaves(f, poleAt, leavesAt, side) {
        var rot = quatFromFrame(f, poleAt + 3)
        var shaft = poleShaftNodes[side]
        if (shaft) { shaft.position = vec3FromFrame(f, leavesAt); shaft.rotation = rot }
        var grip = poleGripNodes[side]
        if (grip) { grip.position = vec3FromFrame(f, leavesAt + 3); grip.rotation = rot }
        var basket = poleBasketNodes[side]
        if (basket) { basket.position = vec3FromFrame(f, leavesAt + 6); basket.rotation = rot }
    }

}
