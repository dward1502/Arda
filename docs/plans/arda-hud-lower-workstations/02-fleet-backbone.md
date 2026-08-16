---
soterion:
  sigil: "SCROLL"
  role: "workstation_audit"
  owner: "AULE"
  status: "implemented_phase_5_native_acceptance_pending"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Fleet and Backbone workstation audit | owner: AULE | status: Phase 5 implemented, native acceptance pending | reviewed: 2026-08-16

# Lower Monitor 02 — Fleet and Backbone

## Audit status

> Implementation update: Phase 5 replaced the audited generic composition with the canonical topology-first Fleet owner, and Phase 9 retired the disconnected duplicate renderer. See [`FLEET_BACKBONE.md`](FLEET_BACKBONE.md) and [`ORPHAN_RETIREMENT.md`](ORPHAN_RETIREMENT.md). The audit evidence below records the pre-implementation state and remains useful as provenance.

- Investigation date: 2026-08-16.
- Authority standard: current code, persisted slot state, current files, and actual adapter use.
- Documentation is context only until verified against those authorities.
- This was a current-state audit at capture time, not an approved redesign.
- No application code or runtime state was changed by the audit itself.

## Physical surface and route

```text
boardroom.lower.left_inner
  -> view_desk_control_panel
  -> fleet_and_backbone
  -> Fleet And Backbone Workstation
```

The persisted assignment comes from `core/state/arda_boardroom_slots.json`. The active manifest derives from the `fleet_and_backbone` section and requests:

1. `systems`
2. `operations_and_packages`

The focused workstation therefore uses the generic `SceneWorkstation` shell with those two tabs.

## What currently opens

### Tab 1 — Systems

The live tab is `SystemsModule.tsx`, not a dedicated fleet workstation. Its stacked sections are:

1. Fleet health summary.
2. Lane ownership.
3. Lane headroom.
4. Lane fitness.
5. Routable providers.
6. Runtime drift.
7. Manwe live runtime status.
8. Storage pressure.
9. Automation runtime.
10. Setup readiness.
11. Audit readiness.
12. Operator cockpit.
13. Fleet operating plan.
14. Action contracts.

This combines fleet, inference routing, storage, automation, setup, audit, queue, Warden, and operating-plan concerns in one tab. Several belong to other workstation domains and must be cross-referenced later rather than assumed to belong here.

### Tab 2 — Operations and Packages

This tab is another large generic `ModuleCard` assembled in `App.tsx`. It includes:

- critical tools and package registry projections;
- package enablement and runtime activation;
- storage compaction records;
- governance runtime signals;
- output topology and accounting;
- operations flow and action contracts;
- Paperclip alignment and comparative tasks;
- escalation runtime and human-needed actions;
- storage pressure and reclaim candidates;
- further general operations evidence below the inspected region.

It is not fleet-specific. Governance, storage, task, evidence, package, and operator-action material overlaps other workstation domains.

## Declared Fleet and Backbone source contract

The current source map declares:

| Source | Role | Current file state | Generated timestamp | Content consumed by the live two-tab workstation? |
|---|---|---|---|---|
| `core/state/fleet_runtime.json` | Primary | Present, 81,636 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `core/state/fleet_nodes.json` | Primary | Present, 51,386 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `core/state/fleet_models.json` | Primary | Present, 1,874 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `core/state/fleet_health.json` | Primary | Present, 9,847 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `core/state/fleet_hardware.json` | Primary | Present, 12,665 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `core/state/fleet_backbone.json` | Primary | Present, 6,724 bytes | 2026-07-14 | No dedicated bundle field or content adapter found |
| `config/fleet.toml` | Supplemental | Present | Config, no generated time | Listed as evidence; not the displayed fleet model |
| `core/edge/targets.toml` | Supplemental | Missing | — | No |
| `data/prometheus/fleet_control_last.json` | Supplemental | Missing | — | No |
| `data/fleet/informants/local_last.json` | Supplemental | Missing | — | No |

The source-map/provenance layer can list these paths and their availability, but the live fleet metrics are not derived from the six purpose-built fleet JSON files. “Declared source” is therefore not equivalent to “displayed source.”

## Sources actually used for fleet-like values

### Operator runtime projection

Actual path:

```text
core/state/operator_runtime_status.json
  -> bundle.operatorRuntimeStatus
  -> createArdaFleetHealth()
  -> createArdaFleetViewModel()
  -> SystemsModule props
```

Current projection:

