# SPDX-License-Identifier: GPL-3.0-or-later
"""Triangle ordering is deterministic but must never reverse a face."""

import json
from pathlib import Path
import struct
import tempfile
import unittest

from canonical import canonicalize


def glb(indices, alpha="OPAQUE", mutate=None):
    vertices = bytes(4 * 3 * 4)
    binary = vertices + struct.pack(f"<{len(indices)}H", *indices)
    doc = {"asset": {"version": "2.0"}, "materials": [{"alphaMode": alpha}],
           "meshes": [{"primitives": [{"indices": 0, "material": 0, "attributes": {"POSITION": 1}}]}],
           "accessors": [{"bufferView": 0, "type": "SCALAR", "componentType": 5123,
                          "count": len(indices)}, {"bufferView": 1, "type": "VEC3",
                          "componentType": 5126, "count": 4}],
           "bufferViews": [{"buffer": 0, "byteOffset": len(vertices), "byteLength": len(indices) * 2},
                           {"buffer": 0, "byteLength": len(vertices)}],
           "buffers": [{"byteLength": len(binary)}]}
    if mutate:
        mutate(doc)
    encoded = json.dumps(doc).encode()
    encoded += b" " * (-len(encoded) % 4)
    binary += b"\x00" * (-len(binary) % 4)
    return (struct.pack("<5I", 0x46546C67, 2, 28 + len(encoded) + len(binary),
                        len(encoded), 0x4E4F534A) + encoded
            + struct.pack("<2I", len(binary), 0x004E4942) + binary)


class CanonicalTests(unittest.TestCase):
    def test_order_and_cyclic_rotation_are_canonical(self):
        with tempfile.TemporaryDirectory() as directory:
            a, b = Path(directory) / "a.glb", Path(directory) / "b.glb"
            a.write_bytes(glb([2, 0, 1, 3, 2, 1]))
            b.write_bytes(glb([1, 3, 2, 0, 1, 2]))
            canonicalize(a)
            canonicalize(b)
            self.assertEqual(a.read_bytes(), b.read_bytes())
            before = a.read_bytes()
            canonicalize(a)
            self.assertEqual(before, a.read_bytes())
            # Reversing winding changes the surface and must not compare equal.
            b.write_bytes(glb([0, 2, 1, 1, 3, 2]))
            canonicalize(b)
            self.assertNotEqual(a.read_bytes(), b.read_bytes())

    def test_translucent_draw_order_is_not_rewritten(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "blend.glb"
            path.write_bytes(glb([0, 1, 2], "BLEND"))
            with self.assertRaisesRegex(ValueError, "opaque"):
                canonicalize(path)

    def test_unsupported_layouts_fail_without_writing(self):
        mutations = {
            "overlapping views": lambda d: d["bufferViews"][0].update(byteOffset=0),
            "short view": lambda d: d["bufferViews"][0].update(byteLength=2),
            "attribute alias": lambda d: d["accessors"][1].update(bufferView=0),
            "transmission": lambda d: d["materials"][0].update(
                extensions={"KHR_materials_transmission": {"transmissionFactor": 1}}),
            "mixed primitives": lambda d: d["meshes"][0]["primitives"].append(
                dict(d["meshes"][0]["primitives"][0])),
            "sparse attributes": lambda d: d["accessors"][1].update(sparse={}),
            "index as attribute": lambda d: d["meshes"][0]["primitives"][0]["attributes"].update(POSITION=0),
            "external buffer": lambda d: d["buffers"][0].update(uri="elsewhere.bin"),
        }
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "unsupported.glb"
            for name, mutate in mutations.items():
                with self.subTest(name=name):
                    before = glb([2, 0, 1, 3, 2, 1], mutate=mutate)
                    path.write_bytes(before)
                    with self.assertRaises(ValueError):
                        canonicalize(path)
                    self.assertEqual(path.read_bytes(), before)

    def test_committed_buoy_is_unchanged(self):
        source = Path(__file__).resolve().parents[2] / "assets/replay/authored/buoy.glb"
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "buoy.glb"
            path.write_bytes(source.read_bytes())
            canonicalize(path)
            self.assertEqual(path.read_bytes(), source.read_bytes())


if __name__ == "__main__":
    unittest.main()
