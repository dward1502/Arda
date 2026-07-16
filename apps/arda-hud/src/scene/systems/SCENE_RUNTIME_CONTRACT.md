<!-- sigil: REPAIR -->
# Scene Runtime Contract

This document defines the implementation contract for the ARDA scene runtime.

It is the Phase 1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/ARDA_AUDIT.md`

## Scope

The ARDA scene runtime is responsible for:

- boardroom scene orchestration
- world scene orchestration
- workstation spawning and placement
- camera state transitions
- anchor registration and activation
- diegetic overlay control

It is not responsible for:

- generic page layout
- dashboard chrome
- domain business logic
- module-card presentation semantics

## Scene Modes

The runtime has exactly three primary modes:

1. `boardroom`
2. `world`
3. `workstation_focus`

`workstation_focus` does not replace boardroom/world as a universe root.
It is a presentation state layered on top of a boardroom or world origin.

## Runtime Model

The scene runtime must expose:

- active mode
- active camera state
- registered anchors
- active workstation instances
- scene transition state
- diegetic overlay state

## Required Systems

The runtime must eventually provide explicit implementations for:

- anchor registry
- scene state machine
- camera controller
- interaction dispatcher
- workstation spawner
- native window bridge
- theme-to-scene mapper

## Anchor Types

Anchors are stable named interaction points in 3D space.

Every anchor must define:

- `id`
- `scene`
- `type`
- `position`
- `orientation`
- `activation_behavior`
- `visibility_rule`
- `data_binding`

Anchor types:

- `monitor`
- `console`
- `control`
- `hologram`
- `gate`
- `district`
- `terminal`
- `workstation_spawn`

## Data Projection Rule

Scene code must consume scene-native state, not panel-native state.

That means runtime projection should prefer:

- anchors
- zones
- surfaces
- props
- scene overlays
- workstation manifests
- traversal nodes

Instead of:

- card layouts
- tabs as primary model
- page sections as primary spatial model

## Required Companion Contracts

This runtime contract depends on:

- `boardroom/BOARDROOM_CONTRACT.md`
- `world/WORLD_CONTRACT.md`
- `workstations/WORKSTATION_CONTRACT.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`

## Phase Exit Requirement

Phase 1 is not complete until all companion contracts exist and are specific
enough for an implementation agent to build against them without inventing the
core scene model.
