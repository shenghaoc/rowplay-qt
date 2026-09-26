# SPDX-License-Identifier: GPL-3.0-or-later
"""Export and validate the rowing course dressing (Blender Phase 4).

The course dressing is a modelled asset under ADR 0016's source rule: the
reviewed `assets/replay/authored/rowing-dressing.blend` is its source of
truth. This script models nothing. It opens that file, checks it against the
contract below, and writes two runtime outputs:

- `rowing-dressing.glb`: every structure and every furniture variant, one
  mesh each, its primitives split by material class (the scene's materials
  are declared in `RowingDressing.qml`), with vertex colours and no
  materials;
- `dressing.json`: the structures' classes, shadow roles and footprints, the
  variants' classes, and every furniture instance, one per line, sorted by
  variant and then by tier, so each tier is a prefix of its variant's list.

The contract of the source file:

- a collection `rowplay-dressing` holding `structures`, `furniture-variants`
  and one collection per quality tier, `furniture-low` to `furniture-ultra`;
- `structures`: exactly the names in STRUCTURES, each a mesh named like its
  object, unparented, at the identity world transform (it is modelled where
  it stands: the exporter writes its transform onto the node, and balsam's
  mesh files, all the scene reads, drop it), without modifiers, with a point
  colour attribute `Col` (the albedo, linear) and material slots named from
  CLASSES only;
- `furniture-variants`: exactly the names in VARIANTS, modelled at the
  origin with the same rules;
- every furniture instance a linked duplicate of one variant, turned about
  +Z only, scaled uniformly, tinted by its object colour, in exactly one
  tier, and where its own transform channels put it.

And of the venue it stands in (`rowing-environment.blend` is read for its
terrain, `vegetation.json` for its plants):

- nothing solid inside the lane band, r 22.8 to 33.6 m, below the bridge's
  clearance: the buoy rings are at 23.1 and 33.3 m and the blades reach
  about 33 m on the live lane;
- a land structure stands on the terrain (its base within -0.5 and +0.45 m
  of the ground under it, nothing deeper than a pile driven 0.15 m); a
  water structure floats (its floats 0.2 to 0.6 m under the water, over
  open water); the island lies inside the basin; the bridge clears the
  lanes and lands on the bank;
- no plant inside a structure's footprint, but for the island's lawn.

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
except ImportError:  # test_export_dressing.py checks the pure helpers without Blender
    bpy = None

from canonical import bound_attributes, canonicalize_pack

PREFIX = "dressing:row:"
CLASSES = ("paint", "timber", "metal", "glass", "float", "ground")
TIERS = ("low", "medium", "high", "ultra")

# The structures, their kind (how they meet the world) and their shadow
# roles at High and Ultra, the tiers that cast a shadow at all. Buildings,
# the tower and the bridge cast as the web's tower and bridge do; the
# island receives as the web's island lawn does.
STRUCTURES = {
    "finish-tower": ("land", True, True),
    "start-jetty": ("water", True, True),
    "launch-pontoon": ("water", True, True),
    "coaching-pontoon": ("water", True, True),
    "course-bridge": ("span", True, True),
    "boathouse": ("land", True, True),
    "clubhouse": ("land", True, True),
    "regatta-office": ("land", True, True),
    "wetland-boardwalk": ("land", True, True),
    "wetland-hide": ("land", True, True),
    "distance-board-250": ("land", True, False),
    "distance-board-500": ("land", True, False),
    "distance-board-750": ("land", True, False),
    "island": ("island", False, True),
}
VARIANTS = ("bollard", "bench", "life-ring", "flagpole", "trestle-hull", "finish-buoy")
# Variants that float: their instances stand on the water, not the ground.
WATER_VARIANTS = ("finish-buoy",)

# Budgets (docs/blender-audit.md, "Phase 4"). Triangles are counted as drawn.
BUDGET = {
    "structure": 5_000,
    "structures": 30_000,
    "variant": 600,
    "instances": {"low": 10, "medium": 40, "high": 80, "ultra": 120},
    "drawn": {"low": 4_000, "medium": 10_000, "high": 20_000, "ultra": 30_000},
}
LANE_BAND = (22.8, 33.6)
CLEARANCE = 4.2
ISLAND_LAWN = 12.4       # plants on the island stay inside the lawn
ISLAND_RADIUS = 16.5     # the island's submerged skirt ends here
GROUND_BELOW, GROUND_ABOVE = -0.5, 0.45


class ContractError(ValueError):
    pass


def _srgb(linear):
    """Blender's object colour is scene-linear; QML colour strings are sRGB."""
    if linear <= 0.0031308:
        return 12.92 * linear
    return 1.055 * linear ** (1 / 2.4) - 0.055


