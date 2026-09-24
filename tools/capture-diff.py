#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Compare two directories of gate captures against the capture noise bound.

The runtime-error gate saves every capture twice: a PNG to look at and the
raw PPM this script reads (ROWPLAY_SMOKE_SCREENSHOT_DIR). For every PPM
present in both directories it prints either "identical" or the number of
differing pixels and the largest channel delta, and it marks each capture
that is outside the noise bound for its type (AGENTS.md, "Working
efficiently"):

    3D captures (replay-*, phase-*, step5*): at most 250 px, delta at most 4
    2D screens (everything else):            at most 400 px, delta at most 8

Measured over five pairs of full walks under Xvfb + llvmpipe (CI's Linux
recipe, 270 capture comparisons): 3D captures differed by at most 150
scattered pixels with a channel delta of at most 3, 2D screens by at most
262 pixels with a delta of at most 6. The bounds add margin to those
maxima. They hold for that platform only.

Usage: tools/capture-diff.py <before-dir> <after-dir> [name-substring ...]

With name substrings, only captures whose file name contains one of them
are compared (for example `replay- phase-row`). The exit status is 1 when
any compared capture is outside the bound or differs in size, 2 on a usage
error, else 0. Standard library only.
"""

import sys
from pathlib import Path

# (name prefixes, differing pixels, channel delta): the noise bound per
# capture type; a name matching no prefix is a 2D screen.
BOUNDS_3D = (("replay-", "phase-", "step5"), 250, 4)
BOUND_2D = (400, 8)


def noise_bound(name):
    """(differing pixels, channel delta) allowed as noise for a capture."""
    prefixes, pixels, delta = BOUNDS_3D
    return (pixels, delta) if name.startswith(prefixes) else BOUND_2D


def read_ppm(path):
    """(width, height, pixel bytes) of a binary P6 PPM with maxval 255."""
    data = path.read_bytes()
    fields = []
    pos = 0
    while len(fields) < 4:
        while data[pos : pos + 1].isspace():
            pos += 1
        if data[pos : pos + 1] == b"#":
            pos = data.index(b"\n", pos) + 1
            continue
        start = pos
        while not data[pos : pos + 1].isspace():
            pos += 1
        fields.append(data[start:pos])
    pos += 1  # exactly one whitespace byte ends the header
    if fields[0] != b"P6" or fields[3] != b"255":
        raise ValueError(f"{path}: not an 8-bit binary PPM")
    width, height = int(fields[1]), int(fields[2])
    pixels = data[pos : pos + width * height * 3]
    if len(pixels) != width * height * 3:
        raise ValueError(f"{path}: truncated pixel data")
    return width, height, pixels


def compare(before, after):
    """(differing pixels, largest channel delta) for two same-size buffers."""
    differing = 0
    largest = 0
    chunk = 3 * 1024  # skip identical runs of 1024 pixels at C speed
    for offset in range(0, len(before), chunk):
        a = before[offset : offset + chunk]
        b = after[offset : offset + chunk]
        if a == b:
            continue
        for i in range(0, len(a), 3):
            delta = max(
                abs(a[i] - b[i]), abs(a[i + 1] - b[i + 1]), abs(a[i + 2] - b[i + 2])
            )
            if delta:
                differing += 1
                largest = max(largest, delta)
    return differing, largest


def main(argv):
    if len(argv) < 3:
        print(__doc__.strip().splitlines()[0], file=sys.stderr)
        print("usage: capture-diff.py <before-dir> <after-dir> [name-substring ...]", file=sys.stderr)
        return 2
    before_dir, after_dir = Path(argv[1]), Path(argv[2])
    filters = argv[3:]
    names = sorted(
        path.name
        for path in before_dir.glob("*.ppm")
        if (after_dir / path.name).exists()
        and (not filters or any(f in path.name for f in filters))
    )
    if not names:
        print("no capture present in both directories", file=sys.stderr)
        return 2
    outside = 0
    for name in names:
        wa, ha, pa = read_ppm(before_dir / name)
        wb, hb, pb = read_ppm(after_dir / name)
        if (wa, ha) != (wb, hb):
            print(f"{name}: size {wa}x{ha} vs {wb}x{hb}  OUTSIDE")
            outside += 1
            continue
        if pa == pb:
            print(f"{name}: identical")
            continue
        differing, largest = compare(pa, pb)
        max_pixels, max_delta = noise_bound(name)
        noise = differing <= max_pixels and largest <= max_delta
        share = 100 * differing / (wa * ha)
        verdict = "noise" if noise else "OUTSIDE"
        print(f"{name}: {differing} px ({share:.3f} %), delta {largest}  {verdict}")
        outside += 0 if noise else 1
    print(f"{len(names)} compared, {outside} outside the noise bound")
    return 1 if outside else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
