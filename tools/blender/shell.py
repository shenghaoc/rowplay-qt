# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate the single scull and its sculling oars (Blender Phase 2).

A procedural asset under ADR 0016's source rule: this script is its source.
Every part keeps the V3 rig pack's node name and material role, so the app's
material walker and anchors apply unchanged; build.rs checks the names and
roles against the vendored pack. Geometry is authored in the rig's own
coordinates (metres; x to starboard, y up, z toward the stern; the boat root
on the waterline) and converted to Blender's Z-up axes only when a mesh is
created. The numbers that bind to the rig are named after the Rust constants
they must match; everything else is shape.
"""

import json
import math
from pathlib import Path
import struct

import bpy
import bmesh
from mathutils import Matrix, Vector

from canonical import bound_attributes, canonicalize_pack

# ---- Contract (rowplay-viewmodel replay::anchors, pose::geometry,
# replay::equipment; rowplay-core replay::row_equipment) --------------------
HALF_LENGTH = 3.9                    # the 7.8 m shell
OARLOCK_PIVOT = (0.88, 0.51, 0.28)   # anchors::OARLOCK_PIVOT, the oar-rig clone origin
GRIP_START, GRIP_END = -0.82, -0.50  # the V3 grip the hands close on (oar-rig x)
GRIP_DROP = -0.04                    # pose::geometry::ROW_GRIP_DROP, the grip axis height
GRIP_RADIUS = 0.023                  # row_equipment::SCULL_GRIP_RADIUS
BLADE_OFFSET = (1.82, -0.06, 0.0)    # equipment::BLADE_OFFSET, the blade leaf's origin
TRIANGLE_BUDGET = 60_000             # boat plus both oars, as drawn

# ---- Fit to the athlete the app draws --------------------------------------
# Measured by skinning the V4 athlete with the replay's own pose frames over
# one stroke of demo 1001 (docs/blender-audit.md, "Phase 2"). The pelvis
# rests at y 0.121-0.126 under the ischia and 0.099 on the centre line, 0.16
# aft of the seat origin; the soles lie on a 44-degree plane from the heels
# (y 0.045, z 0.64) to the ball of the foot (y 0.162, z 0.76); the feet
# reach |x| 0.218 and the heels |x| 0.121-0.177.
SEAT_TOP = 0.118
SEAT_CHANNEL = 0.097
SEAT_Z = -0.09                       # pad centre, relative to the moving seat origin
FLOOR_Y = 0.034                      # cockpit floor on the centre line (bob is +-0.02)
RAIL_X = 0.078
SOLE_HEEL = (0.040, 0.638)           # (y, z) under the soles, 4 mm clear of them
SOLE_BALL = (0.157, 0.763)

COCKPIT_BOW, COCKPIT_STERN = -0.86, 0.99
SKIN = 0.007


def lerp(a, b, t):
    return a + (b - a) * t


def smooth(t):
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


def interp(points, x):
    """Monotone cubic (Fritsch-Carlson) through sorted (x, y) points."""
    xs = [p[0] for p in points]
    ys = [p[1] for p in points]
    if x <= xs[0]:
        return ys[0]
    if x >= xs[-1]:
        return ys[-1]
    n = len(xs)
    d = [(ys[i + 1] - ys[i]) / (xs[i + 1] - xs[i]) for i in range(n - 1)]
    m = [d[0]] + [0.0 if d[i - 1] * d[i] <= 0 else (d[i - 1] + d[i]) / 2
                  for i in range(1, n - 1)] + [d[-1]]
    for i in range(n - 1):
        if d[i] == 0:
            m[i] = m[i + 1] = 0.0
        else:
            a, b = m[i] / d[i], m[i + 1] / d[i]
            s = a * a + b * b
            if s > 9:
                t = 3 / math.sqrt(s)
                m[i], m[i + 1] = t * a * d[i], t * b * d[i]
    i = max(j for j in range(n - 1) if xs[j] <= x)
    h = xs[i + 1] - xs[i]
    t = (x - xs[i]) / h
    return ((2 * t ** 3 - 3 * t ** 2 + 1) * ys[i] + (t ** 3 - 2 * t ** 2 + t) * h * m[i]
            + (-2 * t ** 3 + 3 * t ** 2) * ys[i + 1] + (t ** 3 - t ** 2) * h * m[i + 1])


# Plan form (half-beam at the sheer): widest along the stretcher, where the
# athlete's feet reach |x| 0.218; long fine entries at both ends.
BEAM = [(-3.90, 0.004), (-3.72, 0.040), (-3.40, 0.082), (-2.90, 0.130),
        (-2.20, 0.175), (-1.40, 0.207), (-0.60, 0.226), (0.30, 0.232),
        (0.95, 0.230), (1.60, 0.212), (2.40, 0.172), (3.10, 0.118),
        (3.60, 0.062), (3.90, 0.004)]
SHEER = [(-3.90, 0.212), (-3.00, 0.240), (-1.80, 0.270), (-1.00, 0.284),
         (1.10, 0.284), (2.00, 0.268), (3.10, 0.236), (3.90, 0.206)]
# The keel sits 3 cm under the waterline: deep enough for the +-2 cm bob and
# the roll, and the water hides everything below it in the app.
KEEL = [(-3.90, 0.190), (-3.55, 0.115), (-3.00, 0.030), (-2.20, -0.018),
        (-1.20, -0.030), (1.40, -0.030), (2.30, -0.025), (3.10, 0.026),
        (3.60, 0.118), (3.90, 0.188)]
BOB = 0.0194          # the rower's largest bob (course::profile, measured)
SECTION_N = 3.2   # superellipse exponent: a full U with near-vertical topsides


def beam(z):
    return interp(BEAM, z)


def sheer(z):
    return interp(SHEER, z)


def keel(z):
    return interp(KEEL, z)


def crown(z):
    """Deck camber on the centre line above the sheer."""
    if z < 0:
        return 0.040 * smooth((z + HALF_LENGTH) / 1.6)
    return 0.036 * smooth((HALF_LENGTH - z) / 1.6)


def deck_y(x, z):
    b = max(beam(z), 1e-4)
    return sheer(z) + crown(z) * (1 - min(abs(x) / b, 1.0) ** 1.8)


def section_point(z, s, inset=0.0):
    """Hull section, s in [0, 1] from the keel to the sheer (starboard half)."""
    b, top, bottom = beam(z), sheer(z), keel(z)
    depth = max(top - bottom, 1e-4)
    theta = s * math.pi / 2
    x = max(b - inset, 0.0) * math.sin(theta) ** (2 / SECTION_N)
    y = bottom + inset + (depth - inset) * (1 - math.cos(theta) ** (2 / SECTION_N))
    return x, y


def section_at_height(z, y, inset=0.0):
    lo, hi = 0.0, 1.0
    for _ in range(40):
        mid = (lo + hi) / 2
        if section_point(z, mid, inset)[1] < y:
            lo = mid
        else:
            hi = mid
    return section_point(z, hi, inset)[0]


def stations(z0, z1, count, ends=True):
    out = []
    for i in range(count + 1):
        t = i / count
        if ends:
            t = 0.5 - 0.5 * math.cos(math.pi * t)
        out.append(lerp(z0, z1, t))
    return out


# ---- Mesh building ---------------------------------------------------------
# Rings from ring() turn counter-clockwise seen from +axis_u x axis_v. Every
# builder's winding is then checked, not assumed: closed pieces by signed
# volume, open skins against a known outward direction (orient_* below).

class Mesh:
    def __init__(self):
        self.verts = []
        self.faces = []
        self.colors = None

    def add(self, point):
        self.verts.append(tuple(float(c) for c in point))
        return len(self.verts) - 1

    def grid(self, rows, close=False, loop=False):
        """Quads (r,c)->(r,c+1)->(r+1,c+1)->(r+1,c); close wraps columns,
        loop joins the last row back to the first."""
        index = [[self.add(p) for p in row] for row in rows]
        width = len(index[0])
        span = width if close else width - 1
        pairs = list(range(len(index) - 1)) + ([len(index) - 1] if loop else [])
        for r in pairs:
            n = (r + 1) % len(index)
            for c in range(span):
                a, b = index[r][c], index[r][(c + 1) % width]
                d, e = index[n][c], index[n][(c + 1) % width]
                self.faces.append((a, b, e, d))
        return index

    def fan(self, ids, apex, reverse=False):
        a = self.add(apex)
        for i in range(len(ids)):
            j = (i + 1) % len(ids)
            self.faces.append((ids[j], ids[i], a) if reverse else (ids[i], ids[j], a))

    def cap_start(self, ids, apex=None):
        """Close a grid's first (closed) row, consistently with the grid."""
        self.fan(ids, apex or self.centroid(ids), reverse=True)

    def cap_end(self, ids, apex=None):
        """Close a grid's last (closed) row, consistently with the grid."""
        self.fan(ids, apex or self.centroid(ids))

    def centroid(self, ids):
        return tuple(sum(self.verts[i][k] for i in ids) / len(ids) for k in range(3))

    def cap(self, ids):
        self.fan(ids, self.centroid(ids))

    def merge(self, other):
        base = len(self.verts)
        self.verts += other.verts
        self.faces += [tuple(i + base for i in f) for f in other.faces]
        if other.colors is not None or self.colors is not None:
            raise ValueError("colour attributes are assigned after assembly")
        return self

    def flip(self):
        self.faces = [tuple(reversed(f)) for f in self.faces]
        return self

    def triangles(self):
        return sum(len(f) - 2 for f in self.faces)

    def face_normals(self):
        for f in self.faces:
            p = [Vector(self.verts[i]) for i in f]
            n = Vector((0.0, 0.0, 0.0))
            for i in range(len(p)):
                a, b = p[i], p[(i + 1) % len(p)]
                n += Vector(((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y)))
            centre = sum(p, Vector((0.0, 0.0, 0.0))) / len(p)
            yield centre, n / 2   # area-weighted


