# SPDX-License-Identifier: GPL-3.0-or-later
"""Bake Qt IBL, retaining every roughness level at a bounded face resolution."""

import os
from pathlib import Path
import struct
import subprocess

import numpy as np


def compact_ktx(source, output):
    data = source.read_bytes()
    magic = b"\xabKTX 11\xbb\r\n\x1a\n"
    if len(data) < 64 or data[:12] != magic:
        raise ValueError("expected KTX1 from Qt balsam")
    header = list(struct.unpack_from("<13I", data, 12))
    endian, typ, size, fmt, internal, base, width, height, depth, arrays, faces, levels, metadata = header
    if (endian, typ, size, fmt, internal, base, width, height, depth, arrays, faces, levels) != (
            0x04030201, 0x140B, 2, 0x1908, 0x881A, 0x1908, 512, 512, 0, 0, 6, 6):
        raise ValueError(f"unexpected Qt IBL layout: {header}")
    # Qt 6.11.2 qssgiblbaker.cpp writes this exact version-1 marker.
    # The runtime uses its presence to bypass online IBL baking. A changed
    # baker contract must be reviewed, not copied into an apparently valid KTX.
    expected_metadata = struct.pack("<I", 23) + b"QT_IBL_BAKER_VERSION\x001\x00\x00"
    if metadata != len(expected_metadata) or data[64:64 + metadata] != expected_metadata:
        raise ValueError("unexpected Qt IBL baker metadata")
    expected_size = 64 + metadata + sum(4 + 6 * (512 >> level) ** 2 * 8 for level in range(6))
    if len(data) != expected_size:
        raise ValueError("unexpected Qt IBL payload length")
    # RGBA16F rows and every face at both resolutions are multiples of four:
    # KTX1 cubePadding and mipPadding are zero. imageSize is PER FACE.
    # Do not discard top levels: their indices encode roughness, not just size.
    # Box-reduce each independently, including the final diffuse level.
    header[6] = header[7] = 128
    result = bytearray(magic + struct.pack("<13I", *header) + data[64:64 + metadata])
    offset = 64 + metadata
    for level in range(levels):
        face_bytes, = struct.unpack_from("<I", data, offset)
        offset += 4
        edge = width >> level
        if face_bytes != edge * edge * 8:
            raise ValueError("invalid cubemap face length")
        result.extend(struct.pack("<I", face_bytes // 16))
        for _ in range(faces):
            pixels = np.frombuffer(data, dtype="<f2", count=edge * edge * 4,
                                   offset=offset).reshape(edge // 4, 4, edge // 4, 4, 4)
            reduced = pixels.astype(np.float32).mean(axis=(1, 3)).astype("<f2")
            if not np.isfinite(reduced).all():
                raise ValueError("nonfinite probe radiance")
            result.extend(reduced.tobytes())
            offset += face_bytes
    if offset != len(data):
        raise ValueError("unexpected trailing KTX data")
    output.write_bytes(result)


def bake(source, output, balsam, scratch):
    scratch.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen")
    subprocess.run([balsam, "-o", str(scratch), str(source)], env=env, check=True, timeout=180)
    compact_ktx(scratch / source.with_suffix(".ktx").name, output)
