# SPDX-License-Identifier: GPL-3.0-or-later
"""Blender hygiene pass for the baked venue GLBs (Phase 6a, ADR 0010).

Run through the bake script; the direct form is:

    blender --background --threads 1 --python tools/bake-venues/cleanup.py -- \
        --input build/venues-staging --output build/venues-cleaned

What it does, and why only this much:

  * drops loose vertices/edges and dissolves zero-area faces — geometry
    hygiene that a procedural builder inevitably leaves behind;
  * merges Blender's split material copies (``name.001``) back onto the base
    material — Blender's importer copies a shared material when the meshes
    using it disagree about vertex colours, because Blender models vertex
    colour as a shader node while glTF models it as a per-vertex attribute.
    glTF and Qt Quick 3D apply vertex colour per mesh, so one material name is
    correct; leaving the copies breaks the runtime's name-keyed rebinding;
  * verifies the round-trip itself: every mesh/node name, material name,
    vertex-colour layer and UV layer must survive 1:1. Any loss fails the
    bake;
  * re-exports with a fixed glTF option set (notably
    ``export_vertex_color="ACTIVE"``, so vertex colours survive the material
    consolidation), single-threaded and with PYTHONHASHSEED=0 (the
    reproducibility guards the web repo's own rig authoring uses).

Deliberately NOT done, per ADR 0010:

  * merging by material — the archetype + contract split keeps venue files
    small and draw calls low; merging would undo it;
  * baking ambient occlusion — that would embed baked textures, which the
    contract forbids (textures are rebound from the vendored sets and the
    procedural PNGs at runtime).

Blender triangulates on export where the source had n-gons (the web builder's
extrude/rounded-block geometry), which is why the vert counts can shift a few
percent; the bake's ``--finalize`` step re-derives the contract inventory from
the cleaned GLBs. Node names, material names, UV layers and colour layers are
verified 1:1, and with the instancing already split out by the baker the node
count is stable too.
"""

import argparse
import json
import os
import re
import sys

import bpy


def parse_args():
    argv = sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, help="directory of staged GLBs")
    parser.add_argument("--output", required=True, help="directory for cleaned GLBs")
    parser.add_argument("--report", help="optional JSON report path")
    return parser.parse_args(argv)


def mesh_inventory():
    """Structural facts that the hygiene pass must preserve."""
    inventory = {}
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        mesh = obj.data
        inventory[obj.name] = {
            "verts": len(mesh.vertices),
            "polys": len(mesh.polygons),
            "colour_layers": sorted(layer.name for layer in mesh.color_attributes),
            "uv_layers": sorted(layer.name for layer in mesh.uv_layers),
            "materials": [mat.name for mat in mesh.materials if mat],
        }
    return inventory


def material_names():
    return sorted(mat.name for mat in bpy.data.materials)


def clean_meshes():
    for obj in list(bpy.data.objects):
        if obj.type != "MESH":
            continue
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.mesh.delete_loose(use_verts=True, use_edges=True, use_faces=False)
        bpy.ops.mesh.dissolve_degenerate(threshold=1e-4)
        bpy.ops.object.mode_set(mode="OBJECT")
        obj.select_set(False)


DUPLICATE_SUFFIX = re.compile(r"\.\d{3}$")


def consolidate_materials():
    """Fold Blender's ``name.001`` material copies back onto the base name.

    Blender's glTF importer splits a shared material into copies when the
    meshes using it disagree about vertex colours — Blender models vertex
    colour as a shader node, glTF as a per-vertex attribute. The exporter
    then emits ``island-grass-material`` and ``island-grass-material.001``,
    which breaks the runtime's name-keyed rebinding and the bake contract.

    glTF (and Qt Quick 3D) apply vertex colour per mesh, so one material name
    is correct: reassign every mesh slot to the base material here.
    """
    base_by_name = {}
    renamed = []
    for material in list(bpy.data.materials):
        base_name = DUPLICATE_SUFFIX.sub("", material.name)
        if base_name == material.name:
            base_by_name[material.name] = material
            continue
        base = base_by_name.get(base_name)
        if base is None:
            # A genuine distinct asset whose name happens to end in .NNN.
            base_by_name[material.name] = material
            continue
        for obj in bpy.data.objects:
            if obj.type != "MESH":
                continue
            for index, slot_material in enumerate(obj.data.materials):
                if slot_material is material:
                    obj.data.materials[index] = base
        renamed.append((material.name, base_name))
    # Reassigning slots leaves the now-unused copy datablocks behind; remove
    # them so the exported material list matches the contract exactly.
    for name, _ in renamed:
        orphan = bpy.data.materials.get(name)
        if orphan is not None and orphan.users == 0:
            bpy.data.materials.remove(orphan)
    if renamed:
        print(f"materials consolidated: {len(renamed)} copy(ies) -> {sorted({b for _, b in renamed})}")
    return renamed


