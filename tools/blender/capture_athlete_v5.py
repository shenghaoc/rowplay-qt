#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Capture the modelled athlete with the Phase 5 native palette instrument.

The historical V4 instruments and manifests remain unchanged. This adapter
uses their settled-frame capture, transform recorder and material-state
inventory, with a V5-specific current-Cargo artifact selector and eight-role
validation. Run alone: it temporarily instruments two QML files and restores
both in finally. Capture tooling and inputs must first be committed.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

from capture import ROOT, replace_once
from capture_contact import DIAGNOSTIC, SAMPLES
from capture_athlete import AUDIT
from athlete_evidence import git, sha, verify_instrument, MAP_FIELDS
from export_athlete import ROLES

INSTRUMENTS = ['tools/blender/'+name+'.py' for name in (
    'capture_athlete_v5', 'capture', 'capture_contact', 'capture_athlete',
    'athlete_evidence', 'contact_skin', 'export_athlete', 'canonical',
    'athlete_v5_skin', 'audit_athlete')]
INPUTS = [
    'assets/replay/authored/rowplay-athlete-v5.blend',
    'assets/replay/authored/rowplay-athlete-v5.glb',
    'assets/replay/authored/rowplay-athlete-v5.contract.json',
    'assets/replay/authored/sources/rowplay-human-base-male-v1.4.1.blend',
    'assets/replay/authored/rowing-shell.glb', 'assets/replay/rowplay-rigs-v3.glb',
    'qml/RowPlay/Theme.qml', 'qml/RowPlay/Replay/ReplayScene.qml', 'qml/RowPlay/Main.qml',
]


def instrument_record():
    if git(ROOT, 'status', '--porcelain').strip():
        raise ValueError('commit instruments/inputs and keep the worktree clean before capture')
    commit = git(ROOT, 'rev-parse', 'HEAD').decode().strip()
    result = {'commit': commit, 'instruments': {}}
    for path in INSTRUMENTS + INPUTS:
        if (ROOT/path).read_bytes() != git(ROOT, 'show', f'{commit}:{path}'):
            raise ValueError('uncommitted capture input: '+path)
        if path in INSTRUMENTS:
            result['instruments'][path] = sha((ROOT/path).read_bytes())
    verify_instrument(result, paths=INSTRUMENTS)
    return result


def cargo_artifacts(messages, package_id, target, root=ROOT):
    """Resolve this invocation only; never glob a stale target directory."""
    executables = [m['executable'] for m in messages if m.get('reason') == 'compiler-artifact'
                   and m.get('package_id') == package_id and m.get('target') == target and m.get('executable')]
    outputs = [m['out_dir'] for m in messages if m.get('reason') == 'build-script-executed'
               and m.get('package_id') == package_id]
    if len(executables) != 1 or len(outputs) != 1:
        raise ValueError('expected exactly one current executable and OUT_DIR')
    executable = (root/executables[0]).resolve()
    out = (root/outputs[0]).resolve()
    qml = out/'replay-balsam/athlete/Rowplay_athlete_v5.qml'
    meshes = sorted((out/'replay-balsam/athlete/meshes').glob('*.mesh'))
    if len(meshes) != 1 or meshes[0].name != 'rowplayAthleteV5_mesh.mesh':
        raise ValueError('unexpected current athlete mesh inventory')
    files = {'executable': executable, 'generated_qml': qml, 'athlete_mesh': meshes[0],
             'runtime_athlete': out/'replay_athlete_v4.json'}
    if any(not p.is_file() for p in files.values()):
        raise ValueError('missing current Cargo artifact')
    text = qml.read_text()
    if 'objectName: "rowplayAthleteV5"' not in text or text.count('required property color') != 8:
        raise ValueError('generated component is not the static V5 material contract')
    return {'package_id': package_id, 'out_dir': str(out), **{k: str(p) for k,p in files.items()},
            'sha256': {k: sha(p.read_bytes()) for k,p in files.items()}}


def build_capture(out):
    metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=ROOT))
    packages = [p for p in metadata['packages'] if Path(p['manifest_path']).resolve() == ROOT/'crates/rowplay-app/Cargo.toml']
    if len(packages) != 1:
        raise ValueError('ambiguous application package')
    package = packages[0]
    targets = [t for t in package['targets'] if t['kind'] == ['bin'] and t['name'] == 'rowplay-app']
    if len(targets) != 1:
        raise ValueError('ambiguous application target')
    command = ['cargo', 'build', '--locked', '-p', package['id'], '--message-format=json-render-diagnostics']
    log = out/'cargo-build.jsonl'
    with log.open('w') as output:
        subprocess.run(command, cwd=ROOT, stdout=output, check=True)
    messages = [json.loads(line) for line in log.read_text().splitlines()]
    if not any(m.get('reason') == 'build-finished' and m.get('success') is True for m in messages):
        raise ValueError('Cargo did not confirm successful completion')
    result = cargo_artifacts(messages, package['id'], targets[0])
    result.update(command=command, messages_sha256=sha(log.read_bytes()),
                  target_dir_env=os.environ.get('CARGO_TARGET_DIR'))
    return result