def _polar(x, y):
    """Radius and loop angle (degrees, atan2(x, z) as the course places the
    boat) of a Blender point; z is -y."""
    return math.hypot(x, y), math.degrees(math.atan2(x, -y)) % 360.0


def _sector(points):
    """The annular sector [r0, r1, a0, a1] round the basin that holds the
    points; a ring round the whole basin when they straddle 0 degrees by
    more than half a turn."""
    polar = [_polar(x, y) for x, y, _ in points]
    r0, r1 = min(r for r, _ in polar), max(r for r, _ in polar)
    angles = sorted(a for _, a in polar)
    if not angles:
        raise ValueError("no points")
    gaps = [(angles[(i + 1) % len(angles)] - angles[i]) % 360.0 for i in range(len(angles))]
    widest = max(range(len(angles)), key=lambda i: gaps[i])
    if gaps[widest] < 180.0:
        return [round(r0, 3), round(r1, 3), 0.0, 360.0]
    a0 = angles[(widest + 1) % len(angles)]
    a1 = angles[widest]
    return [round(r0, 3), round(r1, 3), round(a0, 3), round(a1 + (360.0 if a1 < a0 else 0.0), 3)]


def inside_sector(footprint, x, z, margin):
    """Whether the Qt point (x, z) lies within `margin` metres of the sector."""
    r0, r1, a0, a1 = footprint
    r, a = math.hypot(x, z), math.degrees(math.atan2(x, z)) % 360.0
    if not r0 - margin <= r <= r1 + margin:
        return False
    if a1 - a0 >= 360.0:
        return True
    slack = math.degrees(margin / max(r, 1.0))
    for turn in (0.0, 360.0):
        if a0 - slack <= a + turn <= a1 + slack:
            return True
    return False


def in_lane_band(radius, extent=0.0):
    """Whether something of horizontal `extent` centred at `radius` reaches
    into the lane band."""
    return radius + extent >= LANE_BAND[0] and radius - extent <= LANE_BAND[1]


def _is_identity(obj):
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
    slots = [s.material.name if s.material else None for s in obj.material_slots]
    if not slots or any(name not in CLASSES for name in slots) or len(set(slots)) != len(slots):
        problems.append(f"{obj.name}: material slots {slots} are not distinct classes from {CLASSES}")
    used = {poly.material_index for poly in obj.data.polygons}
    if any(i >= len(slots) for i in used):
        problems.append(f"{obj.name}: a face uses a missing material slot")
    if len(used) != len(slots):
        problems.append(f"{obj.name}: every material slot must be used by a face")


def _centroids(triangles):
    """A primitive's triangles as a sorted tuple of rounded centroids: the
    signature that survives export and canonicalisation unchanged."""
    return tuple(sorted(tuple(round(sum(p[k] for p in tri) / 3, 3) for k in range(3)) for tri in triangles))


def _slot_signatures(obj):
    """Per material slot, in slot order: (class, centroid signature). The
    GLB's primitives are matched back to the slots by the signature."""
    obj.data.calc_loop_triangles()
    result = []
    for index, slot in enumerate(obj.material_slots):
        tris = [tuple(tuple(obj.data.vertices[v].co) for v in t.vertices) for t in obj.data.loop_triangles
                if obj.data.polygons[t.polygon_index].material_index == index]
        result.append((slot.material.name, _centroids(tris)))
    return result


def _terrain_height(bvh, x, y):
    hit = bvh.ray_cast(Vector((x, y, 500.0)), Vector((0.0, 0.0, -1.0)))
    return None if hit[0] is None else hit[0].z


def _base_vertices(coords):
    lowest = min(z for _, _, z in coords)
    return [(x, y, z) for x, y, z in coords if z <= lowest + 0.35]


