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
    # This pipeline only canonicalises the single, opaque generated buoy.
    # Refuse richer layouts rather than guessing their ordering/aliasing rules.
    if (len(doc["meshes"]) != 1 or len(doc["meshes"][0]["primitives"]) != 1
            or len(doc.get("materials", [])) != 1 or doc.get("extensionsUsed")
            or doc.get("extensionsRequired")):
        raise ValueError("expected one unextended buoy primitive and material")
    primitive = doc["meshes"][0]["primitives"][0]
    material = doc["materials"][0]
    if (primitive.get("mode", 4) != 4 or primitive.get("material") != 0
            or material.get("alphaMode", "OPAQUE") != "OPAQUE"
            or material.get("extensions") or primitive.get("extensions")
            or primitive.get("targets")):
        raise ValueError("canonical export requires unextended opaque triangles")
    buffers = doc["buffers"]
    if (len(buffers) != 1 or "uri" in buffers[0]
            or not 0 <= binary_size - buffers[0]["byteLength"] <= 3):
        raise ValueError("expected one embedded buffer")
    ranges = []
    for view in doc["bufferViews"]:
        start = view.get("byteOffset", 0)
        end = start + view["byteLength"]
        if (view.get("buffer") != 0 or "byteStride" in view or view.get("extensions")
                or start < 0 or end <= start or end > buffers[0]["byteLength"]
                or any(start < hi and lo < end for lo, hi in ranges)):
            raise ValueError("expected disjoint, bounded, tightly packed buffer views")
        ranges.append((start, end))
    if any("sparse" in a or a.get("extensions") for a in doc["accessors"]):
        raise ValueError("expected unextended dense accessors")
    index = primitive["indices"]
    if not 0 <= index < len(doc["accessors"]):
        raise ValueError("invalid index accessor")
    accessor = doc["accessors"][index]
    view_index = accessor["bufferView"]
    if not 0 <= view_index < len(ranges):
        raise ValueError("invalid index buffer view")
    if any(i != index and a.get("bufferView") == view_index
           for i, a in enumerate(doc["accessors"])):
        raise ValueError("index buffer view must not alias vertex attributes")
    if (accessor["type"] != "SCALAR" or "sparse" in accessor
            or accessor.get("normalized") or accessor.get("extensions")
            or accessor["componentType"] not in (5121, 5123, 5125)):
        raise ValueError("expected tightly packed unsigned scalar indices")
    code = {5121: "B", 5123: "H", 5125: "I"}[accessor["componentType"]]
    count = accessor["count"]
    relative = accessor.get("byteOffset", 0)
    size = struct.calcsize(code)
    lo, hi = ranges[view_index]
    if count <= 0 or count % 3 or relative < 0 or relative % size or relative + count * size > hi - lo:
        raise ValueError("incomplete or out-of-bounds triangles")
    attributes = primitive["attributes"]
    if any(i == index or not 0 <= i < len(doc["accessors"]) for i in attributes.values()):
        raise ValueError("invalid vertex attribute accessor")
    vertex_count = doc["accessors"][attributes["POSITION"]]["count"]
    if any(doc["accessors"][i]["count"] != vertex_count for i in attributes.values()):
        raise ValueError("vertex attribute counts differ")
    offset = binary_start + lo + relative
    indices = struct.unpack_from(f"<{count}{code}", data, offset)
    if max(indices) >= vertex_count:
        raise ValueError("index outside vertex attributes")
    triangles = []
    for i in range(0, count, 3):
        a, b, c = indices[i:i + 3]
        triangles.append(min((a, b, c), (b, c, a), (c, a, b)))
    triangles.sort()
    flat = [index for triangle in triangles for index in triangle]
    struct.pack_into(f"<{count}{code}", data, offset, *flat)
    path.write_bytes(data)
