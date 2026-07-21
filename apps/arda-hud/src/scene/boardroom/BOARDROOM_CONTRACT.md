<!-- sigil: REPAIR -->
# Boardroom Contract

This document defines the boardroom scene contract.

## Purpose

The boardroom is the primary ARDA operating scene.

It is the default spatial home of the system and must feel like a real room,
not a dashboard wrapper.

The current target is a seated C-shaped operator cockpit:

- lower desk surfaces wrap around the user
- four upper monitors sit above the lower row
- the rear window carries the cyberpunk city/world view
- ARDA presence rises from a circular desk emitter behind the center control
  surface
- normal runtime is stable and fixed-distance; edit/debug mode is where zones
  may be dragged for tuning

## Required Boardroom Anchors

The boardroom must expose these anchors:

- `desk_left_surface`
- `desk_right_surface`
- `upper_monitor_1`
- `upper_monitor_2`
- `upper_monitor_3`
- `upper_monitor_4`
- `center_console`
- `governance_control`
- `systems_control`
- `network_control`
- `human_control`
- `settings_control`
- `hologram_anchor`
- `world_gate`

## Spatial Zone Layer

The boardroom runtime now separates primitive spatial zones from the data shown
inside those zones. The editable contract is
`apps/arda-hud/src/scene/boardroom/boardroomSpatialLayout.ts`.

Current primitive zones:

- `boardroom.monitor.left`
- `boardroom.monitor.center_left`
- `boardroom.monitor.center_right`
- `boardroom.monitor.right`
- `boardroom.lower.left_wrap`
- `boardroom.lower.left_inner`
- `boardroom.control.center`
- `boardroom.lower.right_inner`
- `boardroom.lower.right_wrap`
- `boardroom.button.hermes`
- `boardroom.button.settings`
- `boardroom.avatar.emitter`
- `boardroom.world.window`

Model assets should attach to zone bindings and inherit each zone position,
rotation, and size. They should not own routing, workstation selection, or data
freshness semantics.

Monitor and desk zones may display ARDA data today, third-party tools
tomorrow, or operator-created components later. The slot assignment layer is the
handoff between the generic spatial HUD and the system-specific adapters.

The current slot assignment authority is
`core/state/arda_boardroom_slots.json`, parsed by
`apps/arda-hud/src/lib/boardroomSlotSettings.ts` and documented in
`ARDA_CONTRACTS_MANIFEST.md`. Each assignment may
carry `surface_layout` metadata that describes:

- adapter type: ARDA component, local service, document, media, terminal,
  agent stream, virtual machine stream, or custom embed
- lightweight boardroom preview mode and full workstation focus mode
- CSS-grid-style preview widgets such as status strips, particle streams,
  metric sparklines, logs, Markdown/PDF/image/video/document previews, and
  terminal text
- refresh cadence, embed URL, service ID, and inline-embed permission

The boardroom scene should render compact preview widgets in the room and open
heavy/interactive surfaces in a focused workstation window unless native
Tauri/WebKit embed proof explicitly allows inline display.

The lower desk zones should read as a C-shaped operator cockpit. The avatar
presence should originate from `boardroom.avatar.emitter`, a circular device on
the desk behind the center control panel. The world window remains a separate
environment and transition surface.

Manual transform and camera tuning guidance lives in `BOARDROOM_TUNING.md`.

Accepted edit-mode positions should be promoted back into
`boardroomSpatialLayout.ts`; browser-local drag overrides are design scratch
state, not the canonical contract.

## Anchor Semantics

### Desk Surfaces

`desk_left_surface` and `desk_right_surface` are always-visible command surfaces.

They are for:

- persistent high-signal operational previews
- not full workstation detail

### Upper Monitors

Upper monitors are configurable preview surfaces.

They are for:

- compact live previews
- tactical snapshots
- workstation entry points

They are not for:

- full dashboard walls
- dense spreadsheet-like content

### Center Console

The center console is the primary local interaction cluster.

It is for:

- mode-affecting commands
- anchor-triggered scene actions
- high-priority operator controls

### Hologram Anchor

The hologram anchor is the embodied presence point for ARDA/Arandur.

It is for:

- guidance presence
- status emphasis
- transition reinforcement

It is not merely decorative.

### World Gate

The world gate is the explicit transition anchor from boardroom to world mode.

It must be spatially legible and visually privileged.

## Camera Requirements

The boardroom must support at least these camera states:

- `boardroom_wide`
- `boardroom_operator`
- `boardroom_console_focus`
- `boardroom_monitor_focus`
- `boardroom_gate_focus`

## Interaction Rules

- hover may reveal affordance
- activation must be explicit
- workstation openings must be predictable
- scene transitions must be animated and reversible
- no anchor should silently teleport the user into unrelated context

## Visual Rules

- the room should read as a physically coherent interior
- monitor surfaces should feel embedded in the environment
- monitor and desk screens should separate glass panes, metallic trim, and
  content surfaces rather than presenting as one flat overlay; runtime
  refinement lives in `boardroomVisualRefinement.ts` until final Blender assets
  supersede it
- overlays should be diegetic where possible
- the boardroom should privilege lighting, materials, and spatial depth over
  decorative HUD clutter

## Exit Requirement

The boardroom contract is satisfied only when a real 3D boardroom scene can be
implemented from this document without falling back to legacy dashboard layout
logic.
