#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify runtime frame equality, measure pixels and compose local evidence."""

import argparse
import json
from pathlib import Path
import subprocess


def frames(directory):
    result = {}
    for line in (directory / "gate.log").read_text().splitlines():
        if "ART_FRAME " in line:
            record = json.loads(line.split("ART_FRAME ", 1)[1])
            result[record["name"]] = record
    return result


def measure(*args):
    return subprocess.check_output(["magick", "-limit", "thread", "1", *map(str, args)], text=True).strip()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    before, after = frames(args.before), frames(args.after)
    names = ["style-row-middrive", "style-low", "style-medium", "style-high", "style-ultra"]
    names += [f"motion-{i:03}" for i in range(13)]
    for name in names:
        # The sequence counter is explicitly packed as float bits; not a pose.
        if before[name]["frame"][1:] != after[name]["frame"][1:]:
            raise ValueError(f"runtime frame changed: {name}")
        if before[name]["grip"] != after[name]["grip"]:
            raise ValueError(f"grip changed: {name}")
        if before[name]["tier"] != after[name]["tier"]:
            raise ValueError(f"tier changed: {name}")
    report = {"matchedRuntimeFrames": len(names), "captures": {}, "motionBandDifference": {}}
    for label, directory in (("before", args.before), ("after", args.after)):
        for name in names:
            path = directory / f"{name}.png"
            alpha_min, alpha_max, deviation = map(float, measure(
                path, "-format", "%[fx:minima.a] %[fx:maxima.a] %[fx:standard_deviation]", "info:").split())
            if alpha_min != 1 or alpha_max != 1 or deviation < 0.01:
                raise ValueError(f"blank or translucent capture: {path}")
            report["captures"][f"{label}/{name}"] = {"alpha": [alpha_min, alpha_max], "std": deviation}
        delta = float(measure(directory / "motion-000.png", directory / "motion-012.png",
                      "-compose", "difference", "-composite", "-alpha", "off", "-crop", "250x230+0+420",
                      "+repage", "-format", "%[fx:mean]", "info:"))
        report["motionBandDifference"][label] = delta
    for name in names[:5]:
        subprocess.run(["magick", str(args.before / f"{name}.png"),
                        str(args.after / f"{name}.png"), "+append", "-resize", "1600x",
                        str(args.output / f"{name}.png")], check=True)
    subprocess.run(["ffmpeg", "-v", "error", "-framerate", "4", "-i",
                    str(args.before / "motion-%03d.png"), "-framerate", "4", "-i",
                    str(args.after / "motion-%03d.png"), "-filter_complex",
                    "[0:v][1:v]hstack=inputs=2", "-c:v", "libx264", "-pix_fmt", "yuv420p",
                    "-y", str(args.output / "motion.mp4")], check=True)
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "captures"}, indent=2))


if __name__ == "__main__":
    main()