- generated: `2026-06-24T10:32:46Z`;
- targets total: present;
- live LLM nodes: 3;
- routable local providers: 3;
- unexpected offline: 6;
- three lane routes;
- three routable-provider records;
- lane headroom and lane fitness records.

This is a snapshot, not a live stream. It is materially older than the investigation date.

### Router projection

Expected path:

```text
core/state/manwe_router.json
```

Current state: missing.

`createArdaFleetViewModel()` prefers provider pressure from this record, then falls back to `operator_runtime_status.json`. The current displayed provider model therefore comes from the fallback projection.

The adapter labels operator runtime as `fresh` and uses the overall bundle load time for its source timestamp rather than the projection's actual `generated_at_utc`. It likewise labels the router source according to object presence. That can make old source data look freshly loaded.

### Runtime drift

Expected path:

```text
core/state/fleet_runtime_drift.json
```

Current state: missing. The Systems tab receives an empty fallback with zero nodes and zero drifted nodes.

### Generic operations projections

The workstation also consumes or attempts to consume unrelated projections including:

- `core/state/storage_pressure.json` — present; generated 2026-07-14;
- `core/state/package_enablement.json` — present; generated 2026-07-14;
- `core/state/package_runtime_activation.json` — present; generated 2026-06-05;
- automation status — expected by the module but the checked `core/state/automation_status.json` path is absent;
- audit readiness — expected by the module but the checked `core/state/audit_readiness.json` path is absent;
- setup readiness, queue/cockpit, governance, output, Paperclip, escalation, and operator-action projections from the shared bundle.

These feeds explain why the workstation is broad but do not make it a coherent Fleet and Backbone surface.

## Live versus snapshot behavior

### Durable bundle

`useArdaBundle()` loads the JSON/JSONL bundle at application start. Its periodic refresh interval defaults to `null`. It refreshes only on retry or explicit refresh/action paths.

### Manwe/Charon live poll

A genuine five-second live poll exists:

```text
useManweLiveSnapshot(5000, viewMode !== 'boardroom')
  -> /healthz or /health
  -> /providers/capabilities
  -> /provider_candidates
```

The focused lower workstation remains in `boardroom` mode, so this poll is disabled for the workstation the lower monitor opens. The Systems tab receives `null`/idle live state there.

### Current classification

| Feed | Classification |
|---|---|
| Six dedicated fleet JSON projections | Snapshot files, loaded at provenance level but content unused by live fleet view |
| Operator runtime | Old snapshot, actively displayed |
| Manwe router projection | Missing; fallback used |
| Runtime drift | Missing; empty fallback displayed |
| Manwe/Charon HTTP snapshot | Real live adapter, disabled in boardroom workstation mode |
| Storage/package projections | Snapshots, actively displayed but domain overlap |
| Fleet controls | Mostly descriptor surfaces; no coherent fleet control center |

## Existing operator actions

The dedicated `FleetViewModel` declares only one action:

- `refresh_fleet_projection` — read-only descriptor.

That descriptor has no command on the view model and the dedicated view that would display it is not selected for `fleet_and_backbone`.

The generic Systems and Operations tabs receive shared system-action descriptors and capability statuses. These can include provider checks, provider-intelligence refresh, setup checks, audit operations, and other system actions. They are not presented as a bounded Fleet and Backbone action model, and their availability depends on adapter capability checks.

No evidence was found that the six dedicated fleet projection files can be edited or refreshed directly from this workstation as one governed flow.

## Historical competing Fleet implementations

At audit time, at least three Fleet-focused UI implementations existed:

1. A `FleetFocusedWorkstationView` defined directly inside `App.tsx`.
2. Another `FleetFocusedWorkstationView` in `scene/workstations/fleetWorkstationView.tsx` with substantially duplicated markup.
3. `components/arda/modules/fleet/FleetWorkstation.tsx`, using a different and wider prop contract.

The routing condition in `App.tsx` selects the inline dedicated view only when `sourceZoneId` is one of:

- `systems_health`;
- `routing_health`;
- `sovereign_world`.

The current lower workstation source zone is `fleet_and_backbone`, so it misses that branch and receives the generic `systems` registry module instead. The other Fleet implementations are not the live lower-monitor path.

This was a concrete overlap/dead-wiring candidate, not a deletion decision at audit time. Phase 5 selected the scene renderer as canonical; Phase 9 later retired the disconnected component renderer after explicit deletion proof.

## Empty, missing, and stale behavior

