# SPDX-License-Identifier: GPL-3.0-or-later
"""The visual contract: where two sets of gate captures may differ, and nowhere else.

`tools/capture-diff.py` says whether a capture is within the repository's
noise bound. A change that is *meant* (the scrubber's fill turning from black
to the accent, a themed icon replacing ours) is outside that bound by design,
and waving the capture through would also wave through a wrong pixel next to
it. So each expected change is described spatially, in version-controlled data
(`expected-visual-diff.json`):

  regions   inclusive rectangles [x0, y0, x1, y1] where pixels may change
  bands     {"outer": [x0, y0, x1, y1], "thickness": t}: only the outline of
            a rectangle, t pixels thick (a focus ring), never its interior
  must_change  the capture must actually change inside the allowed area, so a
            fix that silently stops working fails as loudly as a stray pixel

A capture with no rule is held to capture-diff's own noise bound. A pixel
counts as changed when its channel delta exceeds that capture type's noise
delta, so the scattered anti-aliasing noise every run has is not a change.
Native hardware-GL captures carry more delta-1 dithering than the Xvfb +
llvmpipe runs capture-diff's bound was measured on, so a named noise profile
(`noise_profiles` in the rules file, with its calibration) adds a second way
to be noise there: every differing pixel at delta <= max_delta, and no more
than max_pixels of them. The global bound is untouched. Acceptance requires
zero unexpected pixels. The rules are derived from real
capture pairs (`derive`), never guessed, and a changed rule needs a review.
"""

from __future__ import annotations

import fnmatch
import importlib.util
import json
import sys
from collections import Counter, deque
from dataclasses import dataclass, field
from pathlib import Path

CELL = 16          # derive: clustering grid, in pixels
PAD = 2            # derive: margin added round a derived region
RING_MIN_SIDE = 40 # derive: a cluster smaller than this is a region, never a ring
RING_SHARE = 0.98  # derive: share of a cluster's pixels on its border band to call it a ring
RING_BAND = 6      # derive: how far from the border a ring's pixels may lie


