<!-- sigil: REPAIR -->
# Lighting Contract

This document defines the lighting contract for the ARDA scene runtime.

It is a Phase 6.1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/src/assets/scene/ASSET_PIPELINE_CONTRACT.md`

## Purpose

The photoreal target depends on a disciplined lighting model.

This contract names the lighting rigs required per scene, their purposes,
and the rules that govern tone, exposure, and intensity.

## Core Principles

- lighting carries spatial meaning; it is not cosmetic
- each light in the scene must have a named role
- ambient lifts are limited; scene mood comes from directional and local
  lights, not by raising ambient
- the scene targets physically plausible exposure, not saturated neon

## Required Boardroom Rig

The boardroom must compose its lighting from these named roles:

- `boardroom_key_light` — directional, primary scene anchor
- `boardroom_fill_ambient` — low ambient lift, never above `0.35`
- `boardroom_ceiling_wash` — soft overhead contribution
- `boardroom_console_glow` — local emissive from control cluster
- `boardroom_window_fill` — cool directional tint from city window
- `boardroom_hologram_pulse` — local point light at hologram anchor

Every boardroom scene mesh must be lit by at least one of these roles.

## Required World Rig

The world must compose its lighting from these named roles:

- `world_sky_dome` — environmental dome contribution
- `world_key_light` — directional anchor
- `world_district_accent` — local district-specific tint
- `world_terminal_glow` — emissive terminal highlight
|- `world_traversal_path_light` — low traversal/path contribution
|
|Stars, fog, and atmosphere are governed by `systems/CONTRACTS.md`, not by this rig.
|
|## Tone & Exposure

- the renderer must run in physically correct mode
  (Three.js `physicallyCorrectLights = true`)
- tone mapping targets `ACESFilmicToneMapping`
- output encoding targets `sRGBEncoding` or the current Three.js-recommended
  equivalent
- exposure is tuned per scene through a single `toneMappingExposure` value,
  not through per-light intensity bumps

## Intensity Rules

- ambient lift intensity: `0.0 – 0.35`
- directional key intensity: `0.6 – 1.6`
- emissive local intensity: `0.2 – 1.5`
- point light distance: bounded to room/district scale, not infinite
- shadow casting: enabled for the key light per scene, optional for
  secondary lights

## Forbidden Patterns

- stacking three directional lights to "flatten the scene"
- using emissive materials to compensate for under-lit rigs
- using fullscreen color filters as a substitute for lighting
- per-component `new THREE.*Light()` outside the named roles above
- unbounded point lights (no `distance` argument)

## Mapping Lighting To Scene Transitions

Scene transitions may change:

- key light direction
- exposure
- ambient lift

Scene transitions may not:

- strobe lights for dramatic effect
- drop ambient to zero, creating black frames
- introduce a new light role that is not in this contract

## Authoring Rules

- lighting is authored centrally per scene, not per component
- a component may request a named role by id; it may not instantiate an
  anonymous light
- if a new role is needed, it is added here first

## Companion Contracts

This contract depends on:

- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`

## Exit Requirement

The lighting contract is satisfied only when every light in the boardroom
and world scenes can be traced back to a named role in this document, and
when adjusting scene mood means tuning the named rig rather than scattering
new lights across components.
