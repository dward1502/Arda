# Procedural ARDA monitor materialization.
#
# Runs inside the forge-mind Blender wrapper after a generated GLB has been
# imported. It enforces style independent of whether the generator produced
# useful submesh names:
#   - dark graphite / black metal body
#   - cyan emissive screen surface
#   - cyan emissive edge strips / UI traces
#   - compact articulated rear arm when absent
#   - bevels and smooth normals for cyber-noir ruggedness
#
# Alignment rule: never assume Blender axes. Text-to-3D/import/export can rotate
# monitor slabs between passes, so procedural overlays must infer width/height/
# depth from the imported mesh bounds and anchor to the front face.

import math

import bpy
import mathutils

CYAN = (0.05, 0.95, 1.0, 1.0)
DARK = (0.005, 0.006, 0.007, 1.0)
GRAPHITE = (0.025, 0.028, 0.032, 1.0)
AXES = ("x", "y", "z")


def make_principled_material(name, base, metallic=0.0, roughness=0.5, emission=None, strength=0.0):
    mat = bpy.data.materials.new(name)
    mat.use_nodes = True
    pri = next((n for n in mat.node_tree.nodes if n.type == "BSDF_PRINCIPLED"), None)
    if pri is not None:
        if "Base Color" in pri.inputs:
            pri.inputs["Base Color"].default_value = base
        if "Metallic" in pri.inputs:
            pri.inputs["Metallic"].default_value = metallic
        if "Roughness" in pri.inputs:
            pri.inputs["Roughness"].default_value = roughness
        if emission is not None:
            for socket_name in ("Emission Color", "Emission"):
                if socket_name in pri.inputs:
                    pri.inputs[socket_name].default_value = emission
                    break
            for socket_name in ("Emission Strength", "Emissive Strength"):
                if socket_name in pri.inputs:
                    pri.inputs[socket_name].default_value = strength
                    break
    return mat


def material_by_name(name, base, metallic=0.0, roughness=0.5, emission=None, strength=0.0):
    existing = bpy.data.materials.get(name)
    if existing is not None:
        return existing
    return make_principled_material(name, base, metallic, roughness, emission, strength)


def mesh_objects():
    return [o for o in bpy.data.objects if o.type == "MESH"]


def world_bounds(objects):
    min_co = mathutils.Vector((float("inf"), float("inf"), float("inf")))
    max_co = mathutils.Vector((float("-inf"), float("-inf"), float("-inf")))
    for obj in objects:
        for corner in obj.bound_box:
            wv = obj.matrix_world @ mathutils.Vector(corner)
            min_co.x = min(min_co.x, wv.x)
            min_co.y = min(min_co.y, wv.y)
            min_co.z = min(min_co.z, wv.z)
            max_co.x = max(max_co.x, wv.x)
            max_co.y = max(max_co.y, wv.y)
            max_co.z = max(max_co.z, wv.z)
    return min_co, max_co


def axis_value(vec, axis):
    return getattr(vec, axis)


def set_axis(vec, axis, value):
    setattr(vec, axis, value)


def vec_from_axes(width_axis, width_value, height_axis, height_value, depth_axis, depth_value):
    vec = mathutils.Vector((0.0, 0.0, 0.0))
    set_axis(vec, width_axis, width_value)
    set_axis(vec, height_axis, height_value)
    set_axis(vec, depth_axis, depth_value)
    return vec


def infer_monitor_axes(min_co, max_co):
    extents = {axis: max(axis_value(max_co, axis) - axis_value(min_co, axis), 0.001) for axis in AXES}
    ordered = sorted(AXES, key=lambda axis: extents[axis])
    depth_axis = ordered[0]
    height_axis = ordered[1]
    width_axis = ordered[2]
    return width_axis, height_axis, depth_axis, extents


def assign_material(obj, mat):
    obj.data.materials.clear()
    obj.data.materials.append(mat)


