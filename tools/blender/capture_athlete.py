#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Temporary, controlled Phase 5.2 Qt material/normal and deformation captures.

Uses the Phase 5.1 settled-frame/palette recorder. Run alone with .envrc and
a native Qt renderer. Restores production QML in finally; maps are scratch
files, and no mesh, skeleton, weights, contact target or equipment is edited.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

from capture import ROOT, replace_once
from capture_contact import DIAGNOSTIC, SAMPLES
from athlete_evidence import (verified_palette, instrument_record, build_capture, diagnostic_textures,
                              SCALAR_FIELDS, MAP_FIELDS, TEXTURE_FIELDS, validate_materials)

VIEWS = ['full', 'face', 'torso', 'left', 'right']
VARIANTS = ['A', 'B', 'C', 'D', 'L']
STRESSED = {'row-catch', 'ski-release'}


def surface_palettes(verified_bytes=None):
    source = verified_palette()[0] if verified_bytes is None else verified_bytes
    section = source.decode().split('const SURFACE_PALETTES:')[1].split('\n];', 1)[0]
    result = []
    for role, values in re.findall(r'role: "([\w-]+)",\s+colors: \[(.*?)\n    \]', section, re.S):
        colors = [list(map(float, x)) for x in re.findall(r'\[([.\d]+), ([.\d]+), ([.\d]+)\]', values)]
        result.append((role, colors))
    if len(result) != 8:
        raise ValueError('web surface palette changed; inspect its source')
    return result


def nearest_palette_roles(centers, palette, roles):
    """Exact first-nearest palette lookup with bounded working memory."""
    import numpy as np
    best=np.full(len(centers),np.inf);result=np.zeros(len(centers),dtype=int)
    for color,role in zip(np.asarray(palette),roles):
        distance=np.sum((centers-color)**2,axis=1)
        improve=distance<best
        best[improve]=distance[improve];result[improve]=role
    return result


def diagnostic_maps(out, size=1024, palette_bytes=None):
    """Bake only presentation maps; overlapping UVs remain a measured limitation."""
    import numpy as np
    from PIL import Image, ImageDraw
    from contact_skin import Glb
    glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    primitive=glb.json['meshes'][0]['primitives'][0]
    att=primitive['attributes']
    uv=glb.accessor(att['TEXCOORD_0']).copy()
    # Balsam flips glTF V to Qt/OpenGL bottom-left UVs; verified against
    # every imported position/UV tuple by verify_imported_uv below.
    uv[:,1]=1-uv[:,1]
    uv_min=uv.min(0); uv_span=uv.max(0)-uv_min
    atlas_uv=(uv-uv_min)/uv_span
    colors=glb.accessor(att['COLOR_0'])[:,:3]
    triangles=glb.accessor(primitive['indices']).reshape(-1,3)
    palettes=surface_palettes(palette_bytes)
    palette=[]; roles=[]
    for role, (_, swatches) in enumerate(palettes):
        for c in swatches:
            c=np.array(c)
            palette.extend([c,np.where(c<=.04045,c/12.92,((c+.055)/1.055)**2.4)])
            roles.extend([role,role])
    centers=colors[triangles].mean(1)
    role=nearest_palette_roles(centers,palette,roles)
    roughness=np.array([.48,.86,.86,.70,.78,.70,.18,.50])
    rough=Image.new('L',(size,size),round(.7*255));draw=ImageDraw.Draw(rough)
    valid=0
    # Fit the complete tiled UV domain with a Texture transform. Mesh UVs
    # are unchanged. Only degenerate triangles retain the .7 fallback.
    for tri,r in zip(triangles,role):
        coords=uv[tri]
        a,b=coords[1:]-coords[0]
        if abs(a[0]*b[1]-a[1]*b[0])<1e-14: continue
        coords=atlas_uv[tri]
        draw.polygon([(float(u*(size-1)),float((1-v)*(size-1))) for u,v in coords],fill=round(roughness[r]*255));valid+=1
    rough.save(out/'roughness.png')
    center_uv=atlas_uv[triangles].mean(1)
    ix=np.rint(center_uv[:,0]*(size-1)).astype(int);iy=np.rint((1-center_uv[:,1])*(size-1)).astype(int)
    actual=np.asarray(rough)[iy,ix];wanted=np.rint(roughness[role]*255).astype(int)
    recovery={name:float(np.mean(actual[role==i]==wanted[role==i])) for i,(name,_) in enumerate(palettes)}
    positions=glb.accessor(att['POSITION']);pts=positions[triangles]
    area=np.linalg.norm(np.cross(pts[:,1]-pts[:,0],pts[:,2]-pts[:,0]),axis=1)
    area_recovery={name:float(area[(role==i)&(actual==wanted)].sum()/area[role==i].sum()) for i,(name,_) in enumerate(palettes)}
    if recovery['jersey']<.9 or recovery['lower']<.9:
        raise ValueError('atlas does not preserve sufficient skin/fabric role coverage: '+str(recovery))
    y,x=np.mgrid[:size,:size]/size
    nx=.10*np.sin(2*np.pi*80*x);ny=.10*np.sin(2*np.pi*80*y)
    normal=np.stack([nx,ny,np.sqrt(1-nx*nx-ny*ny)],axis=2)
    Image.fromarray(np.uint8(np.rint((normal+1)*127.5))).save(out/'normal.png')
    return {'size':size,'painted_triangles':valid,'qt_uv_min':uv_min.tolist(),'qt_uv_span':uv_span.tolist(),
            'texture_scale':(1/uv_span).tolist(),'texture_offset':(-uv_min/uv_span).tolist(),
            'triangle_centroid_role_recovery':recovery,'surface_area_role_recovery':area_recovery,'roughness_by_role':roughness.tolist(),
            'normal_xy_amplitude':.10,'normal_periods':80,'normal_strength':.20,
            'overlap_policy':'GLB triangle order, last writer; original UVs retained',
            'fallback_roughness':.70}


