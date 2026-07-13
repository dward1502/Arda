# Grok Prompt 3 — Monitor-Specific Treatment.
#
# Boardroom monitor assets contain meshes with predictable name suffixes:
#   *_rear_bezel, *_left_side_bezel, *_right_side_bezel, *_top_trim   → bezel parts
#   *_screen_plane                                                    → display surface
#   *_mount_arm, *_mount_foot                                         → mount hardware
#   *_ui_bar_N, *_*_cyan_trace, *_*_magenta_trace                     → emissive UI
#
# This template:
#   1. Applies a small bevel to bezel parts (0.008m, 2 seg) — keeps thin-bezel look.
#   2. Applies a sharper chamfer to bezel corners.
#   3. Boosts emissive material strength on UI bars + traces (Joi-blue presence).
#   4. Sets screen material to a clean holographic emissive base.
#   5. Mount hardware: standard moderate bevel (0.015m, 3 seg).

import bpy
import bmesh
import mathutils
from math import radians


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


def deselect_all():
    for obj in bpy.data.objects:
        obj.select_set(False)


def bake_transform(obj):
    obj.data.transform(obj.matrix_world)
    obj.matrix_world = mathutils.Matrix.Identity(4)


def bevel_mesh(obj, offset, segments, profile=0.7):
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.dissolve_degenerate(bm, dist=1e-4, edges=bm.edges[:])
    bmesh.ops.bevel(
        bm,
        geom=bm.edges[:],
        offset=offset,
        offset_type='OFFSET',
        segments=segments,
        profile=profile,
        affect='EDGES',
    )
    bm.normal_update()
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def smart_uv(obj, area, region, window):
    deselect_all()
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    override = {
        "window": window, "screen": window.screen, "area": area, "region": region,
        "scene": bpy.context.scene, "view_layer": bpy.context.view_layer,
        "active_object": obj, "object": obj, "edit_object": obj,
        "selected_objects": [obj], "selected_editable_objects": [obj],
    }
    with bpy.context.temp_override(**override):
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(angle_limit=radians(66), island_margin=0.01, area_weight=0.0)
        bpy.ops.object.mode_set(mode='OBJECT')


def find_principled(mat):
    if not mat or not mat.use_nodes:
        return None
    return next((n for n in mat.node_tree.nodes if n.type == 'BSDF_PRINCIPLED'), None)


def boost_emissive(mat, color, strength):
    if not mat:
        return
    if not mat.use_nodes:
        mat.use_nodes = True
    pri = find_principled(mat)
    if pri is None:
        return
    # Blender 4.x renamed sockets — fall back if "Emission" isn't present.
    for socket_name in ("Emission Color", "Emission"):
        if socket_name in pri.inputs:
            pri.inputs[socket_name].default_value = (*color, 1.0)
            break
    for socket_name in ("Emission Strength", "Emissive Strength"):
        if socket_name in pri.inputs:
            pri.inputs[socket_name].default_value = strength
            break


def classify(name):
    n = name.lower()
    if "bezel" in n or "_top_trim" in n:
        return "bezel"
    if "screen" in n:
        return "screen"
    if "mount" in n:
        return "mount"
    if "ui_bar" in n or "cyan_trace" in n or "magenta_trace" in n or "_trace" in n:
        return "trace"
    return "other"


print("[forge-mind] Prompt 3: monitor treatment starting")

WINDOW, AREA, REGION = find_view3d()
if AREA is None:
    raise RuntimeError("[forge-mind] no VIEW_3D area available")

mesh_objects = [o for o in bpy.data.objects if o.type == 'MESH']
if not mesh_objects:
    raise RuntimeError("[forge-mind] no MESH objects found")

counts = {"bezel": 0, "screen": 0, "mount": 0, "trace": 0, "other": 0}

for obj in mesh_objects:
    kind = classify(obj.name)
    counts[kind] += 1

    bake_transform(obj)

    if kind == "bezel":
        bevel_mesh(obj, offset=0.008, segments=2, profile=0.85)  # thin, sharp
    elif kind == "screen":
        bevel_mesh(obj, offset=0.004, segments=2, profile=0.5)   # minimal — keep flat
        for slot in obj.material_slots:
            if slot.material is not None:
                # Holographic-ready emissive base.
                boost_emissive(slot.material, (0.53, 1.0, 1.0), 0.8)
    elif kind == "mount":
        bevel_mesh(obj, offset=0.015, segments=3, profile=0.7)   # moderate
    elif kind == "trace":
        bevel_mesh(obj, offset=0.003, segments=2, profile=0.5)   # tiny
        for slot in obj.material_slots:
            if slot.material is not None:
                name = slot.material.name.lower()
                if "magenta" in name:
                    boost_emissive(slot.material, (1.0, 0.13, 0.83), 4.5)
                else:
                    boost_emissive(slot.material, (0.13, 1.0, 0.95), 4.5)
    else:
        bevel_mesh(obj, offset=0.01, segments=2, profile=0.7)

    smart_uv(obj, AREA, REGION, WINDOW)

# Final selection state for the forge-mind exporter wrapper.
deselect_all()
for obj in mesh_objects:
    obj.select_set(True)
bpy.context.view_layer.objects.active = mesh_objects[0]

print(
    f"[forge-mind] Prompt 3 complete: "
    f"bezel={counts['bezel']} screen={counts['screen']} mount={counts['mount']} "
    f"trace={counts['trace']} other={counts['other']}"
)
