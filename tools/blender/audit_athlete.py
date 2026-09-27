#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Deterministic Phase 5.2 geometry, weights and replay-deformation audit.

Consumes the canonical base inspector and the unchanged Phase 5.1 native
palette archive. Distances are metres unless a field explicitly says mm.
No clinical anthropometry or anatomical correctness is inferred from a proxy.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path

import numpy as np

from contact_skin import Glb, ROOT, matrix, skin, surface_distance, transform


class Nearest:
    """Exact small KD tree; leaf searches and bounds use squared distances."""
    def __init__(self, points):
        self.points = np.asarray(points)
        def build(ids):
            p = self.points[ids]
            lo, hi = p.min(0), p.max(0)
            if len(ids) <= 24:
                return lo, hi, ids, None
            axis = np.argmax(hi-lo)
            ordered = ids[np.argsort(p[:, axis], kind='stable')]
            mid = len(ids)//2
            return lo, hi, build(ordered[:mid]), build(ordered[mid:])
        self.root = build(np.arange(len(points)))

    def query(self, points):
        indices, distances = [], []
        def bound(p, n):
            d = np.maximum(np.maximum(n[0]-p, p-n[1]), 0)
            return d@d
        for p in points:
            best, index = float('inf'), -1
            stack = [self.root]
            while stack:
                n = stack.pop()
                if bound(p,n) > best:
                    continue
                if n[3] is None:
                    d = self.points[n[2]]-p
                    ds = np.einsum('ij,ij->i',d,d)
                    k = int(np.argmin(ds))
                    if ds[k] < best:
                        best, index = ds[k], int(n[2][k])
                else:
                    a,b = n[2],n[3]
                    if bound(p,a) < bound(p,b): stack.extend([b,a])
                    else: stack.extend([a,b])
            indices.append(index)
            distances.append(np.sqrt(best))
        return np.array(indices), np.array(distances)


def stats(values):
    v = np.asarray(values).ravel()
    v = v[np.isfinite(v)]
    if not len(v): return {'count':0}
    return dict(zip(['min','p01','p05','median','p95','p99','max'],
                    map(float,np.quantile(v,[0,.01,.05,.5,.95,.99,1]))),count=len(v))


def triangle_metrics(p, t):
    v = p[t]
    cross = np.cross(v[:,1]-v[:,0],v[:,2]-v[:,0])
    area = np.linalg.norm(cross,axis=1)/2
    lengths = np.array([np.linalg.norm(v[:,i]-v[:,(i+1)%3],axis=1) for i in range(3)]).T
    # Equilateral=1; tends to infinity for a sliver. No arbitrary quality score.
    aspect = lengths.max(1)**2 / np.maximum(4/np.sqrt(3)*area,1e-30)
    return area, lengths, aspect


def stretch(rest, posed, triangles):
    """Singular values of the triangle's in-plane rest -> posed linear map."""
    r,p = rest[triangles],posed[triangles]
    e1,e2 = r[:,1]-r[:,0],r[:,2]-r[:,0]
    l1 = np.linalg.norm(e1,axis=1)
    u = e1/np.maximum(l1[:,None],1e-30)
    x = np.sum(e2*u,axis=1)
    y = np.linalg.norm(e2-x[:,None]*u,axis=1)
    a = (p[:,1]-p[:,0])/np.maximum(l1[:,None],1e-30)
    b = ((p[:,2]-p[:,0])-x[:,None]*a)/np.maximum(y[:,None],1e-30)
    aa,ab,bb = np.sum(a*a,axis=1),np.sum(a*b,axis=1),np.sum(b*b,axis=1)
    disc = np.sqrt(np.maximum((aa-bb)**2+4*ab*ab,0))
    small = np.sqrt(np.maximum((aa+bb-disc)/2,0))
    large = np.sqrt(np.maximum((aa+bb+disc)/2,0))
    valid = (l1>1e-10)&(y>1e-10)
    return small,large,valid


