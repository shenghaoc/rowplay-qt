# SPDX-License-Identifier: GPL-3.0-or-later
"""Fail-closed contracts for the Phase 5.2 instrument, inputs and interventions.

This is deliberately specific to the imported V4 PrincipledMaterial. We audit
its active colour/PBR/coat/normal/alpha/raster state, every possible texture
input, and gates for inactive emission/height/transmission/Fresnel/colour-mask
features. Parameters behind those disabled gates and point/line settings are
not part of this triangle-mesh experiment.
"""
from copy import deepcopy
import hashlib
import json
from pathlib import Path
import re
import struct
import subprocess

from capture import ROOT

PALETTE_PATH = 'src/lib/replay/renderer3dV4Assets.ts'
INSTRUMENTS = [f'tools/blender/{name}.py' for name in (
    'capture_athlete', 'capture_athlete_lower', 'athlete_evidence',
    'publish_athlete_audit', 'capture', 'capture_contact', 'contact_skin')]
SCALAR_FIELDS = '''vertexColorsEnabled roughness roughnessChannel metalness metalnessChannel
specularAmount specularTint specularSingleChannelEnabled specularChannel
baseColorSingleChannelEnabled baseColorChannel clearcoatAmount clearcoatChannel
clearcoatRoughnessAmount clearcoatRoughnessChannel clearcoatNormalStrength
normalStrength opacity opacityChannel invertOpacityMapValue alphaMode alphaCutoff
lighting blendMode cullMode depthDrawMode occlusionAmount occlusionChannel
emissiveSingleChannelEnabled emissiveChannel heightAmount transmissionFactor thicknessFactor
vertexColorsMaskEnabled fresnelScaleBiasEnabled clearcoatFresnelScaleBiasEnabled'''.split()
MAP_FIELDS = '''baseColorMap roughnessMap metalnessMap specularMap normalMap
clearcoatMap clearcoatRoughnessMap clearcoatNormalMap opacityMap emissiveMap
occlusionMap specularReflectionMap lightProbe heightMap transmissionMap thicknessMap'''.split()
COLOR_FIELDS = ['baseColor']
VECTOR_FIELDS = ['emissiveFactor']
TEXTURE_FIELDS = '''scaleU scaleV positionU positionV pivotU pivotV rotationUV
mappingMode tilingModeHorizontal tilingModeVertical flipU flipV minFilter magFilter
mipFilter generateMipmaps'''.split()


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args], stderr=subprocess.PIPE)


def documented_pin(root=ROOT):
    match = re.search(r'\| `https://github.com/shenghaoc/rowplay` \| `main` \| `([0-9a-f]{40})`',
                      (root/'docs/source-map.md').read_text())
    if not match:
        raise ValueError('documented web reference pin missing')
    return match[1]


def verified_palette(repo=None, pin=None, root=ROOT):
    repo = root/'reference/rowplay' if repo is None else Path(repo)
    pin = documented_pin(root) if pin is None else pin
    head = git(repo, 'rev-parse', 'HEAD').decode().strip()
    if head != pin:
        raise ValueError(f'palette reference HEAD {head} differs from pin {pin}')
    committed = git(repo, 'show', f'{pin}:{PALETTE_PATH}')
    if (repo/PALETTE_PATH).read_bytes() != committed:
        raise ValueError('palette source has local modifications; restore the pinned source')
    # Parse these proven bytes, never a later independent read of the file.
    return committed, {'repository': str(repo.relative_to(root)), 'commit': pin,
                       'actual_head': head, 'path': PALETTE_PATH, 'sha256': sha(committed)}


def verify_palette_record(record, root=ROOT):
    _, actual = verified_palette(root=root)
    if record != actual:
        raise ValueError('canonical palette provenance/hash differs; regenerate')
    return actual


def instrument_record(root=ROOT):
    if git(root, 'status', '--porcelain').strip():
        raise ValueError('commit tooling and ensure a clean worktree before capture')
    commit = git(root, 'rev-parse', 'HEAD').decode().strip()
    hashes = {}
    for name in INSTRUMENTS:
        data = (root/name).read_bytes()
        if data != git(root, 'show', f'{commit}:{name}'):
            raise ValueError('capture instrument is not committed: '+name)
        hashes[name] = sha(data)
    return {'commit': commit, 'instruments': hashes}


