# SPDX-License-Identifier: GPL-3.0-or-later
"""Run with Blender's Python to fault-inject the V5 output validator.

blender -b --python-exit-code 1 --python tools/blender/test_athlete_export_blender.py
Ordinary unittest discovery skips this class when mathutils is unavailable.
"""
from copy import deepcopy
from pathlib import Path
import struct
import sys
import unittest

sys.path.insert(0, str(Path(__file__).parent))
from export_athlete import ROOT, ContractError, read_glb, validate_output

try:
    import mathutils
    HAS_BLENDER = True
except ImportError:
    HAS_BLENDER = False


@unittest.skipUnless(HAS_BLENDER, 'requires Blender mathutils; run the documented Blender command')
class ExportValidatorTests(unittest.TestCase):
    def setUp(self):
        self.doc,self.binary = read_glb(ROOT/'assets/replay/authored/rowplay-athlete-v5.glb')

    def validate(self):
        return validate_output(self.doc,self.binary)

    def test_reviewed_output(self):
        self.assertEqual(self.validate()['triangles'],48632)

    def test_over_75000_rejected(self):
        p = self.doc['meshes'][0]['primitives'][0]
        accessor = self.doc['accessors'][p['indices']]
        view = self.doc['bufferViews'][accessor['bufferView']]
        start = view.get('byteOffset',0)+accessor.get('byteOffset',0)
        width = {5123:2,5125:4}[accessor['componentType']]
        data = self.binary[start:start+accessor['count']*width]*3
        accessor['count'] *= 3
        accessor['bufferView'] = len(self.doc['bufferViews'])
        accessor['byteOffset'] = 0
        self.doc['bufferViews'].append(dict(buffer=0,byteOffset=len(self.binary),byteLength=len(data)))
        self.binary.extend(data)
        with self.assertRaisesRegex(ContractError,'hard ceiling'): self.validate()

    def test_missing_uv(self):
        del self.doc['meshes'][0]['primitives'][0]['attributes']['TEXCOORD_0']
        with self.assertRaisesRegex(ContractError,'attributes'): self.validate()

    def test_duplicate_material_role(self):
        self.doc['materials'][1]['name'] = self.doc['materials'][0]['name']
        with self.assertRaisesRegex(ContractError,'uniqueness'): self.validate()

    def test_external_image(self):
        self.doc['images'] = [{'uri':'unapproved.png'}]
        with self.assertRaisesRegex(ContractError,'dependency'): self.validate()

    def test_unsupported_extension(self):
        self.doc['extensionsUsed'] = ['EXT_unapproved']
        with self.assertRaisesRegex(ContractError,'extension'): self.validate()

    def test_source_path_leak(self):
        self.doc['asset']['extras'] = {'path':'/home/example/source.blend'}
        with self.assertRaisesRegex(ContractError,'path leakage'): self.validate()

    def change_float(self,index,value):
        a = self.doc['accessors'][index];v = self.doc['bufferViews'][a['bufferView']]
        struct.pack_into('<f',self.binary,v.get('byteOffset',0)+a.get('byteOffset',0),value)

    def test_bad_inverse_bind(self):
        self.change_float(self.doc['skins'][0]['inverseBindMatrices'],3.0)
        with self.assertRaisesRegex(ContractError,'inverse bind'): self.validate()

    def test_bad_weight(self):
        self.change_float(self.doc['meshes'][0]['primitives'][0]['attributes']['WEIGHTS_0'],2.0)
        with self.assertRaisesRegex(ContractError,'weights'): self.validate()

    def test_nan_position(self):
        self.change_float(self.doc['meshes'][0]['primitives'][0]['attributes']['POSITION'],float('nan'))
        with self.assertRaisesRegex(ContractError,'non-finite'): self.validate()


if __name__ == '__main__':
    unittest.main(argv=[__file__])