# Reuse the audited material inventory exactly, without the V4 map experiment.
MATERIAL_RECORDER = AUDIT.split('    function auditMaterials(variant)')[0] + r'''
    property var auditRoughness: null
    property var auditNormal: null
    property var auditMapHashes: ({})
    function v5View(view) {
        hud.visible=false
        if (view === "left" || view === "right") { contactView(view); return }
        var index=view === "face" ? 4 : view === "lower" ? 0 : 2
        var target=jointNodes[index].mapPositionToScene(Qt.vector3d(0,0,0))
        var offset=view === "face" ? Qt.vector3d(.12,.10,.56)
                  : view === "lower" ? Qt.vector3d(.85,.35,1.10) : Qt.vector3d(.90,.30,1.10)
        camera.position=target.plus(rigGroup.mapDirectionToScene(offset))
        camera.lookAt(target); camera.fieldOfView=45
    }
    function v5Dump(name) {
        console.log("ATHLETE_V5 " + JSON.stringify({name:name,
            materials:name.endsWith("-mask") ? [] : auditSnapshot(),
            schemeDark:Replay.schemeDark, tier:Replay.effectiveQuality,
            keyBrightness:keyLight.brightness, probeExposure:scene.environment.probeExposure,
            probe:skyProbe.source.toString(),
            enums:{opaque:PrincipledMaterial.Opaque, back:Material.BackFaceCulling}}))
    }
'''


def capture_cases(scheme, tier, selection):
    cases, expected, step = [], [], 300
    def add(action):
        nonlocal step
        cases.append(f'case {step}: {action}; break')
        step += 1
    scene = 'detailColumn.children[3]'
    for workout, samples in SAMPLES.items():
        add(f'Replay.loadWorkout({workout}); Replay.loadGhost(-1); Replay.setQualityIndex({tier}); Replay.setSchemeDark({str(scheme == "dark").lower()})')
        for name, cycle in samples:
            if selection == 'stressed' and name not in ('row-catch', 'ski-release', 'bike-quarter'):
                continue
            add(f'Replay.setGuardCycleStep({cycle}); root.grabSettledScene("{name}-chase")')
            expected.append(name+'-chase')
            for view in ['upper','lower','face','left','right']:
                capture=name+'-'+view
                add(f'{scene}.contactMask(false); {scene}.v5View("{view}"); root.grabSettledScene("{capture}")')
                expected.append(capture)
                if view in ('left','right'):
                    add(f'{scene}.contactMask(true); root.grabSettledScene("{capture}-mask")')
                    expected.append(capture+'-mask')
                    add(f'{scene}.contactMask(false)')
    add('Qt.quit()')
    return cases, expected


