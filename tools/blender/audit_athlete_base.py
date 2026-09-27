#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Inspect the pinned, reviewed base and execute its canonical adaptation in memory.

Run with Blender --background -noaudio --factory-startup --python-exit-code 1 --python this_file -- --output DIR.
Writes diagnostic arrays/metadata only; never saves a blend or exports an asset.
The actual web builder is imported, not reimplemented as a second oracle.
"""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

import bpy
import numpy as np

ROOT = Path(__file__).resolve().parents[2]


def mesh_arrays(obj):
    mesh = obj.data
    mesh.calc_loop_triangles()
    positions = np.array([v.co[:] for v in mesh.vertices], dtype=np.float64)
    # Blender X/right Z/up -Y/forward -> contract X/right Y/up Z/forward.
    positions = positions[:, [0, 2, 1]] * [1, 1, -1]
    triangles = np.array([t.vertices[:] for t in mesh.loop_triangles], dtype=np.int32)
    polygons = np.array([t.polygon_index for t in mesh.loop_triangles], dtype=np.int32)
    normals = np.array([v.normal[:] for v in mesh.vertices])[:, [0, 2, 1]] * [1, 1, -1]
    return positions, triangles, polygons, normals


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--reference', type=Path, default=ROOT / 'reference/rowplay')
    args = p.parse_args(sys.argv[sys.argv.index('--') + 1:])
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    ref = args.reference.resolve()
    source = ref / 'scripts/build-replay-athlete-v4-blender.py'
    spec = importlib.util.spec_from_file_location('canonical_v4', source)
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    bpy.ops.object.select_all(action='SELECT')
    bpy.ops.object.delete(use_global=False)
    arrays = {}
    base_polygons = []
    facts = {'blender': bpy.app.version_string, 'reference': subprocess.check_output(
        ['git', '-C', str(ref), 'rev-parse', 'HEAD'], text=True).strip()}
    paths = [source, ref / 'scripts/extract-replay-athlete-base-blender.py',
             ref / 'scripts/build-replay-rig-v4.mjs', ref / 'src/lib/replay/rigV4.ts',
             module.DEFAULT_BASE_MESH]
    facts['sources'] = {str(path.relative_to(ref)): hashlib.sha256(path.read_bytes()).hexdigest()
                        for path in paths}
    load = module.load_base_objects
    def observe_base(path):
        body, eyes = load(path)
        pos, tri, polygon, normals = mesh_arrays(body)
        arrays.update(base=pos, triangles=tri, base_normals=normals,
                      vertex_face_sets=np.array(module.source_vertex_face_sets(body.data)),
                      triangle_face_sets=np.array([x.value for x in body.data.attributes['.sculpt_face_set'].data])[polygon])
        base_polygons.extend(tuple(p.vertices) for p in body.data.polygons)
        facts['base'] = {'vertices': len(pos), 'triangles': len(tri),
                         'polygons': len(body.data.polygons), 'uv_layers': [x.name for x in body.data.uv_layers],
                         'vertex_groups': [x.name for x in body.vertex_groups],
                         'modifiers': [x.type for x in body.modifiers],
                         'eyes': [{'name': x.name, 'vertices': len(x.data.vertices)} for x in eyes]}
        facts['embedded_provenance'] = bpy.data.texts['ROWPLAY_SOURCE_PROVENANCE.md'].as_string() if 'ROWPLAY_SOURCE_PROVENANCE.md' in bpy.data.texts else None
        return body, eyes
    module.load_base_objects = observe_base
    paint = module.paint_vertex_colors
    def observe_adaptation(body):
        pos, tri, _, normals = mesh_arrays(body)
        assert base_polygons == [tuple(p.vertices) for p in body.data.polygons], 'base polygon topology changed'
        # Blender may choose another diagonal for a deformed quad; preserve
        # both tessellations rather than mistake that for a changed body mesh.
        arrays.update(adapted=pos, adapted_normals=normals, adapted_triangles=tri)
        names = [g.name for g in body.vertex_groups]
        w = np.zeros((len(pos), len(names)))
        for v in body.data.vertices:
            for g in v.groups:
                w[v.index, g.group] = g.weight
        arrays['body_weights'] = w
        facts['weight_names'] = names
        return paint(body)
    module.paint_vertex_colors = observe_adaptation
    bones = module.global_bone_positions()
    surface, centers = module.create_base_production_surface(module.DEFAULT_BASE_MESH, bones)
    armature = module.create_armature(bones, centers)
    facts['bones'] = {b.name: {'head': list(module.from_blender(b.head_local)),
                              'tail': list(module.from_blender(b.tail_local)),
                              'parent': b.parent.name if b.parent else None}
                      for b in armature.data.bones}
    facts['grip_centers'] = {str(k): list(v) for k, v in centers.items()}
    facts['semantic_names'] = module.BONE_NAMES
    facts['helper_names'] = module.HELPER_BONE_NAMES
    facts['digit_face_sets'] = module.GRIP_DIGIT_FACE_SETS
    facts['retarget_chains'] = {k: [[list(module.from_blender(a)), list(module.from_blender(b))] for a,b in pairs]
                              for k,pairs in module.base_retarget_chains(bones).items()}
    positions, triangles, _, normals = mesh_arrays(surface)
    # Blender joins added objects in an unstable order. Retain the body prefix
    # for correspondence and sort the added position multiset; no final normals
    # or triangles are consumed (those are measured from the shipped GLB).
    body_count=len(arrays['base'])
    details=positions[body_count:]
    arrays['final_positions']=np.vstack([positions[:body_count],details[np.lexsort(details.T[::-1])]])
    facts['adapted_with_details'] = {'vertices': len(positions), 'triangles': len(triangles)}
    np.savez_compressed(out / 'base-stages.npz', **arrays)
    (out / 'base-source.json').write_text(json.dumps(facts, indent=2) + '\n')
    print(json.dumps({k: facts[k] for k in ['blender', 'reference', 'base', 'adapted_with_details']}))


if __name__ == '__main__':
    main()
