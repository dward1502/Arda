<!-- sigil: REPAIR -->
# Atmosphere Contract

This document defines the atmospheric contract for the ARDA scene runtime.

It is a Phase 6.1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/src/assets/scene/ASSET_PIPELINE_CONTRACT.md`

## Purpose

Atmosphere distinguishes the boardroom from the world and reinforces the
sense that each scene is a physical space, not a dashboard surface.

This contract governs fog, haze, sky, environmental reflections, and any
other scene-level ambience layer.

## Scope

Atmosphere refers to:

- fog
- haze
- environmental reflection / IBL
- background color and sky composition
- optional particulate layers (dust, rain, mist)
- world-only star field and horizon glow

It does not refer to:

- emissive materials on geometry (see `systems/CONTRACTS.md`)
- directional or local lights (see `systems/CONTRACTS.md`)
- hologram projection (see `systems/CONTRACTS.md`)

## Boardroom Atmosphere

The boardroom must define:

- `fogColor`
- `fogNear`
- `fogFar`
- `backgroundColor`
- `environmentMap` (IBL source from `src/assets/scene/window/`)

Ranges:

- `fogNear`: between camera minimum distance and room mid-depth
- `fogFar`: not beyond the far wall of the room
- `backgroundColor`: deep neutral; not saturated neon

The boardroom may not use a star field.

## World Atmosphere

The world must define:

- `fogColor`
- `fogNear`
- `fogFar`
- `skyColor` or sky dome asset
- `starField` parameters
- optional `horizonGlow`

Ranges:

- `fogNear`: beyond terminal anchors
- `fogFar`: at or before district extents
- stars are permitted but must not dominate the traversal plane

## Environmental Reflection

Each scene must bind an environment map for IBL:

- boardroom: plate derived from the city window asset domain
- world: plate derived from the world asset domain

Reflection probes may not be instantiated per component. A scene may define
at most two active environment maps at once.

## Particulate & Weather

Particulate layers are optional and must be:

- physically plausible in density
- bounded to the scene's spatial extent
- paused when the scene is inactive (`frameloop = 'never'`)

Particulate layers must not:

- cover the viewport with flat overlay gradients
- simulate "digital rain" or neon-grid effects on the primary visual plane

## Transition Rules

Atmospheric parameters (fog range, background color, environment map) may
ramp across a scene transition, but:

- ramps must be bounded in duration
- the transition must not strobe atmosphere
- environment maps may cross-fade; they may not hard-cut mid-gaze

## Forbidden Patterns

- setting `fogFar` to effectively infinite to "see everything"
- using atmosphere color to fake mood that the lighting rig should produce
- binding fog to scroll position or mouse movement
- per-component fog overrides inside React components

## Authoring Rules

- atmosphere is authored as a single scene-level record per mode
- runtime code reads atmospheric parameters from that record; it does not
  inline fog or background values per component
- any new atmospheric layer requires updating this contract first

## Companion Contracts

This contract depends on:

- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`

## Exit Requirement

The atmosphere contract is satisfied only when the fog, sky, and environment
parameters of each scene are owned by a single scene-level record and can be
tuned without editing scene components, and when the boardroom and world
read as distinct physical atmospheres rather than reskinned dashboards.