def topology(points, triangles):
    # Exact positional weld separates exported seam duplicates from geometry.
    unique, inverse = np.unique(points,axis=0,return_inverse=True)
    t = inverse[triangles]
    edges = np.sort(np.concatenate([t[:,[0,1]],t[:,[1,2]],t[:,[2,0]]]),axis=1)
    edges, counts = np.unique(edges,axis=0,return_counts=True)
    parent = np.arange(len(unique))
    def root(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i
    for a,b in edges:
        a,b = root(a),root(b)
        if a!=b: parent[b]=a
    labels = np.array([root(i) for i in range(len(unique))])
    _,component = np.unique(labels,return_inverse=True)
    triangle_components=component[t[:,0]]
    components=[]
    for c in np.unique(triangle_components):
        mask=triangle_components==c
        v=unique[np.unique(t[mask])]
        components.append({'id':int(c),'vertices':len(v),'triangles':int(mask.sum()),
                           'bounds':[v.min(0).tolist(),v.max(0).tolist()]})
    sorted_tri=np.sort(t,axis=1)
    report={'export_vertices':len(points),'unique_positions':len(unique),
            'seam_duplicate_vertices':len(points)-len(unique),'triangles':len(t),
            'components':sorted(components,key=lambda c:-c['triangles']),
            'boundary_edges':int((counts==1).sum()),'nonmanifold_edges':int((counts>2).sum()),
            'duplicate_triangles':len(t)-len(np.unique(sorted_tri,axis=0))}
    return report,edges,counts,inverse,triangle_components


def region(name):
    if any(s in name for s in ['Index','Middle','Ring','Pinky','Thumb']): return 'digits'
    for suffix,label in [('Clavicle','shoulders'),('UpperArm','upper_arms'),('Forearm','forearms'),
                         ('Hand','palms'),('Fingers','palms'),('UpperLeg','thighs'),('LowerLeg','shins'),
                         ('Foot','feet'),('Head','head'),('Neck','neck'),('Chest','chest'),
                         ('Spine','torso'),('Hips','pelvis')]:
        if name.endswith(suffix): return label
    raise ValueError(name)


def sections(points, levels):
    result={}
    for name,y in levels.items():
        # Torso source region, not arms that may cross the same height.
        p=points[np.abs(points[:,1]-y)<.006]
        result[name]={'y':y,'half_band':.006,'vertices':len(p),
                      'breadth':float(np.ptp(p[:,0])) if len(p) else None,
                      'depth':float(np.ptp(p[:,2])) if len(p) else None}
    return result


def analyse(base_dir, output):
    arrays=np.load(base_dir/'base-stages.npz')
    source=json.loads((base_dir/'base-source.json').read_text())
    glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    j=glb.json; primitive=j['meshes'][0]['primitives'][0]; att=primitive['attributes']
    p=glb.accessor(att['POSITION']).astype(float)
    t=glb.accessor(primitive['indices']).reshape(-1,3)
    normals=glb.accessor(att['NORMAL']).astype(float)
    uv=glb.accessor(att['TEXCOORD_0']).astype(float)
    joints=glb.accessor(att['JOINTS_0']).astype(int)
    weights=glb.accessor(att['WEIGHTS_0']).astype(float)
    names=[j['nodes'][i]['name'] for i in j['skins'][0]['joints']]
    inverse_bind=glb.accessor(j['skins'][0]['inverseBindMatrices']).reshape(-1,4,4).transpose(0,2,1)
    rest_matrices=np.linalg.inv(inverse_bind)
    bone=dict(zip(names,rest_matrices))
    output.mkdir(parents=True,exist_ok=True)
    print('Matching canonical adaptation to shipped GLB...',flush=True)
    match, match_distance=Nearest(arrays['final_positions']).query(p)
    body_count=len(arrays['base'])
    # Joining details preserves the body vertex prefix; independently verify it.
    assert np.allclose(arrays['final_positions'][:body_count],arrays['adapted'],atol=1e-10)
    body=match<body_count
    print('Canonical body agreement',stats(match_distance[body]),flush=True)
    print('Added-detail agreement',stats(match_distance[~body]),flush=True)
    if match_distance[body].max()>2e-6:
        raise ValueError('canonical body does not match shipped GLB; inspect lineage')
    # Modifier/export results can vary with Blender patch versions. Keep the
    # disagreement on added details explicit; only matched body vertices are
    # used to attribute anatomical changes back to the reviewed base.
    vertex_sets=np.full(len(p),-1)
    vertex_sets[body]=arrays['vertex_face_sets'][match[body]]
    triangle_body=np.all(body[t],axis=1)
    area,edge_lengths,aspect=triangle_metrics(p,t)
    topo,edges,edge_counts,weld,components=topology(p,t)
    labels=np.array([region(names[i]) for i in joints[np.arange(len(p)),np.argmax(weights,axis=1)]])
    # Majority of three dominant-bone regions, first corner breaks a 3-way tie.
    tl=labels[t]
    tri_labels=np.where((tl[:,0]==tl[:,1])|(tl[:,0]==tl[:,2]),tl[:,0],np.where(tl[:,1]==tl[:,2],tl[:,1],tl[:,0]))
    entropy=-np.sum(np.where(weights>0,weights*np.log2(np.maximum(weights,1e-30)),0),axis=1)
    full_weights=np.zeros((len(p),len(names)))
    for k in range(4): np.add.at(full_weights,(np.arange(len(p)),joints[:,k]),weights[:,k])
    report={'asset_sha256':glb.sha256,'contract_sha256':hashlib.sha256((ROOT/'assets/replay/rowplay-athlete-v4.contract.json').read_bytes()).hexdigest(),
        'source':source,'counts':{'meshes':len(j['meshes']),'primitives':len(j['meshes'][0]['primitives']),
            'skins':len(j['skins']),'material_slots':len(j['materials']),'embedded_textures':len(j.get('textures',[])),
            'semantic_bones':len(source['semantic_names']),'helper_bones':len(source['helper_names']),'total_bones':len(names)},
        'canonical_position_agreement_m':{'body':stats(match_distance[body]),'details':stats(match_distance[~body])},'topology':topo,
        'normal_lengths':stats(np.linalg.norm(normals,axis=1)),
        'tangents_present':'TANGENT' in att,'material':j['materials'],
        'rest_regions':{},'weights':{'sum_error':stats(np.abs(weights.sum(1)-1)),
            'influence_histogram':np.bincount((weights>1e-6).sum(1),minlength=5).tolist(),
            'entropy_bits':stats(entropy),'rigid_fraction':float(np.mean(weights.max(1)>.99999))},
        'proportions':{},'hands':{},'deformation':[]}
    face_normal=np.cross(p[t[:,1]]-p[t[:,0]],p[t[:,2]]-p[t[:,0]])
    face_normal/=np.maximum(np.linalg.norm(face_normal,axis=1)[:,None],1e-30)
    normal_angles=np.degrees(np.arccos(np.clip(np.sum(normals[t]*face_normal[:,None,:],axis=2),-1,1)))
    report['normal_to_face_degrees']=stats(normal_angles)
    ue,uf=uv[t[:,1]]-uv[t[:,0]],uv[t[:,2]]-uv[t[:,0]]
    uv_area=np.abs(ue[:,0]*uf[:,1]-ue[:,1]*uf[:,0])/2
    # UV edge basis can be treated as a 3D planar triangle for singular values.
    uv3=np.column_stack((uv,np.zeros(len(uv))))
    us,ul,valid=stretch(p,uv3,t)
    report['uv']={'bounds':[uv.min(0).tolist(),uv.max(0).tolist()],
        'zero_area_triangles':int((uv_area<1e-14).sum()),
        'uv_area_per_m2':stats(uv_area/np.maximum(area,1e-30)),
        'stretch_anisotropy':stats(ul[us>1e-10]/us[us>1e-10])}
    for label in sorted(set(labels)):
        v=labels==label; q=tri_labels==label
        report['rest_regions'][label]={'vertices':int(v.sum()),'triangles':int(q.sum()),
            'area_m2':float(area[q].sum()),'edge_mm':stats(edge_lengths[q]*1000),
            'aspect_equilateral_one':stats(aspect[q]),'sliver_aspect_over_10':int((aspect[q]>10).sum()),
            'normal_opposite_triangles':int((normal_angles[q].max(1)>90).sum()),'normal_face_deg':stats(normal_angles[q]),'uv_zero_area_triangles':int((uv_area[q]<1e-14).sum()),
            'texels_per_m_at_128':stats(128*np.sqrt(uv_area[q]/np.maximum(area[q],1e-30))),
            'weight_entropy':stats(entropy[v]),'rigid_fraction':float(np.mean(weights[v].max(1)>.99999))}
    for c in topo['components']:
        q=components==c['id']; c['dominant_region']=str(np.unique(tri_labels[q],return_counts=True)[0][np.argmax(np.unique(tri_labels[q],return_counts=True)[1])])
        c['boundary_edges']=int(((edge_counts==1)&np.isin(edges[:,0],np.unique(weld[t[q]]))).sum());c['area_m2']=float(area[q].sum());c['uv_zero_area_triangles']=int((uv_area[q]<1e-14).sum())
    base,adapted=arrays['base'],arrays['adapted']; sets=arrays['vertex_face_sets']
    torso=np.isin(sets,[1,18,19])
    for stage,vertices,levels in [('base',base,{'chest':1.28,'waist':1.05,'pelvis':.94}),
                                  ('adapted',adapted,{'chest':1.28*1.08+.008,'waist':1.05*1.08+.008,'pelvis':.94*1.08+.008})]:
        height=float(np.ptp(vertices[:,1])); chin=float(vertices[sets==22,1].min())
        report['proportions'][stage]={'height':height,'bounds':[vertices.min(0).tolist(),vertices.max(0).tolist()],
            'head_height_chin_faceset22_to_crown':float(vertices[:,1].max()-chin),
            'head_body_ratio':float((vertices[:,1].max()-chin)/height),
            'torso_sections':sections(vertices[torso],levels),
            'rest_x_span':float(np.ptp(vertices[:,0]))}
    report['proportions']['shipped_height_with_details']=float(np.ptp(p[:,1]))
    report['proportions']['retarget_segment_lengths']={}
    for chain,pairs in source['retarget_chains'].items():
        a=np.array(pairs); source_scaled=np.linalg.norm(np.diff(a[:,0],axis=0),axis=1); target=np.linalg.norm(np.diff(a[:,1],axis=0),axis=1)
        unscaled=a[:,0].copy();unscaled[:,1]-=.008;unscaled/=[1.02,1.08,.96]
        report['proportions']['retarget_segment_lengths'][chain]={'base':np.linalg.norm(np.diff(unscaled,axis=0),axis=1).tolist(),
            'after_global_scale':source_scaled.tolist(),'target':target.tolist(),'target_over_scaled_base':(target/source_scaled).tolist()}
    print('Measuring hand landmarks and topology...',flush=True)
    for side, digits in source['digit_face_sets'].items():
        hand_id=10 if side=='Left' else 9
        hand=bone['v4'+side+'Hand']; wrist=hand[:3,3]
        palm=adapted[sets==hand_id]
        _u,_s,axes=np.linalg.svd(palm-palm.mean(0),full_matrices=False)
        hand_report={'palm_pca_extents':np.ptp(palm@axes.T,axis=0).tolist(),'palm_basis':axes.tolist(),
            'palm_centroid':palm.mean(0).tolist(),'wrist':wrist.tolist(),'digits':{}}
        for digit, ids in digits.items():
            digit_mask=np.isin(vertex_sets,ids)
            digit_tri=t[np.all(digit_mask[t],axis=1)]
            helper_names=['v4'+side+digit+suffix for suffix in (['','Intermediate','Distal'] if digit=='Thumb' else ['Proximal','Intermediate','Distal'])]
            origins=np.array([bone[n][:3,3] for n in helper_names])
            axis=bone[helper_names[-1]][:3,1];axis/=np.linalg.norm(axis)
            end=p[digit_mask][np.argmax((p[digit_mask]-origins[-1])@axis)]
            tip_len=max(np.linalg.norm(origins[2]-origins[1])*.92,.012)
            tip_proxy=origins[2]+axis*tip_len
            points=adapted[np.isin(sets,ids)];base_points=base[np.isin(sets,ids)]
            d={'base_pca_extent':np.ptp(base_points@np.linalg.svd(base_points-base_points.mean(0),full_matrices=False)[2].T,axis=0).tolist(),
               'adapted_pca_extent':np.ptp(points@np.linalg.svd(points-points.mean(0),full_matrices=False)[2].T,axis=0).tolist(),
               'helper_origins':origins.tolist(),'helper_phalanges':np.linalg.norm(np.diff(origins,axis=0),axis=1).tolist(),
               'distal_axis_to_skin_tip':float((end-origins[-1])@axis),'estimated_tip_length':float(tip_len),
               'estimated_tip_to_skin_mm':float(surface_distance(tip_proxy[None],p[digit_tri])[0]*1000),
               'origin_to_skin_mm':(surface_distance(origins,p[digit_tri])*1000).tolist(),
               'source_fourth_patch_vertices':int((sets==ids[3]).sum()),
               'fourth_patch_axis_range_from_distal':[((adapted[sets==ids[3]]-origins[-1])@axis).min().item(),((adapted[sets==ids[3]]-origins[-1])@axis).max().item()]}
            # Anatomical proxies are shared vertices of source sculpt-face-set
            # boundaries, not a claim that a surface crease is a joint centre.
            ts=arrays['triangle_face_sets']; tr=arrays['triangles']
            boundaries=[]
            for a,b in [(hand_id,ids[0]),(ids[0],ids[1]),(ids[1],ids[2])]:
                shared=np.intersect1d(np.unique(tr[ts==a]),np.unique(tr[ts==b]))
                boundaries.append({'vertices':shared.tolist(),'centre':adapted[shared].mean(0).tolist() if len(shared) else None})
            d['anatomical_boundary_proxies']=boundaries
            d['helper_to_boundary_centre_mm']=[float(np.linalg.norm(o-np.array(v['centre']))*1000) if v['centre'] else None for o,v in zip(origins,boundaries)]
            hand_report['digits'][digit]=d
        report['hands'][side]=hand_report
    # Exact nearest mirror query; do not assume exporter index symmetry.
    mirrored=p.copy();mirrored[:,0]*=-1
    mi,md=Nearest(p).query(mirrored)
    swap=np.array([names.index(n.replace('Left','TEMP').replace('Right','Left').replace('TEMP','Right')) for n in names])
    report['symmetry']={'rest_mirror_distance_mm':stats(md*1000),
        'mirror_normal_angle_degrees':stats(np.degrees(np.arccos(np.clip(np.sum(normals*np.array([-1,1,1])*normals[mi],axis=1),-1,1)))),
        'weight_l1_at_mirror':stats(np.abs(full_weights-full_weights[mi][:,swap]).sum(1))}
    # Source-region dimensions and rig-space metrics: no anthropometric norm.
    report['proportions']['shoulder_bone_breadth']=float(np.linalg.norm(bone['v4LeftUpperArm'][:3,3]-bone['v4RightUpperArm'][:3,3]))
    report['proportions']['bone_segments']={n:float(np.linalg.norm(np.array(v['tail'])-v['head'])) for n,v in source['bones'].items()}
    report['proportions']['shoulder_region_boundary']={}
    boundary=[]
    for face_set in [20,21]:
        tr=arrays['triangles'];fs=arrays['triangle_face_sets']
        boundary.extend(np.intersect1d(np.unique(tr[fs==face_set]),np.unique(tr[(fs!=face_set)&~np.isin(fs,[11,12,9,10])])).tolist())
    for stage,vertices in [('base',base),('adapted',adapted)]:
        report['proportions']['shoulder_region_boundary'][stage]={'vertex_count':len(boundary),'x_breadth':float(np.ptp(vertices[boundary,0]))}
    report['proportions']['extremities']={}
    for side, palm_id in [('Left',10),('Right',9)]:
        hand_ids=[palm_id]+[i for ids in source['digit_face_sets'][side].values() for i in ids]
        vals={}
        for stage,vertices in [('base',base),('adapted',adapted)]:
            hand_points=vertices[np.isin(sets,hand_ids)]
            axes=np.linalg.svd(hand_points-hand_points.mean(0),full_matrices=False)[2]
            hp=hand_points@axes.T
            palm_points=vertices[sets==palm_id]@axes.T
            # Central palm section reduces wrist/webbing contamination of thickness.
            middle=np.median(palm_points[:,0]); slab=palm_points[np.abs(palm_points[:,0]-middle)<.004]
            vals[stage]={'hand_pca_extents':np.ptp(hp,axis=0).tolist(),
                         'palm_pca_extents_in_hand_basis':np.ptp(palm_points,axis=0).tolist(),
                         'central_palm_8mm_slab_width_thickness':np.ptp(slab,axis=0)[1:].tolist()}
            foot_mask=np.isin(sets,[13,26]+list(range(27,46)) if side=='Left' else [14,25]+list(range(46,64)))
            fp=vertices[foot_mask]
            vals[stage]['foot_axis_bounds_width_height_length']=np.ptp(fp,axis=0).tolist()
            foot_axes=np.linalg.svd(fp-fp.mean(0),full_matrices=False)[2]
            vals[stage]['foot_pca_length_width_height']=np.ptp(fp@foot_axes.T,axis=0).tolist()
        report['proportions']['extremities'][side]=vals
    report['anatomical_source_regions']={}
    base_area,_,_=triangle_metrics(base,arrays['triangles'])
    bs,bl,bvalid=stretch(base,adapted,arrays['triangles'])
    for ids,label in [([20,21],'upper_arm'),([11,12],'forearm'),([9,10],'palm'),(list(range(64,104)),'digits'),([23,24],'thigh'),([15,16],'shin')]:
        q=np.isin(vertex_sets[t],ids).sum(1)>=2
        report['anatomical_source_regions'][label]={'triangles':int(q.sum()),'area_m2':float(area[q].sum()),'edge_mm':stats(edge_lengths[q]*1000)}
        bq=np.isin(arrays['triangle_face_sets'],ids)&bvalid
        report['anatomical_source_regions'][label]['base_to_adapted']={'area_ratio':stats((bs*bl)[bq]),'max_stretch':stats(bl[bq]),'rest_area_fraction_stretched_over_two':float(base_area[bq & (bl>2)].sum()/base_area[bq].sum())}
    # Edges use export indices to preserve seam-normal disagreement.
    raw_edges=np.unique(np.sort(np.concatenate([t[:,[0,1]],t[:,[1,2]],t[:,[2,0]]]),axis=1),axis=0)
    jump=np.abs(full_weights[raw_edges[:,0]]-full_weights[raw_edges[:,1]]).sum(1)
    report['weights']['adjacent_edge_l1']=stats(jump)
    report['weights']['edges_l1_over_one']=int((jump>1).sum())
    wrong_side=np.array([('Right' in n) for n in names])[None,:]&(p[:,0]<-.05)[:,None] | (np.array([('Left' in n) for n in names])[None,:]&(p[:,0]>.05)[:,None])
    report['weights']['contralateral_weight_over_001_vertices']=int(((full_weights*wrong_side).sum(1)>.01).sum())
    report['weights']['contralateral_vertices']=[{'position':p[i].tolist(),'weights':{n:float(w) for n,w in zip(names,full_weights[i]) if w>.001}} for i in np.flatnonzero((full_weights*wrong_side).sum(1)>.01)]
    report['bones']=[]
    for n,v in source['bones'].items():
        origin=bone[n][:3,3];i=names.index(n); influenced=full_weights[:,i]>.01
        report['bones'].append({'name':n,'kind':'semantic' if n in source['semantic_names'] else 'helper',
            'origin':origin.tolist(),'weighted_vertices':int(influenced.sum()),
            'influenced_vertex_distance_m':stats(np.linalg.norm(p[influenced]-origin,axis=1)),
            'nearest_skin_mm':float(surface_distance(origin[None],p[t])[0]*1000)})
    first=np.unique(weld,return_index=True)[1]
    normal_dot=np.sum(normals*normals[first[weld]],axis=1)
    report['split_normals_degrees']=stats(np.degrees(np.arccos(np.clip(normal_dot,-1,1))))
    report['normal_opposite_face']={'triangles':int((normal_angles.max(1)>90).sum()),'area_m2':float(area[normal_angles.max(1)>90].sum())}
    report['canonical_detail_mismatch_by_component']=[{'id':c['id'],'triangles':c['triangles'],
        'max_mm':float(match_distance[np.unique(t[components==c['id']])].max()*1000)} for c in topo['components']]
    frames=json.loads(gzip.decompress((ROOT/'docs/evidence/blender05/final/frames.json.gz').read_bytes()))
    for record in frames:
        if not record['name'].endswith('-chase'): continue
        world,_,_,_,_=skin(glb,record)
        posed=transform(world,np.linalg.inv(matrix(record['rig'])))
        small,large,valid=stretch(p,posed,t)
        result={'name':record['name'],'regions':{}}
        for label in sorted(set(tri_labels)):
            q=(tri_labels==label)&valid
            result['regions'][label]={'area_ratio':stats((small*large)[q]),'min_stretch':stats(small[q]),'max_stretch':stats(large[q]),
                'rest_area_fraction_compressed_below_half':float(area[q & (small*large<.5)].sum()/area[q].sum()),
                'rest_area_fraction_stretched_over_two':float(area[q & (large>2)].sum()/area[q].sum())}
        report['deformation'].append(result)
    output.mkdir(parents=True,exist_ok=True)
    (output/'athlete-metrics.json').write_text(json.dumps(report,indent=2,allow_nan=False)+'\n')
    np.savez_compressed(output/'audit-arrays.npz',positions=p,triangles=t,source_match=match,source_body=body,
                        vertex_face_sets=vertex_sets,tri_labels=tri_labels,vertex_labels=labels,weights=full_weights)
    print('Wrote athlete-metrics.json',flush=True)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--base',type=Path,required=True)
    p.add_argument('--output',type=Path,required=True)
    args=p.parse_args()
    analyse(args.base,args.output)


if __name__=='__main__': main()
