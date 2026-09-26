#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Verify runtime frame equality, measure pixels and compose local evidence.

Pixels are read with Pillow and numpy when they are installed, and with
ImageMagick (`magick`) otherwise; the clip needs FFmpeg either way.
"""

import argparse
import json
from pathlib import Path
import subprocess

try:
    import numpy as np
    from PIL import Image
except ImportError:  # the Linux host this began on had ImageMagick instead
    np = None


def frames(directory):
    result = {}
    for line in (directory / "gate.log").read_text().splitlines():
        if "ART_FRAME " in line:
            record = json.loads(line.split("ART_FRAME ", 1)[1])
            result[record["name"]] = record
    return result


def measure(*args):
    return subprocess.check_output(["magick", "-limit", "thread", "1", *map(str, args)], text=True).strip()


def pixels(path):
    return np.asarray(Image.open(path).convert("RGBA")).astype(np.int16)


def stats(path):
    """(alpha min, alpha max, RGB standard deviation), all on a 0-1 scale."""
    if np is None:
        return tuple(map(float, measure(
            path, "-format", "%[fx:minima.a] %[fx:maxima.a] %[fx:standard_deviation]", "info:").split()))
    p = pixels(path)
    return p[..., 3].min() / 255, p[..., 3].max() / 255, float(p[..., :3].std() / 255)


# A fixed 250 x 230 patch of near water, left of the oar sweep and above the
# HUD. Phase 1 took it at (0, 420); in the current 2400 x 1600 layout that
# patch lies across the far bank and the sky, so it measured the horizon.
BAND = (0, 1150, 250, 230)


def band_difference(first, last):
    """Mean RGB difference of the water band: visual change, not perceived speed."""
    x, y, w, h = BAND
    if np is None:
        return float(measure(first, last, "-compose", "difference", "-composite", "-alpha", "off",
                             "-crop", f"{w}x{h}+{x}+{y}", "+repage", "-format", "%[fx:mean]", "info:"))
    a, b = pixels(first), pixels(last)
    return float(np.abs(a - b)[y:y + h, x:x + w, :3].mean() / 255)


def differing(first, second):
    """(pixels that differ, largest channel delta) between two captures."""
    a, b = pixels(first)[..., :3], pixels(second)[..., :3]
    delta = np.abs(a - b).max(-1)
    return int((delta > 0).sum()), int(delta.max())


def side_by_side(left, right, output):
    if np is None:
        subprocess.run(["magick", str(left), str(right), "+append", "-resize", "1600x", str(output)], check=True)
        return
    a, b = Image.open(left).convert("RGB"), Image.open(right).convert("RGB")
    sheet = Image.new("RGB", (a.width + b.width, max(a.height, b.height)))
    sheet.paste(a, (0, 0))
    sheet.paste(b, (a.width, 0))
    sheet.resize((1600, round(sheet.height * 1600 / sheet.width)), Image.LANCZOS).save(output)


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
    report = {"matchedRuntimeFrames": len(names), "captures": {}, "motionBandDifference": {},
              "sameStateCaptures": {}}
    for label, directory, recorded in (("before", args.before, before), ("after", args.after, after)):
        for name in names:
            path = directory / f"{name}.png"
            alpha_min, alpha_max, deviation = stats(path)
            if alpha_min != 1 or alpha_max != 1 or deviation < 0.01:
                raise ValueError(f"blank or translucent capture: {path}")
            report["captures"][f"{label}/{name}"] = {"alpha": [alpha_min, alpha_max], "std": deviation}
        report["motionBandDifference"][label] = band_difference(
            directory / "motion-000.png", directory / "motion-012.png")
        # World-fixed water and environment: style-medium and motion-000 are
        # the same replay state (Medium at the approved moment) grabbed at
        # different wall-clock times. Anything animated on its own clock, such
        # as a scrolling texture, would make them differ.
        if recorded["style-medium"]["frame"][1:] != recorded["motion-000"]["frame"][1:]:
            raise ValueError(f"{label}: style-medium and motion-000 are not the same state")
        if np is not None:
            count, delta = differing(directory / "style-medium.png", directory / "motion-000.png")
            if count:
                raise ValueError(f"{label}: the same replay state rendered {count} different pixels "
                                 f"(delta {delta}); something moves on its own clock")
            report["sameStateCaptures"][label] = {"differingPixels": count, "maxDelta": delta}
    for name in names[:5]:
        side_by_side(args.before / f"{name}.png", args.after / f"{name}.png", args.output / f"{name}.png")
    subprocess.run(["ffmpeg", "-v", "error", "-framerate", "4", "-i",
                    str(args.before / "motion-%03d.png"), "-framerate", "4", "-i",
                    str(args.after / "motion-%03d.png"), "-filter_complex",
                    "[0:v][1:v]hstack=inputs=2", "-c:v", "libx264", "-pix_fmt", "yuv420p",
                    "-y", str(args.output / "motion.mp4")], check=True)
    (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: value for key, value in report.items() if key != "captures"}, indent=2))


if __name__ == "__main__":
    main()