def verify_imported_uv(path):
    """Strict proof of the importer convention for this pinned V4, no new parser.

    meshdebug supplies the version/layout. Search candidate buffer starts,
    then require equality of the ENTIRE position/UV multiset, allowing the
    importer's vertex reordering and exact seam duplicates. Refuse drift.
    """
    import numpy as np
    from contact_skin import Glb
    glb=Glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
    att=glb.json['meshes'][0]['primitives'][0]['attributes']
    p=glb.accessor(att['POSITION']);uv=glb.accessor(att['TEXCOORD_0'])
    expected=np.column_stack([p,uv[:,0],1-uv[:,1]])
    expected=expected[np.lexsort(expected.T[::-1])]
    path=Path(path)
    layout=subprocess.check_output(['meshdebug',str(path)],stderr=subprocess.STDOUT,text=True)
    if not all(value in layout for value in ['fileVersion: 7','stride: 80','entry count: 6','name: "attr_uv0"']):
        raise ValueError('current imported mesh layout changed; inspect meshdebug output')
    data=path.read_bytes();start=0
    while True:
        offset=data.find(p[0].tobytes(),start)
        if offset<0 or offset+len(p)*80>len(data): break
        v=np.ndarray((len(p),20),dtype='<f4',buffer=data,offset=offset,strides=(80,4))
        actual=np.column_stack([v[:,:3],v[:,6:8]])
        if np.array_equal(actual[np.lexsort(actual.T[::-1])],expected):
            return {'mesh_path':str(path.resolve()),'mesh_sha256':hashlib.sha256(data).hexdigest(),'vertices':len(p),
                    'position_uv_tuple_max_error':0,'qt_uv':'(glTF.u, 1 - glTF.v)',
                    'mesh_version':7,'stride':80,'uv_offset':24}
        start=offset+1
    raise ValueError('cannot verify the imported V4 UV convention; inspect meshdebug output')