def add_bevel(obj, width, segments):
    bevel = obj.modifiers.new("arda_micro_bevel", "BEVEL")
    bevel.width = width
    bevel.segments = segments
    bevel.profile = 0.65
    bevel.affect = "EDGES"
    weighted = obj.modifiers.new("arda_weighted_normals", "WEIGHTED_NORMAL")
    weighted.keep_sharp = True


def active_mesh_after_add(name):
    obj = getattr(bpy.context, "object", None)
    if obj is None:
        obj = getattr(getattr(bpy.context, "view_layer", None), "objects", None)
        obj = getattr(obj, "active", None)
    if obj is None:
        selected_meshes = [o for o in bpy.context.selected_objects if o.type == "MESH"]
        obj = selected_meshes[-1] if selected_meshes else None
    if obj is None:
        meshes = mesh_objects()
        obj = meshes[-1] if meshes else None
    if obj is None:
        raise RuntimeError(f"[forge-mind] failed to create {name}: no active mesh after bpy.ops add")
    obj.name = name
    return obj


def cube_obj(name, location, scale, mat):
    bpy.ops.mesh.primitive_cube_add(size=1.0, location=location)
    obj = active_mesh_after_add(name)
    obj.scale = scale
    assign_material(obj, mat)
    add_bevel(obj, min(scale) * 0.15 if min(scale) > 0 else 0.002, 2)
    return obj


def plane_rect_obj(name, center, width_axis, width, height_axis, height, depth_axis, plane_value, mat):
    """Create a single coplanar rectangular overlay on the inferred monitor face.

    Emissive overlays must not be thick cubes embedded into or protruding far
    from the generated body. A mesh plane gives a deterministic surface decal:
    all vertices share the same face-normal coordinate, so color cannot bleed
    through the body from hidden thickness or z-fight inside the source mesh.
    """
    hw = width / 2.0
    hh = height / 2.0
    coords = []
    for w, h in ((-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)):
        v = mathutils.Vector(center)
        set_axis(v, width_axis, axis_value(center, width_axis) + w)
        set_axis(v, height_axis, axis_value(center, height_axis) + h)
        set_axis(v, depth_axis, plane_value)
        coords.append(tuple(v))
    mesh = bpy.data.meshes.new(f"{name}_mesh")
    mesh.from_pydata(coords, [], [(0, 1, 2, 3)])
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    assign_material(obj, mat)
    return obj


def cylinder_obj(name, location, radius, depth, mat, rotation=(0.0, 0.0, 0.0), vertices=32):
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=depth, location=location, rotation=rotation)
    obj = active_mesh_after_add(name)
    assign_material(obj, mat)
    add_bevel(obj, radius * 0.08, 2)
    return obj


def cylinder_between(name, start, end, radius, mat, vertices=24):
    start_v = mathutils.Vector(start)
    end_v = mathutils.Vector(end)
    delta = end_v - start_v
    length = delta.length
    if length <= 0.0001:
        raise RuntimeError(f"[forge-mind] cannot create zero-length cylinder {name}")
    midpoint = (start_v + end_v) / 2.0
    rotation = delta.to_track_quat("Z", "Y").to_euler()
    return cylinder_obj(name, tuple(midpoint), radius, length, mat, rotation=rotation, vertices=vertices)


def loc_from_axes(origin, width_axis, width_offset, height_axis, height_offset, depth_axis, depth_value):
    loc = mathutils.Vector(origin)
    set_axis(loc, width_axis, axis_value(origin, width_axis) + width_offset)
    set_axis(loc, height_axis, axis_value(origin, height_axis) + height_offset)
    set_axis(loc, depth_axis, depth_value)
    return loc


def classify_existing_screen(obj):
    n = obj.name.lower()
    return "screen" in n or "display" in n or "glass" in n


def classify_existing_trace(obj):
    n = obj.name.lower()
    return "trace" in n or "ui_bar" in n or "status" in n


print("[forge-mind] ARDA monitor materialization starting")

# Idempotency: strip previous procedural ARDA overlays before adding a fresh pass.
# This lets the materializer safely re-run on an already materialized GLB.
for _obj in list(bpy.data.objects):
    if _obj.name.startswith("arda_"):
        bpy.data.objects.remove(_obj, do_unlink=True)

