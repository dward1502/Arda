---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "implementation_plan"
  status: "active"
  owner: "HERMES"
  reviewed: "2026-09-15"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-09-15

# HUD Visual Pass — Remaining Work

## Scope and retained evidence

Own visual consistency and maintainable HUD shell composition, not execution,
media acquisition or Mirromere. [Personal media usefulness](../PERSONAL_SYSTEM_EXPERIENCE.md)
takes priority. World View remains passive; intentional sparse/low-contrast
instruments and lower WebGL apertures are not redesign targets. No new visual
language or geometry change follows from this docs reconciliation.

The [WS3b avatar record](../../archive/arda-hud-visual-pass/03b-avatar-cortana-redesign.md)
retains the historical native closeout. The [WS3a assessment](../../archive/arda-hud-visual-pass/03a-boardroom-workstations-assessment.md)
retains occupied-workstation/support-marker gaps. No fresh native acceptance is
claimed. Old App.tsx line counts and token-consumer counts are not current backlog.

## Design constraints

Sharp `--hud-*` / `--arda-*` grammar, `src/styles/foundation/tokens.css`, the existing
phase-8 visual contract and `boardroomSpatialLayout.ts` remain authorities. Rounded
glassy tokens are legacy. Use focused shell/components rather than expanding
App.tsx; prefer rem/em for DOM sizing with shared tokens for fixed hairlines/glows.
Do not convert WebGL coordinates indiscriminately. Broad accessibility remains
explicitly deferred, not silently completed.

## Work checklist

- [ ] WS1 — Inventory remaining soft-radius/surface and non-token consumers; converge
  on existing sharp tokens or record a bounded justified exception. Do not restyle
  accepted surfaces based solely on an old assessment.
- [ ] WS2 — Inspect current App.tsx composition and extract only still-coupled header/
  rail/dock/workstation-host regions without behavioral change. Preserve ownership,
  session restoration and media paths; do not repeat completed extractions.
- [ ] WS3 — Inspect genuinely occupied sessions on all five upper monitors, four
  desk-console workstations and Control Core: spacing, hierarchy, loading/empty/
  failure states, readability and support-marker consistency. Prioritize capture,
  next action, Personal Operations and review. Inspect the actual `AgentPresenceOrbit`
  consumer before removal; do not reopen completed avatar geometry work.
- [ ] WS4 — Inventory DOM px/out-of-tree CSS; consolidate common constants and styles
  into `src/styles/` or justify scoped modules. Retire `nightcity.tokens.ts` only after
  its actual consumers migrate. Accompany each affected module rather than blanket edits.

## Native acceptance gate

- [ ] H1 — WS1–WS4 changes have affected Vitest/phase-8 contract and TypeScript checks,
  supported Tauri build evidence and before/after native inspection.
- [ ] H2 — Genuine assigned content remains readable with correct source/session
  interaction, ownership, loading/error states and no frame-timing regression.
- [ ] H3 — Operator accepts the residual visual consistency; retain the accepted
  phone-to-HUD flow and passive World View without adding workflow controls there.

Exit gate: residual styling/shell changes and their native behavior are verified;
unit tests and historical screenshots alone do not close it. WS1 precedes WS2/WS3,
WS4 accompanies affected work. Media renderer/ownership defects route to the media
owner, not a duplicate matrix here. Retire after H1–H3 pass; archive evidence rather
than keeping another visual planning library. Mirromere remains separately held.
