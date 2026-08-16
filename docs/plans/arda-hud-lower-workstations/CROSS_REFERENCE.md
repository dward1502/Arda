---
soterion:
  sigil: "SCROLL"
  role: "cross_reference_audit"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Lower workstation cross-reference | owner: HERMES | status: active | reviewed: 2026-08-16

# Lower Workstation Cross-Reference

## Scope and evidence rule

This record compares the five audited lower surfaces:

1. [Governance and Guardhouse](01-governance-guardhouse.md)
2. [Fleet and Backbone](02-fleet-backbone.md)
3. [Command Core](03-command-core.md)
4. [Routing and Communications](04-routing-communications.md)
5. [Human, Business, and Personal](05-human-business-personal.md)

It classifies overlap, disconnected data, competing code, stale projections, and provisional information ownership. It does not approve deletion or redesign.

Current source/code/state beats documentation. Older contracts are retained only where live code still confirms them.

## Executive findings

1. Fleet and Routing are genuine UI duplicates: both open the same `Systems` and `Operations and Packages` modules.
2. Governance contains several distinct workflows stacked into one tab, while its second tab only lists source metadata.
3. Command Core is a distinct physical control surface, but its controls mostly navigate and its central screen opens a mismatched Sovereign/Fleet workstation.
4. Human/Business/Personal omits Personal despite loading a useful personal projection.
5. Dedicated Warden and Fleet projections have real producers and tests but no HUD content adapters.
6. Freshness often describes bundle load time or source-family worst state rather than the generation time of the displayed record.
7. Multiple authorities define assignments, panels, roles, and fallback layouts. They disagree in ways visible to the user.
8. Three frontend artifacts are confirmed orphan/deletion candidates; other duplicates need consolidation decisions first.
9. Cross-domain links are needed, but each information class should have one owning workstation.
10. The user’s list/detail example is especially suitable for approvals and business/routing records, but the five workstations do not need one identical shell composition.

## Structural overlap matrix

| Surface | Current modules/behavior | Direct overlap |
|---|---|---|
| Governance | `governance_controls`, `section_focus` | Review/approval content overlaps Planning and business drafts; Section Focus duplicates source metadata |
| Fleet | `systems`, `operations_and_packages` | Exact module duplication with Routing |
| Command Core | fixed animated signal + navigation controls | Links into Planning, Governance, Routing, World; main screen duplicates Fleet/Sovereign content |
| Routing | `systems`, `operations_and_packages` | Exact module duplication with Fleet |
| Human/Business/Personal | `human_realm`, `business` | Plan Shelf overlaps Planning; approvals overlap Governance; Personal omitted |

## Exact duplicate module composition

### Fleet and Routing

Both source zones adapt to:

```text
systems
operations_and_packages
```

Both receive the same preconstructed React nodes from the shared `moduleRegistry`. There is no per-workstation filtering inside either module.

Consequences:

- Fleet shows routing ownership, route actions, headroom, and fitness.
- Routing shows Fleet health, unexpected-offline nodes, storage, setup, audit, tasks, and agents.
- Both show the full generic operations/package card stack.
- The workstation titles differ, but the primary content does not.

This is confirmed overlap, not just shared data.

## Shared data that is not automatically duplication

Some information legitimately supports more than one surface if one owner retains the detail and other surfaces show bounded references:

| Information | Owning detail | Allowed reference elsewhere |
|---|---|---|
| Provider availability | Fleet | Routing can reference availability when evaluating a route |
| Provider/model route assignment | Routing | Fleet can show “currently assigned” on node/provider detail |
| Pending approval | Governance | Domain workstations can show a linked approval state |
| Plan/task ownership | Planning and Queue | Command Core can show attention count; Human can link commitments |
| Source trust/freshness | Cross-cutting metadata | Every domain may display compact provenance without duplicating an evidence workstation |
| Business draft approval | Business record detail | Governance owns the approval decision and receipt |
| Personal priority | Human/Personal | Command Core can encode current attention without reproducing private detail |

Phase 3 implements the cross-cutting source-truth boundary in [`SOURCE_TRUTH.md`](SOURCE_TRUTH.md). Domain previews consume shared `live`, `snapshot`, `projected`, `stale`, `unavailable`, and `missing` states; family-specific adapters may select matching authorities but may not infer authority from unrelated loaded data.

## Provisional ownership map

This map is for the later design pass; it does not prescribe layout.

### Governance and Guardhouse owns

- editable governance weights, thresholds, and permission/policy controls;
- active ruleset and authority chain;
- Guardhouse incidents, blocks, quarantine, edge enforcement, and policy violations;
- approval/review queue;
- selected approval detail, evidence, decision, approvers, and receipt;
- autonomy mutation gates and supervised unlocks.

It should link to the affected Fleet, Routing, Business, or task record rather than duplicate their full detail.

### Fleet and Backbone owns

- node and target inventory;
- node reachability and intentional/unexpected offline state;
- hardware and accelerator inventory;
- installed/available local models by node;
- backbone/mesh topology and node connectivity;
- drift between configured, observed, and runtime state;
- node maintenance and observation receipts.

