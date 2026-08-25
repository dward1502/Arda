---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "plan"
  status: "active"
  owner: "visual-pass"
  last_reviewed: "2026-08-22"
---

# WS3a — Boardroom Desk & Monitor Workstation Visual Assessment

Authority files: `boardroomSpatialLayout.ts` (all geometry), `BOARDROOM_TUNING.md`
(manual tuning guide), `UpperAmbientMonitorScreen.tsx`, `LowerInstrumentScreen.tsx`,
`CommandCoreInstrumentScreen.tsx`, `MonitorSessionWorkstation.tsx`.

## Layout inventory (from `boardroomSpatialLayout.ts`)

### Upper monitor rail (5 slots, `upper_monitor` kind)
| Zone | Slot | Position | Size |
|---|---|---|---|
| boardroom.monitor.left | monitor_1 | [-3.8, 1.48, -0.62] yaw 0.415 | 1.63 × 0.8 |
| boardroom.monitor.center_left | monitor_2 | [-1.9, 1.54, -0.78] yaw 0.218 | 1.63 × 0.8 |
| boardroom.monitor.center | monitor_3 | [0, 1.57, -0.84] yaw 0 | 1.63 × 0.8 |
| boardroom.monitor.center_right | monitor_4 | [1.9, 1.54, -0.78] yaw −0.218 | 1.63 × 0.8 |
| boardroom.monitor.right | monitor_5 | [3.8, 1.48, -0.62] yaw −0.415 | 1.63 × 0.8 |

All identical size/frames — good consistency. Yaw progression is symmetric.
Idle identities are distinct per slot (confirmed visually 2026-08-22).

### Lower desk consoles (5 surfaces + control core + emitter)
| Zone | Slot | Position | Size |
|---|---|---|---|
| Governance Console | view_desk_l | [-3.8, 0.65, 0.05] | 1.58 × 0.04 × 0.72 |
| Systems Console | view_desk_control_panel | [-1.9, 0.67, -0.08] | 1.24 × 0.04 × 0.62 |
| Control Core | center | [0, 0.68, -0.12] | 1.58 × 0.04 × 0.74 |
| Network Console | view_desk_r | [1.9, 0.67, -0.08] | 1.24 × 0.04 × 0.62 |
| Human Console | view_desk_aux | [3.8, 0.65, 0.05] | 1.58 × 0.04 × 0.72 |

Desk base wraps: 5 segments at y=−0.18 spanning x −3.3…+3.3.

## Visual observations (live screenshot)

Working well:
- Upper rail framing of the city window is correct; frames are uniform dark
  with consistent bezels.
- Lower instruments are near-textless, distinct, and legible at seated distance.
- Yaw symmetry reads naturally; outer wraps angle inward correctly.

Issues:
1. **Avatar emitter oversized relative to desk** (operator-reported, confirmed):
   emitter zone size is `[1.45, 0.2, 1.45]` — nearly the full depth of the
   center console footprint (1.72 × 2.08) and wider than any single console.
   The purple pedestal ring dominates the center desk visually.
2. The pastel swatch grid beside the hologram clashes with the sharp neon
   language (see README visual evidence) — this is part of
   `AgentPresenceOrbit` support markers / orbit decoration, not a monitor.
3. Idle upper monitors show abstract patterns only; occupied-state content
   quality can't be assessed without opening sessions — needs a live pass with
   real workstation sessions before sign-off on WS3 module styling.

## Recommended changes

R1. Shrink `boardroom.avatar.emitter` zone to ~`[0.9, 0.16, 0.9]` and/or sink
    it lower (y 0.38 → ~0.30) so it reads as a projector puck, not a stage.
    Verify hologram origin still aligns (avatar rises from emitter per tuning
    guide).
R2. Restyle AgentPresenceOrbit support markers to sharp wireframe/diamond
    vocabulary matching the HUD kit (kill rounded pastel squares).
R3. Live-session pass over all 5 monitor slots + 4 desk consoles with actual
    assignments before WS3 module styling decisions.

## Acceptance
- Geometry edits via `boardroomSpatialLayout.ts` only (per BOARDROOM_TUNING.md).
- Screenshot comparison before/after; scene tests green
  (`vitest src/scene/boardroom`).
