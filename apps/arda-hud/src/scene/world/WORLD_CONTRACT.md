<!-- sigil: REPAIR -->
# World Contract

This document defines the world scene contract.

## Purpose

The world scene is a distinct traversal layer beyond the boardroom.

It is not:

- a page
- a dashboard tab
- a fullscreen panel

It is:

- a separate 3D space
- a traversal context
- a spatial operations layer

## Required World Structures

The world scene must define:

- districts
- traversal paths
- terminal anchors
- route overlays
- return path to boardroom
- configurable district/terminal surface assignments

## Required World Anchors

The world must expose at least:

- `district_command`
- `district_knowledge`
- `district_operations`
- `district_finance`
- `district_communications`
- `district_governance`
- `district_monitoring`
- `terminal_primary`
- `terminal_secondary`
- `terminal_tertiary`
- `boardroom_return_gate`

District names may evolve, but the world model must remain zone-based.

## Surface Assignment Contract

World district and terminal surfaces are configured through
`core/state/arda_world_surfaces.json`.

Each assignment must define:

- stable `surface_id`
- `source_zone_id` focus target
- district or terminal role
- module ids and presentation modes
- `surface_layout` preview widgets, refresh cadence, focus mode, and embed
  policy

The world runtime may use hardcoded geometry placement, but click routing and
surface behavior should come from this contract whenever a matching surface is
configured.

## World Behaviors

The world scene must support:

- spatial traversal
- district targeting
- district inspection
- workstation opening from world anchors
- return transition to boardroom

## Camera Requirements

The world must support at least:

- `world_entry`
- `world_traverse`
- `world_district_focus`
- `world_terminal_focus`
- `world_return_focus`

## Overlay Rules

World overlays must be:

- tactical
- spatially subordinate to the world itself
- legible without flattening the scene into dashboard chrome

World overlays must not become the primary visual plane.

## Visual Rules

- the world must read as a real 3D environment
- traversal and district identity must be spatially coherent
- reference from the older cybercity prototype may inform composition, but not
  dictate final style
- photoreal target takes precedence over neon infographic aesthetic

## Exit Requirement

The world contract is satisfied only when implementation can support traversal,
district interaction, workstation entry, and return-to-boardroom behavior as a
true scene flow.