def signed_volume(mesh):
    total = 0.0
    for f in mesh.faces:
        a = Vector(mesh.verts[f[0]])
        for i in range(1, len(f) - 1):
            b, c = Vector(mesh.verts[f[i]]), Vector(mesh.verts[f[i + 1]])
            total += a.dot(b.cross(c)) / 6
    return total


def orient_closed(mesh):
    if signed_volume(mesh) < 0:
        mesh.flip()
    return mesh


def orient_open(mesh, outward):
    """Flip an open skin so its area-weighted normals agree with outward(p)."""
    score = sum(n.dot(Vector(outward(c))) for c, n in mesh.face_normals())
    if score < 0:
        mesh.flip()
    return mesh


def ring(center, axis_u, axis_v, radius_u, radius_v, count):
    return [tuple(center[k] + axis_u[k] * radius_u * math.cos(2 * math.pi * i / count)
                  + axis_v[k] * radius_v * math.sin(2 * math.pi * i / count) for k in range(3))
            for i in range(count)]


def revolve(profile, segments, axis_point, axis_u, axis_v, along):
    """Solid of revolution; a zero radius at either end of the profile is a
    cone tip rather than a ring."""
    mesh = Mesh()
    first = 1 if profile[0][1] <= 1e-9 else 0
    last = len(profile) - 1 if profile[-1][1] <= 1e-9 else len(profile)
    rows = [ring(axis_point(s), axis_u, axis_v, r, r, segments) for s, r in profile[first:last]]
    index = mesh.grid(rows, close=True)
    mesh.cap_start(index[0], axis_point(profile[0][0]) if first else None)
    mesh.cap_end(index[-1], axis_point(profile[-1][0]) if last < len(profile) else None)
    return orient_closed(mesh)


def lathe_x(profile, segments, y0=0.0, z0=0.0):
    """Solid of revolution about an axis parallel to x through (y0, z0)."""
    return revolve(profile, segments, lambda x: (x, y0, z0), (0, 1, 0), (0, 0, 1), "x")


def lathe_y(profile, segments, x0=0.0, z0=0.0):
    """Solid of revolution about a vertical axis through (x0, z0)."""
    return revolve(profile, segments, lambda y: (x0, y, z0), (0, 0, 1), (1, 0, 0), "y")


def tube(points, radius, segments):
    """Round tube along a polyline (parallel-transported rings), capped."""
    mesh = Mesh()
    rows = []
    n = len(points)
    prev = None
    for i, p in enumerate(points):
        a = Vector(points[max(i - 1, 0)])
        b = Vector(points[min(i + 1, n - 1)])
        d = (b - a).normalized()
        if prev is None:
            helper = Vector((0, 1, 0)) if abs(d.y) < 0.9 else Vector((1, 0, 0))
            u = d.cross(helper).normalized()
        else:
            u = (prev - d * prev.dot(d)).normalized()
        prev = u
        v = d.cross(u).normalized()
        r = radius(i / (n - 1)) if callable(radius) else radius
        rows.append(ring(p, u, v, r, r, segments))
    index = mesh.grid(rows, close=True)
    mesh.cap_start(index[0], points[0])
    mesh.cap_end(index[-1], points[-1])
    return orient_closed(mesh)


def segment(a, b, count):
    return [tuple(lerp(a[k], b[k], i / count) for k in range(3)) for i in range(count + 1)]


def rounded_rect(half_x, half_z, radius, per_corner):
    radius = min(radius, half_x, half_z)
    out = []
    for cx, cz, start in ((half_x - radius, -half_z + radius, -math.pi / 2),
                          (half_x - radius, half_z - radius, 0.0),
                          (-half_x + radius, half_z - radius, math.pi / 2),
                          (-half_x + radius, -half_z + radius, math.pi)):
        for i in range(per_corner + 1):
            a = start + (math.pi / 2) * i / per_corner
            out.append((cx + radius * math.cos(a), cz + radius * math.sin(a)))
    return out


