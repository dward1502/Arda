---
soterion:
  sigil: "SCROLL"
  role: "audit_index"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 lower-workstation audit index | owner: HERMES | status: active | reviewed: 2026-08-16

# ARDA HUD Lower Workstation Audit

## Purpose

This is the evidence index for understanding every lower boardroom surface before redesigning or removing anything.

The audit was completed monitor-by-monitor. The completed records are now cross-referenced to identify:

- duplicated responsibilities, data, controls, and UI;
- information that belongs on a different workstation;
- declared but unconnected sources;
- loaded but unused projections;
- obsolete code, files, folders, and documentation;
- missing adapters and mutation paths;
- inconsistent names, tabs, roles, and authorities;
- opportunities for deliberately different interaction patterns per workstation.

No source code is changed by this record set. The evidence phase is complete; the approved operator direction, external design research, distinct information architectures, and phased implementation plan are now recorded for review before execution.

## Product Direction Recorded From Operator Review

The focused workstations should blend modern data visualization with a restrained science-fiction instrument style while keeping important information quickly readable.

They do not need identical layouts. A workstation may use top-level tabs when that supports its domain; another may use a different hierarchy. Dense packet families such as Arandur governance items should favor a master-detail interaction: a compact selectable list on the left and the selected packet's reason, consequence, evidence, provenance, and controls on the right.

The lower screen itself remains a compact tactical instrument. Detailed reading and guarded mutation belong in the focused workstation opened from it.

Final information architecture and visual design decisions remain deferred until the operator reviews the completed audits and cross-reference.

## Evidence Policy

Audit date: **2026-08-16**.

Evidence priority:

1. running native Tauri behavior and runtime events;
2. live persisted slot state and authoritative ledgers;
3. currently executed source paths and adapters;
4. focused tests demonstrating current behavior;
5. contracts and product doctrine verified against current code;
6. older plans and descriptive documentation, treated only as leads until reverified.

Age alone does not invalidate a document, but an older document does not override newer executable behavior. Every record must state the source's generated/reviewed date when available and whether the evidence is live, projected, static, missing, stale, or merely declared.

## Physical Lower-Surface Inventory

The executable first-level contract defines four configurable screens and one fixed command core in `apps/arda-hud/src/lib/firstLevelTerminalContracts.ts:3-9`.

| Record | Physical zone | Slot | Live assignment | Current focused modules | Audit state |
|---|---|---|---|---|---|
| 01 | `boardroom.lower.left_wrap` | `view_desk_l` | `governance_guardhouse` | `governance_controls` | [Phase 4 implemented](GOVERNANCE_GUARDHOUSE.md) |
| 02 | `boardroom.lower.left_inner` | `view_desk_control_panel` | `fleet_and_backbone` | `systems`, `operations_and_packages` | [Deep audit recorded](02-fleet-backbone.md) |
| 03 | `boardroom.control.center` | none | fixed `command_core_now` | not slot-configurable | [Deep audit recorded](03-command-core.md) |
| 04 | `boardroom.lower.right_inner` | `view_desk_r` | `routing_and_comms` | `systems`, `operations_and_packages` | [Deep audit recorded](04-routing-communications.md) |
| 05 | `boardroom.lower.right_wrap` | `view_desk_aux` | `human_business_personal` | `human_realm`, `business` | [Deep audit recorded](05-human-business-personal.md) |

The persisted assignment authority is `core/state/arda_boardroom_slots.json`, last updated at the document level on `2026-07-31T00:13:37.927Z`. Several individual lower assignments still carry the epoch timestamp `1970-01-01T00:00:00.000Z`; this is not credible freshness evidence.

## Synthesis and implementation records

| Record | Purpose | State |
|---|---|---|
| [Cross-reference](CROSS_REFERENCE.md) | overlap, dead wiring, competing authorities, data families, and ownership split | complete |
| [Design references](DESIGN_REFERENCES.md) | externally grounded sci-fi interaction research translated into ARDA visual and truth-state rules | complete |
| [Implementation plan](PLAN.md) | composition, Command Core control relocation, source wiring, distinct workstation phases, cleanup, and verification | execution started |
| [Phase 0 baseline](BASELINE.md) | automated, native semantic, performance, source, and interaction baseline with explicit qualification limits | complete with recorded native limitations |
| [Phase 1 composition authority](COMPOSITION_AUTHORITY.md) | canonical lower-workstation module composition, compatibility adapters, and old-authority dispositions | implemented and verified |
| [Phase 2 Command Core controls](COMMAND_CORE_CONTROLS.md) | front-plate command/utility banks, detached-row retirement, callback parity, and native semantic evidence | implemented and verified with recorded native launch limitation |
| [Phase 3 source truth](SOURCE_TRUTH.md) | shared live/snapshot/projected/stale/unavailable/missing states, source labels, and non-color cues | implemented and verified with recorded native visual limitation |
| [Phase 4 Governance + Guardhouse](GOVERNANCE_GUARDHOUSE.md) | posture/source rails, selectable decision index, evidence detail, contextual actions, and decision-pressure instrument | implemented and verified with recorded native visual limitation |

