# SPDX-License-Identifier: GPL-3.0-or-later
"""Generate the Direction C assets. Run with Blender, arguments after --."""

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import sys

import bpy
import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from common import contact_sheet, export, linear, material, reset
from probes import bake

SEED = 20260926
ROOT = Path(__file__).resolve().parents[2]


def sky(output, dark):
    scene = reset()
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 8
    scene.cycles.seed = SEED
    scene.render.resolution_x = 512
    scene.render.resolution_y = 256
    scene.render.resolution_percentage = 100
    world = bpy.data.worlds.new("blue-hour" if dark else "overcast")
    world.use_nodes = True
    scene.world = world
    nodes, links = world.node_tree.nodes, world.node_tree.links
    tex = nodes.new("ShaderNodeTexCoord")
    separate = nodes.new("ShaderNodeSeparateXYZ")
    links.new(tex.outputs["Normal"], separate.inputs[0])
    scale = nodes.new("ShaderNodeMath")
    scale.operation = "MULTIPLY_ADD"
    scale.inputs[1].default_value = 0.5
    scale.inputs[2].default_value = 0.5
    links.new(separate.outputs["Z"], scale.inputs[0])
    ramp = nodes.new("ShaderNodeValToRGB")
    ramp.color_ramp.elements.remove(ramp.color_ramp.elements[1])
    # Independent later-ambient palette: a bright cool horizon above darker water.
    colors = ("#424e61", "#aeb9cc", "#7e96b9") if dark else ("#747d7c", "#ccd0cf", "#e1e7eb")
    for i, (position, color) in enumerate(zip((0.0, 0.5, 1.0), colors)):
        elem = ramp.color_ramp.elements[0] if i == 0 else ramp.color_ramp.elements.new(position)
        elem.position = position
        elem.color = linear(color)
    links.new(scale.outputs[0], ramp.inputs[0])
    links.new(ramp.outputs["Color"], nodes["Background"].inputs["Color"])
    nodes["Background"].inputs["Strength"].default_value = 1.15
    cam_data = bpy.data.cameras.new("equirectangular")
    cam_data.type = "PANO"
    cam_data.panorama_type = "EQUIRECTANGULAR"
    cam = bpy.data.objects.new("equirectangular", cam_data)
    scene.collection.objects.link(cam)
    cam.rotation_euler = (math.pi / 2, 0, 0)
    scene.camera = cam
    scene.render.image_settings.file_format = "HDR"
    scene.render.filepath = str(output)
    bpy.ops.render.render(write_still=True)


def water(output):
    """Periodic analytic wave derivatives, tangent-space +Y normal, 4 m tile."""
    size = 512
    rng = np.random.default_rng(SEED)
    v, u = np.mgrid[:size, :size] / size * (2 * math.pi)
    dx, dy = np.zeros_like(u), np.zeros_like(v)
    for _ in range(16):
        kx, ky = int(rng.integers(2, 15)), int(rng.integers(1, 5))
        phase = rng.uniform(0, 2 * math.pi)
        amplitude = 0.028 / math.sqrt(kx * kx + ky * ky)
        slope = amplitude * np.cos(kx * u + ky * v + phase)
        dx += kx * slope
        dy += ky * slope
    normals = np.stack((-dx, -dy, np.ones_like(dx)), -1)
    normals /= np.linalg.norm(normals, axis=-1, keepdims=True)
    pixels = np.ones((size, size, 4), dtype=np.float32)
    pixels[:, :, :3] = normals * 0.5 + 0.5
    img = bpy.data.images.new("water-normal", size, size)
    img.colorspace_settings.name = "Non-Color"
    img.pixels.foreach_set(pixels.ravel())
    img.filepath_raw = str(output)
    img.file_format = "PNG"
    img.save()


def buoy(output, previews):
    reset()
    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=8, radius=1)
    obj = bpy.context.object
    obj.name = "course-buoy"
    obj.scale = (0.14, 0.14, 0.105)
    obj.location.z = 0.035
    bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
    obj.data.materials.append(material("buoy", "#d77838", 0.48))
    for face in obj.data.polygons:
        face.use_smooth = True
    triangles = export(output)
    if previews:
        previews.mkdir(parents=True, exist_ok=True)
        contact_sheet(previews / "buoy-contact-sheet.png")
    return triangles


def placements():
    # Fixed world-space lane edges outside the oar sweep; preserve 26/30 m lanes.
    result = []
    for radius in (23.1, 33.3):
        for i in range(128):
            angle = i * 2 * math.pi / 128
            result.append({"position": [round(radius * math.sin(angle), 6), 0,
                                        round(radius * math.cos(angle), 6)],
                           "color": "#d77838" if i % 16 < 4 else "#e8e7df"})
    return result


def course_json(entries):
    """One buoy per line, so a moved buoy is a one-line diff."""
    lines = (json.dumps(entry, separators=(", ", ": ")) for entry in entries)
    return "[\n" + ",\n".join("  " + line for line in lines) + "\n]\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "assets/replay/authored")
    parser.add_argument("--previews", type=Path, default=ROOT / "build/previews")
    parser.add_argument("--balsam", default=os.environ.get("ROWPLAY_BALSAM", "balsam"))
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    args.output.mkdir(parents=True, exist_ok=True)
    print("Blender", bpy.app.version_string, "seed", SEED)
    sky(args.output / "overcast.hdr", False)
    sky(args.output / "blue-hour.hdr", True)
    for name in ("overcast", "blue-hour"):
        bake(args.output / f"{name}.hdr", args.output / f"{name}.ktx", args.balsam,
             ROOT / "build/blender-probes")
    water(args.output / "water-normal.png")
    triangles = buoy(args.output / "buoy.glb", args.previews)
    (args.output / "course.json").write_text(course_json(placements()))
    files = {}
    for path in sorted(args.output.iterdir()):
        if path.name == "MANIFEST.json" or not path.is_file():
            continue
        data = path.read_bytes()
        files[path.name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    report = {"blender": bpy.app.version_string, "seed": SEED,
              "buoyTriangles": triangles, "buoyInstances": 256,
              "textureMax": 512, "files": files}
    (args.output / "MANIFEST.json").write_text(json.dumps(report, indent=2) + "\n")
    if sum(f["bytes"] for f in files.values()) > 4 * 1024 * 1024:
        raise ValueError("authored source budget exceeds 4 MiB")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