AUDIT = r'''
    property var auditOriginal: []
    property var auditBaseline: []
    property var auditIdentities: []
    property string auditVariantName: "A"
    property string auditViewName: "chase"
    property var auditScalarFields: __SCALAR_FIELDS__
    property var auditMapFields: __MAP_FIELDS__
    property var auditTextureFields: __TEXTURE_FIELDS__
    function auditIdentity(object, kind) {
        for (var i=0; i<auditIdentities.length; ++i)
            if (auditIdentities[i].object === object) return auditIdentities[i].identity
        var identity=kind+":"+auditIdentities.length
        auditIdentities.push({object:object, identity:identity})
        return identity
    }
    function auditTexture(texture) {
        if (!texture) return null
        var known=texture === auditRoughness ? "roughness" : texture === auditNormal ? "normal" : ""
        var value={identity:known ? "audit-"+known : auditIdentity(texture,"texture"),
            source:texture.source.toString(), sha256:known ? auditMapHashes[known+".png"] : null,
            sourceItem:!!texture.sourceItem, textureData:!!texture.textureData}
        for (var i=0; i<auditTextureFields.length; ++i) {
            var key=auditTextureFields[i]
            if (texture[key] === undefined) throw new Error("missing audited texture property "+key)
            value[key]=texture[key]
        }
        return value
    }
    function auditMaterialState(m) {
        var state={baseColor:m.baseColor.toString(),
            emissiveFactor:[m.emissiveFactor.x,m.emissiveFactor.y,m.emissiveFactor.z]}
        for (var i=0; i<auditScalarFields.length; ++i) {
            var key=auditScalarFields[i]
            if (m[key] === undefined) throw new Error("missing audited material property "+key)
            state[key]=m[key]
        }
        for (var j=0; j<auditMapFields.length; ++j) {
            if (m[auditMapFields[j]] === undefined) throw new Error("missing audited map property "+auditMapFields[j])
            state[auditMapFields[j]]=auditTexture(m[auditMapFields[j]])
        }
        return {identity:auditIdentity(m,"material"),objectName:m.objectName,state:state}
    }
    function auditEntries() {
        var result=[]
        function visit(n, path) {
            if (n.materials !== undefined)
                result.push({node:n,path:path,materials:Array.prototype.slice.call(n.materials)})
            var ch=n.children
            for (var i=0; ch && i<ch.length; ++i) visit(ch[i],path+"/"+i)
        }
        visit(athlete,"athlete")
        return result
    }
    function auditSnapshot() {
        var entries=auditEntries(), result=[]
        for (var i=0; i<entries.length; ++i) {
            var e=entries[i], materials=[]
            for (var j=0; j<e.materials.length; ++j) materials.push(auditMaterialState(e.materials[j]))
            result.push({node:e.path,objectName:e.node.objectName,materialCount:materials.length,materials:materials})
        }
        return result
    }
    function auditMaterials(variant) {
        if (auditOriginal.length === 0) {
            auditOriginal=auditEntries()
            var fields=["baseColor","vertexColorsEnabled","roughness","specularAmount","clearcoatAmount","roughnessMap","roughnessChannel","normalMap","normalStrength"]
            for (var i=0; i<auditOriginal.length; ++i) {
                var entry=auditOriginal[i]
                entry.defaults=[]
                for (var j=0; j<entry.materials.length; ++j) {
                    var defaults={}, material=entry.materials[j]
                    for (var f=0; f<fields.length; ++f) {
                        var field=fields[f]
                        // QColor is a live QML value reference. Store an immutable
                        // colour literal or B's clay colour leaks into later A/C.
                        defaults[field]=field === "baseColor" ? material[field].toString() : material[field]
                    }
                    entry.defaults.push(defaults)
                }
            }
            // Freeze untouched production values before the first intervention.
            auditBaseline=JSON.parse(JSON.stringify(auditSnapshot()))
        }
        for (var n=0; n<auditOriginal.length; ++n) {
            var saved=auditOriginal[n]
            saved.node.materials=saved.materials
            for (var k=0; k<saved.materials.length; ++k) {
                var m=saved.materials[k], original=saved.defaults[k]
                for (var key in original) m[key]=original[key]
                if (variant === "B") {
                    m.baseColor="#a3a3a3"; m.vertexColorsEnabled=false
                    m.roughness=.9; m.specularAmount=.1; m.clearcoatAmount=0
                    m.normalMap=null; m.normalStrength=0
                } else if (variant === "C" || variant === "D" || variant === "L") {
                    m.roughness=1; m.roughnessMap=auditRoughness; m.roughnessChannel=Material.R
                    m.specularAmount=.25; m.clearcoatAmount=0
                    m.normalMap=variant === "D" ? auditNormal : null
                    m.normalStrength=variant === "D" ? .20 : 0
                } else if (variant === "A0") { m.normalMap=null; m.normalStrength=0 }
            }
        }
        keyLight.brightness = variant === "L" ? 0 : Replay.sportIndex === 0 ? RowingStyle.keyBrightness : 1.2
        auditVariantName=variant
    }
    function auditView(view) {
        hud.visible=false
        auditViewName=view
        if (view === "left" || view === "right") { contactView(view); return }
        var target, offset
        if (view === "face") {
            target=jointNodes[4].mapPositionToScene(Qt.vector3d(0,.045,.055))
            offset=Qt.vector3d(.26,.03,.48)
        } else if (view === "torso") {
            target=jointNodes[2].mapPositionToScene(Qt.vector3d(0,0,.01))
            offset=Qt.vector3d(.85,.2,1.15)
        } else if (view === "lower") {
            target=jointNodes[0].mapPositionToScene(Qt.vector3d(0,-.16,0))
            offset=Qt.vector3d(1,.10,1.35)
        } else {
            target=jointNodes[0].mapPositionToScene(Qt.vector3d(0,.20,0))
            offset=Qt.vector3d(1.65,.65,2.15)
        }
        camera.position=target.plus(rigGroup.mapDirectionToScene(offset))
        camera.lookAt(target)
        camera.fieldOfView=45
    }
    function auditDump(name) {
        var mask=name.endsWith("-mask")
        console.log("ATHLETE_EXPERIMENT " + JSON.stringify({name:name,variant:mask ? "mask" : auditVariantName,view:auditViewName,
            baseline:auditBaseline,materials:mask ? [] : auditSnapshot(),
            keyBrightness:keyLight.brightness,probeExposure:scene.environment.probeExposure,
            probe:skyProbe.source.toString()}))
    }
'''
AUDIT = AUDIT.replace('__SCALAR_FIELDS__',json.dumps(SCALAR_FIELDS)).replace('__MAP_FIELDS__',json.dumps(MAP_FIELDS)).replace('__TEXTURE_FIELDS__',json.dumps(TEXTURE_FIELDS))


