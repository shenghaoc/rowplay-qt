// SPDX-License-Identifier: GPL-3.0-or-later
// The replay scene (Phase 5a assets + Phase 5b playback): procedural-sky IBL,
// filmic tonemapping, the web's per-sport key light, a ground plane tinted
// from the venue palette, and the V3 equipment / V4 athlete as balsam
// components (ADR 0008). Phase 5b adds the chase camera, athlete joint
// posing, equipment motion (oars, poles, crank, wheels on the course loop)
// and the transport, a floating HUD since the design system (ADR 0013). One
// FrameAnimation drives Replay.tick; one onFrameChanged applies the flat
// frame bundle — one bridge crossing per rendered frame (spec R1.3).
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
    // Read once: every `Replay.meshRoles` access converts the whole JSON map
    // across the bridge, and walkRigs looks a role up for every node of four
    // Rigs components (~115 ms per scene walk in a debug build when read per
    // node, ~1 ms from this copy). The map is Constant.
    readonly property var meshRoles: Replay.meshRoles
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
    // Ghost node caches (same structure as player, separate references).
    property var ghostJointNodes: []
    property var ghostSeatNode: null
    property var ghostOarRigNode: null
    property var ghostOarRigMirror: null
    property var ghostWheelNode: null
    property var ghostWheelMirror: null
    property var ghostDrivetrainNode: null
    property var ghostBladeNodes: ({})
    property var ghostPoleShaftNodes: ({})
    property var ghostGripNodes: ({})
    property var ghostBasketNodes: ({})
    // Gate hook: the shadow check's twin capture renders the same frame with
    // the key light's shadow off (Main.qml, the tier cycle's High step).
    property bool shadowsSuppressed: false
    // ---- HUD text parts (updated by applyFrame) ----
    property string clockText: ""
    property string totalText: ""
    property string distanceText: ""
    property string paceText: ""
    property string rateText: ""
    property string wattsText: ""
    property string heartText: ""

    // Runtime-gate probe: inspect rendered bounds, independent of which
    // layout arranges the labels. Called only by Main's gate walk.
    function gateGapLayoutFits() {
        function inside(item) {
            var at = item.mapToItem(hud, 0, 0)
            var fits = at.x >= Theme.spacingLarge - 1 && at.y >= Theme.spacingLarge - 1
                && at.x + item.width <= hud.width - Theme.spacingLarge + 1
                && at.y + item.height <= hud.height - Theme.spacingLarge + 1
            if (!fits)
                console.log("gate gap bounds:", at.x, at.y, item.width, item.height,
                            "HUD", hud.width, hud.height, "margin", Theme.spacingLarge)
            return fits
        }
        var gap = inlineGap.visible ? inlineGap : compactGap
        if (!gap.visible || !inside(gap) || gap.contentWidth > gap.width + 1
                || gap.contentHeight > gap.height + 1) {
            console.log("gate gap text:", gap.visible, gap.contentWidth, gap.contentHeight,
                        "allocated", gap.width, gap.height)
            return false
        }
        // This demo has all four gauges: missing a chip cannot make it pass.
        for (var i = 0; i < metricChips.count; ++i) {
            var chip = metricChips.itemAt(i)
            if (!chip || !chip.visible || !inside(chip)
                    || chip.implicitWidth > chip.width + 1) {
                console.log("gate metric chip:", i, chip ? chip.implicitWidth : -1,
                            "allocated", chip ? chip.width : -1)
                return false
            }
            var at = chip.mapToItem(hud, 0, 0)
            var gapAt = gap.mapToItem(hud, 0, 0)
            if (at.x < gapAt.x + gap.width && at.x + chip.width > gapAt.x
                    && at.y < gapAt.y + gap.height && at.y + chip.height > gapAt.y)
                return false
        }
        return metricChips.count === 4
    }

    // ---- 3D scene ----
    View3D {
        id: scene
        // The scene fills the route; the transport floats over it.
        anchors.fill: parent

        // The tier settings resolve MSAA and shadow parameters from the
        // current quality index + sport (Replay.tierSettings JSON).
        readonly property var ts: Replay.tierSettings
        readonly property int msaa: ts ? ts.msaaSamples : 4

        environment: ExtendedSceneEnvironment {
            backgroundMode: SceneEnvironment.SkyBox
            antialiasingMode: scene.msaa > 0 ? SceneEnvironment.MSAA
                                              : SceneEnvironment.NoAA
            antialiasingQuality: scene.msaa >= 4 ? SceneEnvironment.High
                                                 : SceneEnvironment.Medium
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
                id: keyLight
                color: Replay.skySun; brightness: 1.2
                castsShadow: !replayRoot.shadowsSuppressed
                             && (scene.ts ? scene.ts.shadows : true)
                // The tier's map size (TierSettings.shadow_map_size, the
                // web's shadow.mapSize): 1024 at High, 2048 at Ultra (#96).
                shadowMapQuality: scene.ts && scene.ts.shadowMapSize >= 2048
                                  ? Light.ShadowMapQualityVeryHigh
                                  : Light.ShadowMapQualityHigh
                shadowFactor: 80
                // With the venue flagged as the web flags it, only the live
                // athlete both casts and receives, and the acne left was a
                // speckle on the body at 0.02 m of bias and 4-sample PCF.
                // 0.05 m with a 32-bit depth map and 16 samples over 5 cm
                // clears it; 0.1 m cleared it too, but thinned the rower's
                // own shadow in the gate's check capture to 0.96 % (#96).
                shadowBias: 0.05; use32BitShadowmap: true
                softShadowQuality: Light.PCF16; pcfFactor: 0.05
                shadowMapFar: 60; csmNumSplits: 2
            }
        }

        // Ground plane (venue palette tint, ADR 0005).
        Model {
            source: "#Rectangle"; y: 0; scale: Qt.vector3d(60, 60, 1)
            eulerRotation.x: -90; receivesShadows: true; castsShadows: false
            materials: PrincipledMaterial { baseColor: Replay.groundColor; roughness: 0.9 }
        }

        // ---- Venue (Phase 6b): the twelve baked venue variants ----
        // The GLBs are authored in the web's course space (innerR 22 /
        // outerR 34, y-up, metres), so the components sit at the origin with
        // no offset. All twelve are instantiated statically — the balsam
        // components are engine-owned, which a dynamically created 3D
        // component is not reliably part of the renderable scene (found in
        // 6b; qt-bridges-notes) — and exactly the (sport, effective tier)
        // match is visible. Visibility flips are instant, so governor
        // step-downs swap the venue without a reload. Each variant is walked
        // (materials, instance buckets, tier textures) once, on first show.
        Node {
            id: venueRoot

            VenueRowerLow {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 0
                         && Replay.effectiveQuality === 0
            }
            VenueRowerMedium {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 0
                         && Replay.effectiveQuality === 1
            }
            VenueRowerHigh {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 0
                         && Replay.effectiveQuality === 2
            }
            VenueRowerUltra {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 0
                         && Replay.effectiveQuality === 3
            }
            VenueSkiLow {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 1
                         && Replay.effectiveQuality === 0
            }
            VenueSkiMedium {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 1
                         && Replay.effectiveQuality === 1
            }
            VenueSkiHigh {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 1
                         && Replay.effectiveQuality === 2
            }
            VenueSkiUltra {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 1
                         && Replay.effectiveQuality === 3
            }
            VenueBikeLow {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 2
                         && Replay.effectiveQuality === 0
            }
            VenueBikeMedium {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 2
                         && Replay.effectiveQuality === 1
            }
            VenueBikeHigh {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 2
                         && Replay.effectiveQuality === 2
            }
            VenueBikeUltra {
                visible: Replay.loadState !== "error" && Replay.sportIndex === 2
                         && Replay.effectiveQuality === 3
            }
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

        // ---- Ghost group (on the ghost loop, inside the same View3D) ----
        Node {
            id: ghostCourseNode
            visible: Replay.hasGhost
            Node {
                id: ghostRigGroup
                Rigs {
                    id: ghostRigs
                    visible: Replay.hasGhost && Replay.loadState !== "error"

                    // Ghost material variants: athlete body parts stay opaque,
                    // equipment and athlete trim/shorts go 45% opacity (the V4
                    // depth contract, materialSpecs.ghostOpacity). The ghost
                    // painted equipment uses the ghost paint colour.
                    PrincipledMaterial { id: gMatSkin; baseColor: Theme.replaySkin; metalness: 0.0; roughness: 0.55 }
                    PrincipledMaterial { id: gMatFabric; baseColor: Theme.replayFabric; metalness: 0.0; roughness: 0.8 }
                    PrincipledMaterial { id: gMatHair; baseColor: Theme.replayHair; metalness: 0.0; roughness: 0.45 }
                    PrincipledMaterial { id: gMatFootwear; baseColor: Theme.replayFootwear; metalness: 0.0; roughness: 0.6 }
                    PrincipledMaterial { id: gMatShorts; baseColor: Theme.replayShorts; metalness: 0.0; roughness: 0.8; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatTrim; baseColor: Theme.replayTrim; metalness: 0.1; roughness: 0.4; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatEye; baseColor: Theme.replayEye; metalness: 0.0; roughness: 0.15 }
                    PrincipledMaterial { id: gMatFaceDetail; baseColor: Theme.replayFaceDetail; metalness: 0.0; roughness: 0.6 }
                    PrincipledMaterial { id: gMatPainted; baseColor: Replay.ghostPaint; metalness: 0.05; roughness: 0.35; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatDark; baseColor: Theme.replayEquipmentDark; metalness: 0.2; roughness: 0.5; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatLight; baseColor: Theme.replayEquipmentLight; metalness: 0.1; roughness: 0.4; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatMetal; baseColor: Theme.replayEquipmentMetal; metalness: 0.9; roughness: 0.25; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatRubber; baseColor: Theme.replayEquipmentRubber; metalness: 0.0; roughness: 0.9; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatGrip; baseColor: Theme.replayEquipmentGrip; metalness: 0.0; roughness: 0.85; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                    PrincipledMaterial { id: gMatTrimEq; baseColor: Theme.replayEquipmentTrim; metalness: 0.3; roughness: 0.4; opacity: 0.45; alphaMode: PrincipledMaterial.Blend }
                }
                Rigs {
                    id: ghostRigsMirror
                    visible: Replay.hasGhost && Replay.loadState !== "error"
                }
                Athlete {
                    id: ghostAthlete
                    visible: Replay.hasGhost && Replay.loadState !== "error"
                }
            }
        }

        // Drive Replay.tick from the rendering loop, but only while something
        // moves: playback, or a short settle after a change made while
        // paused (#93). A still scene renders on demand. On macOS (cocoa,
        // Metal) a running FrameAnimation keeps the window rendering at the
        // display rate, 120 frames/s for a paused replay; under Xvfb +
        // llvmpipe it fires only on frames something else caused, so the
        // settle asks the View3D for each of its frames itself (a window
        // update alone does not re-render a View3D that is not dirty).
        FrameAnimation {
            running: replayRoot.visible && Replay.hasWorkout
                     && Replay.loadState === "ready"
                     && (Replay.playing || replayRoot.settleFrames > 0)
            onTriggered: {
                Replay.tick(frameTime)
                if (Replay.playing) {
                    // Feed the governor with the Quick 3D render pass cost
                    // of playback; a still scene says nothing about it.
                    Replay.sampleRenderTime(scene.renderStats.frameTime)
                } else if (replayRoot.settleFrames > 0) {
                    replayRoot.settleFrames -= 1
                    if (replayRoot.settleFrames > 0) {
                        scene.update()
                    }
                }
                // Bench: collect renderStats.frameTime for measurement,
                // plus wall-clock delta for cross-check.
                if (replayRoot.benchCollecting) {
                    replayRoot.benchSample(scene.renderStats.frameTime)
                    if (replayRoot.benchWarmup <= 0)
                        replayRoot.benchWallSample(frameTime * 1000)
                }
            }
        }

        Component.onCompleted: {
            scene.rebuildSky()
            replayRoot.setupScene()
        }
    }

    // ---- HUD hit areas (round 3's 2f) ----
    // Touch needs a larger target than a pointer: every HUD control answers
    // within at least `minimumTarget` px each way. The mask reaches past the
    // control's edges, so its compact visual and its place in the layout
    // stay as they are; `reachX` caps the sideways reach where another
    // control or the times sit beside it, so no two hit areas overlap.
    readonly property real minimumTarget: 40
    component HitArea: QtObject {
        required property Item target
        required property real minimum
        property real reachX: Infinity
        function contains(point: point) : bool {
            const dx = Math.min(reachX, Math.max(0, (minimum - target.width) / 2))
            const dy = Math.max(0, (minimum - target.height) / 2)
            return point.x >= -dx && point.x <= target.width + dx
                && point.y >= -dy && point.y <= target.height + dy
        }
    }

    // ---- Transport HUD (ADR 0013, 0015) ----
    // Floating controls over the scene: play / pause, elapsed time, the
    // scrubber, total time and distance; then the speed, the metric chips
    // and the race gap, which move under the speed where the two do not fit
    // side by side (a compact window); the verdict gets its own line when a
    // ghost finishes. The controls are the style's (ADR 0015); the panel,
    // the times, the chips and the verdict are content, drawn as before.
    // Back navigation is the toolbar's leading button (Main.qml). The HUD is
    // opaque, on the grouped surface: translucent, text on it fell below AA
    // over dark parts of the scene. All text is pre-rendered in Rust
    // (Replay.hudText). While playing it hides after about 3 s without
    // pointer movement (see "HUD auto-hide" below).
    Rectangle {
        id: hud

        visible: Replay.hasWorkout && opacity > 0
        opacity: replayRoot.hudShown ? 1 : 0
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Theme.spacingXLarge
        width: Math.min(Theme.px(760), parent.width - 2 * Theme.spacingXLarge)
        height: hudColumn.implicitHeight + 2 * Theme.spacingLarge
        radius: Theme.radiusLarge
        color: Theme.overlayBackground
        border.width: Theme.outlineWidth
        border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator

        Behavior on opacity {
            enabled: !Theme.reduceMotion
            NumberAnimation { duration: 200 }
        }

        // The verdict: parts[0] is win / lose / tie (locale ids from the web).
        readonly property var verdictParts: Replay.verdictText.split("|")
        readonly property bool compactGap: Theme.widthClass(replayRoot.width) === Theme.widthCompact
        // The race gap as the web words it: parts[0] is ahead / behind, the
        // metres fill the locale's {m} and the seconds, formatted in Rust
        // with their unit, follow in brackets. Empty without a ghost.
        readonly property string gapLabel: {
            var parts = Replay.gapText.split("|")
            if (parts.length !== 3)
                return ""
            if (parts[0] === "ahead")
                return Tr.t("replay.ahead", { m: parts[1] }) + " (" + parts[2] + ")"
            if (parts[0] === "behind")
                return Tr.t("replay.behind", { m: parts[1] }) + " (" + parts[2] + ")"
            return ""
        }

        ColumnLayout {
            id: hudColumn
            anchors.fill: parent
            anchors.margins: Theme.spacingLarge
            spacing: Theme.spacingMedium

            // Each row of controls is at least the minimum target tall, so
            // the controls' hit areas fit inside their rows and never reach
            // the next one (a style with smaller controls, Fusion, would
            // otherwise overlap them by a couple of pixels).
            RowLayout {
                Layout.fillWidth: true
                Layout.minimumHeight: replayRoot.minimumTarget
                spacing: Theme.spacingMedium

                // Play / pause: a command button showing the platform's play
                // or pause symbol, as the transport state has it.
                CommandButton {
                    id: playButton
                    glyph: Replay.playing ? "pause" : "play"
                    text: Replay.playing ? Tr.t("replay.pause") : Tr.t("replay.play")
                    shortcutText: playShortcut.nativeText
                    icon.width: Theme.px(18)
                    icon.height: Theme.px(18)
                    containmentMask: HitArea {
                        target: playButton
                        minimum: replayRoot.minimumTarget
                    }
                    onClicked: Replay.toggle()
                }

                // The times and the distance keep their width; the
                // scrubber gives way in a narrow window.
                Label {
                    Layout.minimumWidth: implicitWidth
                    text: replayRoot.clockText
                    font: Theme.tabularBody
                    color: Theme.textPrimary
                    Accessible.name: text
                }

                // The scrubber stays the HUD's own (ADR 0015: the HUD is
                // content). The macOS style's slider draws its track beyond
                // the knob at 243 on the HUD's 244 grey, so the rest of the
                // workout vanished. Its hit area reaches up and down to the
                // minimum target, not sideways, where the times sit.
                AppSlider {
                    id: seekSlider
                    Layout.fillWidth: true
                    Layout.minimumWidth: Theme.px(64)
                    from: 0
                    to: 1
                    value: Replay.progress
                    containmentMask: HitArea {
                        target: seekSlider
                        minimum: replayRoot.minimumTarget
                        reachX: 0
                    }
                    onMoved: Replay.seek(value)
                    Accessible.name: Tr.t("replay.seekSlider")
                }

                Label {
                    Layout.minimumWidth: implicitWidth
                    text: replayRoot.totalText
                    font: Theme.tabularBody
                    color: Theme.textSecondary
                    Accessible.name: text
                }
                Label {
                    Layout.leftMargin: Theme.spacingSmall
                    Layout.minimumWidth: implicitWidth
                    text: replayRoot.distanceText
                    font: Theme.tabularBody
                    color: Theme.metricDistance
                    Accessible.name: text
                }
            }

            // The speed beside the chips, or above them where the two need
            // more than the HUD's width. The switch back waits for some
            // spare room, so a value that widens during playback cannot
            // flip the layout back and forth.
            GridLayout {
                id: hudLower

                property bool needsStack: false
                readonly property bool stacked: hud.compactGap || needsStack
                readonly property real needed: speedControl.implicitWidth + columnSpacing
                                               + chipRow.implicitWidth
                // The room the HUD gives the row, not the row's own width:
                // side by side the row is never narrower than its two
                // columns' minimum, which can equal `needed`, so its own
                // width would never show that they do not fit.
                readonly property real room: hudColumn.width

                function restack() {
                    if (room <= 0) {
                        return
                    }
                    if (!needsStack && needed > room) {
                        needsStack = true
                    } else if (needsStack && needed + Theme.px(24) < room) {
                        needsStack = false
                    }
                }
                onRoomChanged: restack()
                onNeededChanged: restack()

                Layout.fillWidth: true
                columns: stacked ? 1 : 2
                columnSpacing: Theme.spacingXLarge
                rowSpacing: Theme.spacingMedium

                // The speed (design.md, "SegmentedControl, per use"): the
                // style's tool buttons, checkable in an exclusive group and
                // all as wide as the widest, at least the minimum target. One
                // tab stop, the checked choice, as in a radio group: the
                // arrows move the choice while it has focus, and neither they
                // nor Space reach the window's seek and play shortcuts then.
                RowLayout {
                    id: speedControl

                    // Every choice is as wide as the widest label in bold,
                    // the checked choice's weight, so a change of speed moves
                    // nothing (a layout's uniform cells take the mean), and
                    // at least the minimum target.
                    readonly property real choiceWidth: {
                        var widest = replayRoot.minimumTarget
                        for (var i = 0; i < speedRepeater.count; ++i) {
                            var choice = speedRepeater.itemAt(i)
                            if (choice) {
                                widest = Math.max(widest, choice.implicitWidth,
                                                  Math.ceil(boldMetrics.advanceWidth(choice.text))
                                                  + choice.leftPadding + choice.rightPadding)
                            }
                        }
                        return widest
                    }

                    FontMetrics {
                        id: boldMetrics
                        font: {
                            var base = speedRepeater.count > 0 ? speedRepeater.itemAt(0).font
                                                               : Qt.application.font
                            return Qt.font({ family: base.family, pixelSize: base.pixelSize,
                                             bold: true })
                        }
                    }

                    function select(index) {
                        if (index < 0 || index >= speedRepeater.count) {
                            return
                        }
                        Replay.setSpeedIndex(index)
                        speedRepeater.itemAt(index).forceActiveFocus(Qt.TabFocusReason)
                    }

                    // A nested layout fills by default; the choices keep
                    // their own width.
                    Layout.fillWidth: false
                    Layout.minimumHeight: replayRoot.minimumTarget
                    spacing: Theme.spacingXxSmall
                    Accessible.role: Accessible.Grouping
                    Accessible.name: Tr.t("replay.playbackSpeed")

                    ButtonGroup {
                        id: speedGroup
                    }

                    Repeater {
                        id: speedRepeater
                        model: Replay.speedLabels

                        ToolButton {
                            id: speedButton

                            required property string modelData
                            required property int index

                            Layout.preferredWidth: speedControl.choiceWidth
                            text: modelData
                            checkable: true
                            checked: index === Replay.speedIndex
                            ButtonGroup.group: speedGroup
                            focusPolicy: checked ? Qt.TabFocus : Qt.NoFocus
                            // Up and down only: the choices are a few pixels
                            // apart.
                            containmentMask: HitArea {
                                target: speedButton
                                minimum: replayRoot.minimumTarget
                                reachX: 0
                            }
                            // The checked choice's label is bold as well as on the
                            // style's checked wash: Basic's wash differs from the
                            // others by 1.4:1 in light and 1.05:1 in dark, too
                            // little to carry the choice alone.
                            font.bold: checked
                            Accessible.role: Accessible.RadioButton
                            Accessible.name: text
                            onClicked: Replay.setSpeedIndex(index)
                            Keys.onLeftPressed: speedControl.select(index - 1)
                            Keys.onRightPressed: speedControl.select(index + 1)
                            Keys.onShortcutOverride: function(event) {
                                if (event.modifiers === Qt.NoModifier
                                        && (event.key === Qt.Key_Left
                                            || event.key === Qt.Key_Right
                                            || event.key === Qt.Key_Space)) {
                                    event.accepted = true
                                }
                            }
                        }
                    }
                }

                // Beside the speed as tall as it, so its items centre in the
                // row exactly as they did in one RowLayout. At its own
                // height (a layout's maximum comes from its items, so
                // fillHeight cannot stretch it) the grid centred it at a
                // rounded 2 px, and the race gap sat 1 px lower.
                GridLayout {
                    id: chipRow
                    Layout.fillWidth: true
                    Layout.minimumHeight: hudLower.stacked
                                          ? 0
                                          : Math.max(speedControl.implicitHeight,
                                                     replayRoot.minimumTarget)
                    columns: hud.compactGap ? 2 : 6
                    columnSpacing: Theme.spacingXLarge
                    rowSpacing: Theme.spacingMedium

                    // Beside the speed the chips sit at the trailing edge,
                    // under it at the leading edge.
                    Item {
                        Layout.fillWidth: true
                        visible: !hudLower.stacked
                    }

                    // Metric chips: the web gauge caption above the tabular
                    // value in its Metric Mapping Rule colour (the caption
                    // names the metric, so the colour is never the only cue).
                    // The model is constant and each chip reads its value by
                    // index, so a new frame updates two labels in place; a
                    // model built from the values would rebuild the chips
                    // whenever one changed.
                    Repeater {
                        id: metricChips
                        model: [
                            { id: "replay.gPace", role: 3 },
                            { id: "replay.gRate", role: 6 },
                            { id: "replay.gPower", role: 4 },
                            { id: "replay.gHeart", role: 5 }
                        ]

                        ColumnLayout {
                            id: chip

                            required property var modelData
                            required property int index
                            readonly property string value: index === 0 ? replayRoot.paceText
                                                          : index === 1 ? replayRoot.rateText
                                                          : index === 2 ? replayRoot.wattsText
                                                          : replayRoot.heartText

                            visible: value.length > 0
                            spacing: 0
                            Accessible.name: Tr.t(modelData.id) + " " + value

                            Label {
                                text: Tr.t(chip.modelData.id)
                                font: Theme.compactLabel
                                color: Theme.textSecondary
                                Accessible.ignored: true
                            }
                            Label {
                                text: chip.value
                                font: Theme.tabularBody
                                color: Theme.metricColor(chip.modelData.role)
                                Accessible.ignored: true
                            }
                        }
                    }

                    // Race gap: the web's words and ▲ / ▼ glyph (visible
                    // when a ghost is loaded).
                    Label {
                        id: inlineGap
                        visible: !hud.compactGap && Replay.hasGhost && hud.gapLabel.length > 0
                        text: hud.gapLabel
                        font: Theme.tabularBody
                        color: Theme.textPrimary
                        Accessible.name: text
                    }

                    Item {
                        Layout.fillWidth: true
                        visible: hudLower.stacked && !hud.compactGap
                    }
                }
            }

            // Long translations get the full compact HUD width rather than
            // competing with all four metric chips on the same line.
            Label {
                id: compactGap
                Layout.fillWidth: true
                visible: hud.compactGap && Replay.hasGhost && hud.gapLabel.length > 0
                text: hud.gapLabel
                font: Theme.tabularBody
                color: Theme.textPrimary
                wrapMode: Text.Wrap
                Accessible.name: text
            }

            // Race verdict at finish (locale ids from the web): the words
            // carry the result, the colour only repeats it.
            Label {
                Layout.fillWidth: true
                visible: Replay.hasGhost && Replay.verdictText.length > 0
                text: {
                    var parts = hud.verdictParts
                    if (parts[0] === "win")
                        return Tr.t("replay.raceVerdictWinSession",
                            { seconds: parts[1], m: parts[2],
                              date: "", distance: "" })
                    if (parts[0] === "lose")
                        return Tr.t("replay.raceVerdictLoseSession",
                            { seconds: parts[1], m: parts[2],
                              date: "", distance: "" })
                    return Tr.t("replay.raceFinished")
                }
                font: Theme.bodyEmphasized
                color: hud.verdictParts[0] === "win" ? Theme.energeticGreen
                     : hud.verdictParts[0] === "lose" ? Theme.alertRed
                     : Theme.textPrimary
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                Accessible.name: text
            }
        }
    }

    // ---- HUD auto-hide ----
    // While playing, the HUD fades out after about 3 s without pointer
    // movement, and the pointer hides with it. Any pointer movement, a tap,
    // a replay key, a focus change or a pause brings it back (instantly
    // under reduce motion). It never hides while keyboard focus is inside it.
    property bool hudShown: true
    readonly property bool hudHasFocus: {
        var item = replayRoot.Window.activeFocusItem
        while (item) {
            if (item === hud) {
                return true
            }
            item = item.parent
        }
        return false
    }
    // The one state in which the HUD may hide: shown, playing, and keyboard
    // focus elsewhere. Every change of it (play or pause, the route shown or
    // left, focus into or out of the HUD) wakes the HUD.
    readonly property bool hudMayHide: visible && Replay.playing && !hudHasFocus

    // Shows the HUD and restarts the countdown, only while it may hide.
    // Timer.restart() starts a stopped timer whatever a `running` binding
    // says, so the countdown is driven here alone, never by a binding.
    function wakeHud() {
        hudShown = true
        if (hudMayHide) {
            hudTimer.restart()
        } else {
            hudTimer.stop()
        }
    }

    onHudMayHideChanged: wakeHud()

    Timer {
        id: hudTimer
        interval: 3000
        onTriggered: {
            if (replayRoot.hudMayHide) {
                replayRoot.hudShown = false
            }
        }
    }

    // Only a real pointer move wakes the HUD. While the scene animates, Qt
    // Quick delivers a synthetic hover event at the last pointer position
    // after every frame that changed an item
    // (QQuickDeliveryAgentPrivate::flushFrameSynchronousEvents, on by
    // default), and waking on those kept the HUD up forever.
    property point lastPointer: Qt.point(-1, -1)
    HoverHandler {
        cursorShape: replayRoot.hudShown ? Qt.ArrowCursor : Qt.BlankCursor
        onPointChanged: {
            const p = point.position
            if (Math.abs(p.x - replayRoot.lastPointer.x) >= 1
                    || Math.abs(p.y - replayRoot.lastPointer.y) >= 1) {
                replayRoot.lastPointer = Qt.point(p.x, p.y)
                replayRoot.wakeHud()
            }
        }
    }
    TapHandler {
        onTapped: replayRoot.wakeHud()
    }
    Connections {
        target: replayRoot.Window.window
        function onActiveFocusItemChanged() {
            if (replayRoot.visible) {
                replayRoot.wakeHud()
            }
        }
    }

    // Error / loading overlay (when no workout is loaded): the message and
    // the way back, on the HUD's surface.
    Rectangle {
        visible: !Replay.hasWorkout
        anchors.centerIn: parent
        width: Math.min(Theme.px(420), parent.width - 2 * Theme.spacingXLarge)
        height: overlayColumn.implicitHeight + 2 * Theme.spacingXLarge
        radius: Theme.radiusLarge
        color: Theme.overlayBackground
        border.width: Theme.outlineWidth
        border.color: Theme.highContrast ? Theme.controlBorder : Theme.separator

        ColumnLayout {
            id: overlayColumn
            anchors.fill: parent
            anchors.margins: Theme.spacingXLarge
            spacing: Theme.spacingLarge

            Label {
                Layout.fillWidth: true
                visible: Replay.loadState !== "ready"
                text: Replay.loadState === "error"
                      ? Tr.t("replay.view3dError")
                        + (Replay.errorText.length > 0 ? " — " + Replay.errorText : "")
                      : Tr.t("replay.view3dLoading")
                font: Theme.body
                color: Replay.loadState === "error" ? Theme.alertRed : Theme.textSecondary
                wrapMode: Text.WordWrap
                horizontalAlignment: Text.AlignHCenter
                Accessible.name: text
            }
            Button {
                Layout.alignment: Qt.AlignHCenter
                text: Tr.t("replay.closePanel")
                Accessible.name: text
                onClicked: Library.closeReplay()
            }
        }
    }

    // ---- Keyboard shortcuts (web replay transport keys) ----
    // Every one of them also brings the HUD back.
    Shortcut { id: playShortcut; enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Space"; onActivated: { Replay.toggle(); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Left"; onActivated: { Replay.seekBy(-10); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Right"; onActivated: { Replay.seekBy(10); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Shift+Left"; onActivated: { Replay.seekBy(-30); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "Shift+Right"; onActivated: { Replay.seekBy(30); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "["; onActivated: { Replay.stepSpeed(-1); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequence: "]"; onActivated: { Replay.stepSpeed(1); replayRoot.wakeHud() } }
    Shortcut { enabled: replayRoot.visible && Replay.hasWorkout; sequences: ["Home", "0"]; onActivated: { Replay.seek(0); replayRoot.wakeHud() } }

    // ---- On-demand rendering while paused (#93) ----
    // A change made while paused (a seek, the sport, tier or scheme, a
    // ghost, the viewport) renders on its own, because it changes the
    // scene. The tick animation then runs a few frames more, each one
    // asking the View3D for a render: buffer uploads and the sky's light
    // probe can take more than one frame to land. After that the scene is
    // still and renders nothing until the next change. The HUD's fade runs
    // its own animation.
    readonly property int settleFrameCount: 6
    property int settleFrames: 0
    function settle() {
        if (!Replay.playing) {
            settleFrames = settleFrameCount
        }
    }
    // Leaving the route pauses the replay. It reloads from the start when
    // the route opens again, so ticking it out of sight only cost frames.
    onVisibleChanged: {
        if (!visible) {
            Replay.pause()
        }
    }

    // ---- Connections ----
    Connections {
        target: Replay
        function onFrameChanged() {
            replayRoot.applyFrame()
            replayRoot.settle()
        }
        function onReplayChanged() {
            if (Replay.loadState === "ready") {
                replayRoot.applySceneRules()
                // Venue: all tier/quality/sport notifications share one
                // signal, so detect what actually changed.
                replayRoot.syncVenue()
            }
            replayRoot.settle()
        }
        function onPlaybackChanged() { replayRoot.settle() }
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
    // Ghost material variant: same keys, but equipment and athlete trim/shorts
    // get 45% opacity (the V4 depth contract).
    property var ghostByRole: ({
        "athlete-skin": gMatSkin, "athlete-fabric": gMatFabric,
        "athlete-hair": gMatHair, "athlete-footwear": gMatFootwear,
        "athlete-shorts": gMatShorts, "athlete-trim": gMatTrim,
        "athlete-eye": gMatEye, "athlete-face-detail": gMatFaceDetail,
        "equipment-painted": gMatPainted, "equipment-dark": gMatDark,
        "equipment-light": gMatLight, "equipment-metal": gMatMetal,
        "equipment-rubber": gMatRubber, "equipment-grip": gMatGrip,
        "equipment-trim": gMatTrimEq,
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
        // Ghost joint lookup.
        var gjn = []
        for (var k = 0; k < semanticBones.length; ++k) gjn.push(null)
        walkForJoints(ghostAthlete, gjn)
        ghostJointNodes = gjn
        if (!validateMaterials()) return
        applySceneRules()
        // The key light's shadow roles, as the web flags its objects: the
        // live athlete casts and receives, the live equipment only casts,
        // the ghost does neither (renderer3d.ts, `finalizeAvatar`). A Qt
        // Quick 3D Model casts and receives by default, three.js meshes
        // neither; the venue's roles come with its walk (#96).
        setShadowRoles(athlete, true, true)
        setShadowRoles(rigs, true, false)
        setShadowRoles(rigsMirror, true, false)
        setShadowRoles(ghostAthlete, false, false)
        setShadowRoles(ghostRigs, false, false)
        setShadowRoles(ghostRigsMirror, false, false)
        venueSchemeCached = Replay.schemeDark
        syncVenue()
        Replay.reportReady()
    }

    function setShadowRoles(node, casts, receives) {
        if (!node) return
        if (node.castsShadows !== undefined) {
            node.castsShadows = casts
            node.receivesShadows = receives
        }
        var ch = node.children
        for (var i = 0; ch && i < ch.length; ++i) setShadowRoles(ch[i], casts, receives)
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
        // Ghost rigs: same materials and anchors, separate node caches.
        // Save and restore the player caches around the ghost walk.
        var savedSeat = seatNode, savedBoat = boatNode
        var savedDrive = drivetrainNode, savedFrame = frameNode
        var savedOar = oarRigNode, savedOarM = oarRigMirror
        var savedWheel = wheelAssemblyNode, savedWheelM = wheelMirror
        var savedBlade = bladeNodes, savedShaft = poleShaftNodes
        var savedGrip = poleGripNodes, savedBasket = poleBasketNodes
        seatNode = null; boatNode = null; drivetrainNode = null; frameNode = null
        oarRigNode = null; oarRigMirror = null
        wheelAssemblyNode = null; wheelMirror = null
        bladeNodes = {}; poleShaftNodes = {}; poleGripNodes = {}; poleBasketNodes = {}
        walkRigs(ghostRigs, anchorsByTemplate, false, ghostByRole)
        walkRigs(ghostRigsMirror, mirrorByTemplate, true, ghostByRole)
        ghostSeatNode = seatNode; ghostOarRigNode = oarRigNode
        ghostOarRigMirror = oarRigMirror; ghostWheelNode = wheelAssemblyNode
        ghostWheelMirror = wheelMirror; ghostDrivetrainNode = drivetrainNode
        ghostBladeNodes = bladeNodes; ghostPoleShaftNodes = poleShaftNodes
        ghostGripNodes = poleGripNodes; ghostBasketNodes = poleBasketNodes
        // Restore player caches.
        seatNode = savedSeat; boatNode = savedBoat
        drivetrainNode = savedDrive; frameNode = savedFrame
        oarRigNode = savedOar; oarRigMirror = savedOarM
        wheelAssemblyNode = savedWheel; wheelMirror = savedWheelM
        bladeNodes = savedBlade; poleShaftNodes = savedShaft
        poleGripNodes = savedGrip; poleBasketNodes = savedBasket
        initPoleScales()
        console.log("replay scene rules:", materialsApplied, "materials,",
                    templatesPlaced, "templates placed,", leavesHidden, "leaves hidden")
        walkGripHelpers()
        equipmentCheck()
    }

    // Finger grip table (Phase 7): the backend solves each sport's digit
    // closure once per sport switch and exposes the helpers' final local
    // rotations as JSON. The clips never animate the finger helpers and the
    // per-frame bundle only carries the 19 semantic joints, so one
    // application per sport walk sticks. Ghost rigs share the helper names
    // and get the same table.
    function collectNodesByName(node, map) {
        if (!node) return
        if (node.objectName && node.rotation !== undefined) map[node.objectName] = node
        var ch = node.children
        for (var i = 0; ch && i < ch.length; ++i) collectNodesByName(ch[i], map)
    }

    function walkGripHelpers() {
        var table = {}
        try { table = JSON.parse(Replay.gripPoses) } catch (e) { table = {} }
        var names = Object.keys(table)
        var playerMap = {}, ghostMap = {}
        collectNodesByName(athlete, playerMap)
        collectNodesByName(ghostAthlete, ghostMap)
        var found = 0, missing = []
        for (var i = 0; i < names.length; ++i) {
            var q = table[names[i]]
            var quat = Qt.quaternion(q[3], q[0], q[1], q[2])
            var posed = false
            if (playerMap[names[i]] !== undefined) {
                playerMap[names[i]].rotation = quat; posed = true
            }
            if (ghostMap[names[i]] !== undefined) {
                ghostMap[names[i]].rotation = quat; posed = true
            }
            if (posed) found++
            else missing.push(names[i])
        }
        var sportTag = ["row", "ski", "bike"][Replay.sportIndex]
        if (missing.length > 0)
            console.warn("replay grip FAILED " + sportTag + ": missing helpers",
                         missing.join(","))
        else console.log("replay grip " + sportTag + ":", Replay.gripContacts,
                         "digit contacts")
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
        // Tier readback: log the actual applied scene settings so the bench
        // and gate can verify the tier picker is reaching the renderer.
        var ts = Replay.tierSettings
        var sets = ts ? ts.textureSets : []
        var tierTag = ["low", "medium", "high", "ultra"][Replay.qualityIndex] || "medium"
        console.log("replay tier " + tierTag + ": msaa="
                    + scene.environment.antialiasingMode
                    + " aaQuality=" + scene.environment.antialiasingQuality
                    + " shadow=" + keyLight.castsShadow)
        console.log("replay textures " + tierTag + ":", sets.length, "sets",
                    sets.length > 0 ? sets.join(",") : "(none)")
    }

    // ---- Venue (Phase 6b) ----
    // The twelve baked variants are instantiated statically inside the scene
    // (venueRoot); exactly the (sport, effective tier) match is visible. Each
    // variant is walked once — materials from the contract, bucketed
    // InstanceLists, tier-gated textures — on first show; the walk registries
    // live for the whole session (bounded: ~350 materials and ~70 instance
    // lists across all twelve variants, textures shared by source).
    property var venueMaterials: ({})
    property var venueTextureCache: ({})
    property var venueWalked: ({})
    property bool venueSchemeCached: false
    // The tier's normal-map flag for the walk in progress, read once per walk
    // instead of converting Replay.tierSettings for every texture slot.
    property bool venueNormalMaps: false

    function syncVenue() {
        if (Replay.loadState !== "ready") return
        var key = Replay.sportIndex + "|" + Replay.effectiveQuality
        if (venueWalked[key] === true) {
            if (venueSchemeCached !== Replay.schemeDark) {
                venueSchemeCached = Replay.schemeDark
                retintVenue()
            }
            return
        }
        var plan = Replay.venuePlan
        if (!plan || !plan.inventory) return
        venueWalked[key] = true
        venueSchemeCached = Replay.schemeDark
        var tiers = Replay.tierSettings
        venueNormalMaps = !!(tiers && tiers.normalMaps)
        // The twelve variants sit inside venueRoot in (sport asc, tier asc)
        // declaration order.
        var item = venueRoot.children[Replay.sportIndex * 4 + Replay.effectiveQuality]
        if (!item) {
            failVenue("no static component for " + key)
            return
        }
        applyVenuePlan(item, plan)
    }

    function failVenue(reason) {
        var sportTag = ["row", "ski", "bike"][Replay.sportIndex]
        console.warn("replay venue FAILED " + sportTag + ":", reason)
        Replay.reportError("venue load failed: " + reason)
    }

    // One-time tinted-hex helper: multiply the sRGB channels of "#rrggbb"
    // by the bucket's tint (a linear scatterTint multiplier). Runs at walk
    // time only — never per frame.
    function tintedHex(hex, tint) {
        var r = parseInt(hex.substr(1, 2), 16) / 255.0
        var g = parseInt(hex.substr(3, 2), 16) / 255.0
        var b = parseInt(hex.substr(5, 2), 16) / 255.0
        var channel = function (value) {
            var out = Math.min(1.0, Math.max(0.0, value))
            return Math.round(out * 255).toString(16).padStart(2, "0")
        }
        return "#" + channel(r * tint[0]) + channel(g * tint[1]) + channel(b * tint[2])
    }

    function venueBaseColor(spec, tint) {
        var hex = Replay.schemeDark ? spec.colorDark : spec.colorLight
        return tint ? tintedHex(hex, tint) : hex
    }

    // Build (or reuse) the material for one contract entry. Keys are scoped
    // per variant: tiers bind different texture sets under the same material
    // name, and a reused entry would carry one tier's textures into another.
    // `tint` is set only for instance-bucket materials; `original` is the
    // placeholder material balsam generated, which carries the authored
    // clearcoat values the contract does not duplicate.
    function venueMaterial(plan, variantKey, name, tint, original) {
        var key = variantKey + "|" + name + (tint ? "#" + tint.join(",") : "")
        if (venueMaterials[key] !== undefined)
            return { material: venueMaterials[key].material, bindings: 0 }
        var spec = plan.materials[name]
        if (spec === undefined) {
            failVenue("plan has no material " + name)
            return null
        }
        var properties = {
            objectName: key,
            baseColor: Qt.color(venueBaseColor(spec, tint)),
            metalness: spec.metalness,
            roughness: spec.roughness
        }
        if (spec.alphaMode === "blend") {
            properties.alphaMode = PrincipledMaterial.Blend
            properties.opacity = spec.opacity
        }
        if (spec.type === "basic") properties.lighting = PrincipledMaterial.NoLighting
        if (spec.type === "physical" && original !== undefined) {
            properties.clearcoatAmount = original.clearcoatAmount
            properties.clearcoatRoughnessAmount = original.clearcoatRoughnessAmount
        }
        if (spec.doubleSided) properties.cullMode = PrincipledMaterial.NoCulling
        if (spec.vertexColors) properties.vertexColorsEnabled = true
        var material = Qt.createQmlObject(
            "import QtQuick3D; PrincipledMaterial {}", venueRoot, "venueMaterial")
        for (var property in properties) material[property] = properties[property]
        var bindings = bindVenueTextures(material, spec)
        venueMaterials[key] = { material: material, spec: spec, tint: tint || null }
        return { material: material, bindings: bindings }
    }

    // Tier-gated texture binding (R3.1): instantiate exactly the bindings the
    // contract carries for this tier's variant — none at Low (the web builds
    // those variants with no slots at all), procedural-only at Medium, sets
    // at High, plus normals at Ultra. Texture objects are shared across
    // variants by source (the same Poly Haven derivative); the returned count
    // is the bindings this walk applied. UV repeat is already baked into the
    // GLB's UVs; textures must repeat for tiles beyond 0–1. The contract
    // keeps the web's slot names; Qt's diffuse slot is `baseColorMap`.
    function bindVenueTextures(material, spec) {
        var textures = spec.textures
        if (!textures) return 0
        var bindings = 0
        for (var slot in textures) {
            var binding = textures[slot]
            if (slot === "normalMap" && !venueNormalMaps) continue
            var texture = venueTextureCache[binding.source]
            if (texture === undefined) {
                texture = Qt.createQmlObject(
                    "import QtQuick3D; Texture { tilingModeHorizontal: Texture.Repeat; tilingModeVertical: Texture.Repeat }",
                    venueRoot, "venueTexture")
                texture.source = binding.source
                venueTextureCache[binding.source] = texture
            }
            var qmlSlot = slot === "map" ? "baseColorMap" : slot
            material[qmlSlot] = texture
            // Qt 6.11 renamed PrincipledMaterial.normalScale (vector2d) to
            // normalStrength (float); the web's authored scales are uniform.
            if (slot === "normalMap" && spec.normalScale !== undefined)
                material.normalStrength = spec.normalScale[0]
            bindings += 1
        }
        return bindings
    }

    // Turn one archetype Model into its bucketed InstanceLists. The bucket
    // tints ride on cloned materials (no custom shaders).
    function applyInstanceGroup(venuePlan, variantKey, node, plan) {
        var buckets = plan.buckets
        for (var b = 0; b < buckets.length; ++b) {
            var bucket = buckets[b]
            var qml = "import QtQuick3D; InstanceList { instances: ["
            for (var i = 0; i < bucket.transforms.length; ++i) {
                var t = bucket.transforms[i]
                qml += "Instance { position: Qt.vector3d(" + t[0] + "," + t[1] + "," + t[2] + ")"
                qml += "; rotation: Qt.quaternion(" + t[6] + "," + t[3] + "," + t[4] + "," + t[5] + ")"
                qml += "; scale: Qt.vector3d(" + t[7] + "," + t[8] + "," + t[9] + ") },"
            }
            qml += "] }"
            var list = Qt.createQmlObject(qml, venueRoot, "venueInstances")
            var built = venueMaterial(venuePlan, variantKey, node.materials[0].objectName,
                                      bucket.tint, node.materials[0])
            if (built === null) return 0
            // A Model carries exactly one instancing, so each bucket draws
            // through its own Model over the shared archetype mesh. The
            // archetype sits at identity (the baker premultiplied the group
            // transform into the instance records), so the clones need no
            // transform of their own.
            var target = node
            if (b > 0) {
                target = Qt.createQmlObject("import QtQuick3D; Model {}",
                                            node.parent, "venueBucketModel")
                target.source = node.source
                target.castsShadows = node.castsShadows
                target.receivesShadows = node.receivesShadows
            }
            target.instancing = list
            target.materials = [built.material]
        }
        return buckets.length
    }

    // Walk one static venue variant: re-material every Model from the plan,
    // and convert instance-group archetypes into bucketed instanced draws.
    function walkVenue(variantKey, node, plan) {
        var instanceGroups = plan.instanceGroups
        var children = node.children
        var applied = 0
        for (var i = 0; children && i < children.length; ++i) {
            var child = children[i]
            if (child.instancing !== undefined && child.source !== undefined) {
                // The web's shadow flags, before an instance group copies
                // them onto its bucket Models (#96).
                var shadowFlags = Replay.venueShadowFlags(child.objectName)
                child.castsShadows = (shadowFlags & 1) !== 0
                child.receivesShadows = (shadowFlags & 2) !== 0
                var groupPlan = instanceGroups[child.objectName]
                if (groupPlan !== undefined) {
                    applied += applyInstanceGroup(plan, variantKey, child, groupPlan)
                } else if (child.materials.length > 0) {
                    var built = venueMaterial(plan, variantKey, child.materials[0].objectName,
                                              null, child.materials[0])
                    if (built === null) return applied
                    child.materials = [built.material]
                    applied += built.bindings
                }
            }
            applied += walkVenue(variantKey, child, plan)
        }
        return applied
    }

    function applyVenuePlan(item, plan) {
        var key = Replay.sportIndex + "|" + Replay.effectiveQuality
        var applied = walkVenue(key, item, plan)
        var inv = plan.inventory
        var groups = plan.instanceGroups.length
        var instances = 0
        for (var g = 0; g < plan.instanceGroups.length; ++g) {
            var buckets = plan.instanceGroups[g].buckets
            for (var b = 0; b < buckets.length; ++b)
                instances += buckets[b].transforms.length
        }
        var sportTag = ["row", "ski", "bike"][Replay.sportIndex]
        console.log("replay venue " + sportTag + ":", inv.nodes, "nodes,",
                    groups, "instanced groups,", instances, "instances,",
                    inv.materials, "materials")
        var tierTag = ["low", "medium", "high", "ultra"][Replay.effectiveQuality] || "medium"
        console.log("replay venue " + sportTag + " textures " + tierTag + ":",
                    applied)
    }

    // Scheme change: re-tint the retained materials in place — no re-walk.
    function retintVenue() {
        for (var key in venueMaterials) {
            var entry = venueMaterials[key]
            entry.material.baseColor = Qt.color(venueBaseColor(entry.spec, entry.tint))
        }
    }

    // Walk one balsam Rigs component: assign materials, show/hide by sport,
    // place templates at anchors, cache nodes for per-frame updates.
    // `isMirror` restricts the mirror copy to multi-instance templates only.
    function walkRigs(node, anchorMap, isMirror, materialMap) {
        if (!node) return
        var matMap = materialMap || byRole
        var side = isMirror ? "left" : "right"
        var meta = replayRoot.meshRoles[node.objectName]
        if (meta !== undefined) {
            if (node.materials !== undefined && matMap[meta.role] !== undefined) {
                node.materials = [matMap[meta.role]]
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
        for (var i = 0; ch && i < ch.length; ++i) walkRigs(ch[i], anchorMap, isMirror, materialMap)
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

        // Ghost frame (same layout, separate data).
        if (Replay.hasGhost) applyGhostFrame()
    }

    function applyGhostFrame() {
        var gf = Replay.ghostFrame
        if (!gf || gf.length < fl.length) return

        // Ghost course placement.
        ghostCourseNode.position = Qt.vector3d(gf[fl.courseX], 0, gf[fl.courseZ])
        ghostCourseNode.rotation = quatFromFrame(gf, fl.courseYaw)
        ghostRigGroup.position = Qt.vector3d(0, gf[fl.accentBob], gf[fl.accentSurge])
        ghostRigGroup.rotation = quatFromFrame(gf, fl.accentRoll)

        // Ghost joints.
        for (var j = 0; j < fl.jointCount; ++j) {
            var jn = ghostJointNodes[j]
            if (!jn) continue
            var at = fl.joints + j * fl.jointStride
            jn.position = Qt.vector3d(gf[at], gf[at + 1], gf[at + 2])
            jn.rotation = Qt.quaternion(gf[at + 6], gf[at + 3], gf[at + 4], gf[at + 5])
        }

        // Ghost equipment.
        if (ghostSeatNode) ghostSeatNode.z = gf[fl.seatZ]
        if (ghostOarRigNode) ghostOarRigNode.rotation = quatFromFrame(gf, fl.oarRight)
        if (ghostOarRigMirror) ghostOarRigMirror.rotation = quatFromFrame(gf, fl.oarLeft)
        var wq = quatFromFrame(gf, fl.wheel)
        if (ghostWheelNode) ghostWheelNode.rotation = wq
        if (ghostWheelMirror) ghostWheelMirror.rotation = wq
        if (ghostDrivetrainNode) ghostDrivetrainNode.rotation = quatFromFrame(gf, fl.crank)

        // Ghost blade positions.
        placeLeaf7(ghostBladeNodes["right"], gf, fl.bladeRight)
        placeLeaf7(ghostBladeNodes["left"], gf, fl.bladeLeft)

        // Ghost pole positions.
        applyPoleLeaves(gf, fl.poleLeft, fl.poleLeavesLeft, "left",
                        ghostPoleShaftNodes, ghostGripNodes, ghostBasketNodes)
        applyPoleLeaves(gf, fl.poleRight, fl.poleLeavesRight, "right",
                        ghostPoleShaftNodes, ghostGripNodes, ghostBasketNodes)
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
    function applyPoleLeaves(f, poleAt, leavesAt, side, shaftMap, gripMap, basketMap) {
        var rot = quatFromFrame(f, poleAt + 3)
        var shaft = (shaftMap || poleShaftNodes)[side]
        if (shaft) { shaft.position = vec3FromFrame(f, leavesAt); shaft.rotation = rot }
        var grip = (gripMap || poleGripNodes)[side]
        if (grip) { grip.position = vec3FromFrame(f, leavesAt + 3); grip.rotation = rot }
        var basket = (basketMap || poleBasketNodes)[side]
        if (basket) { basket.position = vec3FromFrame(f, leavesAt + 6); basket.rotation = rot }
    }

    // ---- Bench mode (ROWPLAY_REPLAY_BENCH=1) ----
    // Collects View3D.renderStats.frameTime (the Qt Quick 3D render pass
    // cost: sync + prepare + render, independent of compositor presentation).
    // Must run with QSG_NO_VSYNC=1 — without it, renderStats.frameTime
    // includes the vsync wait and reports the display refresh interval.
    property bool benchCollecting: false
    property var benchTimes: []
    property int benchTarget: 600
    property int benchWarmup: 120
    property int benchExcluded: 0
    property var benchWallTimes: []
    property int benchWallOver50: 0
    property string benchLabel: ""

    function benchStart(label) {
        replayRoot.benchLabel = label
        replayRoot.benchTimes = []
        replayRoot.benchWarmup = 120
        replayRoot.benchExcluded = 0
        replayRoot.benchWallTimes = []
        replayRoot.benchWallOver50 = 0
        replayRoot.benchCollecting = true
    }

    function benchSample(renderTimeMs) {
        if (replayRoot.benchWarmup > 0) { replayRoot.benchWarmup -= 1; return }
        // Cap at 100 ms: GC pauses and buffer uploads produce 200+ ms
        // outliers that aren't sustained render cost.
        if (renderTimeMs > 100) { replayRoot.benchExcluded += 1; return }
        benchTimes.push(renderTimeMs)
        if (benchTimes.length >= benchTarget) {
            benchCollecting = false
            benchReport()
        }
    }

    function benchWallSample(wallMs) {
        replayRoot.benchWallTimes.push(wallMs)
        if (wallMs > 50) replayRoot.benchWallOver50 += 1
    }

    function benchReport() {
        var sorted = benchTimes.slice().sort(function(a, b) { return a - b })
        var n = sorted.length
        if (n === 0) return
        var median = sorted[Math.floor(n / 2)]
        var p95 = sorted[Math.floor(n * 0.95)]
        var wn = replayRoot.benchWallTimes.length
        var wsorted = wn > 0 ? replayRoot.benchWallTimes.slice().sort(function(a,b){return a-b}) : [0]
        var wmedian = wsorted[Math.floor(wsorted.length / 2)]
        var wp95 = wsorted[Math.floor(wsorted.length * 0.95)]
        console.log("replay bench " + benchLabel + ":"
                    + " n=" + n
                    + " excluded=" + replayRoot.benchExcluded
                    + " median=" + median.toFixed(2) + "ms"
                    + " p95=" + p95.toFixed(2) + "ms"
                    + " | wall median=" + wmedian.toFixed(2) + "ms"
                    + " p95=" + wp95.toFixed(2) + "ms"
                    + " >50ms=" + replayRoot.benchWallOver50)
    }
}
