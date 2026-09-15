#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Vendor the golden parity fixtures from a rowplay-studio checkout.

Usage: tools/vendor-fixtures.py <rowplay-studio-checkout>

Copies Tests/RowPlayCoreTests/Fixtures/*.json (plus Concept2/*) into
tests/fixtures/, then rewrites tests/fixtures/manifest.json with the SHA-256 of
every file and the source commit (read from the checkout's git HEAD).
"""
from __future__ import annotations

import hashlib
import json
import pathlib
import shutil
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
DEST = REPO / "tests" / "fixtures"
FILES = [
    "duration-band-parity.json",
    "performance-predictor-parity.json",
    "stroke-pose-parity.json",
    "replay-race-gap-parity.json",
    "replay-race-result-parity.json",
    "replay-rival-sources-parity.json",
    "replay-current-main-motion.json",
    "replay-current-main-grips.json",
    "replay-current-main-equipment.json",
    "replay-current-main-2d.json",
    "Concept2/REDACTION.md",
    "Concept2/rower-steady.fixture.json",
    "Concept2/rower-interval.fixture.json",
    "Concept2/ski-steady.fixture.json",
    "Concept2/bike-steady.fixture.json",
]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    studio = pathlib.Path(sys.argv[1]).resolve()
    src = studio / "Tests" / "RowPlayCoreTests" / "Fixtures"
    commit = subprocess.check_output(["git", "-C", str(studio), "rev-parse", "HEAD"], text=True).strip()
    entries = []
    for rel in FILES:
        source = src / rel
        target = DEST / rel
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        data = target.read_bytes()
        entries.append(
            {
                "path": rel,
                "bytes": len(data),
                "sha256": hashlib.sha256(data).hexdigest(),
                "source": {
                    "repository": "https://github.com/shenghaoc/rowplay-studio",
                    "commit": commit,
                    "path": f"Tests/RowPlayCoreTests/Fixtures/{rel}",
                },
            }
        )
    entries.sort(key=lambda e: e["path"])
    # The row-phase fixture is generated from the rowplay web repo, not
    # vendored from Studio (see PROVENANCE.md); keep its manifest entry so a
    # re-vendor does not strand an unrecorded file on disk.
    local = DEST / "replay-row-phase-parity.json"
    if local.exists():
        existing = json.loads((DEST / "manifest.json").read_text()) if (DEST / "manifest.json").exists() else {"fixtures": []}
        prior = next(
            (e for e in existing["fixtures"] if e["path"] == "replay-row-phase-parity.json"),
            None,
        )
        data = local.read_bytes()
        if prior is None:
            prior = {
                "path": "replay-row-phase-parity.json",
                "source": {
                    "repository": "https://github.com/shenghaoc/rowplay",
                    "commit": "4d96480e7c6fb382f800555bd3aa463d9fe5b1a6",
                    "path": "tests/fixtures/replay-row-phase-parity.json",
                },
            }
        prior["bytes"] = len(data)
        prior["sha256"] = hashlib.sha256(data).hexdigest()
        entries.append(prior)
        entries.sort(key=lambda e: e["path"])
    manifest = {
        "schema": "rowplay-qt.fixtures.manifest.v1",
        "description": "Golden parity fixtures vendored from rowplay-studio. Regenerate with tools/vendor-fixtures.py.",
        "fixtures": entries,
    }
    (DEST / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"vendored {len(entries)} fixtures from {commit}; update tests/fixtures/PROVENANCE.md")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
