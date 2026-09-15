#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Vendor the golden parity fixtures from a rowplay-studio checkout.

Usage: tools/vendor-fixtures.py <rowplay-studio-checkout>

Copies Tests/RowPlayCoreTests/Fixtures/*.json (plus Concept2/*) into
tests/fixtures/, then rewrites tests/fixtures/manifest.json with the SHA-256 of
every file and the source commit (read from the checkout's git HEAD).

Locally generated fixtures (GENERATED_FIXTURES below, produced by the
generators in tools/) are re-hashed in place so a Studio re-vendor never drops
them; regenerate them with their own generators first.
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

# Fixtures generated in this repository from the rowplay web sources (not
# vendored from Studio; see PROVENANCE.md): each generator evaluates the web
# at a pinned commit under Node. Regenerate them with their generators before
# re-vendoring after a reference change. The manifest entry records the
# generator's commit; an existing entry is refreshed in place rather than
# rewritten, so a recorded pin survives re-vendoring.
GENERATED_FIXTURES = {
    "replay-row-phase-parity.json": "4d96480e7c6fb382f800555bd3aa463d9fe5b1a6",
    "replay-rig-phase-parity.json": "011e8303b66b4d2265a6f1ec8b3ed9d8ed497086",
}


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
    manifest_path = DEST / "manifest.json"
    existing = (
        json.loads(manifest_path.read_text()) if manifest_path.exists() else {"fixtures": []}
    )
    for rel, generated_commit in GENERATED_FIXTURES.items():
        target = DEST / rel
        if not target.exists():
            continue
        prior = next((e for e in existing["fixtures"] if e["path"] == rel), None)
        if prior is None:
            prior = {
                "path": rel,
                "source": {
                    "repository": "https://github.com/shenghaoc/rowplay",
                    "commit": generated_commit,
                    "path": f"tests/fixtures/{rel}",
                },
            }
        data = target.read_bytes()
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
