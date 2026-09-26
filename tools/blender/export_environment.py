# SPDX-License-Identifier: GPL-3.0-or-later
"""Export and validate the rowing environment (Blender Phase 3).

The environment is a modelled asset under ADR 0016's source rule: the reviewed
`assets/replay/authored/rowing-environment.blend` is its source of truth. This
script does not model anything. It opens that file, checks it against the
contract below, and writes two runtime outputs:

- `rowing-environment.glb`: the terrain, the far bank, the woodland masses
  and the vegetation variants, one mesh each, with vertex colours and no materials (the scene's
  materials are declared in `RowingEnvironment.qml`);
- `vegetation.json`: every placed vegetation instance, one per line, sorted by
  variant and then by tier, so each tier is a prefix of its variant's list.

The contract of the source file:

- a collection `rowplay-environment` holding `land`, `vegetation-variants`
  and one collection per quality tier, `vegetation-low` to `vegetation-ultra`;
- `land`: exactly `environment:row:terrain`, `environment:row:far-bank` and
  `environment:row:woodland` (the continuous canopy behind the woodland edges);
- `vegetation-variants`: exactly the names in VARIANTS;
- every mesh named like its object, unparented, with an identity world
  transform (the exporter writes it onto the node, and balsam's mesh files,
  all the scene reads, drop it), a point colour attribute `Col` and no
  modifiers;
- every instance a linked duplicate of one variant, turned about +Z only,
  scaled uniformly, tinted by its object colour, in exactly one tier, and
  where its own transform channels put it (the placements are read from
  those, so a parent, constraint or delta transform must not move it).

Blender axes are converted to glTF / Qt axes: (x, y, z) -> (x, z, -y), and a
turn about Blender +Z is the same turn about Qt +Y.
"""

import hashlib
import json
import math
from pathlib import Path
import struct

try:
    import bpy
    from mathutils import Euler, Matrix, Vector
    from mathutils.bvhtree import BVHTree
except ImportError:  # test_export_environment.py checks the pure helpers without Blender
    bpy = None

from canonical import bound_attributes, canonicalize_pack

LAND = ("environment:row:terrain", "environment:row:far-bank", "environment:row:woodland")
VARIANTS = ("tree-broadleaf-a", "tree-broadleaf-b", "tree-conifer", "tree-poplar", "shrub", "reeds")
TIERS = ("low", "medium", "high", "ultra")
PREFIX = "environment:row:"
VENUE_PREFIX = "environment:rower:"

# Budgets (docs/blender-audit.md, "Phase 3"). Triangles are counted as drawn:
# a variant's triangles once per instance shown at that tier.
BUDGET = {
    "terrain": 16_000,
    "far-bank": 8_000,
    "woodland": 6_000,
    "variant": 800,
    "instances": {"low": 100, "medium": 200, "high": 300, "ultra": 400},
    "drawn": {"low": 60_000, "medium": 90_000, "high": 120_000, "ultra": 150_000},
}
# The water ring the environment must stay out of: the outer buoy ring is at
# 33.3 m (course.json) and the oar blades reach about 33 m on the live lane.
# The quay meets the launch pontoon's gangway, at 36.27 m.
SHORE_MIN = 36.2
# The island at the course's centre (Blender Phase 4, `dressing.json`): its
# lawn may carry plants, its beach and skirt may not, and nothing else stands
# inside the basin.
ISLAND_LAWN = 12.4
ISLAND_RADIUS = 16.5
# The course dressing's structures (Blender Phase 4) read from its
# placements as annular sectors (radius and loop angle ranges), which fit the
# compact buildings, the radial bridge and the curved boardwalk alike;
# nothing is planted inside them. The dressing's own export checks that they
# stand on this terrain.
DRESSING = "authored/dressing.json"


class ContractError(ValueError):
    pass


def _srgb(linear):
    """Blender's object colour is scene-linear; QML colour strings are sRGB."""
    if linear <= 0.0031308:
        return 12.92 * linear
    return 1.055 * linear ** (1 / 2.4) - 0.055


def _triangles(obj):
    obj.data.calc_loop_triangles()
    return len(obj.data.loop_triangles)


def _is_identity(obj):
    """The world matrix covers delta transforms, constraints and every rotation mode."""
    m = obj.matrix_world
    return all(abs(m[i][j] - (1.0 if i == j else 0.0)) < 1e-6 for i in range(4) for j in range(4))


