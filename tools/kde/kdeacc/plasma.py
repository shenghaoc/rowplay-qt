# SPDX-License-Identifier: GPL-3.0-or-later
"""Plasma appearance state: snapshot, change, restore, verify.

Appearance tests change the user's live desktop, so they are a transaction:

    with SessionTransaction(...) as tx:   # snapshot first, signal handlers installed
        apply_accent(...) / apply_scheme(...) / apply_font_scale(...)
        ... measure ...
    # on exit, whatever happened (success, failure, exception, SIGINT, SIGTERM):
    #   1. re-apply the snapshot's colour scheme through Plasma's own tool,
    #   2. put the snapshot's exact kdeglobals bytes back,
    #   3. re-read everything and compare it with the snapshot.

Steps 1 and 2 are both needed: `kwriteconfig6 --delete` and the colour tool
leave KDE's own markers behind (`ColorScheme[$d]`, an explicit `ColorScheme=`
line) that are not what the file held before. Verification compares the file's
SHA-256 and the semantic state (current scheme, the portal's appearance keys,
what the bundled Qt reports); a mismatch fails the run and the summary says
what remains changed.
"""

from __future__ import annotations

import hashlib
import os
import re
import signal
import time
from dataclasses import dataclass, field
from pathlib import Path

from .shell import host_env

KDEGLOBALS = Path.home() / ".config" / "kdeglobals"
ACCENT_CANDIDATES = ("#f67400", "#8e44ad", "#27ae60", "#c0392b")  # deliberately loud, well apart


def parse_current_scheme(text):
    """The current scheme from `plasma-apply-colorscheme --list-schemes`: ' * BreezeLight (current color scheme)'."""
    for line in text.splitlines():
        match = re.match(r"\s*\*\s*(\S+)\s+\(current color scheme\)", line)
        if match:
            return match.group(1)
    return None


def parse_portal_appearance(text):
    """{'contrast': 0, 'color-scheme': 2, 'accent-color': (r, g, b)} from gdbus' ReadAll output."""
    out = {}
    for key in ("contrast", "color-scheme"):
        match = re.search(rf"'{key}': <uint32 (\d+)>", text)
        if match:
            out[key] = int(match.group(1))
    match = re.search(r"'accent-color': <\(([^)]*)\)>", text)
    if match:
        out["accent-color"] = tuple(round(float(v), 4) for v in match.group(1).split(","))
    return out


def font_fields(value):
    return value.split(",")


def scale_font(value, factor):
    """A Qt font string with its point size (field 1) multiplied by factor, rounded."""
    fields = font_fields(value)
    if len(fields) < 2:
        raise ValueError(f"not a Qt font string: {value!r}")
    fields[1] = str(round(float(fields[1]) * factor))
    return ",".join(fields)


def font_point_size(value):
    return float(font_fields(value)[1])


def sha256_of(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def hex_from_rgb_csv(csv):
    r, g, b = (int(v) for v in csv.split(","))
    return f"#{r:02x}{g:02x}{b:02x}"


def loudest_accent(current_highlight, window=None, candidates=ACCENT_CANDIDATES):
    """The candidate accent farthest (RGB distance) from the current highlight and window colours."""
    def rgb(h):
        h = h.lstrip("#")
        return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))
    refs = [rgb(current_highlight)] + ([rgb(window)] if window else [])
    def gap(c):
        return min(sum((a - b) ** 2 for a, b in zip(rgb(c), ref)) for ref in refs)
    return max(candidates, key=gap)


@dataclass
class Snapshot:
    kdeglobals_bytes: bytes
    kdeglobals_sha256: str
    scheme: str
    portal: dict
    font: str
    qt: dict = field(default_factory=dict)  # what the bundled Qt reported

    def to_dict(self):
        return {"kdeglobals_sha256": self.kdeglobals_sha256, "kdeglobals_bytes": len(self.kdeglobals_bytes),
                "scheme": self.scheme, "portal": {k: list(v) if isinstance(v, tuple) else v for k, v in self.portal.items()},
                "font": self.font, "qt": self.qt}


QT_KEYS = ("window", "windowText", "base", "button", "highlight", "accent", "colorScheme", "contrast",
           "fontFamily", "fontPointSize", "themeAccentColor")


