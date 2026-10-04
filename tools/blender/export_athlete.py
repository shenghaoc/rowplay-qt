# SPDX-License-Identifier: GPL-3.0-or-later
"""Validate and export the reviewed V5 .blend; this tool models nothing.

Run with Blender 5.2.2, for example::

    blender -b --python tools/blender/export_athlete.py -- --source-sha256 SHA

The saved source owns geometry, UVs, material regions, bone placement and
weights. The exporter rejects drift before exporting. The inherited motion
clips are basis-converted, without changing their timing or motion, from the
immutable V4 reference. Rust remains their only evaluator; Balsam strips its
animation components. No Blender executable is needed by an application build.
"""

import argparse
from collections import Counter, defaultdict
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
import struct
import sys

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'assets/replay/authored/rowplay-athlete-v5.blend'
BASE = ROOT / 'assets/replay/authored/sources/rowplay-human-base-male-v1.4.1.blend'
BASE_SHA = '1defdfb22b53ce3bd779acfa96278ccfdff17f0e1178fa94967d600a9e27c457'
V4_SHA = 'a564a4dbd4922e2ba76ef21a23f5bf0eb1b0180846548f9d7110e55ffd8f760e'
CONTRACT_SHA = '62dd814ba6113ca57419aac42905233f8e07589846be5c3a25bbadcf55d63dd3'
BLENDER = (5, 2, 2)
ROLES = tuple('athlete-' + r for r in
              ('skin', 'fabric', 'shorts', 'footwear', 'hair', 'trim', 'eye', 'face-detail'))
TARGET, CEILING = 60_000, 75_000


class ContractError(ValueError):
    """A source or output differs from the reviewed export contract."""


def require(condition, message):
    if not condition:
        raise ContractError(message)


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_glb(path):
    data = path.read_bytes()
    require(len(data) >= 28, 'truncated GLB')
    magic, version, length, size, kind = struct.unpack_from('<5I', data)
    require((magic, version, length, kind) == (0x46546C67, 2, len(data), 0x4E4F534A),
            'invalid GLB header')
    binary_size, binary_kind = struct.unpack_from('<2I', data, 20 + size)
    require(binary_kind == 0x004E4942 and 28 + size + binary_size == len(data), 'invalid binary chunk')
    return json.loads(data[20:20 + size]), bytearray(data[28 + size:])


def write_glb(path, doc, binary):
    doc['buffers'] = [{'byteLength': len(binary)}]
    encoded = json.dumps(doc, separators=(',', ':'), allow_nan=False).encode()
    encoded += b' ' * (-len(encoded) % 4)
    binary += b'\0' * (-len(binary) % 4)
    path.write_bytes(struct.pack('<5I', 0x46546C67, 2, 28 + len(encoded) + len(binary),
                                 len(encoded), 0x4E4F534A) + encoded +
                     struct.pack('<2I', len(binary), 0x004E4942) + binary)


def accessor(doc, binary, index):
    a = doc['accessors'][index]
    v = doc['bufferViews'][a['bufferView']]
    require('sparse' not in a and 'byteStride' not in v, 'expected dense tightly packed accessors')
    width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4': 16}[a['type']]
    code = {5126: 'f', 5125: 'I', 5123: 'H', 5121: 'B'}[a['componentType']]
    offset = v.get('byteOffset', 0) + a.get('byteOffset', 0)
    values = struct.unpack_from('<' + code * (a['count'] * width), binary, offset)
    return [values[i:i + width] for i in range(0, len(values), width)]


def append_floats(doc, binary, values, kind):
    binary += b'\0' * (-len(binary) % 4)
    flat = [x for row in values for x in row]
    data = struct.pack('<' + 'f' * len(flat), *flat)
    view = len(doc['bufferViews'])
    doc['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': len(data)})
    binary += data
    index = len(doc['accessors'])
    doc['accessors'].append({'bufferView': view, 'componentType': 5126, 'count': len(values),
                             'type': kind, 'min': list(map(min, zip(*values))),
                             'max': list(map(max, zip(*values)))})
    return index


def qmul(a, b):
    x, y, z, w = a
    X, Y, Z, W = b
    return [w*X+x*W+y*Z-z*Y, w*Y-x*Z+y*W+z*X, w*Z+x*Y-y*X+z*W, w*W-x*X-y*Y-z*Z]