def _check_mesh(obj, problems):
    if obj.type != "MESH":
        problems.append(f"{obj.name} is not a mesh")
        return
    if obj.data.name != obj.name:
        problems.append(f"{obj.name}: mesh data is named {obj.data.name!r}")
    if not _is_identity(obj) or obj.parent is not None:
        problems.append(f"{obj.name} must be unparented with an identity world transform")
    if obj.modifiers:
        problems.append(f"{obj.name} carries modifiers; apply them in the source")
    colour = obj.data.color_attributes.get("Col")
    if colour is None or colour.domain != "POINT":
        problems.append(f"{obj.name} has no point colour attribute 'Col'")


def _dressing_footprints(assets):
    """{structure: (r_min, r_max, a_min, a_max)} from the dressing's placements.

    Radii in metres from the basin centre, angles in degrees round the loop
    (atan2(x, z), as the course places the boat); a footprint past 360
    degrees straddles the loop's 0."""
    placements = json.loads((assets / DRESSING).read_text())
    found = {}
    for entry in placements["structures"]:
        r0, r1, a0, a1 = entry["footprint"]
        if not 0.0 <= r0 <= r1 or not 0.0 <= a0 <= a1 <= 720.0:
            raise ContractError(f"{entry['name']}: footprint {entry['footprint']} is not a sector")
        found[entry["name"]] = (r0, r1, a0, a1)
    if not found:
        raise ContractError(f"{DRESSING} names no structures")
    return found


def _inside(footprint, x, z, margin):
    r0, r1, a0, a1 = footprint
    r, a = math.hypot(x, z), math.degrees(math.atan2(x, z)) % 360.0
    if not r0 - margin <= r <= r1 + margin:
        return False
    if a1 - a0 >= 360.0:
        return True
    slack = math.degrees(margin / max(r, 1.0))
    return any(a0 - slack <= a + turn <= a1 + slack for turn in (0.0, 360.0))


def _distance_to_axis(polygon):
    """Horizontal distance from the basin centre to a convex polygon of (x, y)."""
    edges = list(zip(polygon, polygon[1:] + polygon[:1]))
    crosses = [p[0] * q[1] - q[0] * p[1] for p, q in edges]
    # Twice the signed area: a polygon collapsed to a point or a segment
    # cannot contain the centre, whatever the signs say.
    if abs(sum(crosses)) > 1e-12 and (min(crosses) >= 0.0 or max(crosses) <= 0.0):
        return 0.0

    def to_segment(p, q):
        dx, dy = q[0] - p[0], q[1] - p[1]
        length2 = dx * dx + dy * dy
        t = 0.0 if length2 == 0.0 else max(0.0, min(1.0, -(p[0] * dx + p[1] * dy) / length2))
        return math.hypot(p[0] + t * dx, p[1] + t * dy)

    return min(to_segment(p, q) for p, q in edges)


def _above_water_within(triangle, radius):
    """Whether any point of a land triangle ((x, y, z) in Blender axes) is at
    or above the water, z >= 0, closer than `radius` to the basin centre.

    The triangle is flat, so its part at or above the water is the triangle
    clipped by z >= 0, a convex polygon. Vertices alone miss an edge that
    surfaces between a sunken vertex inside the radius and a dry one outside."""
    clipped = []
    for a, b in zip(triangle, triangle[1:] + triangle[:1]):
        if a[2] >= 0.0:
            clipped.append((a[0], a[1]))
        if (a[2] >= 0.0) != (b[2] >= 0.0):
            t = a[2] / (a[2] - b[2])
            clipped.append((a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])))
    return bool(clipped) and _distance_to_axis(clipped) < radius


def _terrain_height(bvh, x, z):
    """Height of the terrain under the Qt point (x, z), by a downward ray."""
    hit = bvh.ray_cast(Vector((x, -z, 500.0)), Vector((0.0, 0.0, -1.0)))
    return None if hit[0] is None else hit[0].z