def load_capture_diff(repo_root):
    """`tools/capture-diff.py` as a module (its name has a hyphen)."""
    path = Path(repo_root) / "tools" / "capture-diff.py"
    sys.dont_write_bytecode = True  # importing it must not leave tools/__pycache__ in the tree
    spec = importlib.util.spec_from_file_location("capture_diff", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def differing_pixels(before, after, width, min_delta=0):
    """[(x, y, delta)] for every pixel whose largest channel delta is over min_delta."""
    out = []
    chunk = 3 * 1024
    for offset in range(0, len(before), chunk):
        a = before[offset:offset + chunk]
        b = after[offset:offset + chunk]
        if a == b:
            continue
        for i in range(0, len(a), 3):
            delta = max(abs(a[i] - b[i]), abs(a[i + 1] - b[i + 1]), abs(a[i + 2] - b[i + 2]))
            if delta > min_delta:
                p = (offset + i) // 3
                out.append((p % width, p // width, delta))
    return out


def in_rect(x, y, rect):
    return rect[0] <= x <= rect[2] and rect[1] <= y <= rect[3]


def in_band(x, y, band):
    x0, y0, x1, y1 = band["outer"]
    t = band["thickness"]
    if not in_rect(x, y, band["outer"]):
        return False
    return x - x0 < t or x1 - x < t or y - y0 < t or y1 - y < t


def allowed(x, y, rule):
    return any(in_rect(x, y, r) for r in rule.get("regions", [])) or any(
        in_band(x, y, b) for b in rule.get("bands", [])
    )


@dataclass
class CaptureResult:
    name: str
    status: str            # PASS or FAIL
    kind: str              # expected-change, noise, unlisted-change or error
    changed: int = 0       # pixels over the noise delta
    inside: int = 0        # of those, inside the allowed area
    outside: int = 0       # of those, outside it: the unexpected pixels
    bbox: list = field(default_factory=list)
    detail: str = ""
    colours: list = field(default_factory=list)  # commonest before->after pairs


def _bbox(points):
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    return [min(xs), min(ys), max(xs), max(ys)] if points else []


def is_3d(name, capture_diff):
    return name.startswith(capture_diff.BOUNDS_3D[0])


def noise_ok(pixels, name, capture_diff, profile):
    """Whether these differing pixels [(x, y, delta)] are only noise for this capture type."""
    max_pixels, max_delta = capture_diff.noise_bound(name)
    largest = max((d for _, _, d in pixels), default=0)
    if len(pixels) <= max_pixels and largest <= max_delta:
        return True
    if profile:
        limit = profile["max_pixels_3d"] if is_3d(name, capture_diff) else profile["max_pixels_2d"]
        return largest <= profile["max_delta"] and len(pixels) <= limit
    return False


def classify(name, before, after, rule, capture_diff, profile=None):
    """CaptureResult for one capture. before and after are (width, height, pixels) or None.

    profile is a noise profile from the rules file (see the module docstring), or None."""
    if before is None or after is None:
        side = "before" if before is None else "after"
        return CaptureResult(name, "FAIL", "error", detail=f"missing on the {side} side")
    (wa, ha, pa), (wb, hb, pb) = before, after
    if (wa, ha) != (wb, hb):
        return CaptureResult(name, "FAIL", "error", detail=f"size {wa}x{ha} vs {wb}x{hb}")
    max_pixels, max_delta = capture_diff.noise_bound(name)
    every = differing_pixels(pa, pb, wa, 0)
    largest = max((d for _, _, d in every), default=0)
    over = [p for p in every if p[2] > max_delta]
    result = CaptureResult(name, "PASS", "noise", changed=len(over), bbox=_bbox(over))
    if rule is None:
        if noise_ok(every, name, capture_diff, profile):
            return result
        result.status = "FAIL"
        result.kind = "unlisted-change"
        result.outside = len(over)
        result.detail = (f"{len(every)} px differ (bound {max_pixels}), largest delta {largest} "
                         f"(bound {max_delta}); no rule allows a change here")
        result.colours = _colours(pa, pb, wa, over)
        return result
    result.kind = "expected-change"
    inside = [p for p in over if allowed(p[0], p[1], rule)]
    result.inside = len(inside)
    result.outside = len(over) - len(inside)
    result.colours = _colours(pa, pb, wa, over)
    problems = []
    if result.outside:
        stray = [p for p in over if not allowed(p[0], p[1], rule)]
        problems.append(f"{result.outside} px changed outside the allowed area, bbox {_bbox(stray)}")
    if rule.get("must_change", True) and not inside:
        problems.append("the expected change is missing")
    # Scattered anti-aliasing noise is allowed up to the capture-diff bound, but
    # only outside the allowed area: inside it, the change's own edges are free.
    quiet = [p for p in every if p[2] <= max_delta and not allowed(p[0], p[1], rule)]
    if not noise_ok(quiet, name, capture_diff, profile):
        problems.append(f"{len(quiet)} sub-threshold px outside the allowed area exceed the noise bound")
    if problems:
        result.status = "FAIL"
        result.detail = "; ".join(problems)
    return result


def _colours(pa, pb, width, points, top=3):
    pairs = Counter()
    for x, y, _ in points:
        i = (y * width + x) * 3
        pairs[(pa[i:i + 3].hex(), pb[i:i + 3].hex())] += 1
    return [[a, b, n] for (a, b), n in pairs.most_common(top)]


def load_rules(path, set_name, profile_name=None):
    """(rule set, noise profile or None) from the rules file."""
    data = json.loads(Path(path).read_text())
    if set_name not in data["sets"]:
        raise KeyError(f"no rule set {set_name!r} in {path}: have {sorted(data['sets'])}")
    profile = None
    if profile_name:
        if profile_name not in data.get("noise_profiles", {}):
            raise KeyError(f"no noise profile {profile_name!r} in {path}")
        profile = data["noise_profiles"][profile_name]
    return data["sets"][set_name], profile


def compare_dirs(before_dir, after_dir, rules, capture_diff, include=None, profile=None):
    """[CaptureResult] for every capture in either directory (smoke, the animated one, excluded)."""
    before_dir, after_dir = Path(before_dir), Path(after_dir)
    names = sorted({p.stem for p in before_dir.glob("*.ppm")} | {p.stem for p in after_dir.glob("*.ppm")})
    names = [n for n in names if not n.startswith(capture_diff.SKIPPED)]
    if include:
        names = [n for n in names if any(fnmatch.fnmatch(n, pat) for pat in include)]
    results = []
    listed = rules.get("captures", {})
    for name in names:
        def read(directory):
            path = directory / f"{name}.ppm"
            return capture_diff.read_ppm(path) if path.exists() else None
        results.append(classify(name, read(before_dir), read(after_dir), listed.get(name), capture_diff, profile))
    # A rule for a capture neither side has is a stale rule, and a failure.
    seen = set(names)
    for name in sorted(set(listed) - seen):
        if not include:
            results.append(CaptureResult(name, "FAIL", "error", detail="a rule names a capture neither side has"))
    return results


def summarize(results):
    expected = [r for r in results if r.kind == "expected-change"]
    return {
        "captures": len(results),
        "expected_changes": len(expected),
        "noise_only": sum(1 for r in results if r.kind == "noise"),
        "unexpected_pixels": sum(r.outside for r in results),
        "failed": [r.name for r in results if r.status == "FAIL"],
        "changed_pixels_inside_rules": sum(r.inside for r in expected),
    }


# ---------------------------------------------------------------- deriving rules

def _clusters(points):
    """Group changed pixels into clusters: 16 px cells, joined when they touch (8-neighbour)."""
    cells = {}
    for x, y, _ in points:
        cells.setdefault((x // CELL, y // CELL), []).append((x, y))
    seen, groups = set(), []
    for start in cells:
        if start in seen:
            continue
        queue, members = deque([start]), []
        seen.add(start)
        while queue:
            cx, cy = queue.popleft()
            members.extend(cells[(cx, cy)])
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    nxt = (cx + dx, cy + dy)
                    if nxt in cells and nxt not in seen:
                        seen.add(nxt)
                        queue.append(nxt)
        groups.append(members)
    return groups


def _merge(rects):
    rects = [list(r) for r in rects]
    merged = True
    while merged:
        merged = False
        for i in range(len(rects)):
            for j in range(i + 1, len(rects)):
                a, b = rects[i], rects[j]
                if a[0] <= b[2] + 1 and b[0] <= a[2] + 1 and a[1] <= b[3] + 1 and b[1] <= a[3] + 1:
                    rects[i] = [min(a[0], b[0]), min(a[1], b[1]), max(a[2], b[2]), max(a[3], b[3])]
                    del rects[j]
                    merged = True
                    break
            if merged:
                break
    return rects


def derive_rule(name, before, after, capture_diff):
    """A rule describing exactly where a capture pair changed, or None when it only has noise."""
    (wa, ha, pa), (wb, hb, pb) = before, after
    if (wa, ha) != (wb, hb):
        raise ValueError(f"{name}: size {wa}x{ha} vs {wb}x{hb}")
    _, max_delta = capture_diff.noise_bound(name)
    over = differing_pixels(pa, pb, wa, max_delta)
    if not over:
        return None
    regions, bands = [], []
    for group in _clusters(over):
        box = _bbox(group)
        w, h = box[2] - box[0] + 1, box[3] - box[1] + 1
        dist = [min(x - box[0], box[2] - x, y - box[1], box[3] - y) for x, y in group]
        on_border = [d for d in dist if d < RING_BAND]
        if min(w, h) >= RING_MIN_SIDE and len(on_border) >= RING_SHARE * len(group):
            # The band is as thick as its border pixels are and no thicker: the few pixels deeper inside (under
            # 2 %) must not widen it towards a filled rectangle. They stay exact, as padded regions of their own.
            bands.append({"outer": box, "thickness": max(on_border) + 2})
            inside = [(x, y, 0) for (x, y), d in zip(group, dist) if d >= RING_BAND]
            for outliers in _clusters(inside):
                ob = _bbox(outliers)
                regions.append([max(0, ob[0] - PAD), max(0, ob[1] - PAD), min(wa - 1, ob[2] + PAD), min(ha - 1, ob[3] + PAD)])
        else:
            regions.append([max(0, box[0] - PAD), max(0, box[1] - PAD),
                            min(wa - 1, box[2] + PAD), min(ha - 1, box[3] + PAD)])
    rule = {"size": [wa, ha], "changed_pixels_observed": len(over),
            "colours_observed": _colours(pa, pb, wa, over), "regions": _merge(regions), "must_change": True}
    if bands:
        rule["bands"] = bands
    return rule


def derive_set(before_dir, after_dir, capture_diff):
    """{"captures": {name: rule}} for every capture that changed beyond its noise delta."""
    before_dir, after_dir = Path(before_dir), Path(after_dir)
    out = {}
    for path in sorted(after_dir.glob("*.ppm")):
        name = path.stem
        if name.startswith(capture_diff.SKIPPED) or not (before_dir / path.name).exists():
            continue
        rule = derive_rule(name, capture_diff.read_ppm(before_dir / path.name), capture_diff.read_ppm(path), capture_diff)
        if rule:
            out[name] = rule
    return {"captures": out}