def verify(before, after, materials_expected, stem):
    """Check the hygiene pass preserved everything the runtime contract needs.

    ``materials_expected`` is the material name set the GLB should end up
    with: the source names minus the ``.NNN`` copies Blender split out (folded
    back onto their base name by :func:`consolidate_materials`).
    """
    problems = []
    if set(before) != set(after):
        missing = sorted(set(before) - set(after))
        added = sorted(set(after) - set(before))
        problems.append(f"mesh names changed (missing: {missing}; added: {added})")
    actual_materials = sorted(material_names())
    if actual_materials != materials_expected:
        problems.append(
            f"material names drifted\nexpected: {materials_expected}\nactual:   {actual_materials}"
        )
    for name, source in before.items():
        target = after.get(name)
        if target is None:
            continue
        if source["uv_layers"] != target["uv_layers"]:
            problems.append(f"{name}: UV layers changed {source['uv_layers']} -> {target['uv_layers']}")
        if source["colour_layers"] != target["colour_layers"]:
            problems.append(
                f"{name}: colour layers changed {source['colour_layers']} -> {target['colour_layers']}"
            )
        if len(source["materials"]) != len(target["materials"]):
            problems.append(f"{name}: material slot count changed {source['materials']} -> {target['materials']}")
        if target["polys"] == 0:
            problems.append(f"{name}: has no faces after cleanup")
    if problems:
        raise RuntimeError(f"{stem}: hygiene pass verification failed:\n  " + "\n  ".join(problems))


def main():
    args = parse_args()
    os.makedirs(args.output, exist_ok=True)
    sources = sorted(name for name in os.listdir(args.input) if name.endswith(".glb"))
    if not sources:
        raise SystemExit(f"no .glb files in {args.input}")

    report = {}
    for name in sources:
        stem = os.path.splitext(name)[0]
        source = os.path.join(args.input, name)
        target = os.path.join(args.output, name)

        bpy.ops.wm.read_factory_settings(use_empty=True)
        bpy.ops.import_scene.gltf(filepath=source)
        before = mesh_inventory()
        materials_before = material_names()

        clean_meshes()
        consolidate_materials()

        # The names the GLB should end up with: source names with the .NNN
        # copies folded onto their base material.
        materials_expected = sorted({DUPLICATE_SUFFIX.sub("", name) for name in materials_before})
        after = mesh_inventory()
        verify(before, after, materials_expected, stem)

        # `export_vertex_color="ACTIVE"` exports COLOR_0 from every mesh with a
        # colour attribute. The default ("MATERIAL") keys off shader usage,
        # and consolidating the split materials changes which shader a mesh
        # points at, so the default would silently drop vertex colours.
        bpy.ops.export_scene.gltf(
            filepath=target,
            export_format="GLB",
            export_yup=True,
            export_vertex_color="ACTIVE",
        )
        report[stem] = {
            "bytes_in": os.path.getsize(source),
            "bytes_out": os.path.getsize(target),
            "meshes": len(after),
            "verts_before": sum(m["verts"] for m in before.values()),
            "verts_after": sum(m["verts"] for m in after.values()),
            "polys_after": sum(m["polys"] for m in after.values()),
        }
        print(
            f"{stem}: {report[stem]['bytes_in']} B -> {report[stem]['bytes_out']} B | "
            f"verts {report[stem]['verts_before']} -> {report[stem]['verts_after']} | "
            f"{report[stem]['polys_after']} tris"
        )

    if args.report:
        with open(args.report, "w") as handle:
            json.dump(report, handle, indent=2, sort_keys=True)
            handle.write("\n")


main()
