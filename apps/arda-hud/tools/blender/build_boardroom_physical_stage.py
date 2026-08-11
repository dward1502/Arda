from __future__ import annotations

import math
import sys
from pathlib import Path

import bpy
from mathutils import Vector


SCRIPT_PATH = Path(__file__).resolve()
HUD_ROOT = SCRIPT_PATH.parents[2]
ASSET_DIR = HUD_ROOT / "src/assets/scene/world/boardroom_physical_stage"
GLB_PATH = ASSET_DIR / "boardroom_physical_stage.glb"
BLEND_PATH = ASSET_DIR / "boardroom_physical_stage.blend"
RENDER_PATH = ASSET_DIR / "boardroom_physical_stage_reference.png"
ATMOSPHERE_PATH = HUD_ROOT / "src/assets/scene/window/boardroom_reference_atmosphere/boardroom_reference_atmosphere.jpg"


def reset_scene() -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete(use_global=False)
    for datablocks in (bpy.data.meshes, bpy.data.curves, bpy.data.materials, bpy.data.cameras, bpy.data.lights):
        for block in list(datablocks):
            if block.users == 0:
                datablocks.remove(block)


def make_material(
    name: str,
    color: tuple[float, float, float, float],
    *,
    metallic: float = 0.0,
    roughness: float = 0.45,
    emission: tuple[float, float, float, float] | None = None,
    emission_strength: float = 0.0,
) -> bpy.types.Material:
    material = bpy.data.materials.new(name)
    material.diffuse_color = color
    material.use_nodes = True
    principled = material.node_tree.nodes.get("Principled BSDF")
    principled.inputs["Base Color"].default_value = color
    principled.inputs["Metallic"].default_value = metallic
    principled.inputs["Roughness"].default_value = roughness
    if emission is not None:
        principled.inputs["Emission Color"].default_value = emission
        principled.inputs["Emission Strength"].default_value = emission_strength
    return material


def add_beveled_box(
    name: str,
    location: tuple[float, float, float],
    dimensions: tuple[float, float, float],
    material: bpy.types.Material,
    *,
    rotation: tuple[float, float, float] = (0.0, 0.0, 0.0),
    bevel: float = 0.04,
    parent: bpy.types.Object | None = None,
    collection: bpy.types.Collection | None = None,
) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(location=location, rotation=rotation)
    obj = bpy.context.object
    obj.name = name
    obj.dimensions = dimensions
    bpy.ops.object.transform_apply(location=False, rotation=False, scale=True)
    if bevel > 0:
        modifier = obj.modifiers.new(name="Precision bevel", type="BEVEL")
        modifier.width = bevel
        modifier.segments = 4
        modifier.limit_method = "ANGLE"
    obj.data.materials.append(material)
    if parent is not None:
        world_matrix = obj.matrix_world.copy()
        obj.parent = parent
        obj.matrix_world = world_matrix
    if collection is not None:
        for source_collection in list(obj.users_collection):
            source_collection.objects.unlink(obj)
        collection.objects.link(obj)
    return obj


def add_cylinder_between(
    name: str,
    start: tuple[float, float, float],
    end: tuple[float, float, float],
    radius: float,
    material: bpy.types.Material,
    *,
    vertices: int = 24,
    collection: bpy.types.Collection | None = None,
) -> bpy.types.Object:
    start_vec = Vector(start)
    end_vec = Vector(end)
    direction = end_vec - start_vec
    midpoint = (start_vec + end_vec) * 0.5
    bpy.ops.mesh.primitive_cylinder_add(vertices=vertices, radius=radius, depth=direction.length, location=midpoint)
    obj = bpy.context.object
    obj.name = name
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = direction.to_track_quat("Z", "Y")
    obj.data.materials.append(material)
    bevel = obj.modifiers.new(name="Edge bevel", type="BEVEL")
    bevel.width = min(radius * 0.28, 0.025)
    bevel.segments = 3
    if collection is not None:
        for source_collection in list(obj.users_collection):
            source_collection.objects.unlink(obj)
        collection.objects.link(obj)
    return obj