objects = mesh_objects()
if not objects:
    raise RuntimeError("[forge-mind] no MESH objects found for ARDA monitor materialization")

# Replace generic generated desktop-stand parts with the reference-driven
# articulated mount below. Keeping them produced a duplicate simple office stand
# that failed the source-fidelity gate.
for _obj in list(objects):
    _name = _obj.name.lower()
    if any(token in _name for token in ("mount_arm", "mount_foot", "desktop_stand", "office_stand")):
        bpy.data.objects.remove(_obj, do_unlink=True)
objects = mesh_objects()
if not objects:
    raise RuntimeError("[forge-mind] no MESH objects remain after removing generic stand parts")

body_mat = material_by_name("arda_dark_graphite_body", GRAPHITE, metallic=0.75, roughness=0.38)
bezel_mat = material_by_name("arda_black_beveled_bezel", DARK, metallic=0.85, roughness=0.32)
screen_mat = material_by_name("arda_cyan_emissive_screen", (0.0, 0.055, 0.060, 1.0), metallic=0.0, roughness=0.32, emission=CYAN, strength=0.55)
trace_mat = material_by_name("arda_cyan_emissive_traces", (0.0, 0.28, 0.30, 1.0), metallic=0.0, roughness=0.26, emission=CYAN, strength=1.45)

min_co, max_co = world_bounds(objects)
center = (min_co + max_co) / 2.0
width_axis, height_axis, depth_axis, extents = infer_monitor_axes(min_co, max_co)
width = extents[width_axis]
height = extents[height_axis]
depth = extents[depth_axis]
size = max(width, height, depth)

screen_found = False
existing_trace_count = 0
for obj in objects:
    if classify_existing_screen(obj):
        assign_material(obj, screen_mat)
        screen_found = True
    elif classify_existing_trace(obj):
        assign_material(obj, trace_mat)
        existing_trace_count += 1
    else:
        assign_material(obj, body_mat)
    # Do not bevel imported/generated source meshes here. Re-materializing an
    # already-exported GLB with another bevel pass can explode vertex counts and
    # GLB size. Keep cleanup style on procedural overlays only.

# Add a canonical front overlay so even one-piece generated monitor meshes gain
# readable ARDA style. The overlay is axis-inferred and anchored just above the
# source front face, not placed by hard-coded Y/Z assumptions.
front_value = axis_value(max_co, depth_axis)
front_clearance = max(depth * 0.018, 0.004)
front_plane = front_value + front_clearance
screen_w = width * 0.54
screen_h = height * 0.32
screen_center = mathutils.Vector(center)
set_axis(screen_center, width_axis, axis_value(center, width_axis))
set_axis(screen_center, height_axis, axis_value(center, height_axis) - height * 0.04)
set_axis(screen_center, depth_axis, front_plane)
if not screen_found:
    plane_rect_obj(
        "arda_screen_cyan_emissive_overlay",
        screen_center,
        width_axis,
        screen_w,
        height_axis,
        screen_h,
        depth_axis,
        front_plane,
        screen_mat,
    )
    screen_found = True