def qinv(q):
    return [-q[0], -q[1], -q[2], q[3]]


def parents(doc):
    return {child: i for i, n in enumerate(doc['nodes']) for child in n.get('children', [])}


def world_rotations(doc):
    parent = parents(doc)
    cache = {}
    def world(i):
        if i not in cache:
            q = doc['nodes'][i].get('rotation', [0, 0, 0, 1])
            cache[i] = qmul(world(parent[i]), q) if i in parent else q
        return cache[i]
    return [world(i) for i in range(len(doc['nodes']))]


def inherit_motion(doc, binary, old, old_binary):
    """Change the coordinate bases, never the motion graph, phase or key times.

    V4 semantic rest bases are identity. If Q is a new world rest basis,
    A' = inverse(Q_parent) * A * Q_joint. Telescoping over the hierarchy
    preserves every animated world delta. Translation channels only move
    the root, whose absolute position is subsequently owned by Rust.
    """
    # V4 node TRS contains an initial animated pose, not its bind pose.
    # Establish the identity-basis premise from actual inverse binds.
    animated = {c['target']['node'] for a in old['animations'] for c in a['channels']}
    skin = old['skins'][0]
    for node, inverse in zip(skin['joints'], accessor(old, old_binary, skin['inverseBindMatrices'])):
        if node in animated:
            require(all(abs(inverse[c*4+r] - (r == c)) < 1e-7
                        for r in range(3) for c in range(3)),
                    'inherited semantic bind rotation is not identity')
    by_name = {n['name']: i for i, n in enumerate(doc['nodes']) if 'name' in n}
    parent, world = parents(doc), world_rotations(doc)
    doc['animations'] = []
    for old_clip in old['animations']:
        clip = {'name': old_clip['name'], 'channels': [], 'samplers': []}
        for channel in old_clip['channels']:
            name = old['nodes'][channel['target']['node']]['name']
            target = by_name[name]
            path = channel['target']['path']
            sampler = old_clip['samplers'][channel['sampler']]
            require(sampler['interpolation'] == 'LINEAR', 'unexpected inherited interpolation')
            times = accessor(old, old_binary, sampler['input'])
            values = accessor(old, old_binary, sampler['output'])
            require(path in ('rotation', 'translation'), 'unexpected inherited motion channel')
            if path == 'rotation':
                basis = world[parent[target]] if target in parent else [0, 0, 0, 1]
                values = [qmul(qmul(qinv(basis), q), world[target]) for q in values]
            else:
                require(name == 'v4Hips', 'only root translation is inherited')
            clip['channels'].append({'sampler': len(clip['samplers']),
                                     'target': {'node': target, 'path': path}})
            clip['samplers'].append({'input': append_floats(doc, binary, times, 'SCALAR'),
                                     'output': append_floats(doc, binary, values,
                                                            'VEC4' if path == 'rotation' else 'VEC3'),
                                     'interpolation': 'LINEAR'})
        doc['animations'].append(clip)


