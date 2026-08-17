---
soterion:
  sigil: "CONVERGE"
  role: "workstation_composition_authority"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: ⟁ Phase 1 converges lower-workstation module composition without changing persisted physical assignment authority.

# Phase 1 — Workstation Composition Authority

## Result

`apps/arda-hud/src/lib/workstationComposition.ts` is the canonical authority for workstation titles, module composition, accepted source-panel taxonomy, presentation modes, and static Settings/Hermes manifests.

`core/state/arda_boardroom_slots.json` remains the authority for which source zone occupies each physical scene slot. Phase 1 does not edit that runtime document or the user-owned queue projections.

## Canonical lower compositions

| Physical slot | Source zone | Canonical modules |
|---|---|---|
| `view_desk_l` | `governance_guardhouse` | `governance_controls`, `section_focus` |
| `view_desk_control_panel` | `fleet_and_backbone` | `systems`, `operations_and_packages` |
| `view_desk_r` | `routing_and_comms` | `systems`, `operations_and_packages` |
| `view_desk_aux` | `human_business_personal` | `human_realm`, `business`, `personal_growth` |

The Personal module is now retained instead of being discarded while adapting the Human/Business/Personal source taxonomy.

## Previous authorities and disposition

| Previous source | Phase 1 disposition |
|---|---|
| `core/state/arda_boardroom_slots.json` | Retained as physical assignment, visualization, and persisted profile authority. Persisted module/title fields remain compatibility metadata rather than focused-workstation composition authority. |
| `ardaSource.ts` manifest derivation | Retained as the source-map adapter. Its `resolveWorkstationProfile()` dependency now delegates to the canonical composition registry and preserves rejected source-panel evidence. |
| `firstLevelTerminalContracts.ts` | Retained for the five physical terminal identities and as a compatibility adapter over the canonical registry. Its private parallel profile table was removed. |
| `sceneSlotWorkstationTemplates.ts` | Retained as the scene-slot fallback boundary. All four configurable lower templates now derive title, modules, and presentation modes from the canonical source-zone composition. Upper-monitor templates remain separate because they are ambient/service fallbacks rather than the audited lower workstations. |
| `settingsLayout.ts` | Retained for global module order and legacy/non-workstation panel fallbacks. Canonical workstation source-zone IDs resolve through `workstationComposition.ts` first. |
| `ardaSurfaces.ts` | Retained as the lookup boundary. Duplicated Settings and Hermes manifest literals were replaced by canonical static-manifest lookups. |
| `workstationRoles.ts` | Retained as conceptual operator-role vocabulary. It does not own source-zone module composition. |

No suspect file was deleted. Phase 9 still owns retirement after consumer and ownership proof.

Phase 9 later retired only the two proven orphans, retained the now-imported canonical Fleet scene renderer, and documented the evidence in [`ORPHAN_RETIREMENT.md`](ORPHAN_RETIREMENT.md).

## Contract preservation

- `FIRST_LEVEL_TERMINALS` still defines four configurable desks and fixed `boardroom.control.center` / `command_core_now`.
- `resolveWorkstationProfile()` retains its original return shape for existing consumers.
- Settings keeps `settings_workstation` and `settings_workstation_entry`.
- Hermes keeps `hermes_dashboard_workstation` and `hermes_dashboard_entry`.
- Unknown source-panel labels remain visible through `rejectedPanelIds`; they are not silently treated as modules.
- Persisted physical assignments and visualization selections are unchanged.

## Verification

- RED: the new authority test initially failed because `workstationComposition.ts` did not exist.
- GREEN: full Vitest suite — **135 files, 529 tests passed**.
- TypeScript/Vite production build — passed; Vite retained its pre-existing mixed static/dynamic import advisory for `boardroomSlotSettings.ts`.
- Native AT-SPI acceptance — **96 nodes**, **36 named semantic controls**, no duplicate meaningful names.
- Settings, Hermes CLI, and Hermes Dashboard semantic controls remained discoverable.
- `git diff --check` — passed.

Evidence:

- [`evidence/phase1-native-atspi-20260816.json`](evidence/phase1-native-atspi-20260816.json)
- `workstationComposition.test.ts`
- `firstLevelTerminalContracts.test.ts`
- `sceneSlotWorkstationTemplates.test.ts`

## Deferred

Phase 1 deliberately does not:

- relocate Command Core controls;
- remove the bottom utility row;
- redesign workstation shells;
- add Fleet/Warden adapters;
- delete legacy fallback tables whose non-workstation consumers remain active.

Those changes remain ordered behind this authority boundary in [`PLAN.md`](PLAN.md).