def validate(assets):
    """Check the open file against the contract; return the export plan."""
    # World matrices as the file now stands: deltas and constraints included.
    bpy.context.view_layer.update()
    problems = []
    root = bpy.data.collections.get("rowplay-environment")
    if root is None:
        raise ContractError("no collection 'rowplay-environment'")
    names = {c.name for c in root.children}
    expected = {"land", "vegetation-variants", *(f"vegetation-{t}" for t in TIERS)}
    if names != expected:
        raise ContractError(f"rowplay-environment holds {sorted(names)}, expected {sorted(expected)}")
    land = {o.name: o for o in bpy.data.collections["land"].objects}
    if set(land) != set(LAND):
        problems.append(f"land holds {sorted(land)}, expected {sorted(LAND)}")
    variants = {o.name: o for o in bpy.data.collections["vegetation-variants"].objects}
    if set(variants) != {PREFIX + v for v in VARIANTS}:
        problems.append(f"vegetation-variants holds {sorted(variants)}")
    for obj in [*land.values(), *variants.values()]:
        _check_mesh(obj, problems)
    # A part that is not a mesh is already a problem; it has no triangles to count.
    triangles = {name: _triangles(obj) for name, obj in [*land.items(), *variants.items()]
                 if obj.type == "MESH"}
    for name, count in triangles.items():
        part = name[len(PREFIX):]
        limit = BUDGET.get(part, BUDGET["variant"])
        if count > limit:
            problems.append(f"{name}: {count} triangles over its budget of {limit}")
    if problems:
        raise ContractError("; ".join(problems))

    # The waterline: no land at or above the water inside SHORE_MIN, anywhere
    # on its surface (the loop triangles were computed with the counts).
    for name in LAND:
        vertices = land[name].data.vertices
        wet = sum(1 for tri in land[name].data.loop_triangles
                  if _above_water_within([tuple(vertices[i].co) for i in tri.vertices], SHORE_MIN))
        if wet:
            problems.append(f"{name} is at or above the water inside {SHORE_MIN} m in {wet} triangles")
    terrain = land[LAND[0]]
    bvh = BVHTree.FromObject(terrain, bpy.context.evaluated_depsgraph_get())

    footprints = _dressing_footprints(assets)

    seen = {}
    instances = []
    for tier, tier_name in enumerate(TIERS):
        for obj in bpy.data.collections[f"vegetation-{tier_name}"].objects:
            if obj.name in seen:
                problems.append(f"{obj.name} is in two tiers")
                continue
            seen[obj.name] = tier
            variant = obj.data.name[len(PREFIX):] if obj.type == "MESH" else None
            if variant not in VARIANTS:
                problems.append(f"{obj.name} is not a linked duplicate of a variant")
                continue
            if obj.parent is not None or obj.modifiers:
                problems.append(f"{obj.name} must be unparented and unmodified")
            rx, ry, _ = obj.rotation_euler
            if abs(rx) > 1e-6 or abs(ry) > 1e-6 or obj.rotation_mode != "XYZ":
                problems.append(f"{obj.name} is turned about more than +Z")
            s = obj.scale
            if max(s) - min(s) > 1e-6 or not 0.3 <= s.x <= 2.0:
                problems.append(f"{obj.name} scale {tuple(s)} is not uniform in 0.3-2.0")
            colour = obj.color
            if abs(colour[3] - 1.0) > 1e-6 or not all(0.0 <= c <= 1.0 for c in colour[:3]):
                problems.append(f"{obj.name} tint {tuple(colour)} is not an opaque colour")
            x, y, z = obj.location.x, obj.location.z, -obj.location.y
            radius = math.hypot(x, z)
            on_island = radius <= ISLAND_LAWN
            if radius < (35.0 if variant == "reeds" else SHORE_MIN + 2.0) and not on_island:
                problems.append(f"{obj.name} stands in the course water (r {radius:.2f} m)")
            margin = 0.3 if variant == "reeds" else 2.0
            for name, footprint in footprints.items():
                if name == "island":
                    continue
                if _inside(footprint, x, z, margin):
                    problems.append(f"{obj.name} stands in the {name}")
            yaw = math.degrees(obj.rotation_euler.z) % 360.0
            entry = {
                "variant": variant, "tier": tier, "name": obj.name,
                "position": [round(x, 4), round(y, 4), round(z, 4)],
                "yaw": round(yaw, 3), "scale": round(s.x, 4),
                "color": "#" + "".join(f"{round(_srgb(c) * 255):02x}" for c in colour[:3]),
            }
            # What is written must be where Blender shows the object, to 1 mm:
            # a delta transform or a constraint moves it without touching the
            # channels read above.
            px, py, pz = entry["position"]
            written = Matrix.LocRotScale(Vector((px, -pz, py)), Euler((0.0, 0.0, math.radians(entry["yaw"]))),
                                         Vector((entry["scale"],) * 3))
            if max(abs(a - b) for row, other in zip(obj.matrix_world, written) for a, b in zip(row, other)) > 1e-3:
                problems.append(f"{obj.name}: its world transform differs from its location, rotation and scale")
            instances.append(entry)
    instanced = {o.name for o in bpy.data.objects if o.type == "MESH" and o.data.name.startswith(PREFIX)
                 and o.data.name[len(PREFIX):] in VARIANTS and o.name not in variants}
    stray = sorted(instanced - set(seen))
    if stray:
        problems.append(f"instances outside every tier: {stray[:5]}")
    instances.sort(key=lambda e: (e["variant"], e["tier"], e["name"]))
    counts = {v: [sum(1 for e in instances if e["variant"] == v and e["tier"] <= t)
                  for t in range(len(TIERS))] for v in VARIANTS}
    for variant, per_tier in counts.items():
        if per_tier[0] == 0:
            problems.append(f"{variant} has no Low instance; an empty instance table is never drawn")
    shown = {t: sum(c[i] for c in counts.values()) for i, t in enumerate(TIERS)}
    drawn = {t: sum(counts[v][i] * triangles[PREFIX + v] for v in VARIANTS) for i, t in enumerate(TIERS)}
    for t in TIERS:
        if shown[t] > BUDGET["instances"][t]:
            problems.append(f"{t}: {shown[t]} instances over {BUDGET['instances'][t]}")
        if drawn[t] > BUDGET["drawn"][t]:
            problems.append(f"{t}: {drawn[t]} vegetation triangles drawn over {BUDGET['drawn'][t]}")
    if problems:
        raise ContractError("rowing environment: " + "; ".join(problems))
    return {"land": land, "variants": variants, "instances": instances, "counts": counts,
            "shown": shown, "drawn": drawn, "triangles": triangles}


