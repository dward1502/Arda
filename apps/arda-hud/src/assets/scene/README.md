<!-- sigil: REPAIR -->
# Scene Assets

App-consumed visual assets for the ARDA scene runtime.

See `ASSET_PIPELINE_CONTRACT.md` in this directory for the full rules
(domains, naming, metadata sidecars, formats, ingestion, runtime loading).

## Domain Layout

- `window/` — boardroom city plates, skyline layers, gate visuals
- `hologram/` — avatar textures, projection masks, references
- `world/` — district visuals, traversal references
- `materials/` — reusable PBR texture sets shared across domains

Within a domain, group by subject, not by file type:

```
materials/
  boardroom_floor/
    boardroom_floor_albedo.png
    boardroom_floor_normal.png
    boardroom_floor_roughness.png
    metadata.json
```

Every non-trivial asset group ships a `metadata.json` sidecar; the contract
document lists the required fields.

## Sourcing

For a photoreal target with clean licensing:

- **Textures (CC0)** — [ambientCG](https://ambientcg.com/) and
  [Poly Haven textures](https://polyhaven.com/textures). Primary source for
  the `materials/` domain.
- **HDRIs / environment maps (CC0)** — [Poly Haven HDRIs](https://polyhaven.com/hdris).
  Feeds the boardroom and world `environmentMap` per `systems/CONTRACTS.md`.
- **Meshes (CC0, limited)** — [Poly Haven models](https://polyhaven.com/models).
  Selection is thin; expect to author the boardroom desk and monitor
  bezels in Blender.
- **Larger mesh kits (commercial)** — Quixel Megascans, Kitbash3D. Worth the
  license for the world districts; overkill for the boardroom shell.
- **Hologram / presence** — author in-house. No stock asset matches the
  restraint rules in `systems/CONTRACTS.md`; the shader carries most
  of the look.

Main tradeoff: CC0 sources are free and license-clean but force you to
author any mesh they do not already provide. Paid kits trade license
obligations for a faster ramp.

## What Not To Use

- CDN hotlinks at runtime — violates the contract's ingestion rule
- Sketchfab without strict license filtering — the default license mix is
  not safe for a shipped product
- Textures.com mixed-license catalog — too easy to grab a non-CC0 asset
  by mistake
- AI-generated textures without verifiable provenance — the metadata
  `source` and `license` fields become unfalsifiable

## Current Stubs

Drop CC0 assets into each folder using the filenames listed in its
`metadata.json` to activate real rendering.

Structural boardroom trio (PBR materials, consumed via
`src/scene/systems/sceneMaterials.ts`):

- `materials/boardroom_floor/` — floor plane (albedo / normal / roughness)
- `materials/boardroom_wall/` — back wall plate (albedo / normal / roughness)
- `materials/boardroom_desk/` — desk surface (albedo / normal / roughness /
  metalness — metalness map required because desk finishes land in the
  suspect-metalness range the material contract flags)

Boardroom IBL (image-based lighting, governed by
`systems/CONTRACTS.md`):

- `window/boardroom_environment/` — boardroom HDRI environment map (.hdr).
  Single biggest photoreal lever; activating this lifts every PBR material
  in the scene at once.

## Phase 6.2 Entry Order

When replacing placeholder geometry:

1. Structural boardroom surfaces — floor, wall, desk
2. Monitor bezels and control housings
3. Environment map (boardroom IBL)
4. World ground plate and district structures
5. Hologram/presence treatment (last — depends on shader work)
