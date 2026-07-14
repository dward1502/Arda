# Render a GLB from N camera angles via bpy. Driven entirely by env vars so
# forge-mind can shell out cleanly from Rust.
#
# Required env:
#   FORGE_GLB_PATH      absolute path to .glb to import
#   FORGE_OUTPUT_DIR    absolute directory to write PNGs into
#   FORGE_ASSET_ID      filename prefix for outputs
# Optional env:
#   FORGE_RENDER_WIDTH  (default 768)
#   FORGE_RENDER_HEIGHT (default 768)
#   FORGE_ANGLES        comma-separated names from {front, three_quarter, side, top, back} (default "front,three_quarter,side")
#
# Outputs: <OUTPUT_DIR>/<ASSET_ID>_<angle>.png for each requested angle.

import math
import os

import bpy
import mathutils


GLB_PATH = os.environ["FORGE_GLB_PATH"]
OUTPUT_DIR = os.environ["FORGE_OUTPUT_DIR"]
ASSET_ID = os.environ["FORGE_ASSET_ID"]
WIDTH = int(os.environ.get("FORGE_RENDER_WIDTH", "768"))
HEIGHT = int(os.environ.get("FORGE_RENDER_HEIGHT", "768"))
ANGLES = [
    a.strip()
    for a in os.environ.get("FORGE_ANGLES", "front,three_quarter,side").split(",")
    if a.strip()
]

os.makedirs(OUTPUT_DIR, exist_ok=True)


def clear_scene():
    for coll in (
        bpy.data.objects,
        bpy.data.meshes,
        bpy.data.materials,
        bpy.data.images,
        bpy.data.lights,
        bpy.data.cameras,
        bpy.data.armatures,
    ):
        for item in list(coll):
            coll.remove(item)


def world_bounds():
    min_co = mathutils.Vector((float("inf"), float("inf"), float("inf")))
    max_co = mathutils.Vector((float("-inf"), float("-inf"), float("-inf")))
    found = False
    for obj in bpy.data.objects:
        if obj.type != "MESH":
            continue
        found = True
        for v in obj.bound_box:
            wv = obj.matrix_world @ mathutils.Vector(v)
            min_co.x = min(min_co.x, wv.x)
            min_co.y = min(min_co.y, wv.y)
            min_co.z = min(min_co.z, wv.z)
            max_co.x = max(max_co.x, wv.x)
            max_co.y = max(max_co.y, wv.y)
            max_co.z = max(max_co.z, wv.z)
    if not found:
        raise RuntimeError("[forge-render] no MESH objects in GLB")
    return min_co, max_co


def add_area_light(name, location, energy, size=4.0):
    data = bpy.data.lights.new(name=name, type="AREA")
    data.energy = energy
    data.size = size
    obj = bpy.data.objects.new(name=name, object_data=data)
    bpy.context.scene.collection.objects.link(obj)
    obj.location = location
    return obj


def setup_white_world():
    world = bpy.data.worlds.new("ForgeWorld")
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    if bg is not None:
        bg.inputs[0].default_value = (1.0, 1.0, 1.0, 1.0)
        bg.inputs[1].default_value = 1.0
    bpy.context.scene.world = world


def angle_position(angle_name, center, distance):
    cx, cy, cz = center
    d = distance
    if angle_name == "front":
        return (cx, cy - d, cz + d * 0.25)
    if angle_name == "three_quarter":
        return (cx + d * 0.8, cy - d * 0.7, cz + d * 0.45)
    if angle_name == "side":
        return (cx + d, cy, cz + d * 0.25)
    if angle_name == "top":
        return (cx, cy - d * 0.3, cz + d * 1.4)
    if angle_name == "back":
        return (cx, cy + d, cz + d * 0.25)
    raise ValueError(f"unknown angle: {angle_name}")


def look_at(obj, target):
    direction = mathutils.Vector(target) - mathutils.Vector(obj.location)
    rot_quat = direction.to_track_quat("-Z", "Y")
    obj.rotation_euler = rot_quat.to_euler()


def select_engine():
    engine_enum = bpy.types.RenderSettings.bl_rna.properties["engine"].enum_items
    keys = [item.identifier for item in engine_enum]
    for candidate in ("BLENDER_EEVEE_NEXT", "BLENDER_EEVEE", "CYCLES"):
        if candidate in keys:
            return candidate
    return keys[0]


def main():
    clear_scene()
    bpy.ops.import_scene.gltf(filepath=GLB_PATH)

    min_co, max_co = world_bounds()
    center = (min_co + max_co) / 2.0
    size = max(max_co.x - min_co.x, max_co.y - min_co.y, max_co.z - min_co.z)
    distance = max(size * 1.5, 0.5) + 1.0

    add_area_light("key", (center.x + distance, center.y - distance, center.z + distance), 900.0)
    add_area_light("fill", (center.x - distance, center.y + distance * 0.4, center.z + distance * 0.6), 450.0)
    add_area_light("rim", (center.x, center.y + distance * 1.4, center.z + distance * 0.7), 350.0)
    setup_white_world()

    cam_data = bpy.data.cameras.new("ForgeCam")
    cam_obj = bpy.data.objects.new("ForgeCam", cam_data)
    bpy.context.scene.collection.objects.link(cam_obj)
    bpy.context.scene.camera = cam_obj

    scene = bpy.context.scene
    scene.render.engine = select_engine()
    scene.render.resolution_x = WIDTH
    scene.render.resolution_y = HEIGHT
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False

    for angle in ANGLES:
        cam_obj.location = angle_position(angle, center, distance)
        look_at(cam_obj, center)
        out_path = os.path.join(OUTPUT_DIR, f"{ASSET_ID}_{angle}.png")
        scene.render.filepath = out_path
        bpy.ops.render.render(write_still=True)
        print(f"[forge-render] wrote {out_path}")

    print(f"[forge-render] complete: {len(ANGLES)} angles for asset_id={ASSET_ID}")


if __name__ == "__main__":
    main()
