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
import export_dressing
import export_environment
import shell
from water import normals as water_field

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
    """The Phase 3 water normal (water.py), written through Blender's image API."""
    normals, record = water_field()
    size = record["size"]
    pixels = np.ones((size, size, 4), dtype=np.float32)
    pixels[:, :, :3] = normals * 0.5 + 0.5
    img = bpy.data.images.new("water-normal", size, size)
    img.colorspace_settings.name = "Non-Color"
    img.pixels.foreach_set(pixels.ravel())
    img.filepath_raw = str(output)
    img.file_format = "PNG"
    img.save()
    return record


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


def rowing_shell(output, previews):
    """The single scull and its sculls (Phase 2), budget-checked before export."""
    reset()
    counts = shell.build(material("rowplay-neutral", "#8c9399", 0.5))
    total = shell.rendered_triangles(counts)
    if total > shell.TRIANGLE_BUDGET:
        raise ValueError(f"shell and oars: {total} triangles exceeds {shell.TRIANGLE_BUDGET}")
    shell.export(output)
    shell.validate(output)
    if previews:
        previews.mkdir(parents=True, exist_ok=True)
        shell.contact_sheet(previews / "shell-contact-sheet.png")
    return {"blender": bpy.app.version_string, "triangles": dict(counts, rendered=total),
            "budget": shell.TRIANGLE_BUDGET}


def course_json(entries):
    """One buoy per line, so a moved buoy is a one-line diff."""
    lines = (json.dumps(entry, separators=(", ", ": ")) for entry in entries)
    return "[\n" + ",\n".join("  " + line for line in lines) + "\n]\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=ROOT / "assets/replay/authored")
    parser.add_argument("--previews", type=Path, default=ROOT / "build/previews")
    parser.add_argument("--balsam", default=os.environ.get("ROWPLAY_BALSAM", "balsam"))
    parser.add_argument("--only", choices=("all", "shell", "water", "environment", "dressing"), default="all",
                        help="rebuild one part alone and re-pin it in MANIFEST.json: shell (Phase 2), "
                             "water or environment (Phase 3), dressing (Phase 4)")
    args = parser.parse_args(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else [])
    args.output.mkdir(parents=True, exist_ok=True)
    print("Blender", bpy.app.version_string, "seed", SEED)
    if args.only != "all":
        report = json.loads((args.output / "MANIFEST.json").read_text())
        if args.only == "shell":
            report["shell"] = rowing_shell(args.output / "rowing-shell.glb", args.previews)
            pinned = ["rowing-shell.glb"]
        elif args.only == "water":
            report["water"] = water(args.output / "water-normal.png")
            pinned = ["water-normal.png"]
        elif args.only == "dressing":
            report["dressing"] = export_dressing.build(args.output / "rowing-dressing.blend", args.output)
            pinned = ["rowing-dressing.blend", "rowing-dressing.glb", "dressing.json"]
        else:
            report["environment"] = export_environment.build(
                args.output / "rowing-environment.blend", args.output)
            pinned = ["rowing-environment.blend", "rowing-environment.glb", "vegetation.json"]
        for name in pinned:
            data = (args.output / name).read_bytes()
            report["files"][name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        report["files"] = dict(sorted(report["files"].items()))
        write_manifest(args.output, report)
        return
    sky(args.output / "overcast.hdr", False)
    sky(args.output / "blue-hour.hdr", True)
    for name in ("overcast", "blue-hour"):
        bake(args.output / f"{name}.hdr", args.output / f"{name}.ktx", args.balsam,
             ROOT / "build/blender-probes")
    water_report = water(args.output / "water-normal.png")
    triangles = buoy(args.output / "buoy.glb", args.previews)
    (args.output / "course.json").write_text(course_json(placements()))
    shell_report = rowing_shell(args.output / "rowing-shell.glb", args.previews)
    # Last: each opens its source file in place of the scene. The dressing
    # goes first, since the environment's export reads its footprints.
    dressing_report = export_dressing.build(args.output / "rowing-dressing.blend", args.output)
    environment_report = export_environment.build(args.output / "rowing-environment.blend", args.output)
    files = {}
    for path in sorted(args.output.iterdir()):
        if path.name == "MANIFEST.json" or not path.is_file():
            continue
        data = path.read_bytes()
        files[path.name] = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    report = {"blender": bpy.app.version_string, "seed": SEED,
              "buoyTriangles": triangles, "buoyInstances": 256,
              "textureMax": 512, "shell": shell_report, "water": water_report,
              "environment": environment_report, "dressing": dressing_report, "files": files}
    write_manifest(args.output, report)


# The authored pack's plain-Git budget (ADR 0011). Blender Phase 3 raised it
# from 4 to 6 MiB for the environment's source file and its outputs, and
# Phase 4 to 8 MiB for the dressing's.
PACK_BUDGET = 8 * 1024 * 1024


def write_manifest(output, report):
    if sum(f["bytes"] for f in report["files"].values()) > PACK_BUDGET:
        raise ValueError(f"authored pack exceeds {PACK_BUDGET} bytes")
    (output / "MANIFEST.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