if existing_trace_count == 0:
    # No source trace/surface detail was detected, so synthesize minimal procedural
    # overlays. Existing detailed monitor assets are recolored in place instead;
    # adding duplicate plates in front of them reads as color bleeding through geometry.
    # Rugged bezel bars around the screen, aligned to the same inferred plane.
    bar_t = max(min(width, height) * 0.028, 0.008)
    bar_depth = max(depth * 0.006, 0.0025)
    bezel_plane = front_plane + max(depth * 0.006, 0.0015)

    def overlay_loc(width_offset=0.0, height_offset=0.0, plane=bezel_plane):
        loc = mathutils.Vector(screen_center)
        set_axis(loc, width_axis, axis_value(screen_center, width_axis) + width_offset)
        set_axis(loc, height_axis, axis_value(screen_center, height_axis) + height_offset)
        set_axis(loc, depth_axis, plane)
        return tuple(loc)

    cube_obj(
        "arda_bezel_top_graphite",
        overlay_loc(height_offset=screen_h / 2.0 + bar_t / 2.0),
        tuple(vec_from_axes(width_axis, screen_w / 2.0 + bar_t, height_axis, bar_t / 2.0, depth_axis, bar_depth)),
        bezel_mat,
    )
    cube_obj(
        "arda_bezel_bottom_graphite",
        overlay_loc(height_offset=-screen_h / 2.0 - bar_t / 2.0),
        tuple(vec_from_axes(width_axis, screen_w / 2.0 + bar_t, height_axis, bar_t / 2.0, depth_axis, bar_depth)),
        bezel_mat,
    )
    cube_obj(
        "arda_bezel_left_graphite",
        overlay_loc(width_offset=-screen_w / 2.0 - bar_t / 2.0),
        tuple(vec_from_axes(width_axis, bar_t / 2.0, height_axis, screen_h / 2.0 + bar_t, depth_axis, bar_depth)),
        bezel_mat,
    )
    cube_obj(
        "arda_bezel_right_graphite",
        overlay_loc(width_offset=screen_w / 2.0 + bar_t / 2.0),
        tuple(vec_from_axes(width_axis, bar_t / 2.0, height_axis, screen_h / 2.0 + bar_t, depth_axis, bar_depth)),
        bezel_mat,
    )

    # Cyan UI strips / edge traces. Keep them small and coplanar so they read as
    # etched/inset detail rather than floating detached slabs.
    trace_t = max(bar_t * 0.20, 0.003)
    trace_plane = bezel_plane + max(depth * 0.006, 0.001)
    plane_rect_obj(
        "arda_cyan_top_trace",
        mathutils.Vector(overlay_loc(height_offset=screen_h / 2.0 + bar_t * 1.10, plane=trace_plane)),
        width_axis,
        screen_w * 0.68,
        height_axis,
        trace_t * 2.0,
        depth_axis,
        trace_plane,
        trace_mat,
    )
    plane_rect_obj(
        "arda_cyan_bottom_trace",
        mathutils.Vector(overlay_loc(height_offset=-screen_h / 2.0 - bar_t * 1.10, plane=trace_plane)),
        width_axis,
        screen_w * 0.44,
        height_axis,
        trace_t * 2.0,
        depth_axis,
        trace_plane,
        trace_mat,
    )
    plane_rect_obj(
        "arda_cyan_status_pip_left",
        mathutils.Vector(overlay_loc(width_offset=-screen_w * 0.34, height_offset=-screen_h * 0.34, plane=trace_plane)),
        width_axis,
        trace_t * 2.8,
        height_axis,
        trace_t * 2.8,
        depth_axis,
        trace_plane,
        trace_mat,
    )
    plane_rect_obj(
        "arda_cyan_status_pip_right",
        mathutils.Vector(overlay_loc(width_offset=screen_w * 0.34, height_offset=screen_h * 0.34, plane=trace_plane)),
        width_axis,
        trace_t * 2.8,
        height_axis,
        trace_t * 2.8,
        depth_axis,
        trace_plane,
        trace_mat,
    )


# Reference-fidelity cyberpunk hard-surface pass.
#
# Vision iteration can produce a valid but under-designed office monitor. The
# source pack's monitors are rugged cyberpunk tablets: wide armored graphite
# casings, side modules, cyan inset details, visible hinge discs, rear housing,
# and articulated arms. Add those deterministic parts every pass so the family
# style does not depend on text-to-3D keeping the art direction.
panel_center = mathutils.Vector(center)
set_axis(panel_center, width_axis, axis_value(center, width_axis))
set_axis(panel_center, height_axis, axis_value(center, height_axis))

