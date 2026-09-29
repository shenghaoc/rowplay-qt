#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Evaluate all eight V5 primitives using the unchanged Phase 5 LBS instrument.

Anatomical hand masks are authored surface labels, independent of weights.
Native silhouette agreement is mandatory before interpreting measurements.
The historical V4 skin/geometry instruments and evidence remain unchanged.
"""
import argparse
from copy import copy, deepcopy
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

from contact_skin import (ROOT, Glb, skin, matrix, transform, project, native_view,
                          surface_distance, sampled_surface, equipment, mesh_surface,
                          cylinder_sdf)
from audit_athlete import stretch, triangle_metrics, stats, region
from athlete_evidence import sha, verify_instrument


def skin_all(glb, contract, record):
    parts, triangles, masks, rest, weights, joint_indices = [], [], {}, [], [], []
    offset = 0
    labels = list(contract['contactRegionsByPrimitive'][0])
    for index, primitive in enumerate(glb.json['meshes'][0]['primitives']):
        proxy = copy(glb); proxy.json = deepcopy(glb.json)
        proxy.json['meshes'][0]['primitives'] = [primitive]
        p, t, _, r, w = skin(proxy, record)
        parts.append(p); triangles.append(t+offset); rest.append(r); weights.append(w)
        joint_indices.append(glb.accessor(primitive['attributes']['JOINTS_0']).astype(int))
        for label in labels:
            mask = np.zeros(len(p), dtype=bool)
            mask[contract['contactRegionsByPrimitive'][index][label]] = True
            masks.setdefault(label, []).append(mask)
        offset += len(p)
    return (np.concatenate(parts), np.concatenate(triangles),
            {k: np.concatenate(v) for k,v in masks.items()}, np.concatenate(rest),
            np.concatenate(weights), np.concatenate(joint_indices))


def validate_mask(glb, contract, record, output):
    world, triangles, masks, _, _, _ = skin_all(glb, contract, record)
    native = native_view(record, output)
    if native is None:
        raise ValueError('missing native capture: '+record['name'])
    image = np.asarray(native)
    actual = (image[:,:,0]>150)&(image[:,:,2]>150)&(image[:,:,1]<80)
    xy, depth = project(world, record)
    xy *= np.array(native.size)/record['viewport']
    predicted = Image.new('1', native.size); draw = ImageDraw.Draw(predicted)
    for t in triangles[np.all(depth[triangles]>.1,axis=1)]:
        if np.isfinite(xy[t]).all(): draw.polygon([tuple(v) for v in xy[t]],fill=1)
    side = 'Left' if '-left-' in record['name'] else 'Right'
    hand = np.logical_or.reduce([v for k,v in masks.items() if k.startswith(side)])
    bounds = xy[hand & (depth>.1)]
    lo = np.maximum(np.floor(bounds.min(0)-5).astype(int), [0,0])
    hi = np.minimum(np.ceil(bounds.max(0)+5).astype(int), native.size)
    roi = (slice(lo[1],hi[1]), slice(lo[0],hi[0]))
    a,b = actual[roi], np.asarray(predicted)[roi]
    if not (a|b).any(): raise ValueError('empty hand silhouette')
    return dict(name=record['name'],hand_roi=[*lo.tolist(),*hi.tolist()],
                native_pixels=int(a.sum()),cpu_pixels=int(b.sum()),
                iou=float((a&b).sum()/(a|b).sum()),alpha_min=int(image[:,:,3].min()))


def deformation(glb, contract, record):
    world,t,_,rest,w,joints = skin_all(glb,contract,record)
    posed = transform(world,np.linalg.inv(matrix(record['rig'])))
    small,large,valid = stretch(rest,posed,t)
    area,_,_ = triangle_metrics(rest,t)
    names = [glb.json['nodes'][i]['name'] for i in glb.json['skins'][0]['joints']]
    labels = np.array([region(names[i]) for i in joints[np.arange(len(w)),np.argmax(w,axis=1)]])
    tl = labels[t]
    tri_labels = np.where((tl[:,0]==tl[:,1])|(tl[:,0]==tl[:,2]),tl[:,0],np.where(tl[:,1]==tl[:,2],tl[:,1],tl[:,0]))
    result = {'name':record['name'], 'regions':{}}
    for label in sorted(set(tri_labels)):
        q = (tri_labels==label)&valid
        result['regions'][label] = {'triangles':int(q.sum()),'rest_area_m2':float(area[q].sum()),
            'area_ratio':stats((small*large)[q]),'min_stretch':stats(small[q]),'max_stretch':stats(large[q]),
            'rest_area_fraction_compressed_below_half':float(area[q & (small*large<.5)].sum()/area[q].sum()),
            'rest_area_fraction_stretched_over_two':float(area[q & (large>2)].sum()/area[q].sum())}
    return result


def contacts(glb, contract, rig, record):
    world,t,masks,_,_,_ = skin_all(glb,contract,record)
    inv = np.linalg.inv(matrix(record['rig'])); p = transform(world,inv)
    result = {'name':record['name'],'sport':record['sport'],'helpers':record['contacts'],'hands':{}}
    for side in ['Left','Right']:
        surface = mesh_surface(rig,record,side)
        origin,axis,radius,half = equipment(record,side)
        frame = contract['handCalibration'][side.lower()]
        hand = inv@matrix(record['nodes']['v4'+side+'Hand'])
        palm = transform(np.array([frame['palmContact']]),hand)[0]
        palm_tri = t[np.all(masks[side+'Palm'][t],axis=1)]
        report = {'palm_landmark_to_palm_skin_mm':float(surface_distance(palm[None],p[palm_tri])[0]*1000),
                  'palm_landmark_to_equipment_mm':float(surface_distance(palm[None],surface)[0]*1000),
                  'regions':{}}
        for digit in ['Palm','Index','Middle','Ring','Pinky','Thumb']:
            points,edges,tris = sampled_surface(p,t,masks[side+digit])
            distance = surface_distance(points,surface)
            signed,_,_ = cylinder_sdf(points,origin,axis,radius,half)
            row = {'triangles':len(tris),'actual_min_mm':float(distance.min()*1000),
                   'actual_p05_mm':float(np.quantile(distance,.05)*1000),
                   'actual_fraction_within_2mm':float(np.mean(distance<=.002)),
                   'sample_cover_bound_mm':1.0,
                   'proxy_min_signed_mm':float(signed.min()*1000),
                   'proxy_penetration_fraction':float(np.mean(signed<0))}
            if digit != 'Palm':
                name = 'v4'+side+digit+'Distal'
                distal = inv@matrix(record['nodes'][name])
                tip = transform(np.array([[0,frame['terminalLengths'][name],0]]),distal)
                row['terminal_to_own_skin_mm'] = float(surface_distance(tip,p[tris])[0]*1000)
                row['terminal_to_equipment_mm'] = float(surface_distance(tip,surface)[0]*1000)
            report['regions'][digit] = row
        result['hands'][side] = report
    return result


def palm_reach_bound(glb, contract, rig, record):
    """Conservative bound, allowing EVERY descendant joint arbitrary rotation.

    A bone origin is at most the sum of its chain's translation lengths
    from the fixed shoulder. A weighted skin point is a convex sum of bone
    transforms, so its radius is at most the weighted sum of each chain
    length plus that point's inverse-bind radius. The maximum over all palm
    vertices bounds every point of every palm triangle. Joint limits,
    collisions and credible anatomy can only reduce this upper bound.
    """
    _, _, masks, rest, weights, indices = skin_all(glb,contract,record)
    doc = glb.json; nodes = doc['nodes']; joints = doc['skins'][0]['joints']
    names = [nodes[i]['name'] for i in joints]
    parents = {child:i for i,n in enumerate(nodes) for child in n.get('children',[])}
    inverse = glb.accessor(doc['skins'][0]['inverseBindMatrices']).reshape(-1,4,4).transpose(0,2,1)
    hom = np.column_stack((rest,np.ones(len(rest))))
    inv_rig = np.linalg.inv(matrix(record['rig']))
    rows = []
    for side in ['Left','Right']:
        shoulder_name = 'v4'+side+'UpperArm'
        root = joints[names.index(shoulder_name)]
        origin = (inv_rig@matrix(record['nodes'][shoulder_name]))[:3,3]
        radius = np.zeros(len(rest)); selected = masks[side+'Palm']; chains = {}
        for k in range(4):
            for joint in np.unique(indices[selected,k]):
                q = selected & (indices[:,k]==joint) & (weights[:,k]>0)
                if not q.any(): continue
                node = joints[joint]; length = 0.0
                while node != root:
                    if node not in parents:
                        raise ValueError('palm influence outside shoulder subtree: '+names[joint])
                    length += float(np.linalg.norm(nodes[node].get('translation',[0,0,0])))
                    node = parents[node]
                local = (hom[q]@inverse[joint].T)[:,:3]
                radius[q] += weights[q,k]*(length+np.linalg.norm(local,axis=1))
                chains[names[joint]] = length
        maximum = float(radius[selected].max())
        nearest = float(surface_distance(origin[None],mesh_surface(rig,record,side))[0])
        rows.append(dict(name=record['name'],side=side,shoulder=origin.tolist(),
                         conservative_palm_skin_radius_m=maximum,nearest_actual_hood_m=nearest,
                         minimum_possible_gap_m=nearest-maximum,chains_m=chains))
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('capture',type=Path)
    parser.add_argument('--contacts',action='store_true')
    args = parser.parse_args()
    manifest = json.loads((args.capture/'manifest.json').read_text())
    verify_instrument(manifest['tooling'], paths=list(manifest['tooling']['instruments']))
    for path, expected in manifest['inputs'].items():
        if sha((ROOT/path).read_bytes()) != expected: raise ValueError('capture input drift: '+path)
    glb = Glb(ROOT/'assets/replay/authored/rowplay-athlete-v5.glb')
    contract = json.loads((ROOT/'assets/replay/authored/rowplay-athlete-v5.contract.json').read_text())
    records = json.loads((args.capture/'frames.json').read_text())
    masks = [validate_mask(glb,contract,r,args.capture) for r in records if r['name'].endswith('-mask')]
    (args.capture/'skin-validation.json').write_text(json.dumps(masks,indent=2)+'\n')
    if not masks or min(r['iou'] for r in masks)<.97:
        raise ValueError('native/CPU silhouettes disagree; do not interpret measurements')
    print('native/CPU minimum IoU',min(r['iou'] for r in masks),flush=True)
    result = [deformation(glb,contract,r) for r in records if r['name'].endswith('-chase')]
    (args.capture/'deformation.json').write_text(json.dumps(result,indent=2)+'\n')
    rig = Glb(ROOT/'assets/replay/rowplay-rigs-v3.glb')
    reach = [row for r in records if r['sport']==2 and r['name'].endswith('-chase')
             for row in palm_reach_bound(glb,contract,rig,r)]
    (args.capture/'reach-bound.json').write_text(json.dumps(reach,indent=2)+'\n')
    if args.contacts:
        rigs = [Glb(ROOT/'assets/replay/authored/rowing-shell.glb'), Glb(ROOT/'assets/replay/rowplay-rigs-v3.glb')]
        rows = []
        for r in records:
            if not r['name'].endswith('-chase'): continue
            rows.append(contacts(glb,contract,rigs[0 if r['sport']==0 else 1],r))
            print(r['name'],flush=True)
        (args.capture/'skin.json').write_text(json.dumps(rows,indent=2)+'\n')


if __name__ == '__main__': main()
