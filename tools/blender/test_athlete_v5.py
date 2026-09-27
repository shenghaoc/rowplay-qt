# SPDX-License-Identifier: GPL-3.0-or-later
"""Fault injection for the modelled source's motion and native evidence chain."""
from copy import deepcopy
import json
from pathlib import Path
import struct
from tempfile import TemporaryDirectory
import unittest

import numpy as np

from export_athlete import ROOT, ROLES, ContractError, read_glb, inherit_motion
from capture_athlete_v5 import cargo_artifacts, validate_materials
from athlete_evidence import MAP_FIELDS, SCALAR_FIELDS
from athlete_v5_skin import Glb, skin_all, palm_reach_bound


class SkinTests(unittest.TestCase):
    def test_all_primitives_recover_bind_and_fit_conservative_reach_bound(self):
        glb = Glb(ROOT/'assets/replay/authored/rowplay-athlete-v5.glb')
        contract = json.loads((ROOT/'assets/replay/authored/rowplay-athlete-v5.contract.json').read_text())
        rig = Glb(ROOT/'assets/replay/rowplay-rigs-v3.glb')
        names = [glb.json['nodes'][i]['name'] for i in glb.json['skins'][0]['joints']]
        inverse = glb.accessor(glb.json['skins'][0]['inverseBindMatrices']).reshape(-1,4,4).transpose(0,2,1)
        def basis(m):
            return [m[:3,3].tolist(), *[(m[:3,3]+m[:3,i]).tolist() for i in range(3)]]
        record = dict(nodes={n:basis(m) for n,m in zip(names,np.linalg.inv(inverse))},
                      rig=basis(np.eye(4)), name='bind-test', sport=2,
                      equipment={'equipment:bike:frame-assembly:brake-hoods':basis(np.eye(4))})
        posed,t,masks,rest,weights,_ = skin_all(glb,contract,record)
        self.assertEqual(len(t),contract['measurements']['triangles'])
        np.testing.assert_allclose(posed,rest,atol=2e-7)
        bounds = palm_reach_bound(glb,contract,rig,record)
        for row in bounds:
            measured = np.linalg.norm(posed[masks[row['side']+'Palm']]-row['shoulder'],axis=1).max()
            self.assertLessEqual(measured,row['conservative_palm_skin_radius_m']+1e-7)


class MotionTests(unittest.TestCase):
    def test_nonidentity_legacy_bind_is_rejected(self):
        old, binary = read_glb(ROOT/'assets/replay/rowplay-athlete-v4.glb')
        index = old['skins'][0]['inverseBindMatrices']
        a = old['accessors'][index]
        view = old['bufferViews'][a['bufferView']]
        offset = view.get('byteOffset', 0) + a.get('byteOffset', 0)
        struct.pack_into('<f', binary, offset, .5)
        new, new_binary = read_glb(ROOT/'assets/replay/authored/rowplay-athlete-v5.glb')
        with self.assertRaisesRegex(ContractError, 'bind rotation'):
            inherit_motion(new, new_binary, old, binary)


class CargoTests(unittest.TestCase):
    def setUp(self):
        self.temp = TemporaryDirectory(); self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        out = self.root/'out'; (out/'replay-balsam/athlete/meshes').mkdir(parents=True)
        (self.root/'app').write_text('current executable')
        (out/'replay-balsam/athlete/meshes/rowplayAthleteV5_mesh.mesh').write_bytes(b'current mesh')
        (out/'replay_athlete_v4.json').write_text('{}')
        (out/'replay-balsam/athlete/Rowplay_athlete_v5.qml').write_text(
            'objectName: "rowplayAthleteV5"\n'+'required property color x\n'*8)
        self.target = {'kind': ['bin'], 'name': 'rowplay-app'}
        self.messages = [dict(reason='compiler-artifact', package_id='current',
                              target=self.target, executable='app'),
                         dict(reason='build-script-executed', package_id='current', out_dir='out')]

    def select(self):
        return cargo_artifacts(self.messages, 'current', self.target, self.root)

    def test_only_this_invocation_is_selected(self):
        self.messages.append(dict(reason='build-script-executed', package_id='stale', out_dir='missing'))
        self.assertEqual(self.select()['executable'], str(self.root/'app'))

    def test_no_fallback_to_existing_output(self):
        self.messages.pop()
        with self.assertRaisesRegex(ValueError, 'exactly one'): self.select()

    def test_ambiguous_current_output(self):
        self.messages.append(self.messages[-1].copy())
        with self.assertRaisesRegex(ValueError, 'exactly one'): self.select()

    def test_wrong_target(self):
        self.messages[0]['target'] = {'kind': ['test'], 'name': 'rowplay-app'}
        with self.assertRaisesRegex(ValueError, 'exactly one'): self.select()

    def test_missing_static_role(self):
        (self.root/'out/replay-balsam/athlete/Rowplay_athlete_v5.qml').write_text('old athlete')
        with self.assertRaisesRegex(ValueError, 'static V5'): self.select()


class MaterialTests(unittest.TestCase):
    def setUp(self):
        self.contract = json.loads((ROOT/'assets/replay/authored/rowplay-athlete-v5.contract.json').read_text())
        materials = []
        for role in ROLES:
            state = dict.fromkeys(SCALAR_FIELDS, 0)
            state.update(dict.fromkeys(MAP_FIELDS, None))
            state.update(self.contract['materialParameters'][role])
            state.update(vertexColorsEnabled=True, alphaMode=3, opacity=1, cullMode=0, emissiveFactor=[0,0,0])
            materials.append(dict(objectName=role, identity=role, state=state))
        self.record = dict(name='row-catch-upper', schemeDark=False, tier=1,
                           enums={'opaque':3, 'back':0}, materials=[dict(
                               objectName='rowplayAthleteV5', materialCount=8, materials=materials)])

    def validate(self):
        return validate_materials([self.record], self.contract, 'light', 1)

    def test_eight_source_roles(self):
        self.assertTrue(self.validate()['productionMaterialStateVerified'])

    def test_texture_injection(self):
        self.record['materials'][0]['materials'][0]['state']['normalMap'] = {'source':'unapproved'}
        with self.assertRaisesRegex(ValueError, 'texture'): self.validate()

    def test_shared_material_object(self):
        self.record['materials'][0]['materials'][1]['identity'] = ROLES[0]
        with self.assertRaisesRegex(ValueError, 'share'): self.validate()

    def test_source_roughness_drift(self):
        self.record['materials'][0]['materials'][2]['state']['roughness'] = .1
        with self.assertRaisesRegex(ValueError, 'roughness'): self.validate()

    def test_wrong_scheme(self):
        self.record['schemeDark'] = True
        with self.assertRaisesRegex(ValueError, 'scheme'): self.validate()


if __name__ == '__main__':
    unittest.main()