- Missing operator runtime yields an explicit “Fleet projection unavailable” view model.
- The current operator runtime exists, so the workstation can report health even though the purpose-built fleet files are not consumed.
- Missing `manwe_router.json` is represented in source references, but provider fallback can make the surface still look complete.
- Missing runtime drift becomes zero/empty output, which can be mistaken for “no drift.”
- Old operator runtime is marked `fresh` using bundle load time instead of source generation time.
- Disabled live polling is not prominent to the operator.
- Purpose-built fleet files can be present while their data remains invisible.

## Why this workstation is currently incoherent

1. The source-map contract and displayed data model are different.
2. Six purpose-built projections are present but their contents are unused.
3. An older operator snapshot supplies the visible fleet posture.
4. A missing router projection silently falls back to operator data.
5. The one genuinely live provider feed is disabled in boardroom mode.
6. Three competing Fleet UI implementations exist.
7. The active route bypasses all dedicated Fleet implementations.
8. Systems and Operations tabs mix fleet, routing, governance, storage, setup, audit, package, queue, and human-action material.
9. Missing drift can look like zero drift.
10. Source freshness reflects bundle loading rather than source age.

## Information this domain appears to require

This is a current-domain inventory, not a final layout:

- physical/virtual fleet node inventory and status;
- hardware capability and resource headroom;
- backbone and edge connectivity;
- configured versus observed state;
- expected, intentional, and unexpected offline nodes;
- deployed models and local/runtime availability;
- recovery history and unresolved findings;
- freshness and provenance per source;
- tightly scoped read-only checks and governed recovery controls.

Provider selection, model routing policy, context-window compatibility, and route fitness overlap heavily with Routing and Communications. Their final ownership must wait for Monitor 4's audit.

## Design implications to retain, not implement yet

- The lower instrument should communicate fleet/backbone posture at a glance.
- The focused workstation should not be another full-height systems dashboard.
- Dense node or finding sets should support compact list -> selected detail behavior.
- Hardware/network visualization can differ from the approvals layout used by Governance.
- Live, stale, missing, expected-offline, and unknown states must be visually distinct.
- Fleet inventory and routing policy should not be duplicated across two workstations.

## Cross-reference candidates

Do not resolve these until all records exist:

- Fleet versus Routing ownership of providers, models, routes, and lane fitness.
- Fleet versus Command Core ownership of health summaries and incidents.
- Operations/Packages material versus a general operations or settings surface.
- Storage pressure versus the subsystem that actually owns maintenance.
- Queue/Warden material versus Governance and Command Core.
- The three Fleet UI implementations.
- Purpose-built fleet projections versus the older operator-runtime adapter.

## Evidence anchors

- `core/state/arda_boardroom_slots.json`
- `core/state/arda_source_map.json`
- `apps/arda-hud/src/App.tsx`
- `apps/arda-hud/src/components/arda/modules/SystemsModule.tsx`
- `apps/arda-hud/src/scene/workstations/fleetWorkstationView.tsx`
- `apps/arda-hud/src/scene/workstations/adapters/ardaAdapter.ts`
- `apps/arda-hud/src/components/arda/hooks/useArdaBundle.ts`
- `apps/arda-hud/src/components/arda/hooks/useManweLiveSnapshot.ts`
- `apps/arda-hud/src/lib/manweLive.ts`
- `apps/arda-hud/src/lib/ardaSource.ts`
- `apps/arda-hud/src/lib/ardaBundleTypes.ts`
- `apps/arda-hud/src/lib/settingsLayout.ts`
- `core/state/operator_runtime_status.json`
- the fleet projection files listed above

## Verification required after later approved changes

- Source/adapter unit tests proving every displayed fleet value's source and age.
- Missing/stale/expected-offline fixtures.
- A route test proving the lower Fleet monitor opens the intended dedicated modules.
- Boardroom-mode live-feed behavior tests.
- Action capability tests for every visible control.
- Visual acceptance at the real native workstation size.
- Focused Vitest, TypeScript, and Tauri build gates.

## Phase 10 closeout status

Fleet implementation and canonical renderer contracts are green within the 142-file, 576-test Phase 10 suite, and the optimized Tauri build passed. The disconnected component-level Fleet renderer listed in the baseline audit was retired in Phase 9. Native topology selection, refresh receipt, source/timestamp, missing-state, hidden-polling, keyboard, reduced-motion, screenshot, and frame-rate checks remain blocked because the current release exposed no controllable native window. See [`ACCEPTANCE_MATRIX.md`](ACCEPTANCE_MATRIX.md) and [`VERIFICATION_CLOSEOUT.md`](VERIFICATION_CLOSEOUT.md).
