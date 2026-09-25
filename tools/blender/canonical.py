# SPDX-License-Identifier: GPL-3.0-or-later
"""Canonicalise opaque GLB triangle order without changing winding or vertices."""

import json
import struct


def canonicalize(path):
    data = bytearray(path.read_bytes())
    magic, version, length, json_size, kind = struct.unpack_from("<5I", data)
    if (magic, version, length, kind) != (0x46546C67, 2, len(data), 0x4E4F534A):
        raise ValueError("expected a glTF 2.0 GLB")
    doc = json.loads(data[20:20 + json_size])
    binary_size, binary_kind = struct.unpack_from("<2I", data, 20 + json_size)
    binary_start = 28 + json_size
    if binary_kind != 0x004E4942 or binary_start + binary_size != len(data):
        raise ValueError("expected one binary chunk")
    visited = set()
    for mesh in doc["meshes"]:
        for primitive in mesh["primitives"]:
            material = doc.get("materials", [{}])[primitive.get("material", 0)]
            if primitive.get("mode", 4) != 4 or material.get("alphaMode", "OPAQUE") != "OPAQUE":
                raise ValueError("canonical export requires opaque triangles")
            index = primitive["indices"]
            if index in visited:
                continue
            visited.add(index)
            accessor = doc["accessors"][index]
            view = doc["bufferViews"][accessor["bufferView"]]
            if accessor["type"] != "SCALAR" or "sparse" in accessor or "byteStride" in view:
                raise ValueError("expected tightly packed scalar indices")
            code = {5121: "B", 5123: "H", 5125: "I"}[accessor["componentType"]]
            count = accessor["count"]
            if count % 3:
                raise ValueError("incomplete triangle")
            offset = binary_start + view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
            indices = struct.unpack_from(f"<{count}{code}", data, offset)
            triangles = []
            for i in range(0, count, 3):
                a, b, c = indices[i:i + 3]
                triangles.append(min((a, b, c), (b, c, a), (c, a, b)))
            triangles.sort()
            flat = [index for triangle in triangles for index in triangle]
            struct.pack_into(f"<{count}{code}", data, offset, *flat)
    path.write_bytes(data)