outer_w = width * 1.05
outer_h = height * 1.05
armor_t = max(min(width, height) * 0.055, 0.035)
armor_depth = max(depth * 0.22, 0.035)
front_detail_plane = front_plane + max(depth * 0.030, 0.006)
rear_plane = axis_value(min_co, depth_axis) - max(depth * 0.10, 0.018)
side_pod_w = armor_t * 1.35
corner_w = armor_t * 1.45
corner_h = armor_t * 1.25

# Wide, layered tablet housing around the generated body. These pieces are
# intentionally named arda_* so repeated materialization removes and rebuilds
# them idempotently before export.
for suffix, h_off in (("top", outer_h / 2.0), ("bottom", -outer_h / 2.0)):
    cube_obj(
        f"arda_armor_{suffix}_beveled_graphite",
        tuple(loc_from_axes(panel_center, width_axis, 0.0, height_axis, h_off, depth_axis, front_detail_plane)),
        tuple(vec_from_axes(width_axis, outer_w, height_axis, armor_t, depth_axis, armor_depth)),
        bezel_mat,
    )

for suffix, w_off in (("left", -outer_w / 2.0), ("right", outer_w / 2.0)):
    cube_obj(
        f"arda_armor_{suffix}_side_module",
        tuple(loc_from_axes(panel_center, width_axis, w_off, height_axis, 0.0, depth_axis, front_detail_plane)),
        tuple(vec_from_axes(width_axis, side_pod_w, height_axis, outer_h, depth_axis, armor_depth * 1.08)),
        bezel_mat,
    )

for sx, sy in ((-1, -1), (-1, 1), (1, -1), (1, 1)):
    cube_obj(
        f"arda_corner_armor_{'l' if sx < 0 else 'r'}_{'b' if sy < 0 else 't'}",
        tuple(loc_from_axes(panel_center, width_axis, sx * outer_w / 2.0, height_axis, sy * outer_h / 2.0, depth_axis, front_detail_plane + armor_depth * 0.12)),
        tuple(vec_from_axes(width_axis, corner_w, height_axis, corner_h, depth_axis, armor_depth * 1.25)),
        bezel_mat,
    )

# Backplate and under-screen rail create depth and a non-flat silhouette.
cube_obj(
    "arda_layered_rear_backplate",
    tuple(loc_from_axes(panel_center, width_axis, 0.0, height_axis, 0.0, depth_axis, rear_plane)),
    tuple(vec_from_axes(width_axis, outer_w * 0.76, height_axis, outer_h * 0.68, depth_axis, armor_depth * 0.85)),
    body_mat,
)
cube_obj(
    "arda_under_screen_mount_rail",
    tuple(loc_from_axes(panel_center, width_axis, 0.0, height_axis, -outer_h * 0.62, depth_axis, rear_plane - armor_depth * 0.18)),
    tuple(vec_from_axes(width_axis, outer_w * 0.68, height_axis, armor_t * 0.56, depth_axis, armor_depth * 0.65)),
    bezel_mat,
)

# Cyan reference features: thin top edge, side status bars, and corner pips. All
# are planar decals on the inferred front face to avoid the previous bleed issue.
trace_t = max(armor_t * 0.16, 0.006)
for name, w_mul, h_off in (
    ("arda_cyan_top_edge_light", 0.68, outer_h / 2.0 + armor_t * 0.06),
    ("arda_cyan_lower_short_status", 0.30, -outer_h / 2.0 - armor_t * 0.04),
):
    plane_rect_obj(
        name,
        loc_from_axes(panel_center, width_axis, 0.0, height_axis, h_off, depth_axis, front_detail_plane + 0.004),
        width_axis,
        outer_w * w_mul,
        height_axis,
        trace_t,
        depth_axis,
        front_detail_plane + 0.004,
        trace_mat,
    )

for side, sx in (("left", -1), ("right", 1)):
    for idx, h_mul in enumerate((-0.22, 0.12, 0.34)):
        plane_rect_obj(
            f"arda_cyan_{side}_module_status_{idx}",
            loc_from_axes(panel_center, width_axis, sx * (outer_w / 2.0 + side_pod_w * 0.02), height_axis, outer_h * h_mul, depth_axis, front_detail_plane + 0.005),
            width_axis,
            trace_t * 1.6,
            height_axis,
            outer_h * 0.12,
            depth_axis,
            front_detail_plane + 0.005,
            trace_mat,
        )