def _check_placement(name, kind, coords, terrain, problems):
    """How a structure meets the water, the lanes and the terrain."""
    lowest = min(z for _, _, z in coords)
    highest = max(z for _, _, z in coords)
    for x, y, z in coords:
        r, _ = _polar(x, y)
        if in_lane_band(r) and z < CLEARANCE:
            problems.append(f"{name} reaches into the lane band at r {r:.2f} m, {z:.2f} m up")
            break
    if kind == "island":
        if max(_polar(x, y)[0] for x, y, _ in coords) > ISLAND_RADIUS or highest > 1.6 or lowest > -0.3:
            problems.append(f"{name}: the island must lie inside {ISLAND_RADIUS} m, below 1.6 m, with a sunken rim")
        return
    if kind == "water":
        if not -0.6 <= lowest <= -0.2:
            problems.append(f"{name}: a water structure's floats reach {lowest:.2f} m; expected -0.6 to -0.2")
        # Its floats and piles end above the bed: the water is deep enough.
        for x, y, z in coords:
            if z > -0.2:
                continue
            ground = _terrain_height(terrain, x, y)
            if ground is not None and ground >= z:
                problems.append(f"{name} is grounded: {z:.2f} m over a bed at {ground:.2f} m")
                break
        return
    if lowest < -0.15 and kind != "span":
        problems.append(f"{name} reaches {lowest:.2f} m, below the water")
    if kind == "land":
        # A land structure stands on the terrain: every base vertex meets it.
        for x, y, z in _base_vertices(coords):
            ground = _terrain_height(terrain, x, y)
            if ground is None:
                problems.append(f"{name} stands where there is no terrain ({x:.1f}, {y:.1f})")
                break
            if not ground + GROUND_BELOW <= z <= ground + GROUND_ABOVE:
                problems.append(f"{name}: its base at {z:.2f} m does not meet the ground at {ground:.2f} m")
                break
        return
    # A span lands on the bank: its lowest point past the shore meets the ground.
    landing = [(x, y, z) for x, y, z in coords if _polar(x, y)[0] > 39.5]
    if landing:
        x, y, z = min(landing, key=lambda p: p[2])
        ground = _terrain_height(terrain, x, y)
        if ground is None or not ground + GROUND_BELOW <= z <= ground + GROUND_ABOVE:
            problems.append(f"{name}: its landing at {z:.2f} m does not meet the bank at {ground}")


