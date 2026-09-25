#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Exercise the material extensions this pack actually uses through local balsam."""

import argparse
import json
import os
from pathlib import Path
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--balsam", default=os.environ.get("ROWPLAY_BALSAM", "balsam"))
    parser.add_argument("--output", type=Path, default=ROOT / "build/import-probe")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    data = (ROOT / "assets/replay/authored/buoy.glb").read_bytes()
    size, = struct.unpack_from("<I", data, 12)
    doc = json.loads(data[20:20 + size])
    material = doc["materials"][0]
    extensions = {
        "KHR_materials_clearcoat": {"clearcoatFactor": 0.7, "clearcoatRoughnessFactor": 0.2},
        "KHR_materials_transmission": {"transmissionFactor": 0.25},
        "KHR_materials_ior": {"ior": 1.4},
    }
    material["extensions"] = extensions
    doc["extensionsUsed"] = list(extensions)
    doc["images"] = [{"uri": "water-normal.png"}]
    doc["textures"] = [{"source": 0}]
    material["normalTexture"] = {"index": 0, "scale": 0.3}
    encoded = json.dumps(doc, separators=(",", ":")).encode()
    encoded += b" " * (-len(encoded) % 4)
    binary_chunk = data[20 + size:]
    path = args.output / "probe.glb"
    path.write_bytes(struct.pack("<5I", 0x46546C67, 2, 20 + len(encoded) + len(binary_chunk),
                                 len(encoded), 0x4E4F534A) + encoded + binary_chunk)
    (args.output / "water-normal.png").write_bytes(
        (ROOT / "assets/replay/authored/water-normal.png").read_bytes())
    converted = args.output / "converted"
    converted.mkdir(exist_ok=True)
    result = subprocess.run([args.balsam, "-o", str(converted), str(path)],
                            env=dict(os.environ, QT_QPA_PLATFORM="offscreen"),
                            capture_output=True, text=True, check=True, timeout=120)
    qml = (converted / "Probe.qml").read_text()
    for property in ("clearcoatAmount", "clearcoatRoughnessAmount", "transmissionFactor",
                     "indexOfRefraction", "normalMap"):
        if property not in qml:
            raise RuntimeError(f"balsam dropped {property}; inspect {converted}")
    report = "Qt material probe passed: clearcoat, transmission, IOR and PNG normal map.\n"
    (args.output / "report.txt").write_text(report + result.stdout + result.stderr)
    print(report, end="")


if __name__ == "__main__":
    main()
