---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "plan"
  status: "complete"
  owner: "visual-pass"
  last_reviewed: "2026-08-25"
---

# WS3b — Avatar Redesign: Cortana-style Presence + Emitter Rescale

## Goal
Replace the current hologram presence with a **Cortana-like** figure (Halo):
feminine humanoid hologram, tall drifting data-lines/particles, cool cyan-blue
emissive body, faint circuitry patterning, slight translucency, elegant idle
motion. Also fix the emitter mount being oversized for the desk.

## Current implementation (file:line evidence)

- `src/scene/boardroom/PresenceAvatar.tsx` — the avatar component:
  - Fallback (`PresenceFallback`, :21-32): sphere (r 0.18 @ y1.54) +
    capsule (0.26×0.7 @ y0.95) — abstract orb+pillar.
  - Loaded path: GLB from scene asset binding `hologram_anchor` (:148) →
    `src/assets/scene/hologram/presence_rig/presence_rig.glb` (80 KB,
    procedurally generated; metadata source:
    `procedural_script_tools_generate_arda_assets_py` — **generator script is
    not in the repo**, so the rig can't be regenerated in place).
  - Materials: emissive cyan body (#a9fbff / #45eaff), projection mask +
    scanline mask textures (`presence_masks/*.png`), additive scan planes
    0.95 × 1.85 (:140-146).
  - Motion: gentle bob (±0.025, pulseRate), slow yaw sway ±0.08 rad,
    scan group rotates at 0.18 rad/s (:118-126).
  - Default placement `[0, 0.18, -0.04]`, scale `1.1` (:36-37).
- `src/scene/boardroom/AgentPresenceOrbit.tsx` — orbit decoration incl. pastel
  support-marker sprites (`scale [0.42, 0.16, 1]`) — style clash to resolve.
- `src/scene/boardroom/AvatarPresenceLayer.tsx:118` — separate procedural
  holographic form (`arda-holographic-avatar-form`, scale 0.95): stacked
  meshes forming an orb/chalice silhouette.
- **Operator-confirmed (2026-08-22): the form visible in-scene is this
  stacked-mesh path — a cone/chalice with an orb on top, floating up and
  down.** The `presence_rig.glb` path in `PresenceAvatar.tsx` is not what
  renders today. The redesign replaces the *visible* stacked-mesh form.
- Emitter zone: `boardroomSpatialLayout.ts:200-207` — size `[1.45, 0.2, 1.45]`,
  y=0.38 → oversized vs. center console (operator-confirmed visually).

## Design direction (Cortana-style)

Silhouette & materials:
- Slender humanoid female figure ~1.7 scene-units tall, standing on emitter.
- Body: translucent cyan-blue (#7df2ff range, opacity ~0.55–0.75) with
  stronger emissive edge glow; keep existing projection/scanline masks as the
  holographic treatment (they already match Cortana's scanline look).
- Surface detail: subtle darker circuit-line texture over limbs/torso — can be
  a second material pass using the existing projection mask, no new assets.

Motion (Cortana mannerisms):
- Weight-shift idle sway instead of pure bob; occasional head-turn pause.
- Data-mote particles drifting upward along the body (replace pastel swatch
  grid with sharp cyan motes/diamond particles matching HUD kit).
- Alert state: hue shift toward warning amber/red via existing
  `presenceVisualState` channels (keep state-driven behavior intact).

## Options for the model

A. **Procedural rebuild (recommended)** — author a new low-poly humanoid rig
   script (Blender headless, pattern after
   `tools/blender/build_boardroom_physical_stage.py`) that outputs
   `presence_cortana.glb` + metadata into
   `src/assets/scene/hologram/presence_cortana/`. Repo-owned, regenerable,
   tiny budget. Restores the lost generator capability.


Recommend A — Blender confirmed installed on this machine (2026-08-22).
Since the visible form is the `AvatarPresenceLayer` stacked-mesh path, option
A replaces that component's geometry with a Blender-authored humanoid GLB
loaded through the existing hologram material stack (projection + scanline
masks, state-driven emissive). The unused `PresenceAvatar.tsx` GLB path is
either retired or unified with the new asset to avoid two competing avatar
implementations (matches "no duplicate authority" hygiene).

## Emitter rescale

- Shrink zone to `[0.9, 0.16, 0.9]`, lower to y≈0.30 in
  `boardroomSpatialLayout.ts`.
- Adjust avatar default scale so figure (~1.7u) rises naturally from the puck;
  update `PresenceAvatar` defaults and any position overrides stored in
  localStorage `arda.boardroom.zone_positions.v1` (note: stale operator
  overrides will mask layout changes — document reset step).

## Acceptance
- Live screenshot: Cortana-style presence rising from a desk-scaled emitter.
- Presence states (idle/listening/thinking/speaking/alert) still drive visuals
  through `presenceState` — no hardcoded moods.
- Scene tests green; asset budget check passes.

## Completion evidence (2026-08-25)

- `presence_form.glb` is loaded by `AvatarPresenceLayer` and sampled by
  `PresenceParticleSystem`; the former opaque primitive fill was removed after
  live inspection showed additive saturation flattening the figure to white.
- The live Tauri HUD now shows a bounded cyan particle humanoid above the
  desk-scaled emitter, with visible particle separation and no white blowout.
- Idle/active assembly and alert tint remain derived from `presenceState` and
  `presenceVisualState`; no visual mood is independently authored.
- Generator metadata now regenerates the canonical `presence_form` binding and
  records the repository-relative generator path.
- Verified: `pnpm exec tsc --noEmit`; 150 Vitest files / 614 tests; boardroom
  asset budget 31,489,592 / 37,748,736 bytes with zero violations. `oxlint`
  exits successfully with pre-existing unrelated warnings.