def slab(outline, y_bottom, y_top, bevel, center=(0.0, 0.0, 0.0), top_fn=None, rings=2):
    """An outline (x, z) extruded from y_bottom to y_top with rounded edges;
    top_fn(x, z) shapes the top surface."""
    mesh = Mesh()
    cx, cy, cz = center
    mid_x = sum(p[0] for p in outline) / len(outline)
    mid_z = sum(p[1] for p in outline) / len(outline)
    top = top_fn or (lambda x, z: 0.0)
    span = max(p[0] for p in outline) - min(p[0] for p in outline)
    shrink = 1 - 2 * bevel / max(span, 1e-6)

    def row(scale, y_fn):
        return [(cx + mid_x + (x - mid_x) * scale, cy + y_fn(mid_x + (x - mid_x) * scale, mid_z + (z - mid_z) * scale),
                 cz + mid_z + (z - mid_z) * scale) for x, z in outline]

    rows = [row(shrink, lambda x, z: y_bottom)]
    for i in range(1, rings + 1):
        a = (math.pi / 2) * i / rings
        rows.append(row(lerp(shrink, 1.0, math.sin(a)), lambda x, z, a=a: y_bottom + bevel * (1 - math.cos(a))))
    for i in range(rings + 1):
        a = (math.pi / 2) * i / rings
        rows.append(row(lerp(1.0, shrink, 1 - math.cos(a)),
                        lambda x, z, a=a: y_top - bevel + bevel * math.sin(a) + top(x, z)))
    for scale in (shrink * 0.66, shrink * 0.33):
        rows.append(row(scale, lambda x, z: y_top + top(x, z)))
    index = mesh.grid(rows, close=True)
    mesh.cap_start(index[0], (cx + mid_x, cy + y_bottom, cz + mid_z))
    mesh.cap_end(index[-1], (cx + mid_x, cy + y_top + top(mid_x, mid_z), cz + mid_z))
    return orient_closed(mesh)


def place(mesh, origin, basis):
    """p -> origin + basis . p for a right-handed basis (u_x, u_y, u_z)."""
    ux, uy, uz = (Vector(b) for b in basis)
    if ux.dot(uy.cross(uz)) <= 0:
        raise ValueError("left-handed placement would turn the mesh inside out")
    o = Vector(origin)
    mesh.verts = [tuple(o + ux * p[0] + uy * p[1] + uz * p[2]) for p in mesh.verts]
    return mesh


def mirror_x(mesh):
    out = Mesh()
    out.verts = [(-x, y, z) for x, y, z in mesh.verts]
    out.faces = [tuple(reversed(f)) for f in mesh.faces]
    return out


def both_sides(mesh):
    return Mesh().merge(mesh).merge(mirror_x(mesh))


def strip(rows, inside):
    """An open profile swept along rows and capped at both ends: rails, caps
    and stripes that sit on another surface. The sweep faces away from
    inside(p); each end cap faces away from the next row in."""
    mesh = Mesh()
    mesh.grid(rows)
    orient_open(mesh, lambda c: Vector(c) - Vector(inside(c)))

    def centroid(row):
        return sum((Vector(p) for p in row), Vector((0.0, 0.0, 0.0))) / len(row)

    for end, neighbour in ((rows[0], rows[1]), (rows[-1], rows[-2])):
        cap = Mesh()
        cap.cap([cap.add(p) for p in end])
        away = centroid(end) - centroid(neighbour)
        mesh.merge(orient_open(cap, lambda c, away=away: away))
    return mesh


# ---- Hull, decks and trim ------------------------------------------------------

def hull():
    """Outer skin from the keel to the sheer, closed at both stems."""
    mesh = Mesh()
    zs = stations(-HALF_LENGTH, HALF_LENGTH, 132)[1:-1]
    ss = [i / 22 for i in range(23)]
    rows = []
    for z in zs:
        half = [section_point(z, s) for s in ss]
        rows.append([(-x, y, z) for x, y in reversed(half[1:])] + [(x, y, z) for x, y in half])
    index = mesh.grid(rows)
    for z, ids in ((-HALF_LENGTH, index[0]), (HALF_LENGTH, index[-1])):
        stem = Mesh()
        stem.verts = [mesh.verts[i] for i in ids]
        apex = stem.add((0.0, (sheer(z) + keel(z)) / 2, z))
        stem.faces = [(c, c + 1, apex) for c in range(len(ids) - 1)]
        orient_open(stem, lambda c, z=z: (0.0, 0.0, math.copysign(1.0, z)))
        mesh.merge(stem)
    return orient_open(mesh, lambda c: (c[0], c[1] - (sheer(c[2]) + keel(c[2])) / 2, 0.0))


def wet_band(mesh):
    """Per-vertex masks for the hull material: R scales roughness and G the
    clearcoat roughness, so the band at the waterline reads as a wet sheen.
    Meshes without the attribute keep a mask of 1 (Qt 6.11 vertex pipeline)."""
    mesh.colors = []
    for x, y, z in mesh.verts:
        wet = 1 - smooth((y - 0.012) / 0.040)
        mesh.colors.append((lerp(1.0, 0.34, wet), lerp(1.0, 0.40, wet), 1.0, 1.0))
    return mesh


def canvas(z0, z1, count):
    mesh = Mesh()
    rows = []
    across = 14
    for z in stations(z0, z1, count, ends=False):
        b = beam(z) + 0.0015
        rows.append([(b * i / across, deck_y(b * i / across, z), z) for i in range(-across, across + 1)])
    mesh.grid(rows)
    return orient_open(mesh, lambda c: (0.0, 1.0, 0.0))


def bow_deck():
    return canvas(-HALF_LENGTH + 0.012, COCKPIT_BOW, 64)


def stern_deck():
    return canvas(COCKPIT_STERN, HALF_LENGTH - 0.012, 60)


def accent_strakes():
    """White sheer stripes, deck spines, a breakwater and the bow ball."""
    out = Mesh()
    rows = []
    for z in stations(-HALF_LENGTH + 0.05, HALF_LENGTH - 0.05, 110):
        b, top = beam(z), sheer(z)
        rows.append([(b + 0.0005, top - 0.020, z), (b + 0.0030, top - 0.017, z),
                     (b + 0.0030, top - 0.007, z), (b + 0.0005, top - 0.004, z)])
    out.merge(both_sides(strip(rows, lambda c: (beam(c[2]) - 0.002, sheer(c[2]) - 0.012, c[2]))))
    for z0, z1 in ((-3.70, COCKPIT_BOW - 0.30), (COCKPIT_STERN + 0.03, 3.62)):
        path = [(0.0, deck_y(0.0, z) + 0.001, z) for z in stations(z0, z1, 24, ends=False)]
        out.merge(tube(path, lambda t: 0.0055 * (0.55 + 0.45 * math.sin(math.pi * t)), 8))
    apex_z, foot_z, spread = COCKPIT_BOW - 0.30, COCKPIT_BOW - 0.04, 0.17
    arm = Vector((spread, 0.0, foot_z - apex_z)).normalized()
    across = Vector((arm.z, 0.0, -arm.x))          # horizontal, normal to the arm
    rows = []
    for i in range(-8, 9):
        t = abs(i) / 8
        side = math.copysign(1.0, i) if i else 0.0
        centre = Vector((side * spread * t, 0.0, lerp(apex_z, foot_z, t)))
        normal = Vector((across.x * (side or 1.0), 0.0, across.z)) if i else Vector((0.0, 0.0, -1.0))
        base = deck_y(centre.x, centre.z) - 0.002
        h = 0.012 + 0.030 * t
        a, b = centre - normal * 0.003, centre + normal * 0.003
        rows.append([(a.x, base, a.z), (a.x, base + h, a.z), (b.x, base + h, b.z), (b.x, base, b.z)])
    plate = Mesh()
    index = plate.grid(rows, close=True)
    plate.cap_start(index[0])
    plate.cap_end(index[-1])
    out.merge(orient_closed(plate))
    ball = lathe_y([(sheer(-HALF_LENGTH) - 0.044 + 0.022 * (1 - math.cos(math.pi * i / 10)),
                     0.022 * math.sin(math.pi * i / 10)) for i in range(11)], 16, 0.0, -HALF_LENGTH - 0.010)
    out.merge(ball)
    return out


