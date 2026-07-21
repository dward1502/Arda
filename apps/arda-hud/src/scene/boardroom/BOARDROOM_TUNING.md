<!-- sigil: REPAIR -->
# Boardroom Tuning

This is the manual tuning guide for the ARDA boardroom cockpit.

## Where to Adjust the Room

Edit:

`apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts`

Each zone has:

- `position: [x, y, z]`
- `rotation: [pitch, yaw, roll]`
- `size: [width, height, depth]`

Three.js units are arbitrary scene units.

Coordinate meaning:

- `x`: left/right. Negative moves left, positive moves right.
- `y`: height. Larger moves up.
- `z`: depth. Larger moves toward the seated operator/camera; smaller moves
  toward the city window.

Rotation meaning:

- `rotation[0]`: pitch. Negative tips the surface back/up toward the operator.
- `rotation[1]`: yaw. Positive turns a left-side surface inward; negative turns
  a right-side surface inward.
- `rotation[2]`: roll. Use sparingly for subtle tilted-panel correction.

## Current Cockpit Shape

Lower cockpit surfaces:

- `boardroom.lower.left_wrap`
- `boardroom.lower.left_inner`
- `boardroom.control.center`
- `boardroom.lower.right_inner`
- `boardroom.lower.right_wrap`

The outer lower screens should have the strongest yaw and sit farther forward
on `z`. Inner screens should have softer yaw and sit closer to the center.

Upper monitor rail:

- `boardroom.monitor.left`
- `boardroom.monitor.center_left`
- `boardroom.monitor.center_right`
- `boardroom.monitor.right`

These stay above the desk row and should frame the city/window surface.

Avatar origin:

- `boardroom.avatar.emitter`

This is the circular desk device behind the center control panel. The avatar
should rise from this zone, not from the city/window.

## Camera / User Viewport

The viewport camera lives in:

`apps/arda-hud/src/scene/boardroom/BoardroomViewport.tsx`

Look for `OrbitControls`.

Current behavior:

- mouse wheel zoom is disabled; the operator has a fixed seated distance
- click-drag rotates the operator view left and right around the room
- pan is disabled so the physical scene stays stable
- scene objects are stable in normal mode
- in edit/debug mode, cockpit zones are draggable and their positions are saved
  in browser local storage as `arda.boardroom.zone_positions.v1`

Useful camera values:

- `target`: where the operator camera looks
- `enableZoom`: remains `false` for a fixed operator distance
- `minAzimuthAngle` / `maxAzimuthAngle`: left/right look limits
- `minPolarAngle` / `maxPolarAngle`: vertical look lock
- camera `position`: initial operator viewpoint
- camera `fov`: field of view

For cockpit tuning, drag zones in edit mode first. Change camera values only
when the operator seat itself feels wrong.

## Dragging Zones

Turn on boardroom edit/debug mode, then drag a screen/control zone in the scene.
The dragged `position` override is stored locally so you can iterate visually.

To reset dragged positions, clear this browser local storage key:

`arda.boardroom.zone_positions.v1`

After a layout feels right, copy the resulting positions back into
`boardroomSpatialLayout.ts` so the default contract matches the design.

## Visual Surfaces

Monitor and desk surfaces are intentionally rendered as simple live screen slabs
for now. The older monitor/control GLB models are not used for these surfaces in
the cockpit pass because they made the scene read as old asset placement instead
of a shaped operator desk.

The city/window currently uses a JPG plate. Longer term it can become a richer
2D/3D parallax or WebGL scene without changing the boardroom zone contract.