def verify_instrument(record, root=ROOT, paths=None):
    paths = INSTRUMENTS if paths is None else paths
    commit = record['commit']
    result = subprocess.run(['git', '-C', str(root), 'merge-base', '--is-ancestor', commit, 'HEAD'],
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    if result.returncode:
        raise ValueError('capture tooling commit is not an ancestor of HEAD; regenerate')
    if set(record['instruments']) != set(paths):
        raise ValueError('capture instrument inventory differs; regenerate')
    for name in paths:
        historical = git(root, 'show', f'{commit}:{name}')
        if (sha(historical) != record['instruments'][name] or
                historical != git(root, 'show', f'HEAD:{name}') or
                historical != (root/name).read_bytes()):
            raise ValueError('capture was produced by a different instrument; regenerate: '+name)
    return record


def cargo_artifacts(messages, package_id, target, root=ROOT):
    """Select only this invocation's package/target records, never old builds."""
    executables = [m['executable'] for m in messages
                   if m.get('reason') == 'compiler-artifact' and m.get('package_id') == package_id
                   and m.get('target') == target and m.get('executable')]
    outputs = [m['out_dir'] for m in messages
               if m.get('reason') == 'build-script-executed' and m.get('package_id') == package_id]
    if len(executables) != 1 or len(outputs) != 1:
        raise ValueError(f'expected one current rowplay-app executable and OUT_DIR; got {len(executables)}/{len(outputs)}')
    executable = (root/executables[0]).resolve()
    out = (root/outputs[0]).resolve()
    meshes = sorted((out/'replay-balsam/athlete/meshes').glob('*.mesh'))
    if len(meshes) != 1:
        raise ValueError(f'expected one current generated athlete mesh; got {len(meshes)} in {out}')
    qml = out/'replay-balsam/athlete/Rowplay_athlete_v4.qml'
    for path in [executable, meshes[0], qml]:
        if not path.is_file():
            raise ValueError('missing current Cargo artifact: '+str(path))
    return {'package_id': package_id, 'executable': str(executable), 'out_dir': str(out),
            'athlete_mesh': str(meshes[0]), 'generated_qml': str(qml),
            'sha256': {key: sha(path.read_bytes()) for key, path in
                       [('executable', executable), ('athlete_mesh', meshes[0]), ('generated_qml', qml)]}}


def build_capture(out, root=ROOT):
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--format-version=1', '--no-deps'], cwd=root))
    packages = [p for p in metadata['packages']
                if Path(p['manifest_path']).resolve() == (root/'crates/rowplay-app/Cargo.toml').resolve()]
    if len(packages) != 1:
        raise ValueError('ambiguous rowplay-app Cargo package')
    package = packages[0]
    targets = [t for t in package['targets'] if t['kind'] == ['bin'] and t['name'] == 'rowplay-app']
    if len(targets) != 1:
        raise ValueError('ambiguous rowplay-app binary target')
    command = ['cargo', 'build', '-p', package['id'], '--message-format=json-render-diagnostics']
    log = out/'cargo-build.jsonl'
    with log.open('w') as output:
        subprocess.run(command, cwd=root, stdout=output, check=True)
    messages = [json.loads(line) for line in log.read_text().splitlines()]
    if not any(m.get('reason') == 'build-finished' and m.get('success') is True for m in messages):
        raise ValueError('Cargo did not report a successful build')
    result = cargo_artifacts(messages, package['id'], targets[0], root)
    result.update(command=command, messages_sha256=sha(log.read_bytes()))
    return result


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def diagnostic_textures(out, maps, hashes):
    result = {}
    for key, filename in [('roughness', 'roughness.png'), ('normal', 'normal.png')]:
        fields = dict(scaleU=1, scaleV=1, positionU=0, positionV=0, pivotU=0, pivotV=0,
                      rotationUV=0, mappingMode=0, tilingModeHorizontal=3, tilingModeVertical=3,
                      flipU=False, flipV=False, minFilter=2, magFilter=2, mipFilter=0, generateMipmaps=False)
        if key == 'roughness':
            fields.update(scaleU=f32(maps['texture_scale'][0]), scaleV=f32(maps['texture_scale'][1]),
                          positionU=f32(maps['texture_offset'][0]), positionV=f32(maps['texture_offset'][1]),
                          tilingModeHorizontal=1, tilingModeVertical=1)
        result[key] = dict(identity='audit-'+key, source=(out/filename).resolve().as_uri(),
                           sha256=hashes[filename], sourceItem=False, textureData=False, **fields)
    return result


def expected_materials(baseline, variant, textures):
    if variant not in ['A', 'B', 'C', 'D', 'L', 'A0']:
        raise ValueError('unknown material variant '+variant)
    expected = deepcopy(baseline)
    for node in expected:
        for material in node['materials']:
            state = material['state']
            if variant == 'B':
                state.update(baseColor='#a3a3a3', vertexColorsEnabled=False, roughness=f32(.9),
                             specularAmount=f32(.1), clearcoatAmount=0, normalMap=None, normalStrength=0)
            elif variant in ['C', 'D', 'L']:
                state.update(roughness=1, roughnessMap=textures['roughness'], roughnessChannel=0,
                             specularAmount=f32(.25), clearcoatAmount=0,
                             normalMap=textures['normal'] if variant == 'D' else None,
                             normalStrength=f32(.2) if variant == 'D' else 0)
            elif variant == 'A0':
                state.update(normalMap=None, normalStrength=0)
    return expected


def validate_baseline(baseline):
    if not baseline or len({n['node'] for n in baseline}) != len(baseline):
        raise ValueError('empty/duplicate production material node inventory')
    fields = set(SCALAR_FIELDS+MAP_FIELDS+COLOR_FIELDS+VECTOR_FIELDS)
    for node in baseline:
        if set(node) != {'node', 'objectName', 'materialCount', 'materials'}:
            raise ValueError('invalid material node schema')
        if node['materialCount'] != len(node['materials']) or not node['materials']:
            raise ValueError('invalid production material count')
        for material in node['materials']:
            if set(material) != {'identity', 'objectName', 'state'} or not material['identity']:
                raise ValueError('invalid material identity/schema')
            state = material['state']
            if set(state) != fields:
                raise ValueError('incomplete audited material contract')
            # Actual generated V4 settings, independently checked before intervention.
            anchors = dict(baseColor='#ffffff', vertexColorsEnabled=True, roughness=f32(.64),
                           metalness=0, clearcoatAmount=f32(.03), clearcoatRoughnessAmount=f32(.85),
                           opacity=1, alphaMode=3, emissiveFactor=[0, 0, 0], heightAmount=0,
                           transmissionFactor=0, thicknessFactor=0, vertexColorsMaskEnabled=False,
                           fresnelScaleBiasEnabled=False, clearcoatFresnelScaleBiasEnabled=False)
            if any(state[key] != value for key, value in anchors.items()) or any(state[key] is not None for key in MAP_FIELDS):
                raise ValueError('production baseline differs from untouched imported V4; inspect/regenerate')


def validate_materials(experiments, manifest, lower=False):
    seen = set(); baseline = None; counts = {}
    for record in experiments:
        name = record['name']
        if name in seen:
            raise ValueError('duplicate material capture '+name)
        seen.add(name)
        variant = 'B' if lower else name.rsplit('-', 1)[-1]
        if record['variant'] != variant:
            raise ValueError('variant name/material payload disagree: '+name)
        validate_baseline(record['baseline'])
        if baseline is None:
            baseline = record['baseline']
        if record['baseline'] != baseline:
            raise ValueError('production baseline/inventory drift: '+name)
        if variant == 'mask':
            continue
        wanted = expected_materials(baseline, variant, manifest['diagnostic_textures'])
        if record['materials'] != wanted:
            raise ValueError('unauthorized material state for '+name)
        counts[variant] = counts.get(variant, 0)+1
    if baseline is None:
        raise ValueError('no material captures')
    return {'contract': 1, 'captures_by_variant': counts, 'nodes': len(baseline),
            'materials': sum(n['materialCount'] for n in baseline),
            'baseline_sha256': sha(json.dumps(baseline, sort_keys=True).encode()),
            'validated': ['A untouched baseline', 'B exact clay delta', 'C exact roughness delta',
                          'D C plus exact normal delta', 'L materials equal C', 'A0 exact normal-disable delta'] if not lower else ['B exact clay delta']}


def verify_capture_manifest(manifest, capture, root=ROOT, lower=False):
    verify_instrument(manifest['tooling'], root)
    verify_palette_record(manifest['canonical_palette'], root)
    instruments = manifest['tooling']['instruments']
    if manifest['instrument_sha256'] != instruments['tools/blender/capture_athlete.py']:
        raise ValueError('capture was produced by a different instrument; regenerate')
    if lower and manifest['supplemental_instrument_sha256'] != instruments['tools/blender/capture_athlete_lower.py']:
        raise ValueError('lower capture was produced by a different instrument; regenerate')
    if manifest['asset_sha256'] != sha((root/'assets/replay/rowplay-athlete-v4.glb').read_bytes()):
        raise ValueError('athlete asset differs; regenerate')
    for name, digest in manifest['map_sha256'].items():
        if sha((capture/name).read_bytes()) != digest:
            raise ValueError('diagnostic map hash differs: '+name)
    cargo = manifest['cargo']
    if (cargo['executable'] != cargo['launched_executable'] or
            cargo['athlete_mesh'] != manifest['imported_uv']['mesh_path'] or
            cargo['sha256']['athlete_mesh'] != manifest['imported_uv']['mesh_sha256'] or
            cargo['messages_sha256'] != sha((capture/'cargo-build.jsonl').read_bytes())):
        raise ValueError('capture Cargo artifact/launch/mesh provenance differs; regenerate')
    expected = diagnostic_textures(capture, manifest['maps'], manifest['map_sha256'])
    if manifest['diagnostic_textures'] != expected:
        raise ValueError('diagnostic texture identity/transform differs; regenerate')
    return {'tooling_commit': manifest['tooling']['commit'], 'canonical_palette': manifest['canonical_palette'],
            'instrument_hashes_verified': True, 'map_hashes_verified': True}