def validate_output(doc, binary):
    import numpy as np
    require(not doc.get('extensionsUsed') and not doc.get('extensionsRequired'), 'unsupported glTF extension')
    require(not any(doc.get(k) for k in ('images', 'textures', 'cameras')), 'unapproved external/render dependency')
    encoded = json.dumps(doc)
    require(not any(s in encoded for s in ('/home/', '/tmp/', 'file:', ':\\')), 'source path leakage')
    require(len(doc.get('meshes', [])) == len(doc.get('skins', [])) == 1, 'expected one mesh and skin')
    require(set(m['name'] for m in doc['materials']) == set(ROLES) and len(doc['materials']) == 8,
            'material role completeness/uniqueness')
    primitives = doc['meshes'][0]['primitives']
    require(len(primitives) == 8 and len({p['material'] for p in primitives}) == 8, 'one primitive per role')
    skin = doc['skins'][0]
    joint_ids = set(skin['joints'])
    require(len(joint_ids) == 51, 'expected 51 skin joints')
    pmap = parents(doc)
    matrices = {}
    from mathutils import Matrix, Quaternion, Vector
    def matrix(i):
        if i not in matrices:
            node = doc['nodes'][i]
            require('matrix' not in node, 'expected decomposed node transforms')
            t = Vector(node.get('translation', [0, 0, 0]))
            x, y, z, w = node.get('rotation', [0, 0, 0, 1])
            q = Quaternion((w, x, y, z))
            s = Vector(node.get('scale', [1, 1, 1]))
            require(abs(sum(v*v for v in q) - 1) < 1e-5 and max(abs(v-1) for v in s) < 1e-5, 'non-rigid joint transform')
            local = Matrix.LocRotScale(t, q, s)
            matrices[i] = matrix(pmap[i]) @ local if i in pmap else local
        return matrices[i]
    for i in range(len(doc['nodes'])):
        m = matrix(i)
        require(all(math.isfinite(v) for row in m for v in row), 'non-finite node matrix')
        if i not in joint_ids:
            require(max(abs(m[r][c] - (r == c)) for r in range(4) for c in range(4)) < 1e-5,
                    'nonidentity mesh/root transform')
    inverse = np.array(accessor(doc, binary, skin['inverseBindMatrices'])).reshape(-1, 4, 4).transpose(0, 2, 1)
    for k, i in enumerate(skin['joints']):
        require(np.max(np.abs(np.array(matrix(i)) @ inverse[k] - np.eye(4))) < 2e-5,
                'inverse bind differs from authored rest transform')
    triangles = 0
    counts = {}
    for p in primitives:
        require(p.get('mode', 4) == 4 and not p.get('targets'), 'only triangles; no hidden deformation')
        require(set(p['attributes']) == {'POSITION', 'NORMAL', 'TEXCOORD_0', 'COLOR_0', 'JOINTS_0', 'WEIGHTS_0'},
                'unexpected/missing vertex attributes')
        a = {k: np.array(accessor(doc, binary, v)) for k, v in p['attributes'].items()}
        require(all(np.isfinite(v).all() for v in a.values()), 'non-finite vertex attributes')
        require(np.max(np.abs(np.linalg.norm(a['NORMAL'], axis=1)-1)) < 1e-4, 'nonunit normals')
        w, j = a['WEIGHTS_0'], a['JOINTS_0']
        require(w.shape[1] == 4 and w.min() >= 0 and np.max(np.abs(w.sum(axis=1)-1)) < 1e-5,
                'skin weights must be normalized, nonnegative, at most four influences')
        require(j.min() >= 0 and j.max() < 51, 'invalid skin joint index')
        indices = np.array(accessor(doc, binary, p['indices'])).reshape(-1)
        require(len(indices) % 3 == 0 and indices.min() >= 0 and indices.max() < len(w), 'invalid indices')
        count = len(indices) // 3
        triangles += count
        counts[doc['materials'][p['material']]['name']] = count
    require(triangles <= CEILING, f'{triangles} triangles exceeds the {CEILING} hard ceiling')
    return {'triangles': triangles, 'target': TARGET, 'ceiling': CEILING,
            'targetAchieved': triangles <= TARGET, 'materialTriangles': counts}


