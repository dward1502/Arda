# Grok Prompt 1 — Geometry Cleanup & Bevels.
#
# Runs on every MESH in the scene. forge-mind prepends a GLB import so this
# operates on the asset to upgrade.
#
# Strategy: use bmesh.ops for everything that has a bmesh equivalent (no
# bpy.context dependency, works reliably under the blender-mcp addon's
# restricted exec context). Only operators with no bmesh equivalent
# (UV smart_project) use bpy.ops with a VIEW_3D temp_override.
#
# Operations:
#  1. Apply transforms via direct matrix bake.
#  2. Topology cleanup via bmesh.ops.dissolve_degenerate + triangulate→quads.
#  3. Moderate bevel via bmesh.ops.bevel (0.03m × 3 seg × 0.7 profile).
#  4. UV Smart Project via bpy.ops (with VIEW_3D context override).

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


def deselect_all():
    for obj in bpy.data.objects:
        obj.select_set(False)


def bake_world_transform(obj):
    """Apply Loc/Rot/Scale by baking matrix_world into mesh data."""
    obj.data.transform(obj.matrix_world)
    obj.matrix_world = mathutils.Matrix.Identity(4)


def clean_and_bevel(obj):
    """Topology cleanup + moderate bevel via bmesh.ops (no context needed)."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)

    # Dissolve degenerate vertices/edges.
    bmesh.ops.dissolve_degenerate(bm, dist=1e-4, edges=bm.edges[:])

    # Triangles → quads where the join is reasonable.
    bmesh.ops.join_triangles(
        bm,
        faces=bm.faces[:],
        cmp_seam=False,
        cmp_sharp=False,
        cmp_uvs=False,
        cmp_vcols=False,
        cmp_materials=False,
        angle_face_threshold=radians(40),
        angle_shape_threshold=radians(40),
    )

    # Moderate bevel on all edges.
    bmesh.ops.bevel(
        bm,
        geom=bm.edges[:],
        offset=0.03,
        offset_type='OFFSET',
        segments=3,
        profile=0.7,
        affect='EDGES',
    )

    bm.normal_update()
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def smart_uv_project(obj, area, region, window):
    """Smart UV Project — bpy.ops only, run with explicit VIEW_3D override."""
    deselect_all()
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    override = {
        "window": window,
        "screen": window.screen,
        "area": area,
        "region": region,
        "scene": bpy.context.scene,
        "view_layer": bpy.context.view_layer,
        "active_object": obj,
        "object": obj,
        "edit_object": obj,
        "selected_objects": [obj],
        "selected_editable_objects": [obj],
    }
    with bpy.context.temp_override(**override):
        bpy.ops.object.mode_set(mode='EDIT')
        bpy.ops.mesh.select_all(action='SELECT')
        bpy.ops.uv.smart_project(
            angle_limit=radians(66),
            island_margin=0.01,
            area_weight=0.0,
        )
        bpy.ops.object.mode_set(mode='OBJECT')


print("[forge-mind] Prompt 1: geometry cleanup + bevels starting")

mesh_objects = [o for o in bpy.data.objects if o.type == 'MESH']
if not mesh_objects:
    raise RuntimeError("[forge-mind] no MESH objects found after GLB import")

window, area, region = find_view3d()
if area is None:
    raise RuntimeError("[forge-mind] no VIEW_3D area available")

for obj in mesh_objects:
    bake_world_transform(obj)
    clean_and_bevel(obj)
    smart_uv_project(obj, area, region, window)

# Final selection state for the forge-mind exporter wrapper.
deselect_all()
for obj in mesh_objects:
    obj.select_set(True)
bpy.context.view_layer.objects.active = mesh_objects[0]

print(f"[forge-mind] Prompt 1 complete: {len(mesh_objects)} mesh(es) upgraded")
