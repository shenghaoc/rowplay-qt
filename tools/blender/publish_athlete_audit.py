#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Validate and publish bounded Phase 5.2 evidence from scratch captures.

Requires numpy/Pillow. Copies lossless masks and compressed frame records;
JPEG panels are labelled review aids, never pixel/geometry measurement inputs.
"""
import argparse
import gzip
import hashlib
import json
from pathlib import Path
import shutil
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from contact_skin import Glb, ROOT, skin, project, native_view, matrix, transform

FONT='/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'

def font(size=18): return ImageFont.truetype(FONT,size)
def save_json(path,value): path.write_text(json.dumps(value,indent=2,allow_nan=False)+'\n')
def compressed(path,value): path.write_bytes(gzip.compress(json.dumps(value,separators=(',',':'),allow_nan=False).encode(),mtime=0))

def validate_frames(records,experiments):
    frames={r['name']:r for r in records}; applied={r['name']:r for r in experiments}
    checked=[]
    for name,a in frames.items():
        if not name.endswith('-A') or '-chase-' in name: continue
        prefix=name[:-1]
        if prefix+'B' not in frames: continue
        for variant in ['B','C','D','L','A0']:
            if prefix+variant not in frames: continue
            b=frames[prefix+variant]
            for key in ['frame','nodes','equipment','mirrored','poleGrips','rig','camera','fov','viewport','grip','contacts','tier','sport']:
                if a[key]!=b[key]: raise ValueError(f'{prefix}{variant}: changed {key}')
            for key in ['probe','probeExposure']+([] if variant=='L' else ['keyBrightness']):
                if applied[name][key]!=applied[prefix+variant][key]: raise ValueError(f'changed lighting {key}')
        checked.append(prefix)
    if len(checked)!=10: raise ValueError(f'expected 10 controlled views, found {len(checked)}')
    return checked


def validate_pixels(glb,records,capture,out):
    results=[]
    for r in records:
        if not r['name'].endswith('-mask'): continue
        native=native_view(r,capture);rgba=np.asarray(native)
        actual=(rgba[:,:,0]>150)&(rgba[:,:,2]>150)&(rgba[:,:,1]<80)
        world,tri,*_=skin(glb,r);xy,depth=project(world,r)
        canvas=Image.new('L',(native.width*4,native.height*4));draw=ImageDraw.Draw(canvas)
        for t in tri[np.all(depth[tri]>.1,axis=1)]: draw.polygon([tuple(v*4) for v in xy[t]],fill=255)
        expected=np.asarray(canvas.resize(native.size,Image.Resampling.BOX))>=128
        iou=float((actual&expected).sum()/(actual|expected).sum())
        if iou<.97: raise ValueError(f'{r["name"]}: silhouette IoU {iou}')
        Image.fromarray(actual).save(out/(r['name']+'-silhouette.png'))
        prefix=r['name'][:-4]
        variants={v:np.asarray(native_view(dict(r,name=prefix+v),capture))[:,:,:3].astype(float) for v in ['A','B','C','D','L']}
        # Erode one pixel so anti-aliased boundary/background pixels do not dominate.
        roi=actual.copy()
        for axis in [0,1]: roi &= np.roll(actual,1,axis)&np.roll(actual,-1,axis)
        differences={a+'-'+b:float(np.abs(variants[a]-variants[b])[roi].mean()) for a,b in [('A','B'),('A','C'),('C','D'),('C','L')]}
        for v,img in variants.items():
            if (img[roi].std()<1 or np.mean(np.all(img[roi]>250,axis=1))>.95): raise ValueError(f'blank/saturated {prefix}{v}')
        results.append({'name':r['name'],'iou':iou,'pixels':int(actual.sum()),'mean_abs_rgb_8bit':differences})
    return results


def panel(capture,names,path,columns=3,width=400):
    rows=(len(names)+columns-1)//columns
    result=Image.new('RGB',(columns*width,rows*(round(width*2/3)+30)),'white');draw=ImageDraw.Draw(result)
    for i,name in enumerate(names):
        image=Image.open(capture/(name+'.png')).convert('RGB');image.thumbnail((width,round(width*2/3)))
        x=(i%columns)*width;y=(i//columns)*(round(width*2/3)+30)
        draw.text((x+8,y+5),name,font=font(15),fill='black');result.paste(image,(x,y+30))
    result.save(path,quality=90,subsampling=0)


def scientific_figures(base_dir,metrics_dir,out,records,capture):
    a=np.load(base_dir/'base-stages.npz');m=json.loads((metrics_dir/'athlete-metrics.json').read_text())
    g=np.load(metrics_dir/'audit-arrays.npz');glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    # Same metre scale, orthographic geometry diagrams (not acceptance renders).
    result=Image.new('RGB',(1300,900),'white');draw=ImageDraw.Draw(result)
    for title,p,t,cx in [('Reviewed base',a['base'],a['triangles'],300),('Adapted body',a['adapted'],a['adapted_triangles'],920)]:
        xy=np.column_stack([cx+p[:,0]*330,820-p[:,1]*330])
        # Same horizontal and vertical scale is essential for proportions.
        xy[:,0]=cx+p[:,0]*330
        n=np.cross(p[t[:,1]]-p[t[:,0]],p[t[:,2]]-p[t[:,0]])
        n/=np.maximum(np.linalg.norm(n,axis=1)[:,None],1e-20)
        shade=np.clip(.55+.40*n[:,2],.2,.95)
        for i in np.argsort(p[t,2].mean(1)):
            color=int(shade[i]*230);draw.polygon([tuple(v) for v in xy[t[i]]],fill=(color,color,color))
        draw.text((cx-130,25),title,font=font(24),fill='black')
    draw.text((30,850),'Orthographic X/Y; common 330 px/m; grey surface = body only. Arms change pose as well as length.',font=font(18),fill='black')
    result.save(out/'source-body.png')
    # Bilateral source/adapted hand geometry at one common scale. Source
    # face-set boundaries and dorsal terminal patches remain visible.
    result=Image.new('RGB',(1600,650),'white');draw=ImageDraw.Draw(result)
    for col,(side,stage) in enumerate([(s,v) for s in ['Left','Right'] for v in ['base','adapted']]):
        ids=[10 if side=='Left' else 9]+[i for v in m['source']['digit_face_sets'][side].values() for i in v]
        selected=np.isin(a['vertex_face_sets'],ids);points=a[stage];center=points[selected].mean(0)
        axes=np.linalg.svd(points[selected]-center,full_matrices=False)[2]
        q=np.column_stack([col*400+200+(points-center)@axes[1]*1900,330-(points-center)@axes[0]*1900])
        tr=a['triangles'] if stage=='base' else a['adapted_triangles'];tr=tr[np.all(selected[tr],axis=1)]
        for t in tr: draw.polygon([tuple(v) for v in q[t]],fill='#e6e6e6',outline='#aaa')
        draw.text((col*400+20,20),side+' '+stage,font=font(22),fill='black')
    draw.text((20,610),'Common 1.9 px/mm; independent PCA projection per hand/stage. Spread-hand geometry, not a clinical length measure.',font=font(18),fill='black')
    result.save(out/'hand-proportions.png')
    for side in ['Left','Right']:
        hand=m['hands'][side];ids=[10 if side=='Left' else 9]+[i for v in m['source']['digit_face_sets'][side].values() for i in v]
        selected=np.isin(a['vertex_face_sets'],ids);p=a['adapted'];axes=np.linalg.svd(p[selected]-p[selected].mean(0),full_matrices=False)[2]
        centre=p[selected].mean(0)
        def xy(points): return np.column_stack([500+(points-centre)@axes[0]*3600,440-(points-centre)@axes[1]*3600])
        result=Image.new('RGB',(1000,900),'white');draw=ImageDraw.Draw(result)
        tr=a['adapted_triangles'];tr=tr[np.all(selected[tr],axis=1)];points=xy(p)
        for t in tr: draw.polygon([tuple(v) for v in points[t]],fill='#e6e6e6',outline='#b8b8b8')
        draw.text((20,15),side+' rest hand: red helpers; blue boundaries; green estimated tips',font=font(20),fill='black')
        for digit,d in hand['digits'].items():
            origins=xy(np.array(d['helper_origins']));draw.line([tuple(v) for v in origins],fill='#c62828',width=3)
            for v in origins: draw.ellipse((v[0]-4,v[1]-4,v[0]+4,v[1]+4),fill='#c62828')
            for b in d['anatomical_boundary_proxies']:
                for v in xy(p[b['vertices']]): draw.ellipse((v[0]-2,v[1]-2,v[0]+2,v[1]+2),fill='#1565c0')
            name='v4'+side+digit+'Distal'
            b=m['source']['bones'][name];axis=np.array(b['tail'])-b['head'];axis/=np.linalg.norm(axis)
            tip=xy((np.array(d['helper_origins'][-1])+axis*d['estimated_tip_length'])[None])[0]
            draw.line([tuple(origins[-1]),tuple(tip)],fill='#228b22',width=3)
            draw.ellipse((tip[0]-4,tip[1]-4,tip[0]+4,tip[1]+4),fill='#228b22')
            v=origins[-1];draw.text((v[0]+6,v[1]),digit,font=font(14),fill='black')
        draw.text((20,850),'PCA plane, 3.6 px/mm. Boundaries are sculpt-region proxies, not measured internal joint centres.',font=font(16),fill='black')
        result.save(out/(side.lower()+'-rest-hand.png'))
        # Show exact helper world origins and deformed anatomical boundary vertices
        # over a native Qt hand image, without changing that image's geometry.
        for phase in ['row-catch','ski-release']:
            r=next(r for r in records if r['name']==phase+'-'+side.lower()+'-A')
            native=native_view(r,capture).convert('RGB');draw=ImageDraw.Draw(native)
            world,*_=skin(glb,r)
            for digit,d in hand['digits'].items():
                suffixes=['','Intermediate','Distal'] if digit=='Thumb' else ['Proximal','Intermediate','Distal']
                origins=np.array([matrix(r['nodes']['v4'+side+digit+s])[:3,3] for s in suffixes]);coords,_=project(origins,r)
                draw.line([tuple(v) for v in coords],fill='#ff3030',width=3)
                for v in coords: draw.ellipse((v[0]-4,v[1]-4,v[0]+4,v[1]+4),fill='#ff3030')
                for b in d['anatomical_boundary_proxies']:
                    mask=g['source_body']&np.isin(g['source_match'],b['vertices']);q,_=project(world[mask],r)
                    for v in q: draw.ellipse((v[0]-2,v[1]-2,v[0]+2,v[1]+2),fill='#00dfff')
                distal=matrix(r['nodes']['v4'+side+digit+'Distal']);axis=distal[:3,1]/np.linalg.norm(distal[:3,1])
                tip,_=project((distal[:3,3]+axis*d['estimated_tip_length'])[None],r);tip=tip[0]
                draw.line([tuple(coords[-1]),tuple(tip)],fill='#3cff50',width=3)
                draw.ellipse((tip[0]-4,tip[1]-4,tip[0]+4,tip[1]+4),fill='#3cff50')
            draw.rectangle((0,0,native.width,30),fill='white');draw.text((8,5),'Red helpers; green estimated tips; cyan deformed source boundaries. '+r['name'],font=font(16),fill='black')
            native.save(out/(phase+'-'+side.lower()+'-anatomy.jpg'),quality=92,subsampling=0)


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--capture',type=Path,required=True);p.add_argument('--base',type=Path,required=True)
    p.add_argument('--metrics',type=Path,required=True);p.add_argument('--lower',type=Path,required=True);p.add_argument('--output',type=Path,required=True)
    args=p.parse_args();out=args.output;out.mkdir(parents=True,exist_ok=True)
    records=json.loads((args.capture/'frames.json').read_text());experiments=json.loads((args.capture/'experiment.json').read_text())
    controlled=validate_frames(records,experiments)
    glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    validation=validate_pixels(glb,records,args.capture,out)
    zero_checks=[]
    for phase in ['row-catch','ski-release']:
        a=np.asarray(Image.open(args.capture/(phase+'-face-A.png')));b=np.asarray(Image.open(args.capture/(phase+'-face-A0.png')))
        difference=int(np.max(np.abs(a.astype(int)-b.astype(int))))
        if difference: raise ValueError('A/A0 changed pixels despite absent production normal map')
        zero_checks.append({'phase':phase,'max_channel_difference':difference})
        for view in ['full','face','torso','left','right']:
            panel(args.capture,[phase+'-'+view+'-'+v for v in ['A','B','C','D','L']],out/(phase+'-'+view+'.jpg'))
    phases=['row-catch','row-middrive','row-finish','row-halfslide','ski-approach','ski-pull','ski-release','ski-recovery','bike-top','bike-quarter']
    panel(args.capture,[phase+'-'+v+'-A' for phase in phases for v in ['chase','full','torso']],out/'replay-poses.jpg',width=500)
    panel(args.capture,[phase+'-lower-A' for phase in phases if phase not in ['row-catch','ski-release']],out/'lower-body.jpg',columns=2,width=600)
    lower=json.loads((args.lower/'frames.json').read_text())
    if len(lower)!=10: raise ValueError('expected ten isolated lower-body views')
    for r in lower:
        previous=next(x for x in records if x['name']==r['name'].replace('-lower-isolated','-chase-A'))
        for key in ['nodes','rig','sport','tier','grip','contacts']:
            if r[key]!=previous[key]: raise ValueError('isolated anatomy changed '+key)
        # contactDump walks children even when their rig ancestor is hidden;
        # leaf `visible` is not effective scene visibility. Check recorded
        # transforms, not an empty dictionary. The supplemental capture hides
        # the rig parents explicitly and its native images show that isolation.
        for key in ['equipment','mirrored']:
            if any(previous[key].get(n)!=v for n,v in r[key].items()):
                raise ValueError('isolated anatomy moved equipment')
    panel(args.lower,[phase+'-lower-isolated' for phase in phases],out/'lower-isolated.jpg',columns=2,width=600)
    compressed(out/'lower-frames.json.gz',lower)
    shutil.copyfile(args.lower/'manifest.json',out/'lower-manifest.json')
    scientific_figures(args.base,args.metrics,out,records,args.capture)
    compressed(out/'frames.json.gz',records);compressed(out/'experiment.json.gz',experiments)
    compressed(out/'athlete-metrics.json.gz',json.loads((args.metrics/'athlete-metrics.json').read_text()))
    for name in ['normal.png','roughness.png','manifest.json']: shutil.copyfile(args.capture/name,out/name)
    save_json(out/'validation.json',{'controlled_views':controlled,'silhouettes':validation,'normal_disabled':zero_checks})
    inputs={}
    for directory,paths in [(args.capture,list(args.capture.glob('*.png'))+list(args.capture.glob('*.json'))),(args.lower,list(args.lower.glob('*.png'))+list(args.lower.glob('*.json'))),(args.base,list(args.base.glob('*'))),(args.metrics,list(args.metrics.glob('*')))]:
        for path in paths: inputs[str(path.relative_to(ROOT) if path.is_absolute() else path)]=hashlib.sha256(path.read_bytes()).hexdigest()
    for name in ['audit_athlete.py','audit_athlete_base.py','capture_athlete.py','capture_athlete_lower.py','publish_athlete_audit.py']:
        path=ROOT/'tools/blender'/name;inputs[str(path.relative_to(ROOT))]=hashlib.sha256(path.read_bytes()).hexdigest()
    save_json(out/'input-hashes.json',inputs)
    print(f'{len(controlled)} controlled views; {len(records)} frames; min silhouette IoU {min(v["iou"] for v in validation):.6f}')


if __name__=='__main__': main()