def validate(assets):
    """Check the open file against the contract; return the export plan."""
    bpy.context.view_layer.update()
    problems = []
    root = bpy.data.collections.get("rowplay-dressing")
    if root is None:
        raise ContractError("no collection 'rowplay-dressing'")
    names = {c.name for c in root.children}
    expected = {"structures", "furniture-variants", *(f"furniture-{t}" for t in TIERS)}
    if names != expected:
        raise ContractError(f"rowplay-dressing holds {sorted(names)}, expected {sorted(expected)}")
    structures = {o.name: o for o in bpy.data.collections["structures"].objects}
    if set(structures) != {PREFIX + s for s in STRUCTURES}:
        problems.append(f"structures holds {sorted(structures)}, expected {sorted(PREFIX + s for s in STRUCTURES)}")
    variants = {o.name: o for o in bpy.data.collections["furniture-variants"].objects}
    if set(variants) != {PREFIX + v for v in VARIANTS}:
        problems.append(f"furniture-variants holds {sorted(variants)}")
    for obj in [*structures.values(), *variants.values()]:
        _check_mesh(obj, problems)
    if problems:
        raise ContractError("; ".join(problems))

    triangles = {}
    signatures = {}
    for name, obj in [*structures.items(), *variants.items()]:
        obj.data.calc_loop_triangles()
        triangles[name] = len(obj.data.loop_triangles)
        signatures[name] = _slot_signatures(obj)
        limit = BUDGET["structure"] if name in structures else BUDGET["variant"]
        if triangles[name] > limit:
            problems.append(f"{name}: {triangles[name]} triangles over its budget of {limit}")
    total = sum(triangles[n] for n in structures)
    if total > BUDGET["structures"]:
        problems.append(f"the structures draw {total} triangles, over {BUDGET['structures']}")

    # The terrain the structures stand on, from the environment's source.
    with bpy.data.libraries.load(str(assets / "authored/rowing-environment.blend"), link=False) as (src, dst):
        dst.objects = ["environment:row:terrain"]
    terrain_obj = dst.objects[0]
    bpy.context.scene.collection.objects.link(terrain_obj)
    bpy.context.view_layer.update()
    terrain = BVHTree.FromObject(terrain_obj, bpy.context.evaluated_depsgraph_get())

    footprints = {}
    for name, obj in structures.items():
        coords = [tuple(v.co) for v in obj.data.vertices]
        kind = STRUCTURES[name[len(PREFIX):]][0]
        _check_placement(name, kind, coords, terrain, problems)
        footprints[name[len(PREFIX):]] = [0.0, ISLAND_RADIUS, 0.0, 360.0] if kind == "island" else _sector(coords)
    for name, obj in variants.items():
        extent = max(math.hypot(v.co.x, v.co.y) for v in obj.data.vertices)
        if extent > 4.5 or min(v.co.z for v in obj.data.vertices) < -0.5:
            problems.append(f"{name}: a variant is modelled at the origin, within 4.5 m, above -0.5 m")

    island = bpy.data.objects.get(PREFIX + "island")
    island_bvh = BVHTree.FromObject(island, bpy.context.evaluated_depsgraph_get()) if island else None
    seen = {}
    instances = []
    for tier, tier_name in enumerate(TIERS):
        for obj in bpy.data.collections[f"furniture-{tier_name}"].objects:
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
            if max(s) - min(s) > 1e-6 or not 0.5 <= s.x <= 2.0:
                problems.append(f"{obj.name} scale {tuple(s)} is not uniform in 0.5-2.0")
            colour = obj.color
            if abs(colour[3] - 1.0) > 1e-6 or not all(0.0 <= c <= 1.0 for c in colour[:3]):
                problems.append(f"{obj.name} tint {tuple(colour)} is not an opaque colour")
            bx, by, bz = obj.location
            radius = math.hypot(bx, by)
            extent = max(math.hypot(v.co.x, v.co.y) for v in obj.data.vertices) * s.x
            if in_lane_band(radius, extent):
                problems.append(f"{obj.name} reaches into the lane band (r {radius:.2f} m)")
            if variant in WATER_VARIANTS:
                if abs(bz) > 1e-6 or (radius < ISLAND_RADIUS) or (_terrain_height(terrain, bx, by) or -1.0) > -0.3:
                    problems.append(f"{obj.name} must float on open water at 0 m")
            else:
                ground = _terrain_height(island_bvh, bx, by) if radius < ISLAND_RADIUS and island_bvh else _terrain_height(terrain, bx, by)
                if ground is None or not ground + GROUND_BELOW <= bz <= ground + GROUND_ABOVE:
                    problems.append(f"{obj.name} at {bz:.2f} m does not meet the ground ({ground})")
            x, y, z = bx, bz, -by
            yaw = math.degrees(obj.rotation_euler.z) % 360.0
            entry = {
                "variant": variant, "tier": tier, "name": obj.name,
                "position": [round(x, 4), round(y, 4), round(z, 4)],
                "yaw": round(yaw, 3), "scale": round(s.x, 4),
                "color": "#" + "".join(f"{round(_srgb(c) * 255):02x}" for c in colour[:3]),
            }
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
    shown = {t: sum(c[i] for c in counts.values()) for i, t in enumerate(TIERS)}
    drawn = {t: sum(counts[v][i] * triangles[PREFIX + v] for v in VARIANTS) for i, t in enumerate(TIERS)}
    for t in TIERS:
        if shown[t] > BUDGET["instances"][t]:
            problems.append(f"{t}: {shown[t]} instances over {BUDGET['instances'][t]}")
        if drawn[t] > BUDGET["drawn"][t]:
            problems.append(f"{t}: {drawn[t]} furniture triangles drawn over {BUDGET['drawn'][t]}")

    # The plants keep out of the structures; the island's lawn keeps its trees.
    plants = json.loads((assets / "authored/vegetation.json").read_text())
    for plant in plants:
        x, _, z = plant["position"]
        margin = 0.3 if plant["variant"] == "reeds" else 1.5
        for name, footprint in footprints.items():
            if name == "island":
                if math.hypot(x, z) < ISLAND_RADIUS and math.hypot(x, z) > ISLAND_LAWN:
                    problems.append(f"a {plant['variant']} stands on the island's beach at r {math.hypot(x, z):.2f} m")
                continue
            if inside_sector(footprint, x, z, margin):
                problems.append(f"a {plant['variant']} at ({x:.1f}, {z:.1f}) stands in the {name}")
    bpy.data.objects.remove(terrain_obj, do_unlink=True)
    if problems:
        raise ContractError("rowing dressing: " + "; ".join(sorted(set(problems))))
    return {"structures": structures, "variants": variants, "instances": instances, "counts": counts,
            "shown": shown, "drawn": drawn, "triangles": triangles, "signatures": signatures,
            "footprints": footprints}


def export(glb, plan):
    """One mesh per structure and variant, its primitives split by material
    slot (placeholder materials keep the split without writing any)."""
    scene = bpy.context.scene
    staging = bpy.data.collections.new("export")
    scene.collection.children.link(staging)
    for obj in [*plan["structures"].values(), *plan["variants"].values()]:
        staging.objects.link(obj)
    layer = bpy.context.view_layer.layer_collection.children["export"]
    bpy.context.view_layer.active_layer_collection = layer
    bpy.ops.export_scene.gltf(
        filepath=str(glb), export_format="GLB", export_yup=True,
        use_active_collection=True, use_active_collection_with_nested=False,
        export_materials="PLACEHOLDER", export_vertex_color="NAME", export_vertex_color_name="Col",
        export_all_vertex_colors=False, export_texcoords=False, export_normals=True,
        export_tangents=False, export_extras=False, export_apply=False,
        export_animations=False, export_cameras=False, export_lights=False,
        export_draco_mesh_compression_enable=False, export_meshopt_compression_enable=False,
        export_use_gltfpack=False,
    )
    bpy.data.collections.remove(staging)
    canonicalize_pack(glb)
    bound_attributes(glb)