It should not own route policy, governance approvals, packages, general task lists, or setup consoles.

### Command Core owns

- what requires attention now;
- compact current operating posture;
- tactile entry points to bounded intervention flows;
- intervention receipts and unmistakable blocked/unavailable states.

It should not become another text-heavy workstation or duplicate complete domain lists.

### Routing and Communications owns

- lane-to-provider/model assignment;
- route class, reason, constraints, fallback, and selection history;
- candidate capabilities and compatibility;
- latency, success/failure, headroom, budget/cooldown pressure;
- route observation, proposed change, approved change, and applied receipt as distinct states;
- technical channel/gateway health if Communications remains part of this domain.

It should not own physical node inventory or generic Fleet health.

### Human, Business, and Personal owns

- human priorities, continuity, time, health/energy, family, household, and commitments at an appropriate privacy level;
- relevant notes and readable personal context;
- business opportunities, clients, engagements, commitments, experiments, drafts, receipts, and realized value;
- personal/business attention that should influence daily command.

It should not own the Plan Shelf, generic project queue, or approval decision authority.

## Cross-domain boundary questions still requiring user/design review

1. Whether technical gateway/channel health belongs in Routing or a separate Communications view.
2. Whether Human, Business, and Personal remains one workstation with distinct internal modes or splits into two roles.
3. Whether Command Core opens a minimal “Now” focus surface or only routes to owner workstations.
4. Whether general package/setup/storage maintenance belongs to Fleet, Settings, or a separate maintenance workstation.
5. Whether Evidence/Trust is a dedicated unassigned workstation or only a cross-cutting detail drawer.
6. Whether Planning and Queue remains reachable only from Command Core/other scenes or should occupy a lower physical monitor.

## Source consumption cross-reference

### Produced and displayed

- `operator_runtime_status.json`: displayed by Fleet and Routing through `FleetViewModel`.
- `human_context.json`: displayed by Human Realm, currently empty.
- `business_runtime.json`: displayed by Business, but stale and internally inconsistent with current files.
- `governance_runtime.json`: loaded and partly derived, but its main runtime signals are not in the focused Governance tabs.
- human augmentation and readiness projections: displayed in Governance.

### Produced but not consumed as content by the assigned workstation

#### Fleet projection family

- `fleet_runtime.json`
- `fleet_nodes.json`
- `fleet_models.json`
- `fleet_hardware.json`
- `fleet_health.json`
- `fleet_backbone.json`

These are not unnecessary artifacts. AULË has explicit producer functions for all six in `crates/spine/observability/arda-aule/src/prometheus/core_link/fleet.rs`, and `core_link.rs` invokes them. The HUD lists them as source-map provenance but has no bundle fields/content adapters for them.

#### Warden projection family

- `warden_guardhouse.json`
- `warden_policy_authority.json`
- `warden_edge_contract.json`
- `warden_nightly_doctrine.json`

AULË has producer functions and tests for all four in `core_link/warden.rs`. The first two files are currently missing; the second two exist. The HUD does not parse the existing two into Guardhouse content.

#### Human/Business/Personal

- `personal_runtime.json` is loaded and rendered by the globally registered Personal Growth module but omitted from the assigned combined workstation.
- `hermes_command.json` is loaded/declaratively available but not converted into a coherent Communications view.
- `provider_intelligence.json` drives action status/evidence, not the Routing provider view model.

### Missing but required by current surfaces

- Warden Guardhouse and policy authority projections.
- Manwe router projection.
- Prometheus gate matrix and metrics.
- Arandur request/recommendation/approval queues.
- HADES lifecycle review queue.
- Business company-ops data and projected client files.
- CHRONOS runtime receipt.
- Provider token usage.

Missing does not mean unnecessary. Each must be classified later as:

1. restore producer/runtime;
2. replace with a newer authority;
3. remove from the contract and source map.

## Freshness and truthfulness defects

| Defect | Affected surfaces |
|---|---|
| Bundle load time used as `fresh` reference | Fleet, Routing and other view-model source references |
| Persisted source map declares `ready` despite missing sources | Governance, others |
| Old snapshot still renders populated values | Fleet, Routing, Business, Governance readiness |
| Fallback projection makes missing primary source look complete | Routing, Command Core |
| Static bundle in boardroom mode | All focused lower workstations |
| Live Manwe and runtime pulse explicitly disabled in boardroom | Fleet and Routing especially |
| Source path metadata presented without content adapter | Governance/Guardhouse and Fleet |
| Empty normalized defaults look like observations | Business company operations |

Any later UI must represent `live`, `snapshot`, `derived`, `stale`, `missing`, `disabled`, and `unavailable` separately.

## Assignment and composition authority collisions

Definitions at audit time, from strongest runtime input to fallback/support:

1. `core/state/arda_boardroom_slots.json` — persisted physical assignment and visualization state.
2. source-derived workstation manifests from `arda_source_map.json` plus `resolveWorkstationProfile()`.
3. `FIRST_LEVEL_TERMINALS`/`WORKSTATION_PROFILES` — adapts source panel taxonomy into module IDs.
4. `BOARDROOM_SCENE_SLOT_WORKSTATION_MANIFESTS` — slot-level titles/modules.
5. `SCENE_SLOT_WORKSTATION_TEMPLATES` — viewport fallback/preview manifests.
6. `settingsLayout.ts` `PANEL_LAYOUTS` — global panel and fallback layout.
7. `workstationRoles.ts` — conceptual role contract, mostly type/test authority rather than runtime composition.

