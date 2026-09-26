# SPDX-License-Identifier: GPL-3.0-or-later
"""Render review views of the rowing environment in Blender (Blender Phase 3).

A review aid, not acceptance evidence: Qt is the acceptance renderer (ADR
0016). The views put the committed `rowing-environment.blend` in the context
the replay draws it in, so an edit to the source can be judged before it goes
through the export and the Qt capture:

- the Phase 2 shell (built by `shell.py`) at the approved moment, demo 1001 at
  208.829 s, and the course buoys from `course.json`;
- the course dressing from the committed `rowing-dressing.blend` (Blender
  Phase 4), in place of the web rower venue's structures, which the app
  hides since that phase;
- the water plane with the committed normal map on its 8 m tile;
- the authored sky as the world, the key light from `SUN_OFFSETS` at the
  Medium strength, and Qt 6.11.2's depth fog reproduced in every material:
  `pow(smoothstep(45, 180, distance), 1) * 0.35` towards the fog colour.
  Like Qt, the water reflects only the sky, never the geometry.

    blender -b --factory-startup -P tools/blender/review_environment.py -- \\
        --output build/previews/environment --scheme light

Views: `chase` (the approved camera), `lap` (eight chase views round the
loop), `wide`, `top`, `bank` (near-bank and wetland close-ups), `water` (a
low view along the water), `zones` (chase views into the launch, finish,
bridge, far-bank, coaching-pontoon and wetland zones) and `dressing`
(close-ups of the structures). Each output directory must be new.
"""

import argparse
import json
import math
from pathlib import Path
import sys

import bpy
from mathutils import Matrix, Quaternion, Vector

sys.path.insert(0, str(Path(__file__).resolve().parent))
import shell  # noqa: E402
from common import material  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
AUTHORED = ROOT / "assets/replay/authored"

# The approved moment (ART_FRAME, style-medium) and the rowing chase rig
# (rowplay_viewmodel::replay::camera: back 5.4, lateral 2.16, height 1.78,
# ahead 0.88, aim 0.84, 40 degree vertical field of view).
FRAME_METRES = 986.06103515625
FOV = 40.0
SUN_OFFSET = (-22.0, 18.0, 14.0)
FOG = {"light": "#c7cdd1", "dark": "#94a8bf"}
KEY = {"light": ("#f3f1ea", 0.85), "dark": ("#bbcde5", 0.5)}
TINT = {"light": "#ffffff", "dark": "#b3bfde"}  # RowingStyle.landTint and friends
WATER = "#809296"
VENUE_COLOURS = {  # RowingStyle.venueColor: (light, dark)
    "tower-accent": ("#8f9b9e", "#778593"),
    "tower-material": ("#cbd0d0", "#a5b2c2"),
    "pavilion-body": ("#cbd0d0", "#a5b2c2"),
    "earth|beach|pontoon|path|trunk": ("#979e99", "#657077"),
    "grass|lawn|canopy|shrub|reed": ("#667d6b", "#485e5d"),
}
# RowingDressing.qml's class materials: (roughness, specular, metalness).
DRESSING_CLASSES = {"paint": (0.6, 0.35, 0.0), "timber": (0.85, 0.2, 0.0), "metal": (0.42, 0.6, 0.45),
                    "glass": (0.12, 1.0, 0.0), "float": (0.7, 0.2, 0.0), "ground": (0.95, 0.2, 0.0)}


def gl(p):
    """glTF / Qt (x, y-up, z) to Blender (x, -z, y)."""
    return Vector((p[0], -p[2], p[1]))