def keel_fin():
    """A thin swept fin under the stern, below the water at the top of the bob."""
    root_y, tip_y = keel(2.05) + 0.015, keel(2.05) - 0.105
    rows = []
    for y, za, zb in ((root_y, 1.93, 2.19), (tip_y, 2.03, 2.21)):
        row = []
        for i in list(range(13)) + list(range(11, 0, -1)):
            t = i / 12
            side = 1 if len(row) < 13 else -1
            thick = 0.0065 * math.sin(math.pi * t) ** 0.6 + 0.0004
            row.append((side * thick, y, lerp(za, zb, t)))
        rows.append(row)
    mesh = Mesh()
    index = mesh.grid(rows, close=True)
    mesh.cap_start(index[0])
    mesh.cap_end(index[-1])
    return orient_closed(mesh)


# ---- Cockpit -----------------------------------------------------------------

def floor_y(x):
    """The cockpit floor: a shallow U, clear of the heels at |x| 0.149 and above
    the water at the largest bob plus roll (4.5 degrees) at its edges."""
    return FLOOR_Y + 0.20 * x * x


def cockpit_tub():
    """Inner skin of the open cockpit, its U-shaped floor and the end walls."""
    mesh = Mesh()
    rows = []
    for z in stations(COCKPIT_BOW, COCKPIT_STERN, 36, ends=False):
        top = sheer(z) - 0.004
        x_floor = section_at_height(z, floor_y(0.2) + 0.006, SKIN)
        wall = [(section_at_height(z, y, SKIN), y, z)
                for y in [lerp(top, floor_y(x_floor) + 0.004, i / 8) for i in range(9)]]
        floor = [(x, floor_y(x), z) for x in [lerp(wall[-1][0], -wall[-1][0], i / 16) for i in range(1, 16)]]
        rows.append(wall + floor + [(-x, y, z) for x, y, _ in reversed(wall)])
    mesh.grid(rows)
    orient_open(mesh, lambda c: (-c[0], 0.30 - c[1], 0.0))
    for z, row in ((COCKPIT_BOW, rows[0]), (COCKPIT_STERN, rows[-1])):
        wall = Mesh()
        lower = [wall.add(p) for p in row]
        across = 12
        upper = [wall.add((x, deck_y(x, z), z)) for x in
                 [beam(z) * i / across for i in range(across, -across - 1, -1)]]
        i = j = 0
        while i < len(lower) - 1 or j < len(upper) - 1:
            if j >= len(upper) - 1 or (i < len(lower) - 1 and i / (len(lower) - 1) <= j / (len(upper) - 1)):
                wall.faces.append((lower[i], lower[i + 1], upper[j]))
                i += 1
            else:
                wall.faces.append((lower[i], upper[j + 1], upper[j]))
                j += 1
        orient_open(wall, lambda c, z=z: (0.0, 0.0, -math.copysign(1.0, z)))
        mesh.merge(wall)
    return mesh


def gunwales():
    """Saxboard caps along both cockpit sides, over the skin's top edge."""
    rows = []
    for z in stations(COCKPIT_BOW - 0.02, COCKPIT_STERN + 0.02, 40, ends=False):
        b, top = beam(z), sheer(z)
        rows.append([(b - SKIN - 0.012, top - 0.010, z), (b - SKIN - 0.014, top + 0.004, z),
                     (b - 0.010, top + 0.010, z), (b + 0.006, top + 0.006, z), (b + 0.008, top - 0.012, z)])
    return both_sides(strip(rows, lambda c: (beam(c[2]) - 0.006, sheer(c[2]) - 0.004, c[2])))


def bulkheads():
    """Low coamings across the two cockpit ends (the V3 trim accents)."""
    out = Mesh()
    for z, lean in ((COCKPIT_BOW - 0.006, -1), (COCKPIT_STERN + 0.006, 1)):
        rows = []
        across = 16
        for i in range(-across, across + 1):
            x = (beam(z) - 0.004) * i / across
            base = deck_y(x, z) - 0.004
            rows.append([(x, base, z), (x, base + 0.022, z + 0.002 * lean),
                         (x, base + 0.026, z + 0.008 * lean), (x, base, z + 0.012 * lean)])
        out.merge(strip(rows, lambda c, z=z, lean=lean: (c[0], deck_y(c[0], z) - 0.002, z + 0.006 * lean)))
    return out


def slide_rails():
    star = Mesh()
    z0, z1 = -0.455, 0.365
    x = RAIL_X
    profile = [(-0.007, 0.0), (-0.007, 0.004), (-0.0035, 0.005), (-0.0035, 0.009), (-0.006, 0.010),
               (-0.006, 0.013), (0.006, 0.013), (0.006, 0.010), (0.0035, 0.009), (0.0035, 0.005),
               (0.007, 0.004), (0.007, 0.0)]
    rail = Mesh()
    index = rail.grid([[(x + px, floor_y(x) + py - 0.001, z) for px, py in profile] for z in (z0, z1)], close=True)
    rail.cap_start(index[0])
    rail.cap_end(index[1])
    star.merge(orient_closed(rail))
    for z in (z0 - 0.012, z1 + 0.012):
        star.merge(slab(rounded_rect(0.012, 0.012, 0.004, 3), 0.0, 0.030, 0.003, center=(x, floor_y(x) - 0.001, z)))
    return both_sides(star)


# ---- Stretcher -----------------------------------------------------------------

def sole_frame():
    """Heel point, u up the board toward the toes, n into the board."""
    (y0, z0), (y1, z1) = SOLE_HEEL, SOLE_BALL
    u = Vector((0.0, y1 - y0, z1 - z0)).normalized()
    n = Vector((0.0, -u.z, u.y))
    return Vector((0.0, y0, z0)), u, n


def board_basis():
    origin, u, n = sole_frame()
    # local x across, local y out of the board (toward the feet), local z up it
    return origin, (Vector((1, 0, 0)), -n, u)


BOARD_FROM, BOARD_TO = 0.075, 0.330   # along the sole plane from the heel point


def foot_stretcher():
    """The footboard under the balls of the feet. It starts above the heels:
    at the finish they sink 2 cm past its plane, onto the heel cups and floor
    (measured over the stroke, docs/blender-audit.md)."""
    origin, basis = board_basis()
    _, u, n = sole_frame()
    out = Mesh()
    half = (BOARD_TO - BOARD_FROM) / 2
    out.merge(place(slab(rounded_rect(0.205, half, 0.03, 4), -0.012, 0.0, 0.004),
                    origin + u * (BOARD_FROM + half), basis))
    return out


HEEL = (0.146, 0.672)   # (|x|, z) of the heels where they rest at the finish


def heel_cups():
    """Rubber cups on the floor around the back of each heel, open toward the
    board."""
    hx, hz = HEEL
    rows = []
    for h, r in ((-0.002, 0.040), (0.022, 0.040), (0.027, 0.037), (0.027, 0.033), (-0.002, 0.033)):
        rows.append([(hx + r * math.cos(math.pi * i / 12), floor_y(hx) + h, hz - 0.9 * r * math.sin(math.pi * i / 12))
                     for i in range(13)])
    cup = Mesh()
    index = cup.grid(rows, loop=True)
    first = [index[r][0] for r in range(len(index))]
    last = [index[r][-1] for r in range(len(index))]
    cup.fan(first, cup.centroid(first))
    cup.fan(last, cup.centroid(last), reverse=True)
    return both_sides(orient_closed(cup))