def inspect_source(legacy):
    import bpy
    import bmesh
    import numpy as np
    require(bpy.app.version == BLENDER, f'expected Blender {BLENDER}; got {bpy.app.version}')
    scene = bpy.context.scene
    require(scene.unit_settings.system == 'METRIC' and scene.unit_settings.scale_length == 1, 'source must use metres')
    require(scene.get('rowplaySourceSha256') == BASE_SHA, 'source snapshot identity differs')
    objects = list(scene.objects)
    require({o.name for o in objects} == {'rowplayAthleteV5', 'rowplay-athlete-v5-root'}, 'unexpected source objects')
    body = bpy.data.objects['rowplayAthleteV5']
    rig = bpy.data.objects['rowplay-athlete-v5-root']
    require(body.type == 'MESH' and rig.type == 'ARMATURE' and body.parent == rig, 'source object contract')
    for o in objects:
        require(all(abs(o.matrix_world[r][c]-(r == c)) < 1e-7 for r in range(4) for c in range(4)), 'source root transform')
        require(o.animation_data is None, 'source may not carry competing animation')
    require(len(body.modifiers) == 1 and body.modifiers[0].type == 'ARMATURE'
            and body.modifiers[0].object == rig and not body.modifiers[0].use_deform_preserve_volume,
            'only runtime-compatible linear skinning is allowed')
    # Blender's built-in Render Result is a transient VIEWER, not an image
    # dependency and never exported. File and generated images are forbidden.
    require(not body.data.shape_keys and not any(i.source != 'VIEWER' for i in bpy.data.images)
            and not bpy.data.actions and not bpy.data.texts,
            'unapproved images, actions, corrective shapes or embedded scripts')
    expected = {b['name']: b['parent'] for b in legacy['bones']['hierarchy'] + legacy['bones']['helpers']}
    require({b.name: b.parent.name if b.parent else None for b in rig.data.bones} == expected,
            'semantic/helper names or hierarchy changed')
    require([m.name for m in body.data.materials] == list(ROLES), 'source material slots/order changed')
    require(set(p.material_index for p in body.data.polygons) == set(range(8)), 'unused material role')
    require(len(body.data.uv_layers) == 1 and all(math.isfinite(x) for v in body.data.uv_layers[0].data for x in v.uv),
            'one finite UV set required')
    require(len(body.data.color_attributes) == 1 and body.data.color_attributes[0].name == 'Albedo',
            'only the reviewed albedo attribute is allowed')
    require(all(math.isfinite(x) and 0 <= x <= 1 for v in body.data.color_attributes[0].data for x in v.color),
            'albedo must be finite and bounded')
    require(all(m.use_nodes and all(n.type != 'TEX_IMAGE' for n in m.node_tree.nodes) for m in body.data.materials),
            'no image dependencies permitted')
    names = {g.index: g.name for g in body.vertex_groups}
    require(set(names.values()) == set(expected), 'weight group inventory changed')
    distribution = Counter()
    for v in body.data.vertices:
        require(all(math.isfinite(x) for x in v.co), 'non-finite source position')
        weights = [g.weight for g in v.groups if g.weight > 0]
        require(1 <= len(weights) <= 4 and all(math.isfinite(w) and w >= 0 for w in weights)
                and abs(sum(weights)-1) < 1e-5, 'invalid source weights')
        distribution[len(weights)] += 1
    bm = bmesh.new()
    bm.from_mesh(body.data)
    require(all(e.is_manifold for e in bm.edges) and all(v.link_faces for v in bm.verts),
            'nonmanifold/open/loose source topology')
    bm.free()
    body.data.calc_loop_triangles()
    p = np.array([v.co[:] for v in body.data.vertices])
    t = np.array([t.vertices[:] for t in body.data.loop_triangles])
    a, b, c = [p[t[:, k]] for k in range(3)]
    area = np.linalg.norm(np.cross(b-a, c-a), axis=1)/2
    require(area.min() > 1e-10, 'degenerate source triangles')
    require(len({tuple(sorted(row)) for row in t}) == len(t), 'duplicate source triangles')
    edges = np.stack((np.linalg.norm(b-a, axis=1), np.linalg.norm(c-b, axis=1), np.linalg.norm(a-c, axis=1)), axis=1)
    quality = 4*np.sqrt(3)*area / np.sum(edges*edges, axis=1)
    require(quality.min() > .002, 'extreme source sliver')
    return body, rig, {'sourceVertices': len(p), 'sourceTriangles': len(t),
                       'weightInfluences': dict(sorted(distribution.items())),
                       'singleInfluencePercent': distribution[1]/len(p)*100,
                       'minimumTriangleAreaM2': float(area.min()),
                       'triangleQualityQuantiles': np.quantile(quality, [0, .01, .5, 1]).tolist(),
                       'openEdges': 0, 'nonmanifoldEdges': 0, 'degenerateTriangles': 0,
                       'duplicateTriangles': 0, 'looseVertices': 0}


