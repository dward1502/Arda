<!-- sigil: REPAIR -->
# ARDA Scene Contracts

Merged authority for scene-level layout, assets, workstations, slots, and tuning.

Last merged: 2026-06-23

## Scene World Contract

### Objective

World districts are safe-inspection surfaces, not command dashboards. Each
district must answer where the pressure is, who owns it, what data backs it,
what made it light up, and which safe inspection surface opens next.

### District behavior

- geometry and labels must be grounded in district contracts and projection-backed
  provenance
- urgency rings/color must attach to district geometry/labels
- actions from the world are limited to inspection and safe routing
- mutating queue or provider actions must be gated or draft-only

## Asset Pipeline Contract

### Purpose

The asset pipeline exists to keep runtime loading predictable and sized. Every
asset that ships in the default runtime must satisfy the asset performance
budget and carry predictable metadata sidecars.

### Runtime budget

- GLB/GLTF model: 5 MB per runtime asset
- Texture/image channel: 2 MB per runtime asset
- HDR/environment: 3 MB per runtime asset
- JS runtime chunk: 750 KB per chunk
- Total `dist/assets` payload: 65 MB

### Asset organization

Assets must live in a predictable directory tree with metadata sidecars that the
runtime can read. Heavy assets that exceed the budget must be deferred from the
default runtime import graph until optimized or lazy-loaded.

## Workstation and Slot Contracts

### Workstation contract

A workstation is the focused runtime surface behind a scene slot. It owns what
the slot shows, while the slot owns where it appears.

A workstation must declare:
  - stable workstation identity
  - module set
  - supported presentation modes
  - source zone or origin anchor
  - preview composition rules
  - focus/native window intent

### Slot component contract

Stable scene slots may host different workstation components over time without
renaming the slot.

Slot assignment rules:
  - slot id is geometry identity, not domain identity
  - unassigned slots must still open a slot-specific template workstation
  - slot assignment must be configurable without changing scene code
  - scene visuals must not be named after assigned components

Current visible boardroom scene slot ids:
  - monitor_left_1 through monitor_left_4
  - view_desk_l
  - view_desk_control_panel
  - view_desk_r
  - view_desk_aux

## Boardroom Contract

### Purpose

The boardroom is the primary ARDA operating scene. It must read as a real room,
not a dashboard wrapper.

### Required anchors

- desk_left_surface, desk_right_surface
- upper_monitor_1 through upper_monitor_4
- center_console
- governance_control, systems_control, network_control, human_control
- settings_control
- hologram_anchor
- world_gate

### Spatial zone layer

The boardroom separates primitive spatial zones from data shown inside them.
Current primitive zones mirror the anchors. Model assets attach to zone bindings
rather than owning routing or data freshness themselves.

### Anchor semantics

- desk surfaces are always-visible command surfaces for high-signal previews
- upper monitors are configurable preview surfaces, not full dashboard walls
- center console hosts mode-affecting commands and high-priority controls
- hologram anchor is the embodiment point for ARDA/Arandur presence
- world gate is the explicit boardroom-to-world transition anchor

### Visual rules

- monitor and desk surfaces should separate glass, trim, and content surfaces
- overlays should be diegetic where possible
- the room should privilege lighting, materials, and spatial depth over
  decorative HUD clutter

### Camera requirements

Supported camera states:
  - boardroom_wide
  - boardroom_operator
  - boardroom_console_focus
  - boardroom_monitor_focus
  - boardroom_gate_focus

### Interaction rules

- hover may reveal affordance
- activation must be explicit
- workstation openings must be predictable
- scene transitions must be animated and reversible
- no anchor should silently teleport the user into unrelated context

### Manual tuning guidance

Coordinate meaning:
  - x: left/right
  - y: height
  - z: depth; larger moves toward the seated operator/camera, smaller toward
    the city window

Rotation meaning:
  - pitch [0]: negative tips the surface back/up toward the operator
  - yaw [1]: positive turns a left-side surface inward
  - roll [2]: use sparingly for subtle correction

Camera guidance:
  - mouse wheel zoom is disabled for the seated operator distance
  - click-drag rotates view left and right
  - pan is disabled so scene objects remain stable
  - edit/debug mode allows dragging cockpit zones
  - dragged positions are stored locally under
    `arda.boardroom.zone_positions.v1`

Current lower cockpit surfaces:
  - boardroom.lower.left_wrap
  - boardroom.lower.left_inner
  - boardroom.control.center
  - boardroom.lower.right_inner
  - boardroom.lower.right_wrap

Current upper monitor rail:
  - boardroom.monitor.left
  - boardroom.monitor.center_left
  - boardroom.monitor.center_right
  - boardroom.monitor.right

Avatar origin:
  - boardroom.avatar.emitter

The city/window currently uses a JPG plate. Longer term it can become a richer
2D/3D parallax or WebGL scene without changing the boardroom zone contract.
