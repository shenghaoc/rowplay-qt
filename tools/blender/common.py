# SPDX-License-Identifier: GPL-3.0-or-later
"""Shared metre-scale Blender authoring, export and preview helpers."""

import bpy
from mathutils import Vector

from canonical import canonicalize


def reset():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.context.preferences.system.audio_device = "None"
    scene = bpy.context.scene
    scene.unit_settings.system = "METRIC"
    scene.unit_settings.scale_length = 1.0
    scene.render.threads_mode = "FIXED"
    scene.render.threads = 1
    return scene


def linear(hex_color):
    values = [int(hex_color[i:i + 2], 16) / 255 for i in (1, 3, 5)]
    return tuple(v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
                 for v in values) + (1.0,)


def material(name, color, roughness=0.45, metalness=0.0):
    result = bpy.data.materials.new(name)
    result.use_nodes = True
    result.use_backface_culling = True
    shader = result.node_tree.nodes.get("Principled BSDF")
    shader.inputs["Base Color"].default_value = linear(color)
    shader.inputs["Roughness"].default_value = roughness
    shader.inputs["Metallic"].default_value = metalness
    return result


def export(path, triangle_limit=5000):
    triangles = 0
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.data.calc_loop_triangles()
            triangles += len(obj.data.loop_triangles)
    if triangles > triangle_limit:
        raise ValueError(f"{path.name}: {triangles} triangles exceeds {triangle_limit}")
    bpy.ops.export_scene.gltf(
        filepath=str(path), export_format="GLB", export_yup=True,
        export_animations=False, export_cameras=False, export_lights=False,
        export_extras=True, export_draco_mesh_compression_enable=False,
        export_meshopt_compression_enable=False, export_use_gltfpack=False,
    )
    canonicalize(path)
    return triangles


def camera(location, target, ortho=1.0):
    data = bpy.data.cameras.new("preview-camera")
    data.type = "ORTHO"
    data.ortho_scale = ortho
    obj = bpy.data.objects.new("preview-camera", data)
    bpy.context.collection.objects.link(obj)
    obj.location = location
    obj.rotation_euler = (Vector(target) - obj.location).to_track_quat("-Z", "Y").to_euler()
    bpy.context.scene.camera = obj
    return obj


def contact_sheet(output):
    """Four views combined through Blender's image API; no image dependency."""
    scene = bpy.context.scene
    scene.render.engine = "CYCLES"
    scene.cycles.samples = 16
    scene.cycles.seed = 20260926
    scene.render.resolution_x = 256
    scene.render.resolution_y = 256
    scene.render.resolution_percentage = 100
    scene.world = bpy.data.worlds.new("preview-world")
    scene.world.use_nodes = True
    scene.world.node_tree.nodes["Background"].inputs["Color"].default_value = (0.7, 0.75, 0.8, 1)
    scene.world.node_tree.nodes["Background"].inputs["Strength"].default_value = 0.8
    light = bpy.data.lights.new("softbox", "AREA")
    light.energy = 60
    light.shape = "DISK"
    light.size = 2
    obj = bpy.data.objects.new("softbox", light)
    bpy.context.collection.objects.link(obj)
    obj.location = (1, -1, 2)
    obj.rotation_euler = (-obj.location).to_track_quat("-Z", "Y").to_euler()
    import numpy as np
    sheet = np.zeros((512, 512, 4), dtype=np.float32)
    for i, loc in enumerate(((0, -2, 0.3), (2, 0, 0.3), (0, 0, 2), (1.5, -2, 1.2))):
        cam = camera(loc, (0, 0, 0.05), 0.7)
        path = output.parent / f"buoy-view-{i}.png"
        scene.render.image_settings.file_format = "PNG"
        scene.render.filepath = str(path)
        bpy.ops.render.render(write_still=True)
        img = bpy.data.images.load(str(path))
        pixels = np.array(img.pixels[:], dtype=np.float32).reshape(256, 256, 4)
        sheet[(i // 2) * 256:(i // 2 + 1) * 256, (i % 2) * 256:(i % 2 + 1) * 256] = pixels
        bpy.data.objects.remove(cam, do_unlink=True)
    img = bpy.data.images.new("contact-sheet", 512, 512)
    img.pixels.foreach_set(sheet.ravel())
    img.filepath_raw = str(output)
    img.file_format = "PNG"
    img.save()
