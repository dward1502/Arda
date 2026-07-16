<!-- sigil: REPAIR -->
# Slot Component Contract

Scene slots are stable visual placement IDs that may host different
workstation components over time.

Examples:

- `monitor_left_1`
- `monitor_left_2`
- `monitor_left_3`
- `monitor_left_4`
- `view_desk_l`
- `view_desk_control_panel`
- `view_desk_r`
- `view_desk_aux`

## Purpose

A slot component is the content assigned to a scene slot. The slot owns where
the component appears; the component owns what it shows.

This keeps scene layout independent from domain naming. A monitor can display a
routing surface today and a planning surface later without renaming the monitor.

## Required Model

Each slot assignment must define:

- `slot_id` — stable scene placement ID
- `component_id` — component/workstation identifier
- `source_zone_id` — ARDA section or synthetic template zone
- `title` — operator-facing title
- `module_ids` — modules available inside the workstation surface
- `presentation_modes` — supported modes, currently `in_scene` and/or
  `native_window`

## Template Rule

Unassigned slots must still open a workstation template.

The fallback zone ID format is:

```text
scene_slot:<slot_id>
```

This gives every visual slot a real runtime container before final custom
components exist. These fallback containers must be slot-specific templates, not
one generic placeholder for every slot.

The current template registry lives in
`sceneSlotWorkstationTemplates.ts` and defines title, module set, presentation
modes, source zone, and entry anchor for every boardroom scene slot.

## Customization Rule

Slot assignment must be configurable without changing scene code.

The current implementation loads boardroom assignments from workspace/core state
at `core/state/arda_boardroom_slots.json`, with browser-local operator state as
a fallback while the workspace document is unavailable. The durable assignment
schema and `surface_layout` display contract are documented in
`ARDA_CONTRACTS_MANIFEST.md`.

## Rendering Rule

Scene visuals must not be named after the assigned component unless the mesh is
permanently domain-specific.

Good:

- `monitor_left_1`
- `view_desk_control_panel`

Avoid:

- `governance_monitor`
- `network_console`

## Exit Requirement

This contract is satisfied when every visible slot can open either:

- its assigned workstation/component, or
- a slot-specific template workstation using `scene_slot:<slot_id>`.
