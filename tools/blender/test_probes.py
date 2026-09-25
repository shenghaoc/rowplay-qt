# SPDX-License-Identifier: GPL-3.0-or-later
"""KTX layout regression tests; run with python3 -m unittest discover -s tools/blender."""

from pathlib import Path
import struct
import tempfile
import unittest

import numpy as np

from probes import compact_ktx


class ProbeTests(unittest.TestCase):
    def test_preserves_face_and_roughness_identity(self):
        magic = b"\xabKTX 11\xbb\r\n\x1a\n"
        header = (0x04030201, 0x140B, 2, 0x1908, 0x881A, 0x1908,
                  512, 512, 0, 0, 6, 6, 0)
        source = bytearray(magic + struct.pack("<13I", *header))
        for level in range(6):
            edge = 512 >> level
            source.extend(struct.pack("<I", edge * edge * 8))
            for face in range(6):
                source.extend(np.full((edge, edge, 4), level * 10 + face,
                                      dtype="<f2").tobytes())
        with tempfile.TemporaryDirectory() as temp:
            path, output = Path(temp) / "input.ktx", Path(temp) / "output.ktx"
            path.write_bytes(source)
            compact_ktx(path, output)
            result = output.read_bytes()
            self.assertEqual(struct.unpack_from("<2I", result, 36), (128, 128))
            offset = 64
            for level in range(6):
                size, = struct.unpack_from("<I", result, offset)
                self.assertEqual(size, (128 >> level) ** 2 * 8)
                offset += 4
                for face in range(6):
                    values = np.frombuffer(result, dtype="<f2", count=size // 2, offset=offset)
                    self.assertTrue(np.all(values == level * 10 + face))
                    offset += size
            self.assertEqual(offset, len(result))

    def test_rejects_unknown_container(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "invalid.ktx"
            path.write_bytes(b"not a Qt KTX cubemap")
            with self.assertRaisesRegex(ValueError, "KTX1"):
                compact_ktx(path, Path(temp) / "output.ktx")


if __name__ == "__main__":
    unittest.main()
