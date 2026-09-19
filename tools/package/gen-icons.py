#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate the platform icon files from the vendored rowplay icon.

Source: `assets/icon/rowplay-icon-512.png` (rowplay `static/icon-512.png`,
MIT; see ASSET_PROVENANCE.md). Outputs, both committed and SHA-256-pinned by
`crates/rowplay-app/tests/asset_hashes.rs`:

- `assets/icon/rowplay-qt.icns` — the macOS bundle icon (16–512 px).
- `assets/icon/rowplay-qt.ico`  — the Windows installer / shortcut icon
  (16–256 px, PNG-compressed entries).

Needs Pillow. The outputs are byte-deterministic for a given Pillow version
(measured with 11.3.0: two consecutive runs are identical), which is what
lets the pins hold; a Pillow bump that changes the encoding shows up as a
pin failure and is then re-pinned deliberately, like any asset change.

Usage:
    tools/package/gen-icons.py           # (re)write both files
    tools/package/gen-icons.py --check   # regenerate in memory, compare bytes
"""

import argparse
import io
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "assets" / "icon" / "rowplay-icon-512.png"
OUTPUTS = {
    ROOT / "assets" / "icon" / "rowplay-qt.icns": (
        "ICNS",
        [(16, 16), (32, 32), (64, 64), (128, 128), (256, 256), (512, 512)],
    ),
    ROOT / "assets" / "icon" / "rowplay-qt.ico": (
        "ICO",
        [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    ),
}


def render(fmt: str, sizes: list[tuple[int, int]]) -> bytes:
    try:
        from PIL import Image
    except ImportError:  # pragma: no cover - environment dependent
        sys.exit("gen-icons.py needs Pillow: pip install pillow")
    with Image.open(SOURCE) as image:
        source = image.convert("RGBA")
    if source.size != (512, 512):
        sys.exit(f"{SOURCE} is {source.size}, expected 512x512")
    buffer = io.BytesIO()
    source.save(buffer, format=fmt, sizes=sizes)
    return buffer.getvalue()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="compare freshly rendered bytes against the committed files",
    )
    args = parser.parse_args()
    failed = False
    for path, (fmt, sizes) in OUTPUTS.items():
        rendered = render(fmt, sizes)
        if args.check:
            current = path.read_bytes() if path.exists() else b""
            status = "ok" if current == rendered else "DIFFERS"
            failed |= status != "ok"
            print(f"{path.relative_to(ROOT)}: {status} ({len(rendered)} B)")
        else:
            path.write_bytes(rendered)
            print(f"wrote {path.relative_to(ROOT)} ({len(rendered)} B)")
    if failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