def compare_snapshots(before: Snapshot, after: Snapshot):
    """[str] of every difference that matters; empty means restored."""
    diffs = []
    if before.kdeglobals_sha256 != after.kdeglobals_sha256:
        diffs.append(f"kdeglobals differs (sha256 {before.kdeglobals_sha256[:12]} -> {after.kdeglobals_sha256[:12]})")
    if before.scheme != after.scheme:
        diffs.append(f"colour scheme {before.scheme} -> {after.scheme}")
    for key in sorted(set(before.portal) | set(after.portal)):
        if before.portal.get(key) != after.portal.get(key):
            diffs.append(f"portal {key} {before.portal.get(key)} -> {after.portal.get(key)}")
    if before.font != after.font:
        diffs.append(f"general font {before.font} -> {after.font}")
    for key in QT_KEYS:
        if before.qt.get(key) != after.qt.get(key):
            diffs.append(f"Qt {key} {before.qt.get(key)} -> {after.qt.get(key)}")
    return diffs


class Plasma:
    """The desktop's appearance state, through the tools Plasma ships."""

    def __init__(self, runner, probe_fn, kdeglobals=KDEGLOBALS, settle=3.0):
        self.runner = runner
        self.probe_fn = probe_fn      # () -> dict of what the bundled Qt reports now
        self.kdeglobals = Path(kdeglobals)
        self.settle = settle

    def _host(self, argv, tag):
        return self.runner.run(argv, env=host_env(), tag=tag)

    def read_scheme(self):
        return parse_current_scheme(self._host(["plasma-apply-colorscheme", "--list-schemes"], "plasma-scheme").out)

    def read_portal(self):
        done = self._host(["gdbus", "call", "--session", "--dest", "org.freedesktop.portal.Desktop",
                           "--object-path", "/org/freedesktop/portal/desktop", "--method",
                           "org.freedesktop.portal.Settings.ReadAll", "['org.freedesktop.appearance']"], "plasma-portal")
        return parse_portal_appearance(done.out)

    def read_font(self):
        return self._host(["kreadconfig6", "--file", "kdeglobals", "--group", "General", "--key", "font"], "plasma-font").out.strip()

    def read_selection(self):
        return self._host(["kreadconfig6", "--file", "kdeglobals", "--group", "Colors:Selection", "--key", "BackgroundNormal"], "plasma-selection").out.strip()

    def snapshot(self):
        data = self.kdeglobals.read_bytes()
        return Snapshot(data, hashlib.sha256(data).hexdigest(), self.read_scheme(), self.read_portal(),
                        self.read_font(), self.probe_fn() or {})

    def apply_scheme(self, name, accent=None):
        argv = ["plasma-apply-colorscheme"] + (["--accent-color", accent] if accent else []) + [name]
        done = self._host(argv, "plasma-apply-scheme")
        time.sleep(self.settle)
        return done

    def apply_font_scale(self, factor):
        new = scale_font(self.read_font(), factor)
        done = self._host(["kwriteconfig6", "--file", "kdeglobals", "--group", "General", "--key", "font", new], "plasma-apply-font")
        time.sleep(self.settle)
        return new, done

    def restore(self, snap: Snapshot):
        """Re-apply the scheme, restore the file's exact bytes, and return the differences left (empty: restored)."""
        if snap.scheme:
            self.apply_scheme(snap.scheme)
        tmp = self.kdeglobals.with_name(self.kdeglobals.name + ".kdeacc-restore")
        tmp.write_bytes(snap.kdeglobals_bytes)
        os.replace(tmp, self.kdeglobals)
        time.sleep(self.settle)
        return compare_snapshots(snap, self.snapshot())


class SessionTransaction:
    """snapshot -> (changes) -> restore and verify, on every way out of the block."""

    def __init__(self, plasma: Plasma, allow: bool):
        self.plasma = plasma
        self.allow = allow
        self.snap = None
        self.left = []          # differences after restoring; empty means restored
        self.restored = False
        self._old = {}

    def __enter__(self):
        if not self.allow:
            raise PermissionError("changing the desktop session needs --allow-session-changes")
        self.snap = self.plasma.snapshot()
        def stop(signum, _frame):
            raise KeyboardInterrupt(f"signal {signum}")
        for sig in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            self._old[sig] = signal.signal(sig, stop)
        return self

    def __exit__(self, exc_type, exc, tb):
        # Never interrupted half way: a second signal here is ignored until restored.
        for sig in self._old:
            signal.signal(sig, signal.SIG_IGN)
        try:
            self.left = self.plasma.restore(self.snap)
            self.restored = not self.left
        finally:
            for sig, handler in self._old.items():
                signal.signal(sig, handler)
        return False  # the original exception, if any, still propagates