def contract(doc, binary, legacy, body, rig, source_sha, stats):
    """Record saved source measurements; never alter the authored asset."""
    import numpy as np
    from mathutils import Vector
    pmap = parents(doc)
    nodes = doc['nodes']
    rows = []
    for i in doc['skins'][0]['joints']:
        n = nodes[i]
        parent = nodes[pmap[i]]['name'] if i in pmap and nodes[pmap[i]]['name'].startswith('v4') else None
        rows.append({'name': n['name'], 'parent': parent, 'restLocalTransform': {
            'translation': n.get('translation', [0, 0, 0]),
            'rotationQuaternion': n.get('rotation', [0, 0, 0, 1]), 'scale': n.get('scale', [1, 1, 1])}})
    semantic = legacy['bones']['semanticOrderedNames']
    helpers = legacy['bones']['helperNames']
    by_name = {r['name']: r for r in rows}
    qt = lambda p: [p[0], p[2], -p[1]]
    centres = {b.name: qt(b.head_local) for b in rig.data.bones}
    lengths = {b.name: float(b.length) for b in rig.data.bones}
    hands, contacts = {}, []
    for side in ['Left', 'Right']:
        b = rig.data.bones['v4'+side+'Hand']
        hand = {key: list(b['rowplay'+field]) for key, field in [
            ('palmContact', 'PalmContact'), ('palmNormal', 'PalmNormal'),
            ('longAxis', 'LongAxis'), ('curlAxis', 'CurlAxis')]}
        hand['channelDirection'] = hand['palmNormal']
        hand['seatFlesh'] = 0.0
        hand['terminalLengths'] = {name: float(rig.data.bones[name]['rowplayTerminalLength'])
                                    for name in helpers if name.startswith('v4'+side) and name.endswith('Distal')}
        hands[side.lower()] = hand
        contacts.append({'bone': b.name, 'role': side.lower()+'-hand', 'localOffset': hand['palmContact']})
        foot = rig.data.bones['v4'+side+'Foot']
        require('rowplaySoleContact' in foot, 'authored sole landmark missing')
        contacts.append({'bone': foot.name, 'role': side.lower()+'-foot', 'localOffset': list(foot['rowplaySoleContact'])})
    # Independent anatomical masks come from source surface regions, not skin influence.
    fs = body.data.attributes['.sculpt_face_set']
    sets = defaultdict(Counter)
    digit_sets = {'LeftPalm': [10], 'RightPalm': [9],
                  'LeftIndex': list(range(88,92)), 'RightIndex': list(range(76,80)),
                  'LeftMiddle': list(range(92,96)), 'RightMiddle': list(range(72,76)),
                  'LeftRing': list(range(96,100)), 'RightRing': list(range(68,72)),
                  'LeftPinky': list(range(100,104)), 'RightPinky': list(range(64,68)),
                  'LeftThumb': list(range(84,88)), 'RightThumb': list(range(80,84))}
    labels = {i: label for label, ids in digit_sets.items() for i in ids}
    regions = {'hands': set(labels), 'arms': {11,12,20,21}, 'legs': {15,16,18,23,24},
               # Blender preserves distinct face sets when joining objects:
               # 104/105 are the eyes; 119/121 the authored shoe envelopes.
               'feet': {13,14,119,121,*range(25,64)}, 'headNeck': {2,3,4,5,7,8,17,22}, 'torso': {1,19}}
    allocation = Counter()
    for f in body.data.polygons:
        region = fs.data[f.index].value
        label = labels.get(region, 'other')
        for i in f.vertices:
            sets[i][label] += 1
        role = body.data.materials[f.material_index].name
        group = ('eyes' if region in {104,105} else 'hair' if role == 'athlete-hair' else
                 next(k for k, ids in regions.items() if region in ids))
        allocation[group] += len(f.vertices)-2
    positions = {tuple(qt(v.co)): sets[v.index].most_common(1)[0][0] for v in body.data.vertices}
    masks = []
    for primitive in doc['meshes'][0]['primitives']:
        points = accessor(doc, binary, primitive['attributes']['POSITION'])
        require(all(tuple(p) in positions for p in points), 'exported positions differ from authored source')
        masks.append({label: [i for i, p in enumerate(points) if positions[tuple(p)] == label]
                      for label in digit_sets})
    stats['regionalTriangles'] = dict(sorted(allocation.items()))
    verts = np.array([v.co[:] for v in body.data.vertices])
    stats['heightM'] = float(verts[:, 2].max()-verts[:, 2].min())
    stats['shoulderJointBreadthM'] = float(np.linalg.norm(np.array(centres['v4LeftUpperArm'])-centres['v4RightUpperArm']))
    return {'schema': 'rowplay.replay.athlete.v4', 'schemaVersion': 1,
            'assetIdentity': 'rowplay-athlete-v5', 'sourceBlend': {'path': str(SOURCE.relative_to(ROOT)), 'sha256': source_sha},
            'coordinateSystem': {'units': 'metres', 'upAxis': '+Y', 'forwardAxis': '+Z', 'handedness': 'right-handed'},
            'bones': {'count': 19, 'semanticCount': 19, 'semanticOrderedNames': semantic,
                      'orderedNames': semantic, 'totalCount': 51, 'helperCount': 32, 'helperNames': helpers,
                      'hierarchy': [by_name[n] for n in semantic], 'helpers': [by_name[n] for n in helpers]},
            'animation': deepcopy(legacy['animation']), 'contacts': contacts,
            'surfaces': [{'role': r, 'source': 'authored material primitive'} for r in ROLES],
            'materialParameters': {m['name']: {
                'roughness': m['pbrMetallicRoughness'].get('roughnessFactor', 1.0),
                'metalness': m['pbrMetallicRoughness'].get('metallicFactor', 1.0)} for m in doc['materials']},
            'measurements': stats, 'jointCentresMetres': centres, 'boneLengthsMetres': lengths,
            'handCalibration': hands, 'contactRegionsByPrimitive': masks,
            'provenance': {'licence': 'MIT AND CC0-1.0', 'base': {'path': str(BASE.relative_to(ROOT)), 'sha256': BASE_SHA},
                           'modifications': 'MIT RowPlay modelling, rig, weights and material regions; CC0 anatomical base retained',
                           'blender': '.'.join(map(str, BLENDER)), 'exporter': 'tools/blender/export_athlete.py',
                           'motionSource': {'path': 'assets/replay/rowplay-athlete-v4.glb', 'sha256': V4_SHA},
                           'motionConversion': 'rest-basis conjugation only; inherited key times and motion graph'}}


