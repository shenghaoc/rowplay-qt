# SPDX-License-Identifier: GPL-3.0-or-later
"""Triangle ordering is deterministic but must never reverse a face."""

import json
from pathlib import Path
import struct
import tempfile
import unittest

from canonical import canonicalize


def glb(indices, alpha="OPAQUE"):
    binary = struct.pack(f"<{len(indices)}H", *indices)
    doc = {"asset": {"version": "2.0"}, "materials": [{"alphaMode": alpha}],
           "meshes": [{"primitives": [{"indices": 0}]}],
           "accessors": [{"bufferView": 0, "type": "SCALAR", "componentType": 5123,
                          "count": len(indices)}], "bufferViews": [{"byteLength": len(binary)}]}
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


if __name__ == "__main__":
    unittest.main()
