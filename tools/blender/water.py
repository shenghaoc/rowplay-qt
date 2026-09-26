# SPDX-License-Identifier: GPL-3.0-or-later
"""The water normal's height field (Blender Phase 3), without Blender.

A periodic height field on an 8 m tile, made of integer wave vectors on the
tile's torus so the texture tiles, drawn from a wind-sea spectrum:

- wavelengths log-uniform from 6 cm to 2.8 m, so fine chop, medium ripples
  and a long, low swell are all present and no one frequency dominates;
- directions spread about a 28 degree wind as cos^6 of half the angle, with
  28 % of the components about a second direction 74 degrees away, at 0.55
  of the slope;
- slope amplitude falling as (wavelength / 2.8 m)^0.45, so the short chop
  stays finer than the swell;
- an 18 % seeded amplitude variation across the tile (gusts).

The slopes are scaled to an RMS of 0.11. The Phase 1 map's RMS was 0.0757,
nearly all of it along one tile axis (0.070 against 0.029 across). Spread
over every direction, that same RMS read flat near the boat in Qt, where the
passing texture is the water's motion cue; 0.08, 0.11 and 0.14 were compared
at the approved frame, and 0.11 keeps visible ripples near the boat while the
far water stays subdued (docs/blender-audit.md, "Phase 3"). The scene's normal
strengths are unchanged. `RowingWater.qml` repeats
the tile every 8 m (6000 m / 750): the boat moves over it, it never scrolls.
`build_all.py` writes the normals through Blender's image API; the tests in
`test_water.py` check the field.
"""

import math

import numpy as np

SEED = 20260926
WATER = {"tile": 8.0, "size": 512, "components": 360, "wavelength": [0.06, 2.8],
         "wind": 28.0, "spread": 6.0, "second": {"share": 0.28, "offset": 74.0, "slope": 0.55},
         "tilt": 0.45, "gust": 0.18, "rmsSlope": 0.11}


def components(spec=WATER, seed=SEED):
    """(m, q) -> (slope, phase): integer wave vectors drawn from the spectrum."""
    rng = np.random.default_rng(seed)
    tile = spec["tile"]
    lam_min, lam_max = spec["wavelength"]
    wind = math.radians(spec["wind"])
    second = spec["second"]
    comps = {}
    tries = 0
    while len(comps) < spec["components"]:
        tries += 1
        if tries > spec["components"] * 400:
            raise ValueError("water spectrum: too few distinct wave vectors")
        lam = math.exp(rng.uniform(math.log(lam_min), math.log(lam_max)))
        is_second = rng.uniform() < second["share"]
        centre = wind + (math.radians(second["offset"]) if is_second else 0.0)
        theta = rng.uniform(-math.pi, math.pi)
        if rng.uniform() > abs(math.cos((theta - centre) / 2.0)) ** spec["spread"]:
            continue
        k = tile / lam
        m, q = round(k * math.cos(theta)), round(k * math.sin(theta))
        if (m, q) == (0, 0) or (m, q) in comps or (-m, -q) in comps:
            continue
        lam_real = tile / math.hypot(m, q)
        if not lam_min * 0.8 <= lam_real <= lam_max * 1.25:
            continue
        slope = (lam_real / lam_max) ** spec["tilt"] * (second["slope"] if is_second else 1.0)
        comps[(m, q)] = (slope, rng.uniform(0, 2 * math.pi))
    return comps


def gusts(spec=WATER, seed=SEED):
    """Up to six low wave vectors of the seeded amplitude variation."""
    rng = np.random.default_rng(seed + 1)
    terms = []
    for _ in range(6):
        m, q = int(rng.integers(-2, 3)), int(rng.integers(-2, 3))
        phase = rng.uniform(0, 2 * math.pi)
        if (m, q) != (0, 0):
            terms.append((m, q, phase))
    return terms


def slopes(u, v, comps, terms, spec=WATER):
    """The unscaled slope field (d/du, d/dv) at tile angles u, v in [0, 2 pi)."""
    dx, dy = np.zeros_like(u), np.zeros_like(v)
    for (m, q), (slope, phase) in comps.items():
        k = math.hypot(m, q)
        wave = np.cos(m * u + q * v + phase)
        dx += slope * (m / k) * wave
        dy += slope * (q / k) * wave
    envelope = np.zeros_like(u)
    for m, q, phase in terms:
        envelope += np.cos(m * u + q * v + phase)
    return dx, dy, envelope


def normals(spec=WATER, seed=SEED):
    """Unit tangent-space normals (+Z up), rows bottom-up as Blender stores them.

    Returns (normals (size, size, 3), the manifest's record of the spectrum)."""
    size = spec["size"]
    comps = components(spec, seed)
    terms = gusts(spec, seed)
    v, u = np.mgrid[:size, :size] / size * (2 * math.pi)
    dx, dy, envelope = slopes(u, v, comps, terms, spec)
    envelope = 1.0 + spec["gust"] * envelope / max(np.abs(envelope).max(), 1e-6)
    dx, dy = dx * envelope, dy * envelope
    scale = spec["rmsSlope"] / math.sqrt((dx * dx + dy * dy).mean())
    result = np.stack((-dx * scale, -dy * scale, np.ones_like(dx)), -1)
    result /= np.linalg.norm(result, axis=-1, keepdims=True)
    lams = sorted(spec["tile"] / math.hypot(m, q) for m, q in comps)
    record = dict(spec, components=len(comps), wavelengthDrawn=[round(lams[0], 4), round(lams[-1], 4)])
    return result, record