def linear(hex_colour):
    values = [int(hex_colour[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return tuple(v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4 for v in values) + (1.0,)


def fogged(mat, scheme):
    """Route a material's surface through Qt's depth fog (fog.glsllib)."""
    nt = mat.node_tree
    out = next(n for n in nt.nodes if n.type == "OUTPUT_MATERIAL")
    link = out.inputs["Surface"].links[0]
    shader = link.from_socket
    nt.links.remove(link)
    distance = nt.nodes.new("ShaderNodeCameraData")
    ramp = nt.nodes.new("ShaderNodeMapRange")
    ramp.interpolation_type = "SMOOTHSTEP"
    ramp.inputs["From Min"].default_value = 45.0
    ramp.inputs["From Max"].default_value = 180.0
    nt.links.new(distance.outputs["View Distance"], ramp.inputs["Value"])
    amount = nt.nodes.new("ShaderNodeMath")
    amount.operation = "MULTIPLY"
    amount.inputs[1].default_value = 0.35
    nt.links.new(ramp.outputs["Result"], amount.inputs[0])
    fog = nt.nodes.new("ShaderNodeEmission")
    fog.inputs["Color"].default_value = linear(FOG[scheme])
    mix = nt.nodes.new("ShaderNodeMixShader")
    nt.links.new(amount.outputs[0], mix.inputs["Fac"])
    nt.links.new(shader, mix.inputs[1])
    nt.links.new(fog.outputs[0], mix.inputs[2])
    nt.links.new(mix.outputs[0], out.inputs["Surface"])


def vertex_colour_material(name, scheme, roughness, specular, instanced):
    """The QML materials: vertex colour x scheme tint (x instance colour)."""
    mat = bpy.data.materials.new(name)
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Specular IOR Level"].default_value = specular
    colour = nt.nodes.new("ShaderNodeVertexColor")
    colour.layer_name = "Col"
    result = colour.outputs["Color"]
    if instanced:
        info = nt.nodes.new("ShaderNodeObjectInfo")
        mix = nt.nodes.new("ShaderNodeMix")
        mix.data_type, mix.blend_type = "RGBA", "MULTIPLY"
        mix.inputs["Factor"].default_value = 1.0
        nt.links.new(result, mix.inputs["A"])
        nt.links.new(info.outputs["Color"], mix.inputs["B"])
        result = mix.outputs["Result"]
    tint = nt.nodes.new("ShaderNodeMix")
    tint.data_type, tint.blend_type = "RGBA", "MULTIPLY"
    tint.inputs["Factor"].default_value = 1.0
    nt.links.new(result, tint.inputs["A"])
    tint.inputs["B"].default_value = linear(TINT[scheme])
    nt.links.new(tint.outputs["Result"], bsdf.inputs["Base Color"])
    fogged(mat, scheme)
    return mat


def place_shell(scheme):
    shell.build(material("rowplay-neutral", "#8c9399", 0.5))
    extra = shell.pose_preview(seat_z=0.04, yaw=0.0)
    rig = bpy.data.objects.new("rig", None)
    bpy.context.scene.collection.objects.link(rig)
    for obj in list(bpy.context.scene.objects):
        if obj.name.startswith("equipment:row") and obj.parent is None:
            obj.parent = rig
    for obj in extra:
        if obj.parent is None:
            obj.parent = rig
    for mat in bpy.data.materials:
        if mat.name.startswith("preview-"):
            fogged(mat, scheme)
    return rig


def place_rig(rig, metres):
    """The rig root on the loop (rowplay_viewmodel::replay::course::place)."""
    a = metres / 1000.0 * 2 * math.pi
    yaw = math.atan2(math.cos(a), -math.sin(a)) + math.pi
    rig.matrix_world = Matrix.Translation(gl((30 * math.sin(a), 0.0, 30 * math.cos(a)))) @ Matrix.Rotation(yaw, 4, "Z")


def chase(metres):
    a = metres / 1000.0 * 2 * math.pi
    fx, fz = 30 * math.sin(a), 30 * math.cos(a)
    tx, tz, nx, nz = math.cos(a), -math.sin(a), math.sin(a), math.cos(a)
    eye = (fx - 5.4 * tx + 2.16 * nx, 1.78, fz - 5.4 * tz + 2.16 * nz)
    return eye, (fx + 0.88 * tx, 0.84, fz + 0.88 * tz)


def dressing(scheme):
    """Append the committed dressing's collections and give them the QML materials."""
    with bpy.data.libraries.load(str(AUTHORED / "rowing-dressing.blend"), link=False) as (src, dst):
        dst.collections = ["rowplay-dressing"]
    root = dst.collections[0]
    bpy.context.scene.collection.children.link(root)
    bpy.data.collections["furniture-variants"].hide_render = True
    materials = {}
    for cls, (roughness, specular, metalness) in DRESSING_CLASSES.items():
        mat = vertex_colour_material(f"review-{cls}", scheme, roughness, specular, False)
        mat.node_tree.nodes["Principled BSDF"].inputs["Metallic"].default_value = metalness
        materials[cls] = mat
    for obj in root.all_objects:
        if obj.type == "MESH":
            for slot in obj.material_slots:
                if slot.material and slot.material.name in materials:
                    slot.material = materials[slot.material.name]


def buoys_and_water(scheme):
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(AUTHORED / "buoy.glb"))
    buoy = next(o for o in bpy.data.objects if o not in before and o.type == "MESH")
    buoy.hide_render = True
    mat = bpy.data.materials.new("buoy-review")
    info = mat.node_tree.nodes.new("ShaderNodeObjectInfo")
    mat.node_tree.links.new(info.outputs["Color"], mat.node_tree.nodes["Principled BSDF"].inputs["Base Color"])
    mat.node_tree.nodes["Principled BSDF"].inputs["Roughness"].default_value = 0.48
    fogged(mat, scheme)
    buoy.data.materials.clear()
    buoy.data.materials.append(mat)
    for entry in json.loads((AUTHORED / "course.json").read_text()):
        copy = bpy.data.objects.new("buoy", buoy.data)
        copy.location = gl(entry["position"])
        copy.color = linear(entry["color"])
        bpy.context.scene.collection.objects.link(copy)
    bpy.ops.mesh.primitive_plane_add(size=6000, location=(0, 0, 0))
    water = bpy.context.object
    water.name = "water"
    mat = bpy.data.materials.new("water-review")
    nt = mat.node_tree
    bsdf = nt.nodes["Principled BSDF"]
    bsdf.inputs["Base Color"].default_value = linear(WATER)
    bsdf.inputs["Roughness"].default_value = 0.32
    bsdf.inputs["Metallic"].default_value = 0.15
    bsdf.inputs["Specular IOR Level"].default_value = 0.65
    image = bpy.data.images.load(str(AUTHORED / "water-normal.png"))
    image.colorspace_settings.name = "Non-Color"
    texture = nt.nodes.new("ShaderNodeTexImage")
    texture.image = image
    coords = nt.nodes.new("ShaderNodeTexCoord")
    mapping = nt.nodes.new("ShaderNodeMapping")
    mapping.inputs["Scale"].default_value = (1 / 8.0, 1 / 8.0, 1.0)  # the 8 m tile
    nt.links.new(coords.outputs["Object"], mapping.inputs["Vector"])
    nt.links.new(mapping.outputs["Vector"], texture.inputs["Vector"])
    normal = nt.nodes.new("ShaderNodeNormalMap")
    normal.inputs["Strength"].default_value = 0.28
    nt.links.new(texture.outputs["Color"], normal.inputs["Color"])
    nt.links.new(normal.outputs["Normal"], bsdf.inputs["Normal"])
    water.data.materials.append(mat)
    fogged(mat, scheme)


def environment(scheme):
    """Append the committed source's collections and give them the QML materials."""
    with bpy.data.libraries.load(str(AUTHORED / "rowing-environment.blend"), link=False) as (src, dst):
        dst.collections = ["rowplay-environment"]
    root = dst.collections[0]
    bpy.context.scene.collection.children.link(root)
    bpy.data.collections["vegetation-variants"].hide_render = True
    land = vertex_colour_material("review-land", scheme, 0.95, 0.2, False)
    canopy = vertex_colour_material("review-canopy", scheme, 0.85, 0.25, True)
    for obj in root.all_objects:
        if obj.type == "MESH":
            obj.data.materials.clear()
            obj.data.materials.append(land if obj.data.name.endswith(("terrain", "far-bank")) else canopy)


def lighting(scheme):
    scene = bpy.context.scene
    world = bpy.data.worlds.new(f"sky-{scheme}")
    scene.world = world
    env = world.node_tree.nodes.new("ShaderNodeTexEnvironment")
    env.image = bpy.data.images.load(str(AUTHORED / ("blue-hour.hdr" if scheme == "dark" else "overcast.hdr")))
    world.node_tree.links.new(env.outputs["Color"], world.node_tree.nodes["Background"].inputs["Color"])
    colour, brightness = KEY[scheme]
    sun = bpy.data.lights.new("key", "SUN")
    sun.color = linear(colour)[:3]
    # A Qt DirectionalLight's brightness multiplies radiance; a Blender sun's
    # strength is irradiance, so the same look needs pi times the value.
    sun.energy = brightness * math.pi
    sun.use_shadow = False  # Medium: no dynamic shadows
    key = bpy.data.objects.new("key", sun)
    scene.collection.objects.link(key)
    key.rotation_euler = (-gl(SUN_OFFSET).normalized()).to_track_quat("-Z", "Y").to_euler()
    for obj in scene.objects:
        if obj.type == "MESH" and obj.name != "water":
            obj.visible_glossy = False  # Qt's water reflects only the probe


def camera(name, eye, target, resolution, fov=FOV, ortho=None):
    scene = bpy.context.scene
    data = bpy.data.cameras.new(name)
    data.sensor_fit = "VERTICAL"
    data.clip_start, data.clip_end = 0.1, 3000.0
    if ortho:
        data.type, data.ortho_scale = "ORTHO", ortho
    else:
        data.angle = math.radians(fov)
    obj = bpy.data.objects.new(name, data)
    scene.collection.objects.link(obj)
    obj.location = gl(eye)
    direction = gl(target) - gl(eye)
    obj.rotation_euler = ((0.0, 0.0, 0.0) if ortho else direction.to_track_quat("-Z", "Y").to_euler())
    scene.camera = obj
    scene.render.resolution_x, scene.render.resolution_y = resolution
    return obj


def render(path):
    bpy.context.scene.render.filepath = str(path)
    bpy.ops.render.render(write_still=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scheme", choices=("light", "dark"), default="light")
    parser.add_argument("--views", default="chase,lap,wide,top,bank,water,zones,dressing")
    parser.add_argument("--engine", choices=("BLENDER_EEVEE", "CYCLES"), default="BLENDER_EEVEE")
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    args.output.mkdir(parents=True, exist_ok=False)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    rig = place_shell(args.scheme)
    place_rig(rig, FRAME_METRES)
    dressing(args.scheme)
    buoys_and_water(args.scheme)
    environment(args.scheme)
    lighting(args.scheme)
    scene.render.engine = args.engine
    if args.engine == "CYCLES":
        scene.cycles.samples = 64
        scene.cycles.use_denoising = True
    else:
        scene.eevee.use_raytracing = False
        scene.eevee.taa_render_samples = 32
    scene.view_settings.view_transform = "AgX"
    scene.render.image_settings.file_format = "PNG"
    s = args.scheme
    views = args.views.split(",")
    if "chase" in views:
        camera("chase", *chase(FRAME_METRES), (1600, 998))
        render(args.output / f"{s}-chase.png")
    if "lap" in views:
        for metres in range(0, 1000, 125):
            place_rig(rig, metres)
            camera(f"lap-{metres}", *chase(metres), (800, 499))
            render(args.output / f"{s}-lap-{metres:03d}.png")
        place_rig(rig, FRAME_METRES)
    if "wide" in views:
        camera("wide", (-95.0, 48.0, 120.0), (15.0, 0.0, -5.0), (1600, 998), fov=48)
        render(args.output / f"{s}-wide.png")
    if "top" in views:
        camera("top", (0.0, 900.0, 0.0), (0.0, 0.0, 0.0), (1400, 1400), ortho=560)
        render(args.output / f"{s}-top.png")
    if "bank" in views:
        camera("bank-woodland", (-2.0, 1.8, -31.0), (32.0, 1.5, -48.0), (1600, 998))
        render(args.output / f"{s}-bank-woodland.png")
        camera("bank-wetland", (-40.0, 3.0, 18.0), (8.0, 0.5, 45.0), (1600, 998))
        render(args.output / f"{s}-bank-wetland.png")
        camera("bank-campus", (10.0, 2.2, 20.0), (40.0, 0.5, 40.0), (1600, 998))
        render(args.output / f"{s}-bank-campus.png")
    if "water" in views:
        camera("water", (0.0, 1.2, 29.0), (12.0, 0.0, 26.0), (1600, 998), fov=30)
        render(args.output / f"{s}-water.png")
    if "zones" in views:
        for name, degrees in (("launch", 15), ("finish", 25), ("finish-near", 40), ("bridge", 130),
                              ("farbank", 215), ("coaching", 250), ("wetland-a", 290), ("wetland-b", 320)):
            metres = 1000 * degrees / 360
            place_rig(rig, metres)
            camera(f"zone-{name}", *chase(metres), (1200, 750))
            render(args.output / f"{s}-zone-{name}.png")
        place_rig(rig, FRAME_METRES)
    if "dressing" in views:
        def on_loop(angle, radius, height):
            a = math.radians(angle)
            return (radius * math.sin(a), height, radius * math.cos(a))
        for name, eye, target in (
                ("tower", on_loop(44, 30, 2.5), on_loop(52, 38, 5.0)),
                ("jetty", on_loop(44, 20, 2.2), on_loop(52, 19, 0.5)),
                ("launch-pontoon", on_loop(24, 31, 2.6), on_loop(31, 36, 0.4)),
                ("campus", on_loop(30, 44, 3.0), on_loop(30, 68, 2.5)),
                ("bridge", on_loop(139, 27, 2.4), on_loop(148, 29, 3.5)),
                ("coaching", on_loop(256, 31, 2.2), on_loop(264, 36.5, 0.5)),
                ("boardwalk", on_loop(318, 33.5, 2.4), on_loop(332, 40.5, 0.8)),
                ("hide", on_loop(328, 37, 2.6), on_loop(335, 42.4, 1.5)),
                ("board-250", on_loop(136, 33, 1.9), on_loop(142, 38.6, 2.0)),
                ("island", on_loop(120, 22, 4.0), on_loop(60, 6, 0.5))):
            camera(f"close-{name}", eye, target, (1000, 625), fov=42)
            render(args.output / f"{s}-close-{name}.png")


if __name__ == "__main__":
    main()
