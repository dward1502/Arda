from __future__ import annotations

"""Generate the Cortana-style holographic presence avatar GLB.

Run with:
    flatpak run org.blender.Blender --background --python \
        tools/blender/build_presence_cortana.py

Output:
    src/assets/scene/hologram/presence_cortana/presence_form.glb
    src/assets/scene/hologram/presence_cortana/metadata.json

Stylized feminine humanoid silhouette (Cortana-like) built from primitives,
~1.7 scene units tall, standing on the boardroom emitter.
"""

import json
from pathlib import Path

import bpy


SCRIPT_PATH = Path(__file__).resolve()
HUD_ROOT = SCRIPT_PATH.parents[2]
ASSET_DIR = HUD_ROOT / "src/assets/scene/hologram/presence_cortana"
GLB_PATH = ASSET_DIR / "presence_form.glb"
RENDER_PATH = ASSET_DIR / "presence_cortana_reference.png"

BODY = (0.42, 0.85, 1.0, 1.0)
LIMB = (0.30, 0.72, 0.92, 1.0)
EMISSION_STRENGTH = 3.0


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.meshes, bpy.data.materials):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)


def make_material(name: str, color) -> bpy.types.Material:
    material = bpy.data.materials.new(name)
    material.use_nodes = True
    principled = material.node_tree.nodes.get("Principled BSDF")
    principled.inputs["Base Color"].default_value = color
    try:
        principled.inputs["Alpha"].default_value = 0.65
    except KeyError:
        pass
    for label, val in (("Emission Color", color), ("Emission Strength", EMISSION_STRENGTH)):
        try:
            principled.inputs[label].default_value = val if isinstance(val, tuple) else val
        except KeyError:
            pass
    return material


def _apply(obj, name, material):
    obj.name = name
    obj.data.name = name
    obj.data.materials.append(material)
    return obj


def add_uv_sphere(name, loc, radius, material, scale=None, segments=24, rings=16):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=segments, ring_count=rings, radius=radius, location=loc)
    obj = bpy.context.active_object
    if scale:
        obj.scale = scale
    bpy.ops.object.transform_apply(scale=True)
    return _apply(obj, name, material)


def add_capsule(name, loc, radius, depth, material):
    bpy.ops.mesh.primitive_cylinder_add(vertices=24, radius=radius, depth=depth, location=loc)
    cyl = bpy.context.active_object
    parts = [cyl]
    for dz in (-depth / 2, depth / 2):
        bpy.ops.mesh.primitive_uv_sphere_add(segments=20, ring_count=12, radius=radius, location=(loc[0], loc[1], loc[2] + dz))
        parts.append(bpy.context.active_object)
    for o in parts:
        o.select_set(True)
    bpy.context.view_layer.objects.active = cyl
    bpy.ops.object.join()
    return _apply(bpy.context.active_object, name, material)


def add_cone(name, loc, r_base, r_top, depth, material, vertices=24):
    bpy.ops.mesh.primitive_cone_add(vertices=vertices, radius1=r_base, radius2=r_top, depth=depth, location=loc)
    return _apply(bpy.context.active_object, name, material)


