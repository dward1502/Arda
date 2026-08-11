<!-- sigil: REPAIR -->
# Slot Component Contract

Scene slots are stable visual placement IDs that may host different
workstation components over time.

Examples:

- `monitor_1`
- `monitor_2`
- `monitor_3`
- `monitor_4`
- `monitor_5`
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

Unassigned operator/desk slots may open a workstation template. Unassigned
upper monitors are the exception: they render their slot-specific ambient
identity and must remain non-interactive until a typed session or live claim
occupies the slot.

The fallback zone ID format is:

```text
scene_slot:<slot_id>
```

This gives configurable operator slots a real runtime container before final
custom components exist. These fallback containers must be slot-specific
templates, not one generic placeholder. They must never be used to make an idle
upper monitor open the generic panel or Sovereign World.

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

- `monitor_1`
- `view_desk_control_panel`

Avoid:

- `governance_monitor`
- `network_console`

## Exit Requirement

This contract is satisfied when every visible slot can open either:

- its assigned workstation/component, or
- for operator/desk slots, a slot-specific template workstation using
  `scene_slot:<slot_id>`.

An idle upper monitor satisfies the contract by showing its ambient identity and
performing no activation action.
