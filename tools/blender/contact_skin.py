#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Measure V4 skin using a palette captured from the native Qt frame.

Requires numpy and Pillow. No Blender, retargeting, or pose reconstruction:
GLB POSITION/JOINTS_0/WEIGHTS_0/inverseBindMatrices and Qt's final joint world
matrices drive linear blend skinning. Geometry is evaluated in rig metres.
"""
import argparse
import hashlib
import gzip
import json
from pathlib import Path
import struct

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]


class Glb:
    def __init__(self, path):
        data = path.read_bytes()
        size = struct.unpack_from('<I', data, 12)[0]
        self.json = json.loads(data[20:20+size])
        self.bin = data[28+size:]
        self.sha256 = hashlib.sha256(data).hexdigest()

    def accessor(self, index):
        a = self.json['accessors'][index]
        v = self.json['bufferViews'][a['bufferView']]
        dtype = {5126:'<f4', 5125:'<u4', 5123:'<u2', 5121:'u1'}[a['componentType']]
        count = {'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[a['type']]
        stride = v.get('byteStride', np.dtype(dtype).itemsize*count)
        return np.ndarray((a['count'],count),dtype=dtype,buffer=self.bin,
                          offset=v.get('byteOffset',0)+a.get('byteOffset',0),
                          strides=(stride,np.dtype(dtype).itemsize)).copy()


def matrix(basis):
    b = np.asarray(basis)
    m = np.eye(4)
    m[:3,3] = b[0]
    m[:3,:3] = (b[1:]-b[0]).T
    return m


def transform(points, m):
    return points @ m[:3,:3].T + m[:3,3]


def project(points, record):
    p = transform(points, np.linalg.inv(matrix(record['camera'])))
    w,h = record['viewport']
    f = h/(2*np.tan(np.deg2rad(record['fov'])/2))
    return np.column_stack((w/2-f*p[:,0]/p[:,2], h/2+f*p[:,1]/p[:,2])), -p[:,2]


def skin(glb, record):
    j = glb.json
    primitive = j['meshes'][0]['primitives'][0]
    a = primitive['attributes']
    positions = glb.accessor(a['POSITION']).astype(float)
    joints = glb.accessor(a['JOINTS_0']).astype(int)
    weights = glb.accessor(a['WEIGHTS_0']).astype(float)
    names = [j['nodes'][i]['name'] for i in j['skins'][0]['joints']]
    inverse = glb.accessor(j['skins'][0]['inverseBindMatrices']).reshape(-1,4,4).transpose(0,2,1)
    palette = np.array([matrix(record['nodes'][name]) for name in names]) @ inverse
    hom = np.column_stack((positions, np.ones(len(positions))))
    posed = np.zeros((len(positions),4))
    for k in range(4):
        posed += weights[:,k,None] * np.einsum('nij,nj->ni',palette[joints[:,k]],hom)
    triangles = glb.accessor(primitive['indices']).reshape(-1,3)
    # Dominant region is assigned from aggregate skin influence, not proximity
    # to the grip. This must not cherry-pick the one fingertip that touches.
    labels = []
    for name in names:
        side = next((s for s in ['Left','Right'] if name.startswith('v4'+s)), '')
        digit = next((d for d in ['Index','Middle','Ring','Pinky','Thumb'] if d in name), '')
        labels.append(side+digit if digit else side+'Palm' if name.endswith(('Hand','Fingers')) else 'other')
    groups = {}
    for label in sorted(set(labels)-{'other'}):
        group = np.array([x==label for x in labels])
        groups[label] = np.sum(weights*group[joints],axis=1)
    label_order = list(groups)
    scores = np.array(list(groups.values()))
    winner = np.argmax(scores,axis=0)
    masks = {label:(winner==i)&(scores[i]>=0.5) for i,label in enumerate(label_order)}
    return posed[:,:3], triangles, masks, positions, weights


def cylinder_sdf(points, origin, axis, radius, half_length):
    rel = points-origin
    along = rel@axis
    radial = np.linalg.norm(rel-along[:,None]*axis,axis=1)
    d = np.column_stack((radial-radius,np.abs(along)-half_length))
    return np.minimum(np.maximum(d[:,0],d[:,1]),0)+np.linalg.norm(np.maximum(d,0),axis=1), radial, along


def equipment(record, side):
    inv_rig = np.linalg.inv(matrix(record['rig']))
    sport = record['sport']
    if sport == 0:
        nodes = record['mirrored'] if side=='Left' else record['equipment']
        m = inv_rig @ matrix(nodes['equipment:row:oar-rig:grip'])
        centre = transform(np.array([[-0.66,-0.04,0]]),m)[0]
        axis = m[:3,0]/np.linalg.norm(m[:3,0])
        return centre,axis,0.023,0.16
    if sport == 1:
        m=inv_rig@matrix(record['poleGrips'][side.lower()])
        bounds=np.array([[-.813014,-.9084468,-.5],[.813014,.82844675,.82]])
        centre=transform(bounds.mean(axis=0)[None],m)[0]
        # Which authored component was stretched to the 150 mm grip length?
        lengths=np.linalg.norm(m[:3,:3],axis=0)*(bounds[1]-bounds[0])
        long=int(np.argmax(lengths));axis=m[:3,long]/np.linalg.norm(m[:3,long])
        return centre,axis,.016,.075
    # Web handlebar anchor and hood axis. The analytic cylinder is a contact
    # proxy; actual V3 hood triangles are measured separately.
    return np.array([-.2 if side=='Left' else .2,.84,.51]), np.array([0,np.sin(.24),np.cos(.24)]),.018,.0525


def mesh_surface(glb, record, side):
    sport=record['sport']
    slot=['equipment:row:oar-rig:grip','equipment:ski:pole-grip','equipment:bike:frame-assembly:brake-hoods'][sport]
    node=next(n for n in glb.json['nodes'] if n.get('name')==slot)
    primitive=glb.json['meshes'][node['mesh']]['primitives'][0]
    vertices=glb.accessor(primitive['attributes']['POSITION']).astype(float)
    triangles=glb.accessor(primitive['indices']).reshape(-1,3)
    if sport==1:
        basis=record['poleGrips'][side.lower()]
    else:
        nodes=record['mirrored'] if sport==0 and side=='Left' else record['equipment']
        basis=nodes[slot]
    m=np.linalg.inv(matrix(record['rig']))@matrix(basis)
    vertices=transform(vertices,m)
    if sport==2:
        mask=vertices[:,0]<0 if side=='Left' else vertices[:,0]>0
        triangles=triangles[np.all(mask[triangles],axis=1)]
    return vertices[triangles]


def paired_distance_squared(points, triangles):
    a,b,c=triangles.transpose(1,0,2)
    ab=b-a;ac=c-a;ap=points-a
    normal=np.cross(ab,ac);nn=np.sum(normal*normal,axis=1)
    height=np.sum(ap*normal,axis=1)
    d00=np.sum(ab*ab,axis=1);d01=np.sum(ab*ac,axis=1);d11=np.sum(ac*ac,axis=1)
    d20=np.sum(ap*ab,axis=1);d21=np.sum(ap*ac,axis=1)
    denom=d00*d11-d01*d01
    safe=np.maximum(denom,1e-30)
    u=(d11*d20-d01*d21)/safe;v=(d00*d21-d01*d20)/safe
    best=np.where((denom>1e-24)&(u>=0)&(v>=0)&(u+v<=1),height*height/np.maximum(nn,1e-30),np.inf)
    for start,end in [(a,b),(b,c),(c,a)]:
        edge=end-start;rel=points-start
        t=np.clip(np.sum(rel*edge,axis=1)/np.maximum(np.sum(edge*edge,axis=1),1e-30),0,1)
        delta=rel-t[:,None]*edge
        best=np.minimum(best,np.sum(delta*delta,axis=1))
    return best


def surface_distance(points, triangles):
    """Exact point/triangle distances, pruning by conservative AABB bounds.

    A nearest-centroid triangle gives an upper bound. Every triangle whose
    bounding box could beat it is evaluated, including triangle interiors.
    No equipment vertex-only approximation or fixed nearest-k shortcut.
    """
    lo=triangles.min(axis=1);hi=triangles.max(axis=1);centres=triangles.mean(axis=1)
    result=[]
    for chunk in np.array_split(points,max(1,(len(points)+255)//256)):
        ds=np.sum((chunk[:,None,:]-centres)**2,axis=2)
        initial=np.argmin(ds,axis=1)
        best=paired_distance_squared(chunk,triangles[initial])
        delta=np.maximum(np.maximum(lo-chunk[:,None,:],chunk[:,None,:]-hi),0)
        lower=np.sum(delta*delta,axis=2)
        pi,ti=np.where(lower<=best[:,None]+1e-20)
        distance=paired_distance_squared(chunk[pi],triangles[ti])
        np.minimum.at(best,pi,distance)
        result.extend(np.sqrt(best))
    return np.array(result)


def sampled_surface(vertices, triangles, mask, spacing=.001):
    tri = triangles[np.all(mask[triangles],axis=1)]
    if not len(tri): raise ValueError('empty hand region')
    edges=np.array([np.linalg.norm(vertices[tri[:,i]]-vertices[tri[:,(i+1)%3]],axis=1) for i in range(3)])
    # Every subtriangle edge <= 1 mm. Distance is 1-Lipschitz, so the
    # sampled minimum bounds the true surface minimum within 1 mm.
    divisions=np.maximum(1,np.ceil(edges.max(axis=0)/spacing).astype(int))
    batches=[]
    for n in np.unique(divisions):
        bary=np.array([(i/n,k/n,1-(i+k)/n) for i in range(n+1) for k in range(n+1-i)])
        batches.append(np.einsum('ki,tij->tkj',bary,vertices[tri[divisions==n]]).reshape(-1,3))
    return np.concatenate(batches),edges.ravel(),tri


def calibration(vertices, triangles, masks, hand, palm, axis, side):
    sign=-1 if side=='Left' else 1
    def angle(local):
        direction=hand[:3,:3]@np.array(local)
        direction/=np.linalg.norm(direction)
        return float(np.degrees(np.arccos(np.clip(abs(direction@axis),0,1))))
    palm_tri=triangles[np.all(masks[side+'Palm'][triangles],axis=1)]
    return {'channel_axis_error_deg':angle([-.61,.16*sign,.77*sign]),
            'palm_normal_axis_angle_deg':angle([-.1052*sign,-.8513,.514]),
            'palm_contact_to_all_skin_mm':float(surface_distance(palm[None],vertices[triangles])[0]*1000),
            'palm_contact_to_palm_region_mm':float(surface_distance(palm[None],vertices[palm_tri])[0]*1000)}


def analyse(glb,rig_glb,record,output):
    world,triangles,masks,rest,weights = skin(glb,record)
    inv_rig=np.linalg.inv(matrix(record['rig']))
    vertices=transform(world,inv_rig)
    result={'name':record['name'],'sport':record['sport'],'contacts':record['contacts'],
            'equipment_sha256':rig_glb.sha256,
            'hud_elapsed':record['frame'][4], 'hud_distance':record['frame'][1], 'hands':{}}
    for side in ['Left','Right']:
        origin,axis,radius,half=equipment(record,side)
        surface=mesh_surface(rig_glb,record,side)
        hand=inv_rig@matrix(record['nodes']['v4'+side+'Hand'])
        palm=transform(np.array([[(-1 if side=='Left' else 1)*.08,-.01,.035]]),hand)[0]
        pd=cylinder_sdf(palm[None],origin,axis,radius,half)
        report={'radius_mm':radius*1000,'equipment_centre':origin.tolist(),'equipment_axis':axis.tolist(),
                'palm_helper_proxy_mm':float(pd[0][0]*1000),
                'palm_helper_actual_surface_mm':float(surface_distance(palm[None],surface)[0]*1000),
                'regions':{}}
        report.update(calibration(vertices,triangles,masks,hand,palm,axis,side))
        for digit in ['Palm','Index','Middle','Ring','Pinky','Thumb']:
            points,edges,tris=sampled_surface(vertices,triangles,masks[side+digit])
            signed,radial,axial=cylinder_sdf(points,origin,axis,radius,half)
            actual=surface_distance(points,surface)
            report['regions'][digit]={'vertices':int(masks[side+digit].sum()),'triangles':len(tris),
                'actual_min_mm':float(actual.min()*1000),
                'actual_p05_mm':float(np.quantile(actual,.05)*1000),
                'actual_fraction_within_2mm':float(np.mean(actual<=.002)),
                'sample_cover_bound_mm':1.0,
                'min_signed_mm':float(signed.min()*1000),'min_absolute_mm':float(np.abs(signed).min()*1000),
                'p05_signed_mm':float(np.quantile(signed,.05)*1000),'median_signed_mm':float(np.median(signed)*1000),
                'penetration_fraction':float(np.mean(signed<0)),
                'edge_median_mm':float(np.median(edges)*1000),'edge_max_mm':float(edges.max()*1000)}
            if digit != 'Palm':
                intermediate='v4'+side+digit+'Intermediate'
                distal='v4'+side+digit+'Distal'
                im=inv_rig@matrix(record['nodes'][intermediate]);dm=inv_rig@matrix(record['nodes'][distal])
                # Same explicit tip-length rule used by collect_hand_chains.
                # The independent measured quantity is distance to actual skin.
                tip_length=max(np.linalg.norm(dm[:3,3]-im[:3,3])*.92,.012)
                helper=np.array([im[:3,3],dm[:3,3],transform(np.array([[0,tip_length,0]]),dm)[0]])
                report['regions'][digit]['helper_to_own_skin_mm']=(surface_distance(helper,vertices[tris])*1000).tolist()
        result['hands'][side]=report
    # Project exact deformed triangles as a wire overlay on the same native view.
    im=native_view(record,output)
    if im is not None:
        draw=ImageDraw.Draw(im)
        xy,depth=project(world,record)
        scale=np.array(im.size)/record['viewport']; xy*=scale
        selected=np.logical_or.reduce(list(masks.values()))
        for tri in triangles[np.all(selected[triangles],axis=1)]:
            if np.all(depth[tri]>0): draw.line([tuple(xy[i]) for i in [*tri,tri[0]]],fill=(0,255,255,120),width=1)
        im.save(output/(record['name']+'-skin-overlay.png'))
    return result



def native_view(record, output):
    """Crop the held root grab, never race it with another async View3D grab."""
    root_path=output/(record['name']+'.png')
    if root_path.exists():
        im=Image.open(root_path).convert('RGBA')
        w,h=record['viewport']
        # The first audit's full-screen replay had only its 57 px top bar.
        x,y=record.get('viewportOrigin',[0,im.height-h])
        return im.crop((round(x),round(y),round(x+w),round(y+h)))
    return None


def validate_mask(glb, record, output):
    world, triangles, masks, _, _ = skin(glb,record)
    native=native_view(record,output)
    if native is not None:
        image=np.asarray(native)
        actual=(image[:,:,0]>150)&(image[:,:,2]>150)&(image[:,:,1]<80)
        alpha_min=int(image[:,:,3].min())
    else:
        # The durable evidence stores lossless native segmentation masks;
        # their original PNG hashes are recorded in the capture manifest.
        actual=np.asarray(Image.open(output/(record['name']+'-silhouette.png')).convert('1'))
        image=np.zeros((*actual.shape,4),dtype=np.uint8)
        alpha_min=255
    xy,depth=project(world,record)
    xy *= np.array([image.shape[1],image.shape[0]])/record['viewport']
    predicted=Image.new('1',(image.shape[1],image.shape[0]))
    draw=ImageDraw.Draw(predicted)
    for t in triangles[np.all(depth[triangles]>0.1,axis=1)]:
        if np.isfinite(xy[t]).all(): draw.polygon([tuple(v) for v in xy[t]],fill=1)
    expected=np.asarray(predicted)
    side='Left' if '-left-' in record['name'] else 'Right'
    hand=np.logical_or.reduce([mask for label,mask in masks.items() if label.startswith(side)])
    bounds=xy[hand & (depth>.1)]
    lo=np.maximum(np.floor(bounds.min(0)-5).astype(int),[0,0])
    hi=np.minimum(np.ceil(bounds.max(0)+5).astype(int),[image.shape[1],image.shape[0]])
    roi=(slice(lo[1],hi[1]),slice(lo[0],hi[0]))
    a,b=actual[roi],expected[roi]
    iou=float(np.sum(a&b)/np.sum(a|b))
    return dict(name=record['name'],hand_roi=[*lo.tolist(),*hi.tolist()],
                native_pixels=int(a.sum()),cpu_pixels=int(b.sum()),iou=iou,
                alpha_min=alpha_min)

def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('capture',type=Path)
    p.add_argument('--validate-only',action='store_true',help='check native silhouettes without distance sampling')
    args=p.parse_args()
    glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    rig_glb=Glb(ROOT/'assets/replay/rowplay-rigs-v3.glb')
    row_glb=Glb(ROOT/'assets/replay/authored/rowing-shell.glb')
    frame_path=args.capture/'frames.json'
    raw=frame_path.read_bytes() if frame_path.exists() else gzip.decompress((args.capture/'frames.json.gz').read_bytes())
    records=json.loads(raw)
    validation=[validate_mask(glb,r,args.capture) for r in records if r["name"].endswith("-mask")]
    if not validation:
        raise ValueError('no native masks; capture with --masks before interpreting skin')
    if min(v['iou'] for v in validation) < .97:
        raise ValueError('native/CPU skin silhouettes disagree; do not interpret contact measurements')
    (args.capture/"skin-validation.json").write_text(json.dumps(validation,indent=2)+"\n")
    if args.validate_only:
        print(f'{len(validation)} native masks validated; minimum IoU {min(v["iou"] for v in validation):.6f}')
        return
    results=[]
    for r in records:
        if not r['name'].endswith('-chase'): continue
        result=analyse(glb,row_glb if r['sport']==0 else rig_glb,r,args.capture)
        results.append(result)
        (args.capture/(r['name']+'-metrics.json')).write_text(json.dumps(result,indent=2)+'\n')
        print(r['name'],flush=True)
    (args.capture/'skin.json').write_text(json.dumps({'asset_sha256':glb.sha256,'row_equipment_sha256':row_glb.sha256,'v3_equipment_sha256':rig_glb.sha256,'samples':results},indent=2)+'\n')


if __name__=='__main__': main()