def capture_cases(smoke=False):
    cases, step = [], 300
    def add(action):
        nonlocal step
        cases.append(f'case {step}: {action}; break')
        step += 1
    count = 0
    scene = 'detailColumn.children[3]'
    for workout, samples in SAMPLES.items():
        if smoke and workout != 1001:
            continue
        add(f'Replay.loadWorkout({workout}); Replay.loadGhost(-1); Replay.setQualityIndex(1)')
        for name, cycle in samples:
            if name == 'ski-plant' or (smoke and name != 'row-catch'):
                continue
            add(f'Replay.setGuardCycleStep({cycle}); {scene}.auditMaterials("A"); root.grabSettledScene("{name}-chase-A")')
            count += 1
            views = (['face'] if smoke else VIEWS) if name in STRESSED else ['full','torso','lower']
            for view in views:
                for variant in VARIANTS if name in STRESSED else ['A']:
                    capture=f'{name}-{view}-{variant}'
                    add(f'{scene}.auditMaterials("{variant}"); {scene}.auditView("{view}"); root.grabSettledScene("{capture}")')
                    count += 1
                if name in STRESSED:
                    add(f'{scene}.contactMask(true); root.grabSettledScene("{name}-{view}-mask")')
                    add(f'{scene}.contactMask(false)')
                    count += 1
            if name in STRESSED:
                add(f'{scene}.auditMaterials("A0"); {scene}.auditView("face"); root.grabSettledScene("{name}-face-A0")')
                count += 1
    add('Qt.quit()')
    return cases, count


