<!-- sigil: REPAIR -->
# Asset Pipeline Contract

This document defines the asset pipeline contract for the ARDA scene runtime.

It is the Phase 6.1 execution target referenced by:

- `docs/arda/ARDA_FRONTEND_REBUILD.md`
- `apps/arda-hud/ARDA_IMPLEMENTATION_PLAN.md`

## Purpose

Photoreal presentation requires a disciplined asset pipeline.

This contract defines where assets live, how they are named, what formats are
allowed, and how they map to scene anchors and zones so that the visual
replacement pass in Phase 6.2 can proceed without inventing organizational
rules ad hoc.

## Asset Domains

The asset pipeline is partitioned by scene domain, not by file type.

The four domains are:

- `window` — boardroom city window, skyline plates, gate visuals
- `hologram` — avatar/presence visuals, projection masks, reference cards
- `world` — district visuals, traversal plates, terminal surfaces
- `materials` — reusable PBR texture sets and overlays shared across domains

Every asset must live under one of these four directory roots under
`src/assets/scene/`.

## Required Directory Layout

```
src/assets/scene/
  window/
  hologram/
  world/
  materials/
```

Sub-structure within each domain follows one rule: group by subject, not by
file type.

Example (correct):

```
world/
  district_command/
    albedo.png
    normal.png
    roughness.png
    metadata.json
```

Example (incorrect):

```
world/
  textures/
  normals/
  json/
```

## Supported Formats

Runtime-loaded assets must be one of:

- `.png` — lossless textures, UI plates, masks
- `.jpg` — large environment plates where banding is acceptable
- `.ktx2` — compressed GPU textures for production builds
- `.glb` — self-contained mesh packages with embedded materials
- `.hdr` / `.exr` — environment/IBL inputs
- `.json` — asset metadata sidecars

No other format may be added without updating this contract first.

## Required Metadata Sidecar

Every non-trivial asset group (a `district_*` mesh, a `window_*` plate, a
`hologram_*` rig) must ship with a `metadata.json` describing:

- `id`
- `domain` (`window` | `hologram` | `world` | `materials`)
- `scene_binding` (anchor id, zone id, or `shared`)
- `material_family` (see `systems/CONTRACTS.md`)
- `source` (origin tool or vendor reference)
- `license`

Metadata sidecars are the runtime's authority for how an asset is allowed to
appear in scene. Missing metadata means the asset cannot be loaded by the
scene runtime.

## Naming Rules

- directories use `snake_case`
- asset ids use `snake_case` prefixed by their domain (`world_district_command`)
- texture channels use canonical suffixes: `_albedo`, `_normal`, `_roughness`,
  `_metalness`, `_ao`, `_emissive`, `_mask`
- no human names, ticket numbers, or dates in filenames

## Scene Binding Rules

Assets bind to scene structure, not to UI components.

An asset may bind to:

- a specific anchor id from `systems/runtimeTypes.ts`
- a zone id
- the literal `shared` for cross-domain materials

An asset may not bind to:

- a React component name
- a page route
- a dashboard panel id

If an asset has no valid scene binding, it does not belong in this pipeline.

## Ingestion Rules

- assets enter the repo only after a metadata sidecar exists
- binary assets larger than 1 MB should be reviewed before commit
- runtime assets must satisfy `ARDA_CONTRACTS_MANIFEST.md` before being
  treated as production-ready; oversized starter assets require an explicit
  lazy-load/archive/optimization note
- source/working files (`.blend`, `.psd`, `.spp`) do not belong under
  `src/assets/scene/`; they live outside the repo or under a dedicated
  source tree if added later
- `.gitkeep` files may be retained only while a domain is empty

## Reference-Fidelity Gate

Assets generated from a visual reference pack must not be accepted only because
they are valid GLB files. The creation pattern is:

1. Capture the primary reference path and SHA-256 in `metadata.json`.
2. Render the generated asset from at least front, side, rear, and three-quarter
   camera angles.
3. Run a deterministic structural gate for the asset family when one exists.
4. Compare the rendered asset against the original reference before cloning,
   variant generation, or final art lock.

For `upper_monitor_*` assets derived from
`human/inbox/arda_boardroom_spec_pack_v0_2/monitors.jpg`, the monitor must show
all of the following before it is treated as more than a scaffold:

- wide rectangular screen form
- rugged graphite top/bottom/side armor
- cyan emissive accents that sit on the surface rather than floating or bleeding
- side service modules or equivalent hard-surface detail
- rear backplate and layered mount structure
- visible rotary hinge discs
- articulated support arms

The deterministic gate for this family is
`scripts/remote/verify_arda_monitor_fidelity.py`. It is a structural safety net;
human or vision review remains required for final art direction.

## Runtime Load Rules

Scene code must:

- resolve assets through a single loader path tied to `src/assets/scene/`
- refuse to render an asset with no metadata sidecar
- prefer `.ktx2` / `.glb` at runtime when both raw and compressed forms exist

Scene code must not:

- hotlink external URLs for primary scene assets
- hardcode absolute repo paths into component files
- treat the `src/assets/` root as a shared dumping ground

## Companion Contracts

This pipeline depends on:

- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `shaders/SHADER_CONTRACT.md`

## Exit Requirement

The asset pipeline contract is satisfied only when a new asset can be dropped
into the correct domain folder with a valid metadata sidecar and be consumed
by the scene runtime without any component-level special-case wiring.
