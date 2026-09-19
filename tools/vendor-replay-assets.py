#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Vendor the rowplay replay assets into assets/replay/ byte for byte.

Copies `rowplay-rigs-v3.glb`, `rowplay-athlete-v4.glb`,
`rowplay-athlete-v4.contract.json` and the `environments/` texture sets from
the git-ignored `reference/rowplay` checkout, verifies every byte against the
expected SHA-256 table below (the two GLB hashes and the contract hash are
reviewed literals; the 39 texture hashes are parsed out of the upstream
`environments/README.md`, which pins them per file), and rewrites the
vendored table in `ASSET_PROVENANCE.md`.

Usage: tools/vendor-replay-assets.py [--reference PATH]

Refuses to run without the reference checkout and fails on any hash or size
mismatch, so refreshing the assets is always an explicit, reviewed act.
"""

import argparse
import hashlib
import re
import shutil
import sys
from pathlib import Path

UPSTREAM_COMMIT = "011e8303b66b4d2265a6f1ec8b3ed9d8ed497086"

# Reviewed artifacts. Sizes and hashes recorded in ASSET_PROVENANCE.md; the
# GLB hashes also appear in the upstream README / contract JSON.
ARTIFACTS = {
    "rowplay-rigs-v3.glb": (
        733_864,
        "31418f4808b30fa786830129b0b637fc025b6e5ddbb539d848fc8cab74806925",
    ),
    "rowplay-athlete-v4.glb": (
        4_584_320,
        "a564a4dbd4922e2ba76ef21a23f5bf0eb1b0180846548f9d7110e55ffd8f760e",
    ),
    "rowplay-athlete-v4.contract.json": (
        27_593,
        "62dd814ba6113ca57419aac42905233f8e07589846be5c3a25bbadcf55d63dd3",
    ),
}

# Creators per Poly Haven family, from environments/README.md (CC0-1.0).
CREATORS = {
    "aerial-grass-rock": "Rob Tuytel",
    "bark-brown-01": "Rob Tuytel",
    "brown-planks-03": "Rob Tuytel",
    "brushed-concrete-2": "Dimitrios Savva (photography), Dario Barresi (processing)",
    "cobblestone-floor-03": "Rob Tuytel",
    "concrete-floor-painted": "Rob Tuytel",
    "dry-river-pebbles": "Amal Kumar",
    "forest-leaves-04": "Rob Tuytel",
    "forrest-ground-01": "Rob Tuytel",
    "leafy-grass": "Charlotte Baglioni",
    "rock-01": "Rob Tuytel",
    "snow-02": "Rob Tuytel",
    "wood-floor": "Dimitrios Savva",
}

POLYHAVEN_TITLE = {
    "aerial-grass-rock": "Aerial Grass Rock",
    "bark-brown-01": "Bark Brown 01",
    "brown-planks-03": "Brown Planks 03",
    "brushed-concrete-2": "Brushed Concrete 2",
    "cobblestone-floor-03": "Cobblestone Floor 03",
    "concrete-floor-painted": "Concrete Floor Painted",
    "dry-river-pebbles": "Dry River Pebbles",
    "forest-leaves-04": "Forest Leaves 04",
    "forrest-ground-01": "Forest Ground 01",
    "leafy-grass": "Leafy Grass",
    "rock-01": "Rock 01",
    "snow-02": "Snow 02",
    "wood-floor": "Wood Floor",
}

ENV_TABLE_RE = re.compile(
    r"\|\s*`(?P<path>[A-Za-z0-9._-]+/[A-Za-z0-9._-]+\.jpg)`\s*\|[^|]*\|\s*`(?P<sha>[0-9a-f]{64})`\s*\|"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def environment_hashes(readme: Path) -> dict[str, str]:
    """The per-file SHA-256 table the upstream environments README pins."""
    hashes = {}
    for match in ENV_TABLE_RE.finditer(readme.read_text(encoding="utf-8")):
        hashes[match.group("path")] = match.group("sha")
    if len(hashes) != 39:
        sys.exit(f"expected 39 pinned texture hashes, parsed {len(hashes)}")
    return hashes


def copy_checked(src: Path, dst: Path, size: int, expected: str) -> None:
    if not src.is_file():
        sys.exit(f"missing source asset: {src}")
    shutil.copyfile(src, dst)
    actual_size = dst.stat().st_size
    if actual_size != size:
        sys.exit(f"{dst.name}: size {actual_size} != expected {size}")
    actual = sha256(dst)
    if actual != expected:
        sys.exit(f"{dst.name}: SHA-256 {actual} != expected {expected}")
    print(f"  ok {dst.relative_to(dst.parents[2])} ({size} bytes)")


def provenance_rows(repo_root: Path, vendored: list[tuple[str, int, str, str]]) -> str:
    lines = []
    for relpath, size, digest, licence in vendored:
        lines.append(
            f"| `assets/replay/{relpath}` | rowplay | "
            f"`static/replay-assets/{relpath}` | `{UPSTREAM_COMMIT[:12]}` | "
            f"`{digest}` | {licence} ({size} B) |"
        )
    return "\n".join(lines)


def rewrite_provenance(repo_root: Path, rows: str) -> None:
    path = repo_root / "ASSET_PROVENANCE.md"
    text = path.read_text(encoding="utf-8")
    marker = "## Vendored assets"
    tail_marker = "## Reference sources (not vendored)"
    head, _, rest = text.partition(marker)
    _, _, tail = rest.partition(tail_marker)
    body = (
        marker
        + "\n\nEvery file below is a byte-for-byte copy verified by "
        "`tools/vendor-replay-assets.py` (sizes and SHA-256 of the committed "
        "bytes; the same table is asserted by "
        "`crates/rowplay-app/tests/asset_hashes.rs`). The V3 rig pack and the "
        "V4 athlete are MIT from rowplay; the V4 anatomy derives from Dan "
        "Ulrich / Blender Studio Human Base Meshes v1.4.1 (CC0-1.0) with MIT "
        "modifications. The environment maps are resized 512 px derivatives of "
        "Poly Haven 1K JPEGs, CC0-1.0, creators named per family.\n\n"
        "| Asset | Source repository | Source path | Commit | SHA-256 | Licence |\n"
        "| --- | --- | --- | --- | --- | --- |\n"
        + rows
        + "\n\n"
    )
    path.write_text(head + body + tail_marker + tail, encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--reference",
        type=Path,
        default=None,
        help="rowplay checkout (default: <repo>/reference/rowplay)",
    )
    parser.add_argument(
        "--emit-rust",
        action="store_true",
        help="also print the Rust expectation table for asset_hashes.rs",
    )
    args = parser.parse_args()

    repo_root = Path(__file__).resolve().parents[1]
    reference = args.reference or repo_root / "reference" / "rowplay"
    source = reference / "static" / "replay-assets"
    if not source.is_dir():
        sys.exit(f"reference checkout not found at {reference} (see AGENTS.md)")

    target = repo_root / "assets" / "replay"
    target.mkdir(parents=True, exist_ok=True)

    print("vendoring reviewed artifacts:")
    vendored: list[tuple[str, int, str, str]] = []
    for name, (size, digest) in ARTIFACTS.items():
        copy_checked(source / name, target / name, size, digest)
        licence = "MIT (rowplay)"
        if name == "rowplay-athlete-v4.glb":
            licence = "MIT (rowplay) + CC0-1.0 (Human Base Meshes v1.4.1 base)"
        vendored.append((name, size, digest, licence))

    print("vendoring environment texture sets:")
    env_readme = source / "environments" / "README.md"
    hashes = environment_hashes(env_readme)
    # Staleness guard (same class as convert-locales.mjs's pinned-commit
    # check): the texture hash table is parsed from the checkout's own
    # README, so a checkout carrying different pins would verify itself.
    # The default reference checkout must still pin exactly the bytes the
    # committed vendored README pins; an explicit --reference is the
    # deliberate update path and skips this loudly.
    vendored_readme = repo_root / "assets" / "replay" / "environments" / "README.md"
    if args.reference is not None:
        print(
            f"note: --reference {reference} given; skipping the texture-pin "
            "guard against the committed assets/replay/environments/README.md.",
            file=sys.stderr,
        )
    elif vendored_readme.is_file():
        vendored_hashes = environment_hashes(vendored_readme)
        if vendored_hashes != hashes:
            changed = sorted(set(vendored_hashes) ^ set(hashes)) or "hash values"
            sys.exit(
                f"{env_readme}: texture pin table differs from the committed "
                f"assets/replay/environments/README.md ({changed}); check out "
                "the pinned commit (docs/source-map.md) or re-review and "
                "update the vendored pins deliberately"
            )
    (target / "environments").mkdir(parents=True, exist_ok=True)
    shutil.copyfile(env_readme, target / "environments" / "README.md")
    for family in sorted(CREATORS):
        fam_dir = target / "environments" / family
        fam_dir.mkdir(parents=True, exist_ok=True)
        for jpeg in sorted((source / "environments" / family).glob("*.jpg")):
            relpath = f"environments/{family}/{jpeg.name}"
            # The upstream README table keys `<family>/<file>.jpg`.
            digest = hashes[f"{family}/{jpeg.name}"]
            size = jpeg.stat().st_size
            copy_checked(jpeg, fam_dir / jpeg.name, size, digest)
            vendored.append(
                (
                    relpath,
                    size,
                    digest,
                    f"CC0-1.0 (Poly Haven {POLYHAVEN_TITLE[family]}, {CREATORS[family]})",
                )
            )

    rewrite_provenance(repo_root, provenance_rows(repo_root, vendored))
    print(f"vendored {len(vendored)} files; ASSET_PROVENANCE.md updated")
    if args.emit_rust:
        emit_rust_table(vendored)


def emit_rust_table(vendored: list[tuple[str, int, str, str]]) -> None:
    """Print the Rust expectation table for asset_hashes.rs on a refresh."""
    print("\n// Rust expectation table (crates/rowplay-app/tests/asset_hashes.rs):")
    for relpath, size, digest, _ in sorted(vendored):
        print(f'    ("{relpath}", {size}, "{digest}"),')


if __name__ == "__main__":
    main()