def main(supplemental=False):
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--output', type=Path, required=True)
    p.add_argument('--smoke', action='store_true', help='one stressed pose to validate the instrument')
    args=p.parse_args()
    tooling=instrument_record()
    palette_bytes,palette_provenance=verified_palette()
    out=args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    map_parameters=diagnostic_maps(out,palette_bytes=palette_bytes)
    map_hashes={name:hashlib.sha256((out/name).read_bytes()).hexdigest() for name in ['roughness.png','normal.png']}
    paths=[ROOT/'qml/RowPlay/Main.qml', ROOT/'qml/RowPlay/Replay/ReplayScene.qml']
    originals=[p.read_text() for p in paths]
    main=replace_once(originals[0], '            case 53: Replay.loadWorkout(1001); break     // rower demo workout',
        '            case 53: Replay.pause(); Replay.loadGhost(-1); root.gateStep = 299; break')
    cases, expected=capture_cases(args.smoke)
    main=replace_once(main, '            switch (root.gateStep) {', '            switch (root.gateStep) {\n'+'\n'.join(cases))
    anchor='            result.saveToFile(Settings.screenshotDir + "/" + name + ".ppm")'
    main=replace_once(main, anchor, anchor+'\n            if (root.gateStep >= 300) { detailColumn.children[3].contactDump(name); detailColumn.children[3].auditDump(name) }')
    scene=replace_once(originals[1], '    // ---- applyFrame (the per-tick frame-bundle reader) ----', DIAGNOSTIC+AUDIT+'\n    property var auditMapHashes: '+json.dumps(map_hashes)+'\n    // ---- applyFrame (the per-tick frame-bundle reader) ----')
    scene=replace_once(scene, '        PerspectiveCamera { id: camera; clipNear: 0.1; clipFar: 1000 }', f'''
        DefaultMaterial {{ id: contactMaskMaterial; lighting: DefaultMaterial.NoLighting; diffuseColor: "#ff00ff"; vertexColorsEnabled: false }}
        Texture {{ id: auditRoughness; source: "{(out/'roughness.png').as_uri()}"; scaleU: {map_parameters['texture_scale'][0]}; scaleV: {map_parameters['texture_scale'][1]}; positionU: {map_parameters['texture_offset'][0]}; positionV: {map_parameters['texture_offset'][1]}; tilingModeHorizontal: Texture.ClampToEdge; tilingModeVertical: Texture.ClampToEdge }}
        Texture {{ id: auditNormal; source: "{(out/'normal.png').as_uri()}" }}
        PerspectiveCamera {{ id: camera; clipNear: 0.1; clipFar: 1000 }}''')
    metadata={'base':tooling['commit'],'tooling':tooling,'canonical_palette':palette_provenance,
        'asset_sha256':hashlib.sha256((ROOT/'assets/replay/rowplay-athlete-v4.glb').read_bytes()).hexdigest(),
        'instrument_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'maps':map_parameters,
        'map_sha256':map_hashes,'diagnostic_textures':diagnostic_textures(out,map_parameters,map_hashes),
        'qt':subprocess.check_output(['qmake','-query','QT_VERSION'],text=True).strip(),
        'samples':SAMPLES, 'clock':'fallback 30 spm; phase=step/2000*tau, distance=step*3',
        'scheme':'light','tier':'Medium','renderer':os.environ.get('QSG_RHI_BACKEND'),
        'platform':os.environ.get('QT_QPA_PLATFORM'), 'views':VIEWS, 'variants':VARIANTS}
    if supplemental:
        metadata.update(mode='supplemental isolated lower anatomy; equipment hidden, clay material',
                        views=['lower'],variants=['B'],
                        supplemental_instrument_sha256=tooling['instruments']['tools/blender/capture_athlete_lower.py'])
    (out/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
    try:
        for path,value in zip(paths,[main,scene]): path.write_text(value)
        artifacts=build_capture(out)
        metadata['cargo']=dict(artifacts,target_dir_env=os.environ.get('CARGO_TARGET_DIR'))
        metadata['imported_uv']=verify_imported_uv(artifacts['athlete_mesh'])
        metadata['cargo']['launched_executable']=artifacts['executable']
        (out/'manifest.json').write_text(json.dumps(metadata,indent=2)+'\n')
        env=dict(os.environ, ROWPLAY_SMOKE_GATE='1',ROWPLAY_SYNC_MOCK='1',ROWPLAY_GATE_PROFILE='full',
                 ROWPLAY_FORCE_COLOR_SCHEME='light',ROWPLAY_SMOKE_SCREENSHOT_DIR=str(out),
                 ROWPLAY_DATA_DIR=str(out/'data'),QT_MESSAGE_PATTERN='[%{time process}] %{message}')
        with (out/'gate.log').open('w') as log:
            subprocess.run([artifacts['executable']],cwd=ROOT,env=env,stdout=log,
                           stderr=subprocess.STDOUT,check=True,timeout=1200)
        log=(out/'gate.log').read_text()
        for prefix,filename in [('CONTACT_FRAME ','frames.json'),('ATHLETE_EXPERIMENT ','experiment.json')]:
            records=[json.loads(line.split(prefix,1)[1]) for line in log.splitlines() if prefix in line]
            if len(records)!=expected: raise RuntimeError(f'{prefix}: {len(records)} != {expected}; inspect gate.log')
            (out/filename).write_text(json.dumps(records,indent=2)+'\n')
        material_validation=validate_materials(json.loads((out/'experiment.json').read_text()),metadata,lower=supplemental)
        (out/'material-validation.json').write_text(json.dumps(material_validation,indent=2)+'\n')
        forbidden=['failed to load component','Unexpected token','screenshot FAILED','ReferenceError:','TypeError:','Failed to compile','Failed to generate shader','Shader compilation failed']
        if any(x in log for x in forbidden): raise RuntimeError('invalid capture; inspect gate.log')
    finally:
        for path,original in zip(paths, originals): path.write_text(original)


if __name__ == '__main__':
    main()