def primitive_signatures(glb):
    """Per mesh name, in primitive order: the centroid signature of each
    primitive's triangles, read back from the written file."""
    data = glb.read_bytes()
    size = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20:20 + size])
    start = 28 + size
    if doc.get("materials"):
        raise ContractError("the dressing GLB carries materials; the scene declares them")

    def floats(accessor_index, width):
        accessor = doc["accessors"][accessor_index]
        view = doc["bufferViews"][accessor["bufferView"]]
        offset = start + view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        return struct.unpack_from(f"<{accessor['count'] * width}f", data, offset)

    def indices(accessor_index):
        accessor = doc["accessors"][accessor_index]
        view = doc["bufferViews"][accessor["bufferView"]]
        offset = start + view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
        code = {5121: "B", 5123: "H", 5125: "I"}[accessor["componentType"]]
        return struct.unpack_from(f"<{accessor['count']}{code}", data, offset)

    result = {}
    for mesh in doc["meshes"]:
        entries = []
        for primitive in mesh["primitives"]:
            xyz = floats(primitive["attributes"]["POSITION"], 3)
            # glTF is y-up: (x, y, z) came from Blender's (x, z, -y).
            points = [(xyz[3 * i], -xyz[3 * i + 2], xyz[3 * i + 1]) for i in range(len(xyz) // 3)]
            idx = indices(primitive["indices"])
            entries.append(_centroids([(points[idx[i]], points[idx[i + 1]], points[idx[i + 2]])
                                       for i in range(0, len(idx), 3)]))
        result[mesh["name"]] = entries
    return result


def match_classes(slots, primitives):
    """The class of each primitive, in primitive order, from the slots'
    (class, signature) against the primitives' signatures. Refuses an
    unmatched or ambiguous primitive."""
    if len(slots) != len(primitives):
        raise ContractError(f"{len(primitives)} primitives for {len(slots)} material slots")
    classes = []
    taken = set()
    for signature in primitives:
        candidates = [i for i, (cls, slot_signature) in enumerate(slots)
                      if i not in taken and slot_signature == signature]
        if len(candidates) != 1:
            raise ContractError(f"a primitive of {len(signature)} triangles matches {len(candidates)} material slots")
        taken.add(candidates[0])
        classes.append(slots[candidates[0]][0])
    return classes


def dressing_json(plan, classes):
    """The structures, variants and instances, one per line each."""
    lines = ["{", '  "structures": [']
    entries = []
    for name in STRUCTURES:
        kind, casts, receives = STRUCTURES[name]
        entry = {"name": name, "kind": kind, "classes": classes[PREFIX + name], "casts": casts,
                 "receives": receives, "triangles": plan["triangles"][PREFIX + name],
                 "footprint": plan["footprints"][name]}
        entries.append("    " + json.dumps(entry, separators=(", ", ": ")))
    lines += [",\n".join(entries), "  ],", '  "variants": [']
    entries = []
    for name in VARIANTS:
        entry = {"name": name, "classes": classes[PREFIX + name], "triangles": plan["triangles"][PREFIX + name]}
        entries.append("    " + json.dumps(entry, separators=(", ", ": ")))
    lines += [",\n".join(entries), "  ],", '  "instances": [']
    entries = []
    for e in plan["instances"]:
        entry = {"variant": e["variant"], "tier": e["tier"], "position": e["position"],
                 "yaw": e["yaw"], "scale": e["scale"], "color": e["color"]}
        entries.append("    " + json.dumps(entry, separators=(", ", ": ")))
    lines += [",\n".join(entries), "  ]", "}"]
    return "\n".join(lines) + "\n"


def build(source, output):
    """Open the source, validate it, write the GLB and the placements.

    Returns the manifest's `dressing` section."""
    assets = Path(source).resolve().parents[1]
    bpy.ops.wm.open_mainfile(filepath=str(source))
    plan = validate(assets)
    glb = output / "rowing-dressing.glb"
    export(glb, plan)
    primitives = primitive_signatures(glb)
    classes = {}
    for name, slots in plan["signatures"].items():
        if name not in primitives:
            raise ContractError(f"the GLB lost {name}")
        classes[name] = match_classes(slots, primitives[name])
    (output / "dressing.json").write_text(dressing_json(plan, classes))
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