# Visible rotary hinge discs and compact dual-arm mount. Attach these to the
# inferred rear/center so the monitor reads as boardroom hardware rather than a
# generic desktop stand.
hinge_center = loc_from_axes(panel_center, width_axis, 0.0, height_axis, -outer_h * 0.38, depth_axis, rear_plane - armor_depth * 0.80)
rear_attach = loc_from_axes(panel_center, width_axis, 0.0, height_axis, -outer_h * 0.22, depth_axis, rear_plane - armor_depth * 0.25)
base_attach = loc_from_axes(panel_center, width_axis, 0.0, height_axis, -outer_h * 0.66, depth_axis, rear_plane - armor_depth * 2.20)
left_arm_start = loc_from_axes(rear_attach, width_axis, -outer_w * 0.11, height_axis, 0.0, depth_axis, axis_value(rear_attach, depth_axis))
left_arm_end = loc_from_axes(base_attach, width_axis, -outer_w * 0.05, height_axis, 0.0, depth_axis, axis_value(base_attach, depth_axis))
right_arm_start = loc_from_axes(rear_attach, width_axis, outer_w * 0.11, height_axis, 0.0, depth_axis, axis_value(rear_attach, depth_axis))
right_arm_end = loc_from_axes(base_attach, width_axis, outer_w * 0.05, height_axis, 0.0, depth_axis, axis_value(base_attach, depth_axis))
arm_radius = max(armor_t * 0.22, 0.012)
cylinder_between("arda_left_articulated_support_arm", left_arm_start, left_arm_end, arm_radius, body_mat, vertices=20)
cylinder_between("arda_right_articulated_support_arm", right_arm_start, right_arm_end, arm_radius, body_mat, vertices=20)
cylinder_between("arda_center_cable_spine", rear_attach, base_attach, max(arm_radius * 0.45, 0.006), trace_mat, vertices=14)
cylinder_between(
    "arda_rear_hinge_bridge",
    loc_from_axes(panel_center, width_axis, 0.0, height_axis, -outer_h * 0.22, depth_axis, rear_plane + armor_depth * 0.12),
    rear_attach,
    max(arm_radius * 1.15, 0.014),
    body_mat,
    vertices=20,
)

# Hinge discs lie on the same screen plane axes and are shallow along the depth
# axis. Build them as cylinders between two nearby depth-axis points.
for name, origin, radius in (
    ("arda_rear_rotary_hinge_disc", rear_attach, max(armor_t * 0.92, 0.04)),
    ("arda_lower_rotary_hinge_disc", hinge_center, max(armor_t * 0.82, 0.035)),
):
    start = mathutils.Vector(origin)
    end = mathutils.Vector(origin)
    set_axis(start, depth_axis, axis_value(origin, depth_axis) - armor_depth * 0.34)
    set_axis(end, depth_axis, axis_value(origin, depth_axis) + armor_depth * 0.34)
    cylinder_between(name, start, end, radius, bezel_mat, vertices=40)

cube_obj(
    "arda_low_profile_mount_foot",
    tuple(loc_from_axes(base_attach, width_axis, 0.0, height_axis, -armor_t * 0.55, depth_axis, axis_value(base_attach, depth_axis) - armor_depth * 0.18)),
    tuple(vec_from_axes(width_axis, outer_w * 0.40, height_axis, armor_t * 0.84, depth_axis, armor_depth * 1.85)),
    body_mat,
)

# Select meshes for wrapper export.
for obj in bpy.data.objects:
    obj.select_set(obj.type == "MESH")

print(
    "[forge-mind] ARDA monitor materialization complete: "
    f"source_meshes={len(objects)} screen_found={screen_found} "
    f"axes=width:{width_axis} height:{height_axis} depth:{depth_axis}"
)
