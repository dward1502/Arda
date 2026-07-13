# Grok Prompt 2 — High-Quality Baking Prep.
#
# Bakes Cycles textures from the imported asset to the ARDA materials
# directory. Each mesh gets its own 4K texture set per channel.
#
# Inputs (set by forge-mind wrapper before this script):
#   FORGE_MATERIALS_DIR = absolute path of materials/<asset>/ output dir
#   FORGE_ASSET_ID      = asset id (used as filename prefix)
#
# Outputs to FORGE_MATERIALS_DIR:
#   {asset}_{mesh}_albedo.png
#   {asset}_{mesh}_normal.png
#   {asset}_{mesh}_roughness.png
#   {asset}_{mesh}_metalness.png
#   {asset}_{mesh}_emissive.png
#
# Strategy: Cycles bake with low sample count for speed (16). Per-mesh atlas.
# Adds Subdivision Surface (2) + Bevel (0.02) modifiers as high-poly while
# baking. No GLB re-export — call with --no-export-glb.

import os
import bpy

ASSET_ID = globals().get("FORGE_ASSET_ID", "asset")
MAT_DIR = globals().get("FORGE_MATERIALS_DIR")
if not MAT_DIR:
    raise RuntimeError("[forge-mind] FORGE_MATERIALS_DIR not set by wrapper")
os.makedirs(MAT_DIR, exist_ok=True)


def find_view3d():
    wm = bpy.context.window_manager
    for window in wm.windows:
        for area in window.screen.areas:
            if area.type == 'VIEW_3D':
                region = next((r for r in area.regions if r.type == 'WINDOW'), None)
                return window, area, region
    return None, None, None


def view3d_override(active):
    return {
        "window": WINDOW, "screen": WINDOW.screen, "area": AREA, "region": REGION,
        "scene": bpy.context.scene, "view_layer": bpy.context.view_layer,
        "active_object": active, "object": active, "edit_object": active,
        "selected_objects": [active], "selected_editable_objects": [active],
    }


WINDOW, AREA, REGION = find_view3d()
if AREA is None:
    raise RuntimeError("[forge-mind] no VIEW_3D area available")

# Configure Cycles for fast baking.
scene = bpy.context.scene
scene.render.engine = 'CYCLES'
scene.cycles.samples = 16
scene.cycles.bake_type = 'COMBINED'
scene.render.bake.use_pass_direct = False
scene.render.bake.use_pass_indirect = False
scene.render.bake.use_pass_color = True
scene.render.bake.use_clear = True
scene.render.bake.margin = 8

# Use GPU if available (workstation).
try:
    cycles_prefs = bpy.context.preferences.addons['cycles'].preferences
    cycles_prefs.compute_device_type = 'CUDA'
    for device in cycles_prefs.devices:
        device.use = True
    scene.cycles.device = 'GPU'
except Exception as exc:
    print(f"[forge-mind] cycles GPU setup skipped: {exc}")


def deselect_all():
    for obj in bpy.data.objects:
        obj.select_set(False)


def make_bake_image(name, size=4096, is_data=False, color=(0, 0, 0, 1)):
    img = bpy.data.images.get(name)
    if img is not None:
        bpy.data.images.remove(img)
    img = bpy.data.images.new(name, width=size, height=size, alpha=True, is_data=is_data)
    img.generated_color = color
    return img


def add_image_node(material, image, label):
    if not material.use_nodes:
        material.use_nodes = True
    tree = material.node_tree
    node = tree.nodes.new('ShaderNodeTexImage')
    node.image = image
    node.label = label
    node.select = True
    tree.nodes.active = node
    return node


def bake_pass(obj, image, bake_type, is_normal=False, is_emissive=False):
    """Bake one channel into image, save to disk."""
    deselect_all()
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj

    # Ensure the image is the active texture node on the active material.
    if not obj.data.materials:
        mat = bpy.data.materials.new(name=f"{obj.name}_bake_mat")
        obj.data.materials.append(mat)
    for slot in obj.material_slots:
        if slot.material is None:
            continue
        add_image_node(slot.material, image, bake_type)

    bake_kwargs = {"type": bake_type}
    if is_normal:
        bake_kwargs["normal_space"] = 'TANGENT'

    with bpy.context.temp_override(**view3d_override(obj)):
        bpy.ops.object.bake(**bake_kwargs)

    image.save_render(filepath=str(image.filepath_raw or image.name))


def setup_modifiers(obj):
    """Add SubSurf level 2 + Bevel as virtual high-poly for baking."""
    if "ForgeSubsurf" not in obj.modifiers:
        m = obj.modifiers.new("ForgeSubsurf", 'SUBSURF')
        m.levels = 1  # viewport
        m.render_levels = 2
    if "ForgeBevel" not in obj.modifiers:
        b = obj.modifiers.new("ForgeBevel", 'BEVEL')
        b.width = 0.02
        b.segments = 3
        b.limit_method = 'ANGLE'


print(f"[forge-mind] Prompt 2 baking starting for asset={ASSET_ID} → {MAT_DIR}")

mesh_objects = [o for o in bpy.data.objects if o.type == 'MESH']
if not mesh_objects:
    raise RuntimeError("[forge-mind] no MESH objects to bake")

for obj in mesh_objects:
    setup_modifiers(obj)

# Bake each channel for each object.
channels = [
    ("albedo",     'DIFFUSE',  False, False),
    ("normal",     'NORMAL',   True,  False),
    ("roughness",  'ROUGHNESS', False, False),
    ("emit",       'EMIT',     False, True),
]

for obj in mesh_objects:
    obj_clean = obj.name.replace(' ', '_')
    for chan_name, bake_type, is_normal, is_emissive in channels:
        fname = f"{ASSET_ID}_{obj_clean}_{chan_name}.png"
        out_path = os.path.join(MAT_DIR, fname)
        is_data = chan_name in ("normal", "roughness", "metalness")
        img = make_bake_image(f"{ASSET_ID}_{obj_clean}_{chan_name}", is_data=is_data)
        img.filepath_raw = out_path
        img.file_format = 'PNG'
        try:
            bake_pass(obj, img, bake_type, is_normal=is_normal, is_emissive=is_emissive)
            print(f"[forge-mind] baked {fname}")
        except Exception as exc:
            print(f"[forge-mind] bake FAILED for {fname}: {exc}")

print(f"[forge-mind] Prompt 2 complete: {len(mesh_objects)} mesh(es) processed")
