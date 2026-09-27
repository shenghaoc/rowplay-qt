# SPDX-License-Identifier: GPL-3.0-or-later
"""Fault-injection tests for the Phase 5.2 evidence chain (no real checkout edits)."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory
import unittest

from athlete_evidence import (SCALAR_FIELDS, MAP_FIELDS, PALETTE_PATH, f32, sha,
                              expected_materials, validate_materials, verified_palette,
                              verify_palette_record, verify_instrument, cargo_artifacts)


def baseline():
    state = dict.fromkeys(SCALAR_FIELDS, 0)
    state.update(dict.fromkeys(MAP_FIELDS, None))
    state.update(baseColor='#ffffff', emissiveFactor=[0, 0, 0], vertexColorsEnabled=True,
                 roughness=f32(.64), clearcoatAmount=f32(.03), clearcoatRoughnessAmount=f32(.85),
                 opacity=1, alphaMode=3, normalStrength=1, specularAmount=1, roughnessChannel=1)
    return [dict(node=f'athlete/{n}', objectName=f'mesh{n}', materialCount=2,
                 materials=[dict(identity=f'material:{n}:{i}', objectName='', state=deepcopy(state))
                            for i in range(2)]) for n in range(2)]


class MaterialContractTests(unittest.TestCase):
    def setUp(self):
        self.manifest = {'diagnostic_textures': {'roughness': {'identity': 'audit-roughness', 'source': 'file:///roughness.png', 'sha256': 'rough'},
                                                'normal': {'identity': 'audit-normal', 'source': 'file:///normal.png', 'sha256': 'normal'}}}
        self.records = [dict(name='row-catch-face-'+v, variant=v, baseline=baseline(),
                             materials=expected_materials(baseline(), v, self.manifest['diagnostic_textures']))
                        for v in ['A', 'B', 'C', 'D', 'L', 'A0']]

    def reject(self, variant, change):
        records=deepcopy(self.records)
        record=next(r for r in records if r['variant']==variant)
        change(record)
        with self.assertRaises(ValueError): validate_materials(records,self.manifest)

    def change_state(self, variant, **fields):
        self.reject(variant,lambda r:r['materials'][1]['materials'][1]['state'].update(fields))

    def test_exact_interventions_on_every_node_and_material_pass(self):
        result=validate_materials(self.records,self.manifest)
        self.assertEqual(result['materials'],4)
        self.assertEqual(result['captures_by_variant'],{v:1 for v in ['A','B','C','D','L','A0']})

    def test_b_metalness_drift(self): self.change_state('B',metalness=.4)
    def test_c_rejects_clay_colour_leak(self): self.change_state('C',baseColor='#a3a3a3')
    def test_c_wrong_map_identity(self): self.change_state('C',roughnessMap={'identity':'other','source':'file:///roughness.png'})
    def test_c_wrong_map_source(self): self.change_state('C',roughnessMap={'identity':'audit-roughness','source':'file:///wrong.png','sha256':'rough'})
    def test_d_missing_normal(self): self.change_state('D',normalMap=None)
    def test_d_wrong_strength(self): self.change_state('D',normalStrength=.4)
    def test_l_material_drift(self): self.change_state('L',specularAmount=.1)
    def test_a_leaks_previous_intervention(self): self.change_state('A',roughness=1)
    def test_a0_unlisted_change(self): self.change_state('A0',opacity=.5)
    def test_missing_node(self): self.reject('C',lambda r:r['materials'].pop())
    def test_extra_node(self): self.reject('C',lambda r:r['materials'].append(deepcopy(r['materials'][0])))
    def test_changed_count(self): self.reject('C',lambda r:r['materials'][0].update(materialCount=3))
    def test_changed_identity(self): self.reject('C',lambda r:r['materials'][0]['materials'][0].update(identity='replacement'))
    def test_variant_name_disagrees(self): self.reject('D',lambda r:r.update(variant='C'))
    def test_partial_application(self): self.reject('B',lambda r:r['materials'].__setitem__(1,deepcopy(r['baseline'][1])))
    def test_poisoned_untouched_baseline(self):
        records=deepcopy(self.records)
        for r in records: r['baseline'][0]['materials'][0]['state']['roughness']=1
        with self.assertRaisesRegex(ValueError,'baseline'): validate_materials(records,self.manifest)


def git(repo,*args):
    return subprocess.check_output(['git','-C',str(repo),*args],stderr=subprocess.PIPE).decode().strip()


def commit(repo,message):
    git(repo,'add','--all')
    git(repo,'-c','user.name=Evidence Test','-c','user.email=evidence@example.invalid','commit','-qm',message)
    return git(repo,'rev-parse','HEAD')


class PaletteProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temp=TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.repo=self.root/'reference/rowplay';self.repo.mkdir(parents=True)
        git(self.repo,'init','-q')
        path=self.repo/PALETTE_PATH;path.parent.mkdir(parents=True);path.write_text('verified palette\n')
        self.pin=commit(self.repo,'palette')
        (self.root/'docs').mkdir()
        (self.root/'docs/source-map.md').write_text(f'| `https://github.com/shenghaoc/rowplay` | `main` | `{self.pin}` |')

    def test_correct_pin_and_unrelated_dirty_file_allowed(self):
        (self.repo/'unrelated').write_text('unrelated')
        data,record=verified_palette(root=self.root)
        self.assertEqual(data,b'verified palette\n')
        self.assertEqual(record['sha256'],sha(data))
        self.assertEqual(verify_palette_record(record,self.root),record)

    def test_wrong_reference_head_refused(self):
        (self.repo/'unrelated').write_text('next commit');commit(self.repo,'other')
        with self.assertRaisesRegex(ValueError,'HEAD'): verified_palette(root=self.root)

    def test_modified_palette_refused(self):
        (self.repo/PALETTE_PATH).write_text('modified')
        with self.assertRaisesRegex(ValueError,'local modifications'): verified_palette(root=self.root)

    def test_recorded_hash_refused_by_publication(self):
        _,record=verified_palette(root=self.root);record['sha256']='wrong'
        with self.assertRaisesRegex(ValueError,'hash differs'): verify_palette_record(record,self.root)


class InstrumentHistoryTests(unittest.TestCase):
    def setUp(self):
        self.temp=TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);git(self.root,'init','-q')
        self.path='capture.py';(self.root/self.path).write_text('instrument\n')
        self.pin=commit(self.root,'tooling')
        self.record={'commit':self.pin,'instruments':{self.path:sha(b'instrument\n')}}
        (self.root/'evidence').write_text('captured');commit(self.root,'evidence')

    def verify(self): return verify_instrument(self.record,self.root,[self.path])
    def test_exact_ancestor_match(self): self.assertEqual(self.verify(),self.record)
    def test_wrong_recorded_hash(self):
        self.record['instruments'][self.path]='wrong'
        with self.assertRaisesRegex(ValueError,'different instrument; regenerate'): self.verify()
    def test_working_instrument_changed(self):
        (self.root/self.path).write_text('changed')
        with self.assertRaisesRegex(ValueError,'different instrument; regenerate'): self.verify()
    def test_reviewed_committed_instrument_changed(self):
        (self.root/self.path).write_text('changed');commit(self.root,'changed instrument')
        with self.assertRaisesRegex(ValueError,'different instrument; regenerate'): self.verify()
    def test_nonancestor_refused(self):
        head=git(self.root,'rev-parse','HEAD')
        git(self.root,'checkout','--orphan','unrelated');git(self.root,'rm','-rf','.')
        (self.root/'other').write_text('unrelated');other=commit(self.root,'unrelated')
        git(self.root,'checkout','--detach',head);self.record['commit']=other
        with self.assertRaisesRegex(ValueError,'not an ancestor'): self.verify()


class CargoArtifactTests(unittest.TestCase):
    def setUp(self):
        self.temp=TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.package='path+file:///fixture/crates/rowplay-app#0.1.0'
        self.target={'kind':['bin'],'name':'rowplay-app','src_path':'/fixture/crates/rowplay-app/src/main.rs'}
        self.exe=self.root/'custom-target/debug/rowplay-app'
        self.out=self.root/'custom-target/debug/build/current/out'
        self.mesh=self.out/'replay-balsam/athlete/meshes/current.mesh'
        for path in [self.exe,self.mesh,self.out/'replay-balsam/athlete/Rowplay_athlete_v4.qml',self.root/'target/debug/rowplay-app',self.root/'target/debug/build/stale/out/replay-balsam/athlete/meshes/stale.mesh']:
            path.parent.mkdir(parents=True,exist_ok=True);path.write_text('fixture')
        self.messages=[{'reason':'build-script-executed','package_id':'unrelated','out_dir':str(self.root/'target/debug/build/stale/out')},
                       {'reason':'compiler-artifact','package_id':'unrelated','target':self.target,'executable':str(self.root/'target/debug/rowplay-app')},
                       {'reason':'compiler-artifact','package_id':self.package,'target':self.target,'executable':str(self.exe)},
                       {'reason':'build-script-executed','package_id':self.package,'out_dir':str(self.out)}]

    def resolve(self): return cargo_artifacts(self.messages,self.package,self.target,self.root)
    def test_custom_target_ignores_stale_default_and_unrelated_scripts(self):
        result=self.resolve()
        self.assertEqual(result['executable'],str(self.exe));self.assertEqual(result['out_dir'],str(self.out))
        self.assertEqual(result['athlete_mesh'],str(self.mesh))
    def test_relative_reported_paths(self):
        self.messages[2]['executable']=str(self.exe.relative_to(self.root));self.messages[3]['out_dir']=str(self.out.relative_to(self.root))
        self.assertEqual(self.resolve()['executable'],str(self.exe))
    def test_missing_executable_does_not_fall_back(self):
        self.messages.pop(2)
        with self.assertRaises(ValueError): self.resolve()
    def test_missing_build_script(self):
        self.messages.pop()
        with self.assertRaises(ValueError): self.resolve()
    def test_missing_mesh_does_not_fall_back(self):
        self.mesh.unlink()
        with self.assertRaisesRegex(ValueError,'generated athlete mesh'): self.resolve()
    def test_ambiguous_executable(self):
        self.messages.append(deepcopy(self.messages[2]))
        with self.assertRaises(ValueError): self.resolve()
    def test_ambiguous_out_dir(self):
        self.messages.append(deepcopy(self.messages[3]))
        with self.assertRaises(ValueError): self.resolve()
    def test_ambiguous_mesh(self):
        self.mesh.with_name('other.mesh').write_text('other')
        with self.assertRaisesRegex(ValueError,'generated athlete mesh'): self.resolve()


class RecaptureComparisonTests(unittest.TestCase):
    def test_known_pixel_delta_and_refuse_pose_or_alpha_drift(self):
        from PIL import Image
        from publish_athlete_audit import compare_captures
        with TemporaryDirectory() as temp:
            old=Path(temp)/'old';new=Path(temp)/'new';old.mkdir();new.mkdir()
            records=[{'name':'test-A','nodes':{'joint':[1,2,3]}}]
            for path in [old,new]:
                (path/'frames.json').write_text(json.dumps(records))
                Image.new('RGBA',(2,2),(0,0,0,255)).save(path/'test-A.png')
            image=Image.open(new/'test-A.png');image.putpixel((0,0),(4,0,0,255));image.save(new/'test-A.png')
            result=compare_captures(old,new)['per_capture'][0]
            self.assertEqual(result['changed_pixels'],1)
            self.assertEqual(result['max_channel_difference'],4)
            self.assertAlmostEqual(result['mean_abs_rgb_8bit'],1/3)
            image.putpixel((0,0),(4,0,0,254));image.save(new/'test-A.png')
            with self.assertRaisesRegex(ValueError,'alpha'):compare_captures(old,new)
            (new/'frames.json').write_text(json.dumps([{'name':'test-A','nodes':{}}]))
            with self.assertRaisesRegex(ValueError,'frame/pose'):compare_captures(old,new)


if __name__=='__main__': unittest.main()
