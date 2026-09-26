#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Capture deterministic contact states and the native Qt transform palette.

Instrument the existing gate temporarily, like capture.py. No asset is edited.
Run alone (no concurrent builds) with .envrc and a real Qt renderer. Restores
both QML sources even on failure. Samples use the existing 2000-step debug
clock: fallback 30 spm, distance = step * 3 m; metadata records this explicitly.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

from capture import ROOT, replace_once

SAMPLES = {
    1001: [('row-catch', 0), ('row-middrive', 380), ('row-finish', 760),
           ('row-halfslide', 1380)],
    1003: [('ski-approach', 1900), ('ski-plant', 0), ('ski-pull', 300),
           ('ski-release', 540), ('ski-recovery', 1100)],
    1004: [('bike-top', 0), ('bike-quarter', 500)],
}

DIAGNOSTIC = r'''
    // Temporary Phase 5 instrument; never shipped in the app.
    property var contactSavedMaterials: []
    function contactMask(on) {
        if (on) {
            function visit(n) {
                if (n.materials !== undefined) {
                    contactSavedMaterials.push({node:n, materials:Array.prototype.slice.call(n.materials)})
                    n.materials = [contactMaskMaterial]
                }
                var children = n.children
                for (var i=0; children && i<children.length; ++i) visit(children[i])
            }
            visit(athlete)
        } else {
            for (var i=0; i<contactSavedMaterials.length; ++i)
                contactSavedMaterials[i].node.materials = contactSavedMaterials[i].materials
            contactSavedMaterials = []
        }
        rigs.visible = !on; rigsMirror.visible = !on
        rowingRig.visible = !on; rowingRigMirror.visible = !on
    }
    function contactView(side) {
        var hand = jointNodes[side === "left" ? 8 : 12]
        var target = hand.mapPositionToScene(Qt.vector3d(0, 0, 0))
        var offset = rigGroup.mapDirectionToScene(Qt.vector3d(side === "left" ? -0.23 : 0.23, 0.12, 0.23))
        camera.position = target.plus(offset)
        camera.lookAt(target)
        camera.fieldOfView = 45
    }
    function contactDump(name) {
        function vec(v) { return [v.x, v.y, v.z] }
        function transform(n) {
            return [[0,0,0], [1,0,0], [0,1,0], [0,0,1]].map(function(p) {
                return vec(n.mapPositionToScene(Qt.vector3d(p[0],p[1],p[2])))
            })
        }
        var map = {}, nodes = {}, equipment = {}
        collectNodesByName(athlete, map)
        for (var key in map) nodes[key] = transform(map[key])
        function collect(n, out) {
            if (!n) return
            if (n.objectName && n.mapPositionToScene !== undefined && n.visible)
                out[n.objectName] = transform(n)
            var children = n.children
            for (var i = 0; children && i < children.length; ++i) collect(children[i],out)
        }
        collect(Replay.sportIndex === 0 ? rowingRig : rigs, equipment)
        var mirrored = {}
        collect(Replay.sportIndex === 0 ? rowingRigMirror : rigsMirror, mirrored)
        var record = {name:name, frame:Array.prototype.slice.call(Replay.poseFrame),
            layout:fl, grip:JSON.parse(Replay.gripPoses), contacts:Replay.gripContacts,
            nodes:nodes, equipment:equipment, mirrored:mirrored,
            rig:transform(rigGroup), camera:transform(camera),
            viewport:[scene.width,scene.height],
            viewportOrigin:[scene.mapToItem(null,0,0).x,scene.mapToItem(null,0,0).y], fov:camera.fieldOfView,
            poleGrips: {left:poleGripNodes["left"] ? transform(poleGripNodes["left"]) : null,
                        right:poleGripNodes["right"] ? transform(poleGripNodes["right"]) : null},
            tier:Replay.effectiveQuality, sport:Replay.sportIndex}
        console.log("CONTACT_FRAME " + JSON.stringify(record))
        // The root grab is held by the gate. Do not start a second async
        // scene grab here: the next camera step can overtake its callback.
    }
'''


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--scheme', choices=['light','dark'], default='light')
    p.add_argument("--masks", action="store_true", help="also capture unlit athlete-only segmentation for skin validation")
    args = p.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    paths = [ROOT/'qml/RowPlay/Main.qml', ROOT/'qml/RowPlay/Replay/ReplayScene.qml']
    originals = [path.read_text() for path in paths]
    text = replace_once(originals[0],
        '            case 53: Replay.loadWorkout(1001); break     // rower demo workout',
        '            case 53: Replay.pause(); Replay.loadGhost(-1); root.gateStep = 299; break')
    cases = []
    step = 300
    for workout, samples in SAMPLES.items():
        cases.append(f'case {step}: Replay.loadWorkout({workout}); Replay.loadGhost(-1); Replay.setQualityIndex(1); break')
        step += 1
        for name, cycle in samples:
            cases.append(f'case {step}: Replay.setGuardCycleStep({cycle}); root.grabSettledScene("{name}-chase"); break')
            step += 1
            for side in ['left','right']:
                cases.append(f'case {step}: detailColumn.children[3].contactMask(false); detailColumn.children[3].contactView("{side}"); root.grabSettledScene("{name}-{side}"); break')
                step += 1
                if args.masks:
                    cases.append(f'case {step}: detailColumn.children[3].contactMask(true); root.grabSettledScene("{name}-{side}-mask"); break')
                    step += 1
            cases.append(f'case {step}: detailColumn.children[3].contactMask(false); break')
            step += 1
    cases.append(f'case {step}: Qt.quit(); break')
    text = replace_once(text, '            switch (root.gateStep) {',
                        '            switch (root.gateStep) {\n'+'\n'.join(cases))
    anchor = '            result.saveToFile(Settings.screenshotDir + "/" + name + ".ppm")'
    text = replace_once(text, anchor, anchor+'\n            if (root.gateStep >= 300) detailColumn.children[3].contactDump(name)')
    scene = replace_once(originals[1], '    // ---- applyFrame (the per-tick frame-bundle reader) ----',
                         DIAGNOSTIC+'\n    // ---- applyFrame (the per-tick frame-bundle reader) ----')
    scene = replace_once(scene, '        PerspectiveCamera { id: camera; clipNear: 0.1; clipFar: 1000 }',
        '        DefaultMaterial { id: contactMaskMaterial; lighting: DefaultMaterial.NoLighting; diffuseColor: "#ff00ff"; vertexColorsEnabled: false }\n        PerspectiveCamera { id: camera; clipNear: 0.1; clipFar: 1000 }')
    asset_hashes={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [ROOT/'assets/replay/rowplay-athlete-v4.glb',ROOT/'assets/replay/rowplay-rigs-v3.glb',ROOT/'assets/replay/authored/rowing-shell.glb']}
    patch=subprocess.check_output(['git','diff','--binary','HEAD'],cwd=ROOT)
    (output/'source.patch').write_bytes(patch)
    metadata = dict(asset_sha256=asset_hashes, source_patch_sha256=hashlib.sha256(patch).hexdigest(),
                    instrument_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                    qt=subprocess.check_output(['qmake','-query','QT_VERSION'],text=True).strip(),
                    rustc=subprocess.check_output(['rustc','--version'],text=True).strip(),
                    base=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),
                    samples=SAMPLES, clock='fallback_stroke_pose at 30 spm; phase = step/2000 * tau; metres = step*3',
                    scheme=args.scheme, renderer=os.environ.get('QSG_RHI_BACKEND'),
                    platform=os.environ.get('QT_QPA_PLATFORM'))
    (output/'manifest.json').write_text(json.dumps(metadata, indent=2)+'\n')
    try:
        for path, content in zip(paths, [text,scene]): path.write_text(content)
        subprocess.run(['cargo','build','-p','rowplay-app'],cwd=ROOT,check=True)
        env = dict(os.environ, ROWPLAY_SMOKE_GATE='1', ROWPLAY_SYNC_MOCK='1',
                   ROWPLAY_GATE_PROFILE='full', ROWPLAY_FORCE_COLOR_SCHEME=args.scheme,
                   ROWPLAY_SMOKE_SCREENSHOT_DIR=str(output), ROWPLAY_DATA_DIR=str(output/'data'),
                   QT_MESSAGE_PATTERN='[%{time process}] %{message}')
        with (output/'gate.log').open('w') as log:
            subprocess.run([str(ROOT/'target/debug/rowplay-app')],cwd=ROOT,env=env,
                           stdout=log,stderr=subprocess.STDOUT,check=True,timeout=900)
        log = (output/'gate.log').read_text()
        frames = [json.loads(line.split('CONTACT_FRAME ',1)[1]) for line in log.splitlines() if 'CONTACT_FRAME ' in line]
        (output/'frames.json').write_text(json.dumps(frames,indent=2)+'\n')
        if len(frames) != sum(len(x) for x in SAMPLES.values())*(5 if args.masks else 3):
            raise RuntimeError('incomplete contact capture; inspect gate.log')
        if any(x in log for x in ['screenshot FAILED','ReferenceError:','TypeError:','contact view false']):
            raise RuntimeError('invalid contact capture; inspect gate.log')
    finally:
        for path, original in zip(paths, originals): path.write_text(original)


if __name__ == '__main__':
    main()