def main():
    import bpy
    sys.path.insert(0, str(Path(__file__).parent))
    from canonical import canonicalize_pack, bound_attributes
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source-sha256', required=True)
    parser.add_argument('--output', type=Path, default=SOURCE.with_suffix('.glb'))
    args = parser.parse_args(sys.argv[sys.argv.index('--')+1:] if '--' in sys.argv else [])
    require(sha(SOURCE) == args.source_sha256, 'reviewed source .blend SHA differs')
    require(sha(BASE) == BASE_SHA, 'approved CC0 snapshot SHA differs')
    old_path = ROOT/'assets/replay/rowplay-athlete-v4.glb'
    old_contract = ROOT/'assets/replay/rowplay-athlete-v4.contract.json'
    require(sha(old_path) == V4_SHA and sha(old_contract) == CONTRACT_SHA, 'historical motion reference SHA differs')
    legacy = json.loads(old_contract.read_text())
    bpy.ops.wm.open_mainfile(filepath=str(SOURCE))
    body, rig, stats = inspect_source(legacy)
    bpy.ops.object.select_all(action='DESELECT')
    body.select_set(True)
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    out = args.output.resolve()
    out.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(out), export_format='GLB', use_selection=True, export_yup=True,
        export_materials='EXPORT', export_texcoords=True, export_normals=True, export_tangents=False,
        export_vertex_color='NAME', export_vertex_color_name='Albedo', export_all_vertex_colors=False, export_extras=False,
        export_apply=False, export_animations=False, export_cameras=False, export_lights=False,
        export_def_bones=False, export_leaf_bone=False, export_armature_object_remove=False,
        export_skins=True, export_influence_nb=4, export_all_influences=False,
        export_rest_position_armature=True, export_draco_mesh_compression_enable=False,
        export_meshopt_compression_enable=False, export_use_gltfpack=False)
    canonicalize_pack(out)
    bound_attributes(out)
    doc, binary = read_glb(out)
    stats.update(validate_output(doc, binary))
    old, old_binary = read_glb(old_path)
    inherit_motion(doc, binary, old, old_binary)
    result = contract(doc, binary, legacy, body, rig, args.source_sha256, stats)
    write_glb(out, doc, binary)
    result['runtimeArtifact'] = {'filename': out.name, 'bytes': out.stat().st_size, 'sha256': sha(out)}
    out.with_suffix('.contract.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    require(sha(SOURCE) == args.source_sha256, 'export mutated the source')
    print(json.dumps({'output': str(out), 'sha256': sha(out), 'measurements': stats}, indent=2))


if __name__ == '__main__':
    main()