def add_beveled_beam_between(
    name: str,
    start: tuple[float, float, float],
    end: tuple[float, float, float],
    width: float,
    depth: float,
    material: bpy.types.Material,
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    start_vec = Vector(start)
    end_vec = Vector(end)
    direction = end_vec - start_vec
    obj = add_beveled_box(
        name,
        tuple((start_vec + end_vec) * 0.5),
        (width, depth, direction.length),
        material,
        bevel=min(width, depth) * 0.22,
        collection=collection,
    )
    obj.rotation_mode = "QUATERNION"
    obj.rotation_quaternion = direction.to_track_quat("Z", "Y")
    return obj


def add_curve_rail(
    name: str,
    points: list[tuple[float, float, float]],
    radius: float,
    material: bpy.types.Material,
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    curve_data = bpy.data.curves.new(name, type="CURVE")
    curve_data.dimensions = "3D"
    curve_data.resolution_u = 16
    curve_data.bevel_depth = radius
    curve_data.bevel_resolution = 5
    spline = curve_data.splines.new("BEZIER")
    spline.bezier_points.add(len(points) - 1)
    for point, coordinate in zip(spline.bezier_points, points):
        point.co = coordinate
        point.handle_left_type = "AUTO"
        point.handle_right_type = "AUTO"
    obj = bpy.data.objects.new(name, curve_data)
    curve_data.materials.append(material)
    collection.objects.link(obj)
    return obj


def add_extruded_polygon(
    name: str,
    points: list[tuple[float, float]],
    z_bottom: float,
    z_top: float,
    material: bpy.types.Material,
    collection: bpy.types.Collection,
    bevel: float = 0.08,
) -> bpy.types.Object:
    count = len(points)
    vertices = [(x, y, z_bottom) for x, y in points] + [(x, y, z_top) for x, y in points]
    faces: list[tuple[int, ...]] = [tuple(reversed(range(count))), tuple(range(count, count * 2))]
    for index in range(count):
        next_index = (index + 1) % count
        faces.append((index, next_index, next_index + count, index + count))
    mesh = bpy.data.meshes.new(f"{name}.mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.data.materials.append(material)
    modifier = obj.modifiers.new(name="Continuous shell bevel", type="BEVEL")
    modifier.width = bevel
    modifier.segments = 5
    modifier.limit_method = "ANGLE"
    weighted = obj.modifiers.new(name="Weighted normals", type="WEIGHTED_NORMAL")
    weighted.keep_sharp = True
    return obj


def add_continuous_command_bank(
    material: bpy.types.Material,
    collection: bpy.types.Collection,
) -> bpy.types.Object:
    """Create one curved, sloped console body from the flat desk to the monitors."""
    stations = [
        (-5.0, -0.52, 0.28, 0.93),
        (-3.8, -0.46, 0.55, 1.03),
        (-1.9, -0.36, 0.72, 1.08),
        (0.0, -0.32, 0.8, 1.1),
        (1.9, -0.36, 0.72, 1.08),
        (3.8, -0.46, 0.55, 1.03),
        (5.0, -0.52, 0.28, 0.93),
    ]
    vertices: list[tuple[float, float, float]] = []
    for x, front_y, rear_y, rear_z in stations:
        vertices.extend(
            [
                (x, front_y, 0.34),
                (x, rear_y, rear_z),
                (x, front_y, 0.16),
                (x, rear_y, rear_z - 0.22),
            ]
        )

    faces: list[tuple[int, ...]] = []
    for index in range(len(stations) - 1):
        current = index * 4
        following = (index + 1) * 4
        faces.extend(
            [
                (current, following, following + 1, current + 1),
                (current + 2, current + 3, following + 3, following + 2),
                (current, current + 2, following + 2, following),
                (current + 1, following + 1, following + 3, current + 3),
            ]
        )
    faces.extend(
        [
            (0, 1, 3, 2),
            (
                (len(stations) - 1) * 4,
                (len(stations) - 1) * 4 + 2,
                (len(stations) - 1) * 4 + 3,
                (len(stations) - 1) * 4 + 1,
            ),
        ]
    )

    mesh = bpy.data.meshes.new("CommandBank.Continuous.mesh")
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    bank = bpy.data.objects.new("CommandBank.Continuous", mesh)
    collection.objects.link(bank)
    bank.data.materials.append(material)
    bevel = bank.modifiers.new(name="Continuous command-bank bevel", type="BEVEL")
    bevel.width = 0.075
    bevel.segments = 5
    bevel.limit_method = "ANGLE"
    weighted = bank.modifiers.new(name="Weighted normals", type="WEIGHTED_NORMAL")
    weighted.keep_sharp = True
    return bank


def create_monitor_rig(
    index: int,
    center: tuple[float, float, float],
    width: float,
    height: float,
    target: tuple[float, float, float],
    materials: dict[str, bpy.types.Material],
    collection: bpy.types.Collection,
) -> None:
    center_vec = Vector(center)
    target_vec = Vector(target)
    direction = target_vec - center_vec
    yaw = math.atan2(direction.x, -direction.y)

    rig = bpy.data.objects.new(f"MonitorRig.{index:02d}", None)
    rig.location = center
    rig.rotation_euler[2] = yaw
    collection.objects.link(rig)

    housing = add_beveled_box(
        f"Monitor.{index:02d}.Housing",
        center,
        (width, 0.06, height),
        materials["graphite"],
        rotation=(0.0, 0.0, yaw),
        bevel=0.018,
        collection=collection,
    )
    housing["screen_slot"] = f"boardroom.monitor.{index:02d}"

    forward = Vector((math.sin(yaw), -math.cos(yaw), 0.0))
    glass_center = center_vec + forward * 0.036
    add_beveled_box(
        f"Monitor.{index:02d}.Glass",
        tuple(glass_center),
        (width - 0.1, 0.012, height - 0.1),
        materials["glass"],
        rotation=(0.0, 0.0, yaw),
        bevel=0.012,
        collection=collection,
    )

    # The housing intentionally terminates inside the raised console bank.
    # There is no visible pedestal, post, or square monitor foot from the
    # operator camera; the screen reads as part of the architecture.


def create_lower_insert(
    index: int,
    location: tuple[float, float, float],
    width: float,
    depth: float,
    yaw: float,
    materials: dict[str, bpy.types.Material],
    collection: bpy.types.Collection,
) -> None:
    # The outermost inserts sit lower toward the operator so their content does
    # not read as upright wings at the edge of the command bank.
    pitch = math.radians(32.0 if index in (0, 4) else 29.0)
    rig = bpy.data.objects.new(f"DeskInsert.{index:02d}.Rig", None)
    rig.location = location
    rig.rotation_euler = (pitch, 0.0, yaw)
    rig["screen_slot"] = f"boardroom.lower.{index:02d}"
    collection.objects.link(rig)

    # The continuous command bank twists slightly across each display aperture.
    # Cant the four non-center inserts just enough to keep their inner edges
    # above that shell while leaving the opposite corners flush.  The values are
    # mirrored around the untouched center insert:
    #   0/4 lift the full inner edge; 1/3 lift the inner rear corner.
    # desk_1 (index 0) needs its right/inner edge lifted; desk_5 (index 4)
    # needs the mirrored left/inner edge lifted.  A 0.14 m edge delta clears
    # the twisting bank without raising the outer edges away from the shell.
    outer_edge_lift = 0.14
    inner_corner_lift = 0.055
    surface_fit = {
        0: (
            outer_edge_lift * 0.5,
            0.0,
            -math.atan(outer_edge_lift / width),
        ),
        1: (
            inner_corner_lift * 0.5,
            math.atan((inner_corner_lift * 0.5) / depth),
            -math.atan((inner_corner_lift * 0.5) / width),
        ),
        3: (
            inner_corner_lift * 0.5,
            math.atan((inner_corner_lift * 0.5) / depth),
            math.atan((inner_corner_lift * 0.5) / width),
        ),
        4: (
            outer_edge_lift * 0.5,
            0.0,
            math.atan(outer_edge_lift / width),
        ),
    }
    normal_lift, rear_lift_angle, inner_edge_lift_angle = surface_fit.get(index, (0.0, 0.0, 0.0))
    surface_rig = bpy.data.objects.new(f"DeskInsert.{index:02d}.SurfaceFit", None)
    surface_rig.parent = rig
    surface_rig.location = (0.0, 0.0, normal_lift)
    surface_rig.rotation_euler = (rear_lift_angle, inner_edge_lift_angle, 0.0)
    surface_rig["surface_fit_normal_lift"] = normal_lift
    surface_rig["surface_fit_rear_angle"] = rear_lift_angle
    surface_rig["surface_fit_inner_edge_angle"] = inner_edge_lift_angle
    collection.objects.link(surface_rig)

    def local_box(
        suffix: str,
        local_location: tuple[float, float, float],
        dimensions: tuple[float, float, float],
        material: bpy.types.Material,
        bevel: float,
    ) -> bpy.types.Object:
        obj = add_beveled_box(
            f"DeskInsert.{index:02d}.{suffix}",
            (0.0, 0.0, 0.0),
            dimensions,
            material,
            bevel=bevel,
            collection=collection,
        )
        obj.parent = surface_rig
        obj.location = local_location
        return obj

    # The glass sits nearly flush with the desk. Four separate bezel bars give
    # it a machined aperture without placing a tablet-shaped slab on top.
    glass_material = materials["command_glass"] if index == 2 else materials["tactical_glass"] if index in (0, 4) else materials["glass"]
    glass = local_box("Glass", (0.0, 0.0, 0.012), (width - 0.12, depth - 0.12, 0.018), glass_material, 0.018)
    glass["interactive_surface"] = True
    bezel = 0.045
    local_box("Bezel.Front", (0.0, -depth * 0.5 + bezel * 0.5, 0.018), (width, bezel, 0.035), materials["edge"], 0.018)
    local_box("Bezel.Back", (0.0, depth * 0.5 - bezel * 0.5, 0.018), (width, bezel, 0.035), materials["edge"], 0.018)
    local_box("Bezel.Left", (-width * 0.5 + bezel * 0.5, 0.0, 0.018), (bezel, depth - bezel * 2, 0.035), materials["edge"], 0.018)
    local_box("Bezel.Right", (width * 0.5 - bezel * 0.5, 0.0, 0.018), (bezel, depth - bezel * 2, 0.035), materials["edge"], 0.018)
def create_backdrop(materials: dict[str, bpy.types.Material], render_collection: bpy.types.Collection) -> None:
    material = bpy.data.materials.new("Reference atmosphere")
    material.use_nodes = True
    nodes = material.node_tree.nodes
    links = material.node_tree.links
    nodes.clear()
    output = nodes.new("ShaderNodeOutputMaterial")
    emission = nodes.new("ShaderNodeEmission")
    texture = nodes.new("ShaderNodeTexImage")
    texture.image = bpy.data.images.load(str(ATMOSPHERE_PATH))
    emission.inputs["Strength"].default_value = 0.8
    links.new(texture.outputs["Color"], emission.inputs["Color"])
    links.new(emission.outputs["Emission"], output.inputs["Surface"])
    backdrop = add_beveled_box(
        "RENDER_ONLY.Atmosphere",
        (0.0, 3.9, 3.05),
        (12.5, 0.06, 4.65),
        material,
        bevel=0.0,
        collection=render_collection,
    )
    backdrop.rotation_euler[2] = 0.0

    add_beveled_box(
        "RENDER_ONLY.Floor",
        (0.0, 0.0, -0.28),
        (16.0, 15.0, 0.16),
        materials["floor"],
        bevel=0.03,
        collection=render_collection,
    )


def add_area_light(name: str, location: tuple[float, float, float], color: tuple[float, float, float], energy: float, size: float, target: tuple[float, float, float]) -> None:
    light_data = bpy.data.lights.new(name=name, type="AREA")
    light_data.energy = energy
    light_data.color = color
    light_data.shape = "DISK"
    light_data.size = size
    light = bpy.data.objects.new(name, light_data)
    bpy.context.scene.collection.objects.link(light)
    light.location = location
    direction = Vector(target) - light.location
    light.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def point_camera(camera: bpy.types.Object, target: tuple[float, float, float]) -> None:
    direction = Vector(target) - camera.location
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()


def build_scene() -> tuple[bpy.types.Collection, bpy.types.Collection]:
    reset_scene()
    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 1280
    scene.render.resolution_y = 720
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.render.film_transparent = False
    scene.render.image_settings.color_mode = "RGBA"
    scene.view_settings.look = "AgX - Medium High Contrast"
    scene.world.color = (0.003, 0.004, 0.009)

    physical = bpy.data.collections.new("BOARDROOM_PHYSICAL_STAGE")
    render_only = bpy.data.collections.new("RENDER_ONLY")
    scene.collection.children.link(physical)
    scene.collection.children.link(render_only)

    materials = {
        "obsidian": make_material("Obsidian shell", (0.006, 0.009, 0.014, 1.0), metallic=0.72, roughness=0.3),
        "graphite": make_material("Graphite housing", (0.014, 0.02, 0.03, 1.0), metallic=0.76, roughness=0.3),
        "edge": make_material("Machined edge metal", (0.04, 0.055, 0.072, 1.0), metallic=0.84, roughness=0.24),
        "glass": make_material("Smoked display glass", (0.004, 0.012, 0.018, 1.0), metallic=0.28, roughness=0.1),
        "tactical_glass": make_material("Tactical wing glass", (0.004, 0.014, 0.02, 1.0), metallic=0.22, roughness=0.18),
        "command_glass": make_material("Primary command glass", (0.004, 0.018, 0.024, 1.0), metallic=0.22, roughness=0.16),
        "cyan": make_material("Cyan emitter", (0.03, 0.35, 0.42, 1.0), metallic=0.3, roughness=0.18, emission=(0.05, 0.85, 1.0, 1.0), emission_strength=5.0),
        "cyan_dim": make_material("Cyan data trace", (0.015, 0.12, 0.15, 1.0), metallic=0.35, roughness=0.2, emission=(0.03, 0.5, 0.62, 1.0), emission_strength=1.7),
        "magenta": make_material("Magenta emitter", (0.35, 0.025, 0.22, 1.0), metallic=0.3, roughness=0.18, emission=(1.0, 0.04, 0.5, 1.0), emission_strength=4.0),
        "floor": make_material("Dark floor", (0.01, 0.012, 0.02, 1.0), metallic=0.48, roughness=0.36),
    }

    # Plan-view points are authored directly in Blender coordinates: X right,
    # Y away from the operator, Z up. The inner edge creates the operator bay.
    desk_points = [
        (-5.55, -2.7), (-5.4, -0.65), (-4.2, 0.15), (-2.4, 0.43), (0.0, 0.52),
        (2.4, 0.43), (4.2, 0.15), (5.4, -0.65), (5.55, -2.7), (4.35, -2.46),
        (3.0, -2.08), (1.62, -1.8), (0.0, -1.7), (-1.62, -1.8), (-3.0, -2.08), (-4.35, -2.46),
    ]
    desk = add_extruded_polygon("CommandDesk.Shell", desk_points, -0.12, 0.32, materials["obsidian"], physical, bevel=0.085)
    desk["arda_physical_stage"] = True

    # One neutral recessed material seam breaks up the broad slab without
    # adding illuminated trim, a keyboard, or a field of physical buttons.
    for seam_index, (y, material, radius) in enumerate([(-0.92, materials["edge"], 0.012)]):
        add_curve_rail(
            f"CommandDesk.RecessedSeam.{seam_index}",
            [(-3.8, y - 0.22, 0.337), (-2.1, y + 0.02, 0.337), (0.0, y + 0.12, 0.337), (2.1, y + 0.02, 0.337), (3.8, y - 0.22, 0.337)],
            radius,
            material,
            physical,
        )


    # The operator surface remains flat, then one continuous mesh rises into a
    # curved console bank. Lower displays share that plane and its upper edge
    # conceals every monitor housing bottom.
    monitor_x = [-3.8, -1.9, 0.0, 1.9, 3.8]
    monitor_y = [0.62, 0.78, 0.84, 0.78, 0.62]
    monitor_z = [1.48, 1.54, 1.57, 1.54, 1.48]
    add_continuous_command_bank(materials["graphite"], physical)

    for index, (x, y, z) in enumerate(zip(monitor_x, monitor_y, monitor_z)):
        create_monitor_rig(index, (x, y, z), 1.72, 0.84, (0.0, -8.0, 1.1), materials, physical)

    # Wide embedded touch displays echo the reference console without becoming
    # loose tablet objects placed on top of the desk.
    lower_inserts = [
        (-3.8, -0.05, 0.65, 0.44, 1.7, 0.84),
        (-1.9, 0.08, 0.67, 0.17, 1.36, 0.72),
        (0.0, 0.12, 0.68, 0.0, 1.7, 0.86),
        (1.9, 0.08, 0.67, -0.17, 1.36, 0.72),
        (3.8, -0.05, 0.65, -0.44, 1.7, 0.84),
    ]
    for index, (x, y, z, yaw, width, depth) in enumerate(lower_inserts):
        create_lower_insert(index, (x, y, z), width, depth, yaw, materials, physical)

    create_backdrop(materials, render_only)
    add_area_light("Key neutral", (-4.5, -4.2, 5.2), (0.72, 0.78, 0.84), 850.0, 4.2, (0.0, 0.0, 0.8))
    add_area_light("Rim neutral", (4.8, 1.8, 4.2), (0.55, 0.62, 0.7), 520.0, 3.2, (0.0, 0.0, 1.0))
    add_area_light("Desk softbox", (0.0, -1.0, 5.8), (0.72, 0.78, 0.85), 900.0, 5.0, (0.0, -0.3, 0.2))
    add_area_light("Front fill", (0.0, -7.5, 2.4), (0.62, 0.68, 0.74), 480.0, 5.0, (0.0, 0.0, 1.0))

    camera_data = bpy.data.cameras.new("BoardroomCamera")
    camera = bpy.data.objects.new("BoardroomCamera", camera_data)
    scene.collection.objects.link(camera)
    camera.location = (0.0, -11.2, 3.35)
    camera_data.lens = 46.0
    camera_data.sensor_width = 36.0
    point_camera(camera, (0.0, -0.05, 0.92))
    scene.camera = camera
    return physical, render_only


def render_and_export(physical: bpy.types.Collection) -> None:
    ASSET_DIR.mkdir(parents=True, exist_ok=True)
    scene = bpy.context.scene
    scene.render.filepath = str(RENDER_PATH)
    bpy.ops.render.render(write_still=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(BLEND_PATH))

    bpy.ops.object.select_all(action="DESELECT")
    export_objects: list[bpy.types.Object] = []
    for obj in physical.all_objects:
        if obj.type in {"MESH", "CURVE", "EMPTY"}:
            obj.select_set(True)
            export_objects.append(obj)
    if export_objects:
        bpy.context.view_layer.objects.active = next((obj for obj in export_objects if obj.type == "MESH"), export_objects[0])
    bpy.ops.export_scene.gltf(
        filepath=str(GLB_PATH),
        export_format="GLB",
        use_selection=True,
        export_apply=True,
        export_materials="EXPORT",
        export_extras=True,
        export_yup=True,
    )


if __name__ == "__main__":
    physical_collection, _ = build_scene()
    render_and_export(physical_collection)
    print(f"Rendered {RENDER_PATH}")
    print(f"Exported {GLB_PATH}")
    print(f"Saved {BLEND_PATH}")
