#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""List every replay model and image, including embedded glTF images."""

import argparse
import json
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[2]


def inventory():
    rows = []
    for path in sorted((ROOT / "assets/replay").rglob("*")):
        if path.suffix not in (".glb", ".png", ".jpg", ".jpeg", ".hdr", ".ktx"):
            continue
        data = path.read_bytes()
        detail = "texture"
        if path.suffix == ".glb":
            magic, version, length, size, kind = struct.unpack_from("<5I", data)
            if (magic, version, length, kind) != (0x46546C67, 2, len(data), 0x4E4F534A):
                raise ValueError(f"invalid GLB: {path}")
            doc = json.loads(data[20:20 + size])
            triangles = 0
            for mesh in doc.get("meshes", []):
                for primitive in mesh["primitives"]:
                    if primitive.get("mode", 4) != 4:
                        raise ValueError(f"non-triangle primitive: {path}")
                    accessor = primitive.get("indices", primitive["attributes"]["POSITION"])
                    triangles += doc["accessors"][accessor]["count"] // 3
            detail = f"{triangles:,} mesh triangles; {len(doc.get('images', []))} embedded images"
        elif path.suffix in (".hdr", ".ktx"):
            detail = "generated environment radiance" if path.suffix == ".hdr" else "prefiltered IBL"
        rows.append(f"| `{path.relative_to(ROOT)}` | {len(data):,} | {detail} |")
    return "# Replay asset inventory\n\n| Asset | Bytes | Contents |\n| --- | ---: | --- |\n" + "\n".join(rows) + "\n"


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    report = inventory()
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(report)
    else:
        print(report, end="")
