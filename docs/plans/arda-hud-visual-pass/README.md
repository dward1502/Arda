---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "implementation_plan"
  status: "active"
  owner: "HERMES"
  reviewed: "2026-09-07"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-09-07

# HUD Visual Pass — Remaining Work

## Scope and priority

Own HUD visual consistency and maintainable shell structure, not execution,
media acquisition, or Mirromere. Daily usefulness and the
[personal media flow](../PERSONAL_SYSTEM_EXPERIENCE.md) take priority. World View
remains passive; intentional sparse/low-contrast instruments and lower WebGL
apertures are not redesign targets. No new visual language or geometry changes
are authorized merely by this reconciliation.

## Evidence and consolidation

- The [historical WS3b avatar closeout](../../archive/arda-hud-visual-pass/03b-avatar-cortana-redesign.md) records the August 25 native visual pass and tests. Current `AvatarPresenceLayer.tsx` renders `PresenceParticleSystem`, which samples `presence_form`; `boardroomSpatialLayout.ts` retains emitter position `[0, 0.3, 0.22]` and size `[0.9, 0.16, 0.9]`. This is source corroboration, not a fresh native qualification or Mirromere acceptance.
- The [historical WS3a assessment](../../archive/arda-hud-visual-pass/03a-boardroom-workstations-assessment.md) is consolidated here. Its emitter recommendation is addressed above. Its remaining support-marker consistency and occupied-workstation inspection are retained below; no requirement is discarded as a recurring task.
- `src/styles/foundation/tokens.css`, the existing phase-8 visual convergence test, and `boardroomSpatialLayout.ts` remain the design/geometry authorities. August measurements of App.tsx size, token consumers, and accessibility were snapshots, not current counts.

## Operator decisions retained

Sharp `--hud-*` / `--arda-*` machine grammar is primary; rounded glassy tokens
are legacy. New layout work belongs in focused shell/components, not additions
to an App.tsx monolith. `src/styles/` is the styling home. Prefer rem/em for DOM
sizing; unavoidable fixed hairlines/glows use common tokens. Broad accessibility
work remains explicitly deferred, not completed.

## Remaining sequence

1. **WS1 — Design language:** inventory actual soft-radius/surface and non-token
   consumers; converge on existing sharp tokens or document bounded exceptions.
2. **WS2 — Shell structure:** inspect current App.tsx composition, then extract
   remaining header/rail/dock/workstation-host regions without changing behavior.
   Do not repeat already completed extractions merely because old line counts
   were large. Preserve native ownership and media restoration paths.
3. **WS3 — Useful modules:** inspect actual occupied sessions in all five upper
   monitors and four desk-console workstations, plus Control Core. Check spacing,
   hierarchy, empty/loading/failure states, readability and support-marker
   consistency. The current avatar support markers use octahedra; inspect any
   remaining `AgentPresenceOrbit` path before removing or restyling it. Prioritize
   capture, next action, Personal Operations, and review. Use the personal media
   plan's publication/session evidence rather than duplicating its media matrix.
4. **WS4 — Styling consolidation:** inventory px values and out-of-tree styles;
   migrate DOM units and common constants; retire `nightcity.tokens.ts` only after
   its consumers are migrated. Move CSS into `styles/` or justify genuine scoped
   modules. No blanket WebGL-coordinate conversion.

## Dependencies and acceptance

WS1 precedes WS2/WS3; WS4 accompanies each affected module. Do not disturb the
accepted phone-to-HUD path. A media renderer/ownership defect belongs to
[Personal System Experience](../PERSONAL_SYSTEM_EXPERIENCE.md), not a duplicate
visual backlog. Mirromere remains separately held.

For each actual change, run affected Vitest tests, the phase-8 visual contract,
TypeScript checks, and the supported Tauri build. Inspect the running native
surface with genuine assigned content and compare before/after visuals. Preserve
same-session interaction and frame timing. Historical screenshots and new unit
tests alone cannot close native acceptance. Retire this plan after its owned
remaining work is verified; keep evidence in the archive/operations convention.