def stretcher_hardware():
    origin, u, n = sole_frame()
    out = Mesh()
    top = origin + u * 0.285 + n * 0.030
    out.merge(tube([tuple(top + Vector((-0.19, 0, 0))), tuple(top + Vector((0.19, 0, 0)))], 0.0085, 12))
    star = Mesh()
    b = Vector((section_at_height(0.975, 0.075, SKIN) - 0.012, 0.075, 0.975))
    for start, radius in ((origin + u * 0.270 + n * 0.030, 0.0075), (origin + u * (BOARD_FROM + 0.02) + n * 0.018, 0.0065)):
        a = start + Vector((0.19, 0, 0))
        star.merge(tube(segment(tuple(a), tuple(b), 4), radius, 10))
    star.merge(slab(rounded_rect(0.010, 0.20, 0.004, 2), 0.0, 0.010, 0.002, center=(0.150, floor_y(0.150) - 0.001, 0.80)))
    out.merge(both_sides(star))
    return out


# ---- Rigging ---------------------------------------------------------------------

PIN_BASE_Y, PIN_TOP_Y = 0.388, 0.548
PLATE_TOP_Y = 0.400
SEAT_WASHER = (0.400, 0.412, 0.026)
TOP_CAP = (0.522, 0.532, 0.018)


def riggers():
    """Three-stay riggers to the pin plates, mounted outside the hull."""
    px, py, pz = OARLOCK_PIVOT
    star = Mesh()
    star.merge(slab(rounded_rect(0.034, 0.032, 0.012, 3), -0.005, 0.005, 0.002,
                    center=(px - 0.018, PLATE_TOP_Y - 0.005, pz)))
    node = Vector((px - 0.040, PLATE_TOP_Y - 0.008, pz))
    for dz in (0.300, -0.300):
        z = pz + dz
        mount = Vector((beam(z) + 0.012, 0.268, z))
        star.merge(tube(segment(tuple(mount), tuple(node), 8), 0.0115, 14))
        star.merge(slab(rounded_rect(0.008, 0.030, 0.004, 2), -0.022, 0.022, 0.003, center=(beam(z) + 0.004, 0.268, z)))
    low = Vector((beam(pz) + 0.010, 0.118, pz))
    star.merge(tube(segment(tuple(low), tuple(node - Vector((0, 0.006, 0))), 8), 0.0105, 14))
    star.merge(slab(rounded_rect(0.008, 0.030, 0.004, 2), -0.020, 0.020, 0.003, center=(beam(pz) + 0.002, 0.118, pz)))
    return both_sides(star)


def oarlocks():
    """Pin, seat washer and top cap on each pivot. Round, so the static swivel
    clears the sleeve through the whole sweep and roll."""
    px, py, pz = OARLOCK_PIVOT
    star = Mesh()
    star.merge(lathe_y([(PIN_BASE_Y, 0.0065), (PIN_TOP_Y, 0.0065)], 12, px, pz))
    y0, y1, r = SEAT_WASHER
    star.merge(lathe_y([(y0, r - 0.002), (y0 + 0.002, r), (y1 - 0.002, r), (y1, r - 0.002)], 24, px, pz))
    y0, y1, r = TOP_CAP
    star.merge(lathe_y([(y0, r - 0.002), (y0 + 0.002, r), (y1 - 0.002, r), (y1, r - 0.003)], 24, px, pz))
    star.merge(lathe_y([(y1, 0.011), (PIN_TOP_Y - 0.002, 0.011), (PIN_TOP_Y, 0.009)], 6, px, pz))
    return both_sides(star)


# ---- Seat carriage (relative to the moving seat origin) ------------------------

SEAT_TRAVEL = (-0.180, 0.040, 0.260)   # the seat origin's z over the stroke (frame seatZ)
WHEEL_Y = 0.064   # a 15.5 mm wheel on the 48 mm rail top, under the 79 mm plate


def seat_pad():
    def top(x, z):
        channel = -(SEAT_TOP - SEAT_CHANNEL) * max(0.0, 1 - (x / 0.028) ** 2)
        front = -0.010 * smooth((z - SEAT_Z - 0.06) / 0.06)
        return channel + front
    return slab(rounded_rect(0.152, 0.126, 0.045, 5), 0.090, SEAT_TOP, 0.009,
                center=(0.0, 0.0, SEAT_Z), top_fn=top, rings=3)


def seat_carriage():
    out = slab(rounded_rect(0.118, 0.105, 0.02, 3), 0.079, 0.090, 0.003, center=(0.0, 0.0, SEAT_Z))
    for dz in (-0.078, 0.078):
        out.merge(both_sides(lathe_x([(RAIL_X - 0.014, 0.0035), (RAIL_X + 0.014, 0.0035)], 8, WHEEL_Y, SEAT_Z + dz)))
    return out


def seat_rollers():
    out = Mesh()
    for dz in (-0.078, 0.078):
        out.merge(both_sides(lathe_x([(RAIL_X - 0.0075, 0.009), (RAIL_X - 0.0075, 0.0140), (RAIL_X - 0.006, 0.0155),
                                      (RAIL_X + 0.006, 0.0155), (RAIL_X + 0.0075, 0.0140), (RAIL_X + 0.0075, 0.009)],
                                     20, WHEEL_Y, SEAT_Z + dz)))
    return out


def seat_guides():
    out = Mesh()
    for dx in (-0.0105, 0.0105):
        out.merge(both_sides(slab(rounded_rect(0.0018, 0.105, 0.012, 2), 0.050, 0.080, 0.001,
                                  center=(RAIL_X + dx, 0.0, SEAT_Z))))
    return out


# ---- Oar rig (the right oar; the app turns the left instance pi about y) --------

SHAFT_Y = GRIP_DROP
NECK_X = BLADE_OFFSET[0] - 0.285


def shaft():
    """Tapered carbon shaft from the grip into the blade's neck collar, which
    hides its end (NECK_COLLAR is 1 mm wider)."""
    end = NECK_X + 0.025
    profile = [(GRIP_END - 0.006, 0.0160), (-0.30, 0.0172), (-0.10, 0.0184), (0.20, 0.0184),
               (0.70, 0.0176), (1.20, 0.0163), (end, 0.0140)]
    xs = [lerp(GRIP_END - 0.006, end, i / 26) for i in range(27)]
    return lathe_x([(x, interp(profile, x)) for x in xs], 20, SHAFT_Y, 0.0)


def grip():
    samples = []
    for i in range(25):
        t = i / 24
        samples.append((lerp(GRIP_START, GRIP_END, t),
                        0.0216 + (GRIP_RADIUS + 0.0003 - 0.0216) * math.sin(math.pi * t) ** 0.5))
    return lathe_x(samples, 24, SHAFT_Y, 0.0)


def handle_cap():
    x0 = GRIP_START - 0.012
    return lathe_x([(x0, 0.0), (x0, 0.0170), (x0 + 0.002, 0.0205), (x0 + 0.012, 0.0214), (x0 + 0.012, 0.0)],
                   24, SHAFT_Y, 0.0)


def collar():
    x0, x1 = -0.068, -0.040
    return lathe_x([(x0, 0.0362), (x0 + 0.003, 0.0445), (x0 + 0.006, 0.0470), (x1 - 0.006, 0.0470),
                    (x1 - 0.003, 0.0445), (x1, 0.0362)], 28, SHAFT_Y, 0.0)