def build_figure(body_mat, limb_mat):
    """Stylized feminine humanoid silhouette from primitives.

    y-up; feet ~0.0, head top ~1.70.
    """
    parts = []

    # Head — narrow egg
    parts.append(add_uv_sphere("head", (0, 0, 1.575), 0.105, body_mat, scale=(0.88, 0.95, 1.12)))
    # Neck
    parts.append(add_capsule("neck", (0, 0, 1.45), 0.038, 0.10, body_mat))

    # Torso: chest + tapered waist + hips
    parts.append(add_uv_sphere("chest", (0, 0, 1.28), 0.145, body_mat, scale=(1.05, 0.72, 1.15)))
    parts.append(add_uv_sphere("waist", (0, 0, 1.10), 0.098, body_mat, scale=(1.0, 0.66, 1.0)))
    parts.append(add_uv_sphere("hips", (0, 0, 0.94), 0.125, body_mat, scale=(1.02, 0.74, 0.95)))

    # Shoulders
    for side, sx in (("l", -1), ("r", 1)):
        parts.append(add_uv_sphere(f"shoulder_{side}", (sx * 0.165, 0, 1.36), 0.062, body_mat))

    # Arms — upper arm + forearm, slight outward angle
    for side, sx in (("l", -1), ("r", 1)):
        ux = sx * 0.205
        fx = sx * 0.245
        parts.append(add_capsule(f"upperarm_{side}", (ux, 0, 1.19), 0.042, 0.26, limb_mat))
        parts.append(add_capsule(f"forearm_{side}", (fx, 0.01, 0.97), 0.036, 0.26, limb_mat))
        parts.append(add_uv_sphere(f"hand_{side}", (fx * 1.06, 0.015, 0.82), 0.042, limb_mat))

    # Legs — thigh + shin
    for side, sx in (("l", -1), ("r", 1)):
        tx = sx * 0.07
        parts.append(add_capsule(f"thigh_{side}", (tx, 0, 0.72), 0.058, 0.34, limb_mat))
        parts.append(add_capsule(f"shin_{side}", (tx, -0.005, 0.38), 0.044, 0.34, limb_mat))
        parts.append(add_uv_sphere(f"foot_{side}", (tx, 0.03, 0.05), 0.048, limb_mat, scale=(1.0, 1.4, 0.7)))

    # Hair mass — swept-back volume behind head (Cortana short hair shape)
    parts.append(add_uv_sphere("hair", (0, -0.05, 1.60), 0.112, body_mat, scale=(0.92, 0.9, 1.0)))

    return parts


def join_all(parts):
    for o in parts:
        o.select_set(True)
    bpy.context.view_layer.objects.active = parts[0]
    bpy.ops.object.join()
    return bpy.context.active_object


def main() -> int:
    reset_scene()
    body_mat = make_material("presence_body", BODY)
    limb_mat = make_material("presence_limb", LIMB)

    parts = build_figure(body_mat, limb_mat)
    figure = join_all(parts)
    figure.name = "arda_presence_cortana"

    ASSET_DIR.mkdir(parents=True, exist_ok=True)

    # Export GLB (+Y up convention handled by glTF exporter)
    bpy.ops.export_scene.gltf(filepath=str(GLB_PATH), use_selection=True, export_format="GLB")

    # Render reference image
    cam_data = bpy.data.cameras.new("ref_cam")
    cam = bpy.data.objects.new("ref_cam", cam_data)
    bpy.context.collection.objects.link(cam)
    cam.location = (1.6, 1.2, 1.0)
    direction = cam.location - __import__("mathutils").Vector((0, 0, 0.95))
    cam.rotation_euler = direction.to_track_quat("Z", "Y").to_euler()
    bpy.context.scene.camera = cam
    sun = bpy.data.lights.new("key", type="SUN")
    sun.energy = 3.0
    sun_obj = bpy.data.objects.new("key", sun)
    bpy.context.collection.objects.link(sun_obj)
    sun_obj.rotation_euler = (0.9, 0.2, 0.6)
    bpy.context.scene.render.engine = "BLENDER_EEVEE"
    bpy.context.scene.render.resolution_x = 640
    bpy.context.scene.render.resolution_y = 960
    bpy.context.scene.render.filepath = str(RENDER_PATH)
    bpy.ops.render.render(write_still=True)

    metadata = {
        "id": "hologram_presence_cortana",
        "domain": "hologram",
        "scene_binding": "presence_form",
        "material_family": "hologram_presence",
        "source": "apps/arda-hud/tools/blender/build_presence_cortana.py",
        "license": "Internal",
    }
    (ASSET_DIR / "metadata.json").write_text(json.dumps(metadata, indent=2) + "\n")

    print(f"WROTE {GLB_PATH}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