The Command Core now preserves its command bank and owns the existing Settings, Terminal, and Hermes Dashboard launchers in a separate physical utility bank. The detached bottom utility row is retired. Service Health remains owned by Fleet/Backbone and contributes state rather than retaining a duplicate detached button.

Lower instruments now share an explicit source-truth contract. Every matched source carries a visible source name and one of six textual/symbolic truth states; missing and unreadable families fail visibly, and reduced-motion rendering no longer changes source authority.

Governance now uses one focused decision chamber rather than stacked generic modules and a duplicate source-focus tab. Review packets and active governed tasks share a selectable index; evidence, authority, receipt binding, and source truth remain visible before any valid action is dispatched.

## Current Authority Boundaries

Phase 1 converged focused-workstation composition while preserving separate assignment, source declaration, and presentation contracts:

| Layer | Responsibility it currently claims | Last Git change | Current assessment |
|---|---|---:|---|
| `core/state/arda_boardroom_slots.json` | persisted physical slot assignment and module metadata | runtime state | Current persisted assignment authority, but some metadata is default/epoch-dated |
| `lib/workstationComposition.ts` | canonical source-zone titles, module sets, source-panel taxonomy, presentation modes, utility manifests | 2026-08-16 | Canonical focused-workstation composition authority |
| `lib/boardroomSlotSettings.ts` | defaults, role profiles, component IDs, persisted surface layouts | 2026-08-10 | Active assignment/profile parser; serialized module/title fields remain compatibility metadata |
| `lib/firstLevelTerminalContracts.ts` | five physical lower surfaces and source-map adaptation | 2026-08-16 | Physical terminal contract and compatibility adapter over canonical composition |
| `scene/workstations/sceneSlotWorkstationTemplates.ts` | per-slot fallback manifests | 2026-08-16 | Lower templates derive canonical composition; upper ambient/service templates remain bounded fallbacks |
| `lib/settingsLayout.ts` | global module order and legacy/non-workstation panel layouts | 2026-08-16 | Canonical source zones resolve through `workstationComposition.ts` first |
| `scene/workstations/workstationRoles.ts` | conceptual roles and operator questions | 2026-07-16 | Useful intent contract, but not the physical assignment authority |
| `core/state/arda_source_map.json` | section names, declared panels, source inventories, owners, status | 2026-08-09 | Loaded authority for source declarations; its `ready` statuses can contradict missing files |
| `WORKSTATION_CONTRACT.md` | container/presentation behavior | 2026-07-16 | Older, but its core rules still match current architecture where verified |
| `BOARDROOM_CONTRACT.md` | spatial and upper/lower responsibility boundary | 2026-08-09 | Current architectural contract where verified against executable paths |
| `ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` | product doctrine and truthfulness | 2026-08-12 | Newest product authority in this audit |

## Known Cross-Surface Conflicts To Revisit

These are inventory findings, not deletion decisions:

1. **Fleet and Routing currently open the same two generic modules:** `systems` plus `operations_and_packages`. Their source zones differ, but the focused information architecture is duplicated.
2. **Three role systems coexist:** physical roles (`governance_decisions`, `systems_fleet`, etc.), generic workstation roles (`fleet`, `work`, `decisions`, etc.), and source zones (`governance_guardhouse`, `fleet_and_backbone`, etc.).
3. **Work, Knowledge, Evidence, Decisions, and Settings role profiles exist without a one-to-one mapping to the four configurable lower screens.** They may be reusable assignments rather than physical workstations; this must be proven before retaining or removing them.
4. **Scene-slot templates also define upper-monitor workstation modules**, while the current product boundary says upper screens are independent agent canvases. These templates must be audited separately and must not silently determine lower-workstation ownership.
5. **Persisted preview bindings use generic names** such as `<zone>.summary`, `<zone>.health`, and `<zone>.status`; each audit must prove whether an adapter actually resolves them.
6. **The center command core is outside the configurable slot list.** It needs its own record because it may overlap with actions placed inside focused workstations.

## Per-Workstation Audit Template

Each detailed record must contain:

1. physical slot and open-workstation path;
2. current purpose and operator question;
3. actual tabs/sections and shell structure;
4. all declared data sources;
5. all files actually loaded;
6. derived selectors/view models;
7. renderers and visible fields;
8. live event channels and refresh behavior;
9. mutation/action paths and authority checks;
10. missing, stale, unknown, and error behavior;
11. loaded-but-unused and declared-but-missing data;
12. competing definitions and documentation age;
13. current overlap with other workstations;
14. operator-approved intended ownership;
15. questions deferred until cross-reference;
16. tests required before any implementation claim.

## Records

- [01 — Governance and Guardhouse](01-governance-guardhouse.md)
- [02 — Fleet and Backbone](02-fleet-backbone.md)
- [03 — Command Core](03-command-core.md)
- [04 — Routing and Communications](04-routing-communications.md)
- [05 — Human, Business, and Personal](05-human-business-personal.md)
- [Cross-workstation responsibility matrix](CROSS_REFERENCE.md)