def blade_sleeve():
    x0, x1 = -0.108, 0.165
    return lathe_x([(x0, 0.0190), (x0 + 0.008, 0.0330), (x0 + 0.016, 0.0356), (x1 - 0.030, 0.0356),
                    (x1 - 0.008, 0.0300), (x1, 0.0192)], 24, SHAFT_Y, 0.0)


# ---- Blade (the leaf, authored flat with the concave face up; the web's roll
# squares it with that face toward the stern) ----------------------------------

BLADE_ROOT, BLADE_TIP = -0.200, 0.245
NECK_COLLAR = 0.0150      # the neck's sleeve over the shaft end


def blade_edges(x):
    t = (x - BLADE_ROOT) / (BLADE_TIP - BLADE_ROOT)
    top = -0.018 - 0.034 * smooth(t / 0.7)
    bottom = 0.016 + 0.162 * math.sin(min(t, 1.0) * math.pi / 2) ** 0.8
    if t > 0.88:                       # rounded, slightly raked tip
        k = (t - 0.88) / 0.12
        bottom -= 0.030 * k * k
        top += 0.010 * k * k
    return top, bottom


def blade():
    axis_y = SHAFT_Y - BLADE_OFFSET[1]  # the shaft axis in blade-local y
    neck_start = NECK_X - BLADE_OFFSET[0]
    columns, steps = 18, 30
    upper, lower = [], []
    for i in range(steps + 1):
        x = lerp(BLADE_ROOT, BLADE_TIP, i / steps)
        top, bottom = blade_edges(x)
        zc, half = (top + bottom) / 2, (bottom - top) / 2
        t = (x - BLADE_ROOT) / (BLADE_TIP - BLADE_ROOT)
        up, down = [], []
        for j in range(columns + 1):
            v = j / columns
            z = lerp(top, bottom, v)
            # A spoon: cupped across (2 cm at the edges) and curled toward
            # the tip (4.5 cm), so a flat blade still catches the light.
            y = axis_y - 0.004 + 0.020 * ((z - zc) / max(half, 1e-3)) ** 2 + 0.045 * t * t
            edge = min(v, 1 - v) * 2
            spine = max(0.0, 1 - abs(z) / 0.030) * max(0.0, 1 - (x - BLADE_ROOT) / 0.28)
            up.append((x, y + 0.0022, z))
            down.append((x, y - (0.0028 + 0.0026 * edge ** 0.5 + 0.0065 * spine), z))
        upper.append(up)
        lower.append(down)
    mesh = Mesh()
    up_i = mesh.grid(upper)       # rows along +x, columns along +z: faces +y
    lower_start = len(mesh.faces)
    down_i = mesh.grid(lower)
    mesh.faces[lower_start:] = [tuple(reversed(f)) for f in mesh.faces[lower_start:]]

    def stitch(a, b):
        for k in range(len(a) - 1):
            mesh.faces.append((a[k], a[k + 1], b[k + 1], b[k]))

    stitch([r[0] for r in up_i], [r[0] for r in down_i])       # -z edge
    stitch([r[-1] for r in down_i], [r[-1] for r in up_i])     # +z edge
    stitch(up_i[-1], down_i[-1])                               # tip
    # Neck: loft the shaft circle into the root outline (upper edge over the
    # top from -z to +z, then back along the lower face).
    loop = [mesh.verts[i] for i in up_i[0]] + [mesh.verts[i] for i in reversed(down_i[0])]
    collar_end = neck_start + 0.030
    rows = []
    for x in (neck_start, collar_end):
        rows.append([(x, axis_y + NECK_COLLAR * math.sin(2 * math.pi * k / len(loop)),
                      -NECK_COLLAR * math.cos(2 * math.pi * k / len(loop))) for k in range(len(loop))])
    for i in range(1, 11):
        t = i / 10
        w = smooth(t)
        x = lerp(collar_end, BLADE_ROOT, t)
        row = []
        for k, (lx, ly, lz) in enumerate(loop):
            a = 2 * math.pi * k / len(loop)
            row.append((x, lerp(axis_y + NECK_COLLAR * math.sin(a), ly, w),
                        lerp(-NECK_COLLAR * math.cos(a), lz, w)))
        rows.append(row)
    neck = Mesh()
    index = neck.grid(rows, close=True)
    orient_open(neck, lambda c: (0.0, c[1] - axis_y, c[2]))
    end = Mesh()
    end.cap([end.add(neck.verts[i]) for i in index[0]])
    neck.merge(orient_open(end, lambda c: (-1.0, 0.0, 0.0)))
    mesh.merge(neck)
    if signed_volume(mesh) <= 0:
        raise ValueError("blade winding is inconsistent")
    return mesh


# ---- Assembly ------------------------------------------------------------------

BOAT_PARTS = [
    ("hull", "equipment-dark", hull),
    ("stern-deck", "equipment-painted", stern_deck),
    ("bow-deck", "equipment-painted", bow_deck),
    ("cockpit-tub", "equipment-dark", cockpit_tub),
    ("bulkheads", "equipment-trim", bulkheads),
    ("gunwales", "equipment-light", gunwales),
    ("slide-rails", "equipment-metal", slide_rails),
    ("accent-strakes", "equipment-light", accent_strakes),
    ("foot-stretcher", "equipment-dark", foot_stretcher),
    ("heel-cups", "equipment-rubber", heel_cups),
    ("stretcher-hardware", "equipment-metal", stretcher_hardware),
    ("riggers", "equipment-metal", riggers),
    ("oarlocks", "equipment-metal", oarlocks),
    ("keel-fin", "equipment-dark", keel_fin),
]
SEAT_PARTS = [
    ("seat-pad", "equipment-trim", seat_pad),
    ("seat-carriage", "equipment-metal", seat_carriage),
    ("seat-rollers", "equipment-rubber", seat_rollers),
    ("seat-guides", "equipment-trim", seat_guides),
]
OAR_PARTS = [
    ("shaft", "equipment-light", shaft),
    ("grip", "equipment-grip", grip),
    ("handle-cap", "equipment-dark", handle_cap),
    ("collar", "equipment-metal", collar),
    ("blade-sleeve", "equipment-painted", blade_sleeve),
]
TEMPLATES = [
    ("equipment:row:boat-assembly", BOAT_PARTS),
    ("equipment:row:seat-carriage", SEAT_PARTS),
    ("equipment:row:oar-rig", OAR_PARTS),
]
BLADE_SLOT = ("equipment:row:blade", "equipment-painted")


def to_blender(point):
    x, y, z = point
    return (x, -z, y)


def blender_object(name, mesh, parent=None, sharp_angle=38.0):
    data = bpy.data.meshes.new(name)
    data.from_pydata([to_blender(v) for v in mesh.verts], [], mesh.faces)
    data.update(calc_edges=True)
    if mesh.colors is not None:
        attribute = data.color_attributes.new("wet", "FLOAT_COLOR", "POINT")
        for i, color in enumerate(mesh.colors):
            attribute.data[i].color = color
    # Weld the seams between a part's pieces (coincident by construction).
    work = bmesh.new()
    work.from_mesh(data)
    bmesh.ops.remove_doubles(work, verts=work.verts, dist=1e-6)
    work.to_mesh(data)
    work.free()
    for polygon in data.polygons:
        polygon.use_smooth = True
    data.set_sharp_from_angle(angle=math.radians(sharp_angle))
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    obj.parent = parent
    return obj