def validate_materials(records, contract, scheme, tier):
    expected = contract['materialParameters']
    checked = []
    for record in records:
        if record['schemeDark'] != (scheme == 'dark') or record['tier'] != tier:
            raise ValueError('scheme/tier drift: '+record['name'])
        if record['name'].endswith('-mask'):
            continue
        entries = record['materials']
        if len(entries) != 1 or entries[0]['objectName'] != 'rowplayAthleteV5' or entries[0]['materialCount'] != 8:
            raise ValueError('unexpected visible athlete material inventory')
        materials = entries[0]['materials']
        if [m['objectName'] for m in materials] != list(ROLES):
            raise ValueError('material primitive order/roles drift')
        if len({m['identity'] for m in materials}) != 8:
            raise ValueError('material roles share an object')
        for m in materials:
            state, spec = m['state'], expected[m['objectName']]
            for key in ['metalness','roughness']:
                if abs(state[key]-spec[key]) > 1e-6:
                    raise ValueError(m['objectName']+' '+key+' differs from saved source')
            if not state['vertexColorsEnabled'] or state['alphaMode'] != record['enums']['opaque'] or state['opacity'] != 1:
                raise ValueError('albedo/opacity drift')
            if state['cullMode'] != record['enums']['back']:
                raise ValueError('closed surface culling drift')
            if any(state[key] is not None for key in MAP_FIELDS):
                raise ValueError('unapproved material texture dependency')
            for key in ['clearcoatAmount','heightAmount','transmissionFactor','thicknessFactor']:
                if state[key] != 0:
                    raise ValueError('unexpected material feature: '+key)
            if state['emissiveFactor'] != [0,0,0]:
                raise ValueError('unexpected material emission')
        checked.append(record['name'])
    return {'captures': checked, 'roles': list(ROLES), 'productionMaterialStateVerified': True}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--scheme', choices=['light','dark'], default='light')
    parser.add_argument('--tier', choices=['medium','high'], default='medium')
    parser.add_argument('--selection', choices=['all','stressed'], default='all')
    args = parser.parse_args()
    tooling = instrument_record()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    tier = 1 if args.tier == 'medium' else 2
    cases, expected = capture_cases(args.scheme, tier, args.selection)
    paths = [ROOT/'qml/RowPlay/Main.qml', ROOT/'qml/RowPlay/Replay/ReplayScene.qml']
    originals = [p.read_text() for p in paths]
    main_qml = replace_once(originals[0], '            case 53: Replay.loadWorkout(1001); break     // rower demo workout',
                            '            case 53: Replay.pause(); Replay.loadGhost(-1); root.gateStep = 299; break')
    main_qml = replace_once(main_qml, '            switch (root.gateStep) {',
                            '            switch (root.gateStep) {\n'+'\n'.join(cases))
    anchor = '            result.saveToFile(Settings.screenshotDir + "/" + name + ".ppm")'
    main_qml = replace_once(main_qml, anchor, anchor+'\n            if (root.gateStep >= 300) { detailColumn.children[3].contactDump(name); detailColumn.children[3].v5Dump(name) }')
    scene = replace_once(originals[1], '    // ---- applyFrame (the per-tick frame-bundle reader) ----',
                         DIAGNOSTIC+MATERIAL_RECORDER+'\n    // ---- applyFrame (the per-tick frame-bundle reader) ----')
    anchor = '        PerspectiveCamera { id: camera; clipNear: 0.1; clipFar: 1000 }'
    scene = replace_once(scene, anchor, '        DefaultMaterial { id: contactMaskMaterial; lighting: DefaultMaterial.NoLighting; diffuseColor: "#ff00ff"; vertexColorsEnabled: false }\n'+anchor)
    contract = json.loads((ROOT/INPUTS[2]).read_text())
    manifest = {'schema': 'rowplay.athlete-v5.native.v1', 'tooling': tooling,
                'inputs': {p: sha((ROOT/p).read_bytes()) for p in INPUTS},
                'instrumented_qml': {str(p.relative_to(ROOT)): sha(s.encode()) for p,s in zip(paths,[main_qml,scene])},
                'qt': subprocess.check_output(['qmake','-query','QT_VERSION'],text=True).strip(),
                'scheme': args.scheme, 'tier': args.tier, 'samples': SAMPLES, 'expected': expected,
                'renderer': os.environ.get('QSG_RHI_BACKEND'), 'platform': os.environ.get('QT_QPA_PLATFORM'),
                'clock': 'fallback 30 spm; phase=step/2000*tau; distance=step*3 m',
                'acceptance': 'measurement capture; no automatic visual/contact acceptance claim'}
    try:
        for path, value in zip(paths, [main_qml, scene]): path.write_text(value)
        artifacts = build_capture(out)
        manifest['cargo'] = artifacts
        manifest['cargo']['launched_executable'] = artifacts['executable']
        (out/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        env = dict(os.environ, ROWPLAY_SMOKE_GATE='1', ROWPLAY_SYNC_MOCK='1', ROWPLAY_GATE_PROFILE='full',
                   ROWPLAY_FORCE_COLOR_SCHEME=args.scheme, ROWPLAY_SMOKE_SCREENSHOT_DIR=str(out),
                   ROWPLAY_DATA_DIR=str(out/'data'), QT_MESSAGE_PATTERN='[%{time process}] %{message}')
        with (out/'gate.log').open('w') as log:
            subprocess.run([artifacts['executable']],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=1200)
        log = (out/'gate.log').read_text()
        captured = {}
        for prefix, filename in [('CONTACT_FRAME ', 'frames.json'),('ATHLETE_V5 ', 'materials.json')]:
            records = [json.loads(line.split(prefix,1)[1]) for line in log.splitlines() if prefix in line]
            if [r['name'] for r in records] != expected:
                raise ValueError('missing/duplicate/out-of-order capture: '+prefix)
            captured[filename] = records
            (out/filename).write_text(json.dumps(records,indent=2)+'\n')
        validation = validate_materials(captured['materials.json'],contract,args.scheme,tier)
        (out/'material-validation.json').write_text(json.dumps(validation,indent=2)+'\n')
        forbidden = ['failed to load component','Unexpected token','screenshot FAILED','ReferenceError:','TypeError:',
                     'Failed to compile','Failed to generate shader','Shader compilation failed']
        if any(text in log for text in forbidden):
            raise ValueError('native runtime/capture error; inspect gate.log')
    finally:
        for path, text in zip(paths, originals): path.write_text(text)
    verify_instrument(tooling, paths=INSTRUMENTS)


if __name__ == '__main__':
    main()
