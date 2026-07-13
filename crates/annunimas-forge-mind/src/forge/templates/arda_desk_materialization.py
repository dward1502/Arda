# ARDA desk materialization scaffold.
#
# This is intentionally conservative until the boardroom desk asset family has
# a produced source mesh. Forge-Mind routes desk/table/console-surface assets to
# this family next, then this pass should evolve into beveled graphite slabs,
# inset emissive trim, cable channels, and physically plausible support detail.

import bpy

print("[forge-mind] ARDA desk materialization scaffold starting")

for obj in bpy.data.objects:
    if obj.type == "MESH":
        obj.name = obj.name or "arda_desk_source_mesh"

print("[forge-mind] ARDA desk materialization scaffold complete")
