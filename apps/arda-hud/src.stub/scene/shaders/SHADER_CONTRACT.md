<!-- sigil: REPAIR -->
# Shader Contract

This document defines the shader authoring contract for the ARDA scene
runtime.

It is a Phase 6.1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/src/assets/scene/ASSET_PIPELINE_CONTRACT.md`

## Purpose

Custom shaders extend the photoreal target in places where a standard PBR
material cannot carry the intent — monitors, holograms, environment
atmosphere, and signal meshes.

This contract defines what belongs under `src/scene/shaders/`, how shaders
are organized, and how they integrate with the material and presence
contracts.

## Allowed Shader Categories

The runtime recognizes these shader categories:

- `monitor` — CRT / scanline / subpixel treatment for boardroom monitor
  surfaces
- `hologram` — projection silhouette, distortion, and edge falloff for the
  hologram body
- `signal_mesh` — particle-field and signal-overlay shaders for transitions
  and emphasis
- `window_atmosphere` — parallax / depth treatment for the boardroom city
  window plate
- `environment` — sky-dome, horizon, and ambient shader work for world mode

A shader that does not fit one of these categories is out of scope for this
pipeline and must not land under `src/scene/shaders/`.

## Directory Layout

```
src/scene/shaders/
  monitor/
  hologram/
  signal_mesh/
  window_atmosphere/
  environment/
```

Within a category, each shader lives in its own subfolder containing:

- a vertex file (`*.vert.glsl` or inline in the TS wrapper)
- a fragment file (`*.frag.glsl` or inline in the TS wrapper)
- a TypeScript wrapper exposing the shader as a Three.js material
- a `metadata.json` with `id`, `category`, `scene_binding`, `source`

## Authoring Rules

- shaders are authored as standalone modules, not inlined inside scene
  components
- every shader module exports a single factory function that returns a
  Three.js material
- uniforms are declared explicitly with types; loose `any` uniforms are not
  allowed
- a shader may not read global DOM state; its inputs come from its uniforms

## Integration Rules

- a shader binds to a material family from `systems/CONTRACTS.md`
  when it replaces a standard PBR material on geometry
- the hologram body shader binds exclusively to the `hologram_body` surface
  declared in `systems/CONTRACTS.md`
- monitor shaders bind to the `boardroom_monitor_surface` family
- window-atmosphere shaders bind to the `boardroom_window_plate` family

Scene components may not instantiate raw `ShaderMaterial` calls. They must
go through the category's factory function.

## Performance Rules

- shaders must run at the target `dpr` range (`[1, 2]`) without dropping
  frames on reference hardware
- time-driven uniforms should be updated once per frame via the scene
  runtime, not by components
- heavy post-processing shaders are not covered by this contract; they
  require a separate contract before being introduced

## Forbidden Patterns

- copying shader source between files instead of importing from a category
  module
- shipping uncompiled GLSL strings concatenated inside components
- using `ShaderMaterial` for effects that a `MeshPhysicalMaterial` can
  produce
- shipping shader work without a `metadata.json`

## Companion Contracts

This contract depends on:

- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `src/assets/scene/ASSET_PIPELINE_CONTRACT.md`

## Exit Requirement

The shader contract is satisfied only when every custom shader in the
scene runtime lives under one of the categories above, is accessed through
a category factory, and declares its scene binding through metadata rather
than through component-level wiring.