def build(material):
    """Create the three templates and the blade leaf; triangles per slot."""
    counts = {}
    for template, parts in TEMPLATES:
        root = bpy.data.objects.new(template, None)
        bpy.context.scene.collection.objects.link(root)
        root["replayAssetTemplateSlot"] = template
        root["replayAssetKind"] = "composite"
        root["replayAssetVersion"] = 3
        root["replayAssetPartCount"] = len(parts)
        root["replayMaterialRoles"] = sorted({role for _, role, _ in parts})
        total = 0
        for part, role, make in parts:
            mesh = make()
            if part == "hull":
                wet_band(mesh)
            obj = blender_object(f"{template}:{part}", mesh, root)
            obj.data.materials.append(material)
            obj["replayAssetTemplateSlot"] = template
            obj["replayAssetPart"] = part
            obj["replayMaterialRole"] = role
            obj.data.calc_loop_triangles()
            total += len(obj.data.loop_triangles)
        counts[template] = total
    slot, role = BLADE_SLOT
    obj = blender_object(slot, blade(), sharp_angle=50.0)
    obj.data.materials.append(material)
    obj["replayAssetSlot"] = slot
    obj["replayAssetKind"] = "leaf"
    obj["replayMaterialRole"] = role
    obj.data.calc_loop_triangles()
    counts[slot] = len(obj.data.loop_triangles)
    return counts


def rendered_triangles(counts):
    """Boat and seat once, the oar rig and blade once per side."""
    return (counts["equipment:row:boat-assembly"] + counts["equipment:row:seat-carriage"]
            + 2 * (counts["equipment:row:oar-rig"] + counts["equipment:row:blade"]))


def export(path):
    result = bpy.ops.export_scene.gltf(
        filepath=str(path), export_format="GLB", export_yup=True, export_apply=False,
        export_animations=False, export_cameras=False, export_lights=False, export_extras=True,
        export_materials="EXPORT", export_texcoords=False, export_normals=True, export_tangents=False,
        export_vertex_color="NAME", export_vertex_color_name="wet", export_all_vertex_colors=False,
        export_active_vertex_color_when_no_material=False, export_attributes=False,
        export_draco_mesh_compression_enable=False, export_meshopt_compression_enable=False,
        export_use_gltfpack=False)
    if "FINISHED" not in result:
        raise RuntimeError(f"glTF export failed: {result}")
    canonicalize_pack(path)
    bound_attributes(path)


# ---- Validation (reads the exported bytes, not the Blender scene) --------------

COMPONENTS = {5121: "B", 5123: "H", 5125: "I", 5126: "f"}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def read_pack(path):
    data = path.read_bytes()
    size, = struct.unpack_from("<I", data, 12)
    doc = json.loads(data[20:20 + size])
    binary = data[28 + size:]
    return doc, binary


def positions(doc, binary, mesh_index):
    out = []
    for primitive in doc["meshes"][mesh_index]["primitives"]:
        acc = doc["accessors"][primitive["attributes"]["POSITION"]]
        view = doc["bufferViews"][acc["bufferView"]]
        start = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        values = struct.unpack_from(f"<{acc['count'] * 3}f", binary, start)
        out += [values[i:i + 3] for i in range(0, len(values), 3)]
    return out


def validate(path):
    """The contract the app relies on, checked on the exported file."""
    doc, binary = read_pack(path)
    nodes = {node.get("name"): node for node in doc["nodes"]}
    problems = []
    for template, parts in TEMPLATES:
        root = nodes.get(template)
        if root is None:
            problems.append(f"missing {template}")
            continue
        if any(k in root for k in ("translation", "rotation", "scale", "matrix")):
            problems.append(f"{template} root must have identity transforms")
        extras = root.get("extras", {})
        expected = {"replayAssetTemplateSlot": template, "replayAssetKind": "composite",
                    "replayAssetVersion": 3, "replayAssetPartCount": len(parts),
                    "replayMaterialRoles": sorted({role for _, role, _ in parts})}
        for key, value in expected.items():
            if extras.get(key) != value:
                problems.append(f"{template} extras {key}: {extras.get(key)!r} != {value!r}")
        children = {doc["nodes"][c].get("name") for c in root.get("children", [])}
        for part, role, _ in parts:
            name = f"{template}:{part}"
            node = nodes.get(name)
            if node is None or name not in children:
                problems.append(f"missing part {name}")
                continue
            if node.get("extras", {}).get("replayMaterialRole") != role:
                problems.append(f"{name} role")
            if any(k in node for k in ("translation", "rotation", "scale", "matrix")):
                problems.append(f"{name} must bake its placement")
    slot, role = BLADE_SLOT
    extras = nodes.get(slot, {}).get("extras", {})
    if extras.get("replayAssetSlot") != slot or extras.get("replayMaterialRole") != role:
        problems.append("blade leaf extras")
    # Geometry the rig binds to.
    def bounds(name):
        pts = positions(doc, binary, nodes[name]["mesh"])
        return [min(p[k] for p in pts) for k in range(3)], [max(p[k] for p in pts) for k in range(3)], pts
    lo, hi, _ = bounds("equipment:row:boat-assembly:hull")
    if abs(lo[2] + HALF_LENGTH) > 0.002 or abs(hi[2] - HALF_LENGTH) > 0.002:
        problems.append(f"hull runs {lo[2]:.4f}..{hi[2]:.4f}, not the 7.8 m shell")
    _, _, pins = bounds("equipment:row:boat-assembly:oarlocks")
    for side in (1, -1):
        mine = [p for p in pins if p[0] * side > 0]
        cx = (min(p[0] for p in mine) + max(p[0] for p in mine)) / 2
        cz = (min(p[2] for p in mine) + max(p[2] for p in mine)) / 2
        if abs(cx - side * OARLOCK_PIVOT[0]) > 1e-4 or abs(cz - OARLOCK_PIVOT[2]) > 1e-4:
            problems.append(f"oarlock {side:+d} centred at ({cx:.4f}, {cz:.4f}), not on the pivot")
    lo, hi, pts = bounds("equipment:row:oar-rig:grip")
    radius = max(math.hypot(p[1] - GRIP_DROP, p[2]) for p in pts)
    if abs(lo[0] - GRIP_START) > 1e-4 or abs(hi[0] - GRIP_END) > 1e-4 or not GRIP_RADIUS <= radius <= GRIP_RADIUS + 0.0006:
        problems.append(f"grip {lo[0]:.4f}..{hi[0]:.4f} r {radius:.5f} moved off the hands")
    # Everything in the cockpit stays inside the skin, at every seat position.
    inside = ["cockpit-tub", "slide-rails", "foot-stretcher", "heel-cups", "stretcher-hardware"]
    seat = ["seat-pad", "seat-carriage", "seat-rollers", "seat-guides"]
    checks = [(f"equipment:row:boat-assembly:{part}", 0.0) for part in inside]
    checks += [(f"equipment:row:seat-carriage:{part}", dz) for part in seat for dz in SEAT_TRAVEL]
    for name, dz in checks:
        _, _, pts = bounds(name)
        worst = max((abs(x) - section_at_height(z + dz, y, 0.0) for x, y, z in pts
                     if keel(z + dz) < y < sheer(z + dz) - 0.001), default=-1.0)
        if worst > 0.0005:
            problems.append(f"{name} pokes {worst * 1000:.1f} mm through the hull")
    _, _, fin = bounds("equipment:row:boat-assembly:keel-fin")
    exposed = [y for x, y, z in fin if y < keel(z) - 0.0005 and y > -BOB - 0.002]
    if exposed:
        problems.append(f"the fin shows above the water at the top of the bob (y {max(exposed):.4f})")
    if problems:
        raise ValueError("rowing-shell contract: " + "; ".join(problems))


