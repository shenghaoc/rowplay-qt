# SPDX-License-Identifier: GPL-3.0-or-later
"""AppImage packages: build, inventory, compare, and prove no KDE stack was bundled.

The canonical build is `tools/package/linux.sh` unchanged, in an Ubuntu 24.04
container (`tools/kde/ubuntu-package.sh`, the release runner's OS): on a
Fedora 44 host linuxdeploy corrupts RELR-packed system libraries and the
result crashes before main (ADR 0018). Both packages under comparison are built
by the same script in the same image, each with its own target directory.
"""

from __future__ import annotations

import hashlib
import re
import subprocess
from pathlib import Path

# Paths of things the portable AppImage must never carry: the KDE Frameworks, Kirigami, Plasma,
# KDE's Quick Controls styles and the Breeze QML style. Matching is by whole path segment or
# library stem, so an unrelated `CheckDelegate.qml` (which contains "kde") is not a hit.
FORBIDDEN_PATHS = (
    ("KDE Frameworks library", re.compile(r"(^|/)lib(KF|KF6|KF5)[A-Za-z0-9]*\.so")),
    ("KDE Frameworks directory", re.compile(r"(^|/)(KF6|KF5|kf6|kf5)(/|$)")),
    ("Kirigami", re.compile(r"(^|/)[^/]*[Kk]irigami[^/]*(/|$)")),
    ("Plasma", re.compile(r"(^|/)[^/]*[Pp]lasma[^/]*(/|$)")),
    ("qqc2-desktop-style / org.kde.desktop", re.compile(r"qqc2-desktop-style|org/kde/desktop|org\.kde\.desktop")),
    ("KDE QML modules", re.compile(r"(^|/)org/kde(/|$)")),
    ("Breeze style or icons", re.compile(r"(^|/)[^/]*[Bb]reeze[^/]*(/|$)")),
    ("KConfig / KI18n / KIO", re.compile(r"(^|/)(lib)?(KF6)?(KConfig|KI18n|KIO)[A-Za-z]*(\.so|/|$)")),
)
# DT_NEEDED entries that mean a KDE library is linked in.
FORBIDDEN_NEEDED = re.compile(r"^lib(KF[56][A-Za-z0-9]*|Kirigami[A-Za-z0-9]*|plasma[A-Za-z0-9-]*|LayerShellQt[A-Za-z0-9]*|KWin[A-Za-z0-9]*)\.so")


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for block in iter(lambda: handle.read(1 << 20), b""):
            digest.update(block)
    return digest.hexdigest()


def inventory(root):
    """{relative path: size} for every file and symlink under an extracted AppDir."""
    root = Path(root)
    out = {}
    for path in sorted(root.rglob("*")):
        if path.is_file() or path.is_symlink():
            out[str(path.relative_to(root))] = path.lstat().st_size
    return out


def forbidden_in_paths(paths):
    """[(reason, path)] for every path that names a forbidden component."""
    hits = []
    for path in paths:
        for reason, pattern in FORBIDDEN_PATHS:
            if pattern.search(path):
                hits.append((reason, path))
                break
    return hits


def needed_libs(readelf_output):
    """The DT_NEEDED library names from `readelf -d` output."""
    return re.findall(r"\(NEEDED\)\s+Shared library: \[([^\]]+)\]", readelf_output)


def forbidden_needed(libs):
    return sorted({lib for lib in libs if FORBIDDEN_NEEDED.match(lib)})


def scan_needed(runner, root, paths):
    """{path: [forbidden NEEDED libs]} over every ELF file in the inventory."""
    root = Path(root)
    bad = {}
    for rel in paths:
        path = root / rel
        if path.is_symlink() or not path.is_file():
            continue
        with open(path, "rb") as handle:
            if handle.read(4) != b"\x7fELF":
                continue
        done = subprocess.run(["readelf", "-d", str(path)], capture_output=True, text=True)
        hit = forbidden_needed(needed_libs(done.stdout))
        if hit:
            bad[rel] = hit
    return bad


def plugin_inventory(inv):
    """{'platforms': [...], 'platformthemes': [...], ...} from usr/plugins, and the QML module directories."""
    out = {}
    for rel in inv:
        match = re.match(r"usr/plugins/([^/]+)/([^/]+)$", rel)
        if match:
            out.setdefault(match.group(1), []).append(match.group(2))
    qml = sorted({rel.split("/")[2] for rel in inv if rel.startswith("usr/qml/") and rel.count("/") >= 3})
    return {"plugins": {k: sorted(v) for k, v in sorted(out.items())}, "qml_modules": qml}


def compare(a, b):
    """Side-by-side numbers for two package records (each: bytes, files, sha256, inventory)."""
    changed = sorted(p for p in set(a["inventory"]) & set(b["inventory"]) if a["inventory"][p] != b["inventory"][p])
    return {
        "bytes": {"a": a["bytes"], "b": b["bytes"], "delta": b["bytes"] - a["bytes"]},
        "files": {"a": a["files"], "b": b["files"], "delta": b["files"] - a["files"]},
        "added": sorted(set(b["inventory"]) - set(a["inventory"])),
        "removed": sorted(set(a["inventory"]) - set(b["inventory"])),
        "size_changed": [(p, a["inventory"][p], b["inventory"][p]) for p in changed],
    }


def launch_check_line(build_log):
    """The `launch-check: ok — ...` line linux.sh's launch check prints, or None."""
    for line in build_log.splitlines():
        if line.startswith("launch-check:"):
            return line
    return None


def extract(runner, appimage, dest):
    """Extract an AppImage (no FUSE) into dest/squashfs-root; returns that path."""
    dest = Path(dest)
    dest.mkdir(parents=True, exist_ok=True)
    done = runner.run([str(Path(appimage).resolve()), "--appimage-extract"], cwd=dest, tag="appimage-extract", timeout=300)
    if not done.ok:
        raise RuntimeError(f"appimage extraction failed: {done.err[-400:]}")
    return dest / "squashfs-root"