Previously confirmed disagreements, now resolved for canonical lower-workstation module composition:

- Governance: source profile uses Governance Controls + Section Focus; settings layout says Governance Controls + Operating Surface.
- Routing: source profile uses Systems + Operations; settings layout says Section Focus + Operations.
- Sovereign World: persisted source map, fallback source map, first-level profile, and settings layout disagree.
- Human/Business/Personal: source panels included Personal Growth while the old module profile omitted it; Phase 1 now retains `personal_growth`.
- Fleet: dedicated source panel IDs collapse to generic Systems + Operations.

Phase 1 established [`workstationComposition.ts`](../../../apps/arda-hud/src/lib/workstationComposition.ts) as the canonical source-zone composition authority. Physical assignment remains owned by `core/state/arda_boardroom_slots.json`; source-map, scene-slot, settings-layout, and utility-manifest paths now delegate to the canonical registry or remain explicitly bounded legacy fallbacks. The complete disposition is recorded in [`COMPOSITION_AUTHORITY.md`](COMPOSITION_AUTHORITY.md).

## Code and file disposition candidates

### Confirmed orphan/deletion candidates

These have no external source import/use in `apps/arda-hud/src`:

1. `apps/arda-hud/src/lib/providerRouting.ts` — two-byte empty file.
2. `apps/arda-hud/src/scene/workstations/fleetWorkstationView.tsx` — duplicate floating layout helpers and Fleet view; no imports found.
3. `apps/arda-hud/src/components/arda/modules/fleet/FleetWorkstation.tsx` — third Fleet-focused implementation; no imports found outside itself.

Deletion is **not yet approved**. Before removal, the implementation pass must run build/tests and confirm no dynamic or tooling reference.

### Active duplicate implementation

- `App.tsx` contains an inline `FleetFocusedWorkstationView` that is actively selected only for `systems_health`, `routing_health`, and `sovereign_world`, not the assigned `fleet_and_backbone` slot.
- `BoardroomViewport.tsx` has a separate `FleetPreviewSurface` for selected preview assignments.

These are not dead, but their responsibilities overlap with the orphan Fleet files and generic Systems module.

### Competing contract code — consolidate, do not simply delete

- `settingsLayout.ts`
- `firstLevelTerminalContracts.ts`
- `boardroomSlotSettings.ts`
- `sceneSlotWorkstationTemplates.ts`
- `workstationRoles.ts`
- source-map workstation manifest derivation in `ardaSource.ts`

Each serves some active or test/fallback path. The goal is one authority plus explicit adapters, not blanket removal.

### Suspicious backup/source clutter outside the current lower audit

Repo-wide command-name search found sibling Tauri source artifacts:

- `src-tauri/src/lib.rs.bak.tailrestore`
- `src-tauri/src/lib.rs.pre-restore`
- `src-tauri/src/lib.rs.messy`

They are not compiled by the normal Rust module path but contain stale command implementations and can confuse searches. They are cleanup candidates for a separate repository-hygiene verification, not part of this no-code audit.

## Confirmed action integration defect

Routing frontend invokes:

```text
run_manwe_provider_intelligence_refresh
arda.manwe_refresh_provider_intelligence
```

The registered Tauri backend exposes:

```text
run_charon_provider_intelligence_refresh
charon.refresh_provider_intelligence
```

The frontend test mocks the frontend name, so it does not verify native registration. This is a real integration mismatch to fix later.

## Information architecture principles

These constraints are carried into the completed [design research](DESIGN_REFERENCES.md) and [phased implementation plan](PLAN.md):

1. One owner per information class; other surfaces link or summarize.
2. Physical lower screens stay near-textless tactical instruments.
3. Focused workstations may use different layouts and interaction systems.
4. Approval, opportunity, route, incident, and node collections should support fast selection and detailed inspection.
5. The user’s left-list/right-detail pattern is appropriate where there are many records, especially Governance approvals.
6. High-level visualization should reveal state and change, not decorate two arbitrary counts.
7. Controls must correspond to real actions; navigation controls must be named as navigation.
8. Every value must carry source, timestamp, and truth classification.
9. Detailed raw paths/hashes belong in evidence detail, not the primary scan surface.
10. No redesign should reintroduce generic cards, identical tab stacks, or text walls across all domains.

## Next gate

The distinct information architectures, Command Core utility-bank direction, existing-source-first policy, and implementation phases are now written. Before editing code:

1. user reviews [the implementation plan](PLAN.md);
2. Phase 0 captures automated and native baseline evidence;
3. Phase 1 composition convergence is implemented and verified; domain restructuring must consume [`COMPOSITION_AUTHORITY.md`](COMPOSITION_AUTHORITY.md);
4. every implementation slice runs focused tests and native acceptance;
5. obsolete candidates remain until their owning replacement is proven.