# ---- Previews ---------------------------------------------------------------------

PREVIEW_COLORS = {
    "equipment-painted": ("#315784", 0.30, 0.0), "equipment-dark": ("#24282d", 0.40, 0.0),
    "equipment-light": ("#edeee9", 0.34, 0.0), "equipment-metal": ("#a0a5aa", 0.32, 0.9),
    "equipment-trim": ("#9a5700", 0.40, 0.3), "equipment-rubber": ("#1a1d20", 0.9, 0.0),
    "equipment-grip": ("#33373c", 0.85, 0.0),
}


def preview_material(role):
    from common import linear
    color, roughness, metal = PREVIEW_COLORS[role]
    mat = bpy.data.materials.get(f"preview-{role}") or bpy.data.materials.new(f"preview-{role}")
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = linear(color)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metal
    if role == "equipment-painted":
        bsdf.inputs["Coat Weight"].default_value = 0.6
    return mat


def contract_matrix(position=(0, 0, 0), yaw=0.0, scale=(1, 1, 1)):
    """A rig-space placement (translation, yaw about +y, local scale) in Blender axes."""
    x, y, z = position
    sx, sy, sz = scale
    return (Matrix.Translation(Vector(to_blender((x, y, z)))) @ Matrix.Rotation(yaw, 4, "Z")
            @ Matrix.Diagonal((sx, sz, sy, 1.0)))


def pose_preview(seat_z=0.04, yaw=0.0):
    """Clone the oar rig and blade to both pivots like the app does, and
    colour every part by role. Returns the objects to delete afterwards."""
    extra = []
    for obj in list(bpy.context.scene.objects):
        if obj.type == "MESH":
            obj.data.materials.clear()
            obj.data.materials.append(preview_material(obj["replayMaterialRole"]))
    seat = bpy.data.objects["equipment:row:seat-carriage"]
    seat.matrix_world = contract_matrix((0, 0, seat_z))
    oar = bpy.data.objects["equipment:row:oar-rig"]
    blade_obj = bpy.data.objects["equipment:row:blade"]
    px, py, pz = OARLOCK_PIVOT
    oar.matrix_world = contract_matrix((px, py, pz), yaw)
    rot = Matrix.Rotation(yaw, 3, "Y")
    offset = rot @ Vector(BLADE_OFFSET)
    blade_obj.matrix_world = contract_matrix((px + offset.x, py + offset.y, pz + offset.z), yaw)
    for source in (oar, *oar.children):
        clone = source.copy()
        bpy.context.scene.collection.objects.link(clone)
        extra.append(clone)
    left = extra[0]
    for child in extra[1:]:
        child.parent = left
    left.matrix_world = contract_matrix((-px, py, pz), math.pi - yaw)
    mirror = blade_obj.copy()
    bpy.context.scene.collection.objects.link(mirror)
    offset_left = Matrix.Rotation(math.pi - yaw, 3, "Y") @ Vector(BLADE_OFFSET)
    # The app's left blade: the leaf turned pi and scaled -1 in its own z.
    mirror.matrix_world = contract_matrix((-px + offset_left.x, py + offset_left.y, pz + offset_left.z),
                                          math.pi - yaw, (1, 1, -1))
    extra.append(mirror)
    return extra


def render_views(output, rows, samples=48):
    """Render views into one sheet. rows: lists of (name, eye, target, ortho,
    (width, height)); every row of a sheet has the same total width."""
    from common import linear
    import numpy as np
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = samples
    scene.cycles.use_denoising = True
    scene.render.threads_mode = "AUTO"
    scene.render.resolution_percentage = 100
    scene.view_settings.view_transform = "AgX"
    if scene.world is None or scene.world.name != "preview-world":
        world = bpy.data.worlds.new("preview-world")
        world.use_nodes = True
        scene.world = world
        world.node_tree.nodes["Background"].inputs["Color"].default_value = linear("#dfe4e8")
        world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.9
        sun = bpy.data.objects.new("sun", bpy.data.lights.new("sun", "SUN"))
        sun.data.energy = 2.2
        sun.data.angle = math.radians(8)
        sun.rotation_euler = (math.radians(35), math.radians(10), math.radians(30))
        scene.collection.objects.link(sun)
    strips = []
    for row in rows:
        tiles = []
        for name, eye, target, ortho, (width, height) in row:
            scene.render.resolution_x, scene.render.resolution_y = width, height
            data = bpy.data.cameras.new(name)
            cam = bpy.data.objects.new(name, data)
            scene.collection.objects.link(cam)
            cam.location = Vector(to_blender(eye))
            direction = Vector(to_blender(target)) - cam.location
            if abs(direction.normalized().z) > 0.999:
                cam.rotation_euler = (0.0, 0.0, math.pi / 2)   # plan view: stern to the left
            else:
                cam.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
            if ortho:
                data.type = "ORTHO"
                data.ortho_scale = ortho
            else:
                data.lens = 50
            data.clip_start, data.clip_end = 0.05, 100
            scene.camera = cam
            path = output.parent / f"{output.stem}-{name}.png"
            scene.render.filepath = str(path)
            scene.render.image_settings.file_format = "PNG"
            bpy.ops.render.render(write_still=True)
            img = bpy.data.images.load(str(path))
            tiles.append(np.array(img.pixels[:], dtype=np.float32).reshape(height, width, 4))
            bpy.data.images.remove(img)
            bpy.data.objects.remove(cam, do_unlink=True)
        strips.append(np.concatenate(tiles, axis=1))
    sheet = np.concatenate(list(reversed(strips)), axis=0)   # Blender images are bottom-up
    img = bpy.data.images.new(output.stem, sheet.shape[1], sheet.shape[0])
    img.pixels.foreach_set(sheet.ravel())
    img.filepath_raw = str(output)
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


def contact_sheet(output):
    """Top, side, front and three-quarter views of the shell and both oars,
    posed as the app clones them; a second sheet has close-ups."""
    extra = pose_preview()
    render_views(output, [
        [("top", (0.0, 9.0, 0.0), (0.0, 0.0, 0.0), 8.4, (2400, 1760))],
        [("side", (9.0, 0.40, 0.0), (0.0, 0.40, 0.0), 8.4, (2400, 520))],
        [("front", (0.0, 0.55, -9.0), (0.0, 0.35, 0.0), 6.4, (1200, 760)),
         ("three-quarter", (3.3, 2.1, 5.2), (0.0, 0.25, 0.2), None, (1200, 760))],
    ])
    render_views(output.parent / (output.stem.replace("contact-sheet", "details") + ".png"), [
        [("cockpit", (0.55, 0.95, 1.55), (0.0, 0.15, 0.10), None, (1200, 760)),
         ("oarlock", (1.25, 0.80, 0.85), (0.86, 0.46, 0.28), None, (1200, 760))],
        [("blade", (2.9, 0.75, 0.95), (2.72, 0.45, 0.28), None, (1200, 760)),
         ("handle", (0.30, 0.75, 0.70), (0.14, 0.47, 0.30), None, (1200, 760))],
    ])
    for obj in extra:
        bpy.data.objects.remove(obj, do_unlink=True)