def export(glb, plan):
    """One mesh per land part and variant; no materials, no instances."""
    scene = bpy.context.scene
    staging = bpy.data.collections.new("export")
    scene.collection.children.link(staging)
    for obj in [*plan["land"].values(), *plan["variants"].values()]:
        staging.objects.link(obj)
    layer = bpy.context.view_layer.layer_collection.children["export"]
    bpy.context.view_layer.active_layer_collection = layer
    bpy.ops.export_scene.gltf(
        filepath=str(glb), export_format="GLB", export_yup=True,
        use_active_collection=True, use_active_collection_with_nested=False,
        export_materials="NONE", export_vertex_color="NAME", export_vertex_color_name="Col",
        export_all_vertex_colors=False, export_texcoords=False, export_normals=True,
        export_tangents=False, export_extras=False, export_apply=False,
        export_animations=False, export_cameras=False, export_lights=False,
        export_draco_mesh_compression_enable=False, export_meshopt_compression_enable=False,
        export_use_gltfpack=False,
    )
    bpy.data.collections.remove(staging)
    canonicalize_pack(glb)
    bound_attributes(glb)


def placement_json(instances):
    """One instance per line, so a moved tree is a one-line diff."""
    lines = []
    for e in instances:
        entry = {"variant": e["variant"], "tier": e["tier"], "position": e["position"],
                 "yaw": e["yaw"], "scale": e["scale"], "color": e["color"]}
        lines.append("  " + json.dumps(entry, separators=(", ", ": ")))
    return "[\n" + ",\n".join(lines) + "\n]\n"


def build(source, output):
    """Open the source, validate it, write the GLB and the placements.

    Returns the manifest's `environment` section."""
    assets = Path(source).resolve().parents[1]
    bpy.ops.wm.open_mainfile(filepath=str(source))
    plan = validate(assets)
    glb = output / "rowing-environment.glb"
    export(glb, plan)
    (output / "vegetation.json").write_text(placement_json(plan["instances"]))
    return {
        "source": Path(source).name,
        "sourceSha256": hashlib.sha256(Path(source).read_bytes()).hexdigest(),
        "blender": bpy.app.version_string,
        "triangles": {k[len(PREFIX):]: v for k, v in plan["triangles"].items()},
        "instances": plan["counts"],
        "shown": plan["shown"],
        "drawnTriangles": plan["drawn"],
        "budget": BUDGET,
    }
