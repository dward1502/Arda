<!-- sigil: REPAIR -->
# ARDA Scene Core

This directory is the scene-first runtime for ARDA_HUD.

Use it for:
- Three.js / React Three Fiber scene composition
- camera and traversal orchestration
- scene anchors for boardroom, workstation, and world layers
- shader integration
- scene interaction contracts

Do not use it for:
- generic dashboard composition
- module-card level data rendering
- app chrome or page-layout logic

Expected structure:
- `boardroom/` — main room scene, desk, screens, hologram anchor
- `world/` — traversal scene and Gundam-style HUD overlays
- `workstations/` — in-scene floating workstation surfaces
- `shaders/` — scene and monitor shader code
- `systems/` — scene controllers, anchors, and contracts

Canonical contracts:
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `systems/CONTRACTS.md`
- `boardroom/BOARDROOM_CONTRACT.md`
- `world/WORLD_CONTRACT.md`
- `workstations/WORKSTATION_CONTRACT.md`
