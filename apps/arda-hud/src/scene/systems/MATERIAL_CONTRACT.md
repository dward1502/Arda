<!-- sigil: REPAIR -->
# Material Contract

This document defines the material contract for the ARDA scene runtime.

It is a Phase 6.1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/src/assets/scene/ASSET_PIPELINE_CONTRACT.md`

## Purpose

Materials define the physical character of the scene.

This contract forbids ad hoc `meshStandardMaterial` tuning sprinkled across
scene components and instead names the material families that the runtime
will support.

## Target Model

The scene uses physically based rendering.

All non-overlay geometry must be rendered with a PBR material, expressed in
Three.js as `meshStandardMaterial` or `meshPhysicalMaterial`.

`meshBasicMaterial` is permitted only for:

- diegetic UI surfaces that must not react to scene light
- debug overlays during development

## Material Families

The runtime recognizes the following named material families. Every scene
asset must declare one of these in its metadata sidecar.

### Boardroom Families

- `boardroom_floor`
- `boardroom_wall`
- `boardroom_desk`
- `boardroom_monitor_bezel`
- `boardroom_monitor_surface`
- `boardroom_control_housing`
- `boardroom_hologram_plate`
- `boardroom_window_plate`
- `boardroom_gate_metal`

### World Families

- `world_ground_plate`
- `world_district_structure`
- `world_terminal_housing`
- `world_route_marker`
- `world_atmosphere_plate`

### Shared Families

- `shared_trim_metal`
- `shared_trim_glass`
- `shared_emissive_edge`
- `shared_signal_mesh`

No material may be introduced outside this list without updating this
contract.

## Required Channels

A PBR material family must source:

- `albedo`
- `normal`
- `roughness`

A material family may additionally source:

- `metalness`
- `ao`
- `emissive`
- `mask`

Texture channel filenames follow
`src/assets/scene/ASSET_PIPELINE_CONTRACT.md`.

## Parameter Rules

- `roughness` is sourced from a map where possible; literal `1.0` is acceptable
  only for intentionally matte surfaces
- `metalness` values between `0.2` and `0.8` are suspect and must be justified
  in the metadata sidecar (`notes` field)
- `emissiveIntensity` may not exceed `1.5` on any surface except the
  `hologram_plate` and `emissive_edge` families
- `transparent: true` must be paired with an explicit `opacity` below `1.0` and
  should avoid overlap with shadow-casting geometry

## Forbidden Patterns

- per-component one-off color hex values instead of a named material family
- neon saturation on structural surfaces (floor, wall, desk, ground plate)
- emissive overrides stacked on top of textured surfaces to fake glow
- `meshPhongMaterial` anywhere in scene geometry
- wireframe-only geometry outside debug mode

## Mapping Materials To Anchors

Material families bind to anchor types as follows:

| Anchor Type         | Default Family                 |
|---------------------|--------------------------------|
| `monitor`           | `boardroom_monitor_surface`    |
| `console`           | `boardroom_control_housing`    |
| `control`           | `boardroom_control_housing`    |
| `hologram`          | `boardroom_hologram_plate`     |
| `gate`              | `boardroom_gate_metal`         |
| `district`          | `world_district_structure`     |
| `terminal`          | `world_terminal_housing`       |
| `workstation_spawn` | `world_terminal_housing`       |

These defaults are overrides-only — an anchor may declare a non-default
family through scene authoring, but component code may not silently bypass
the registry.

## Authoring Rules

- new materials are introduced by adding a family above, not by ad hoc
  tuning in a component
- material families are the authoritative source of look; per-anchor color
  tweaks are not permitted
- if a scene needs a new look, it needs a new family entry here first

## Companion Contracts

This contract depends on:

- `src/assets/scene/ASSET_PIPELINE_CONTRACT.md`
- `systems/CONTRACTS.md`
- `shaders/SHADER_CONTRACT.md`

## Exit Requirement

The material contract is satisfied only when every rendered mesh in the
boardroom and world scenes can point to a named family in this document,
and when introducing a new look requires editing this list rather than
editing component color literals.
