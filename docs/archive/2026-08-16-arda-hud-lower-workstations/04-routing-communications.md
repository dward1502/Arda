---
soterion:
  sigil: "SCROLL"
  role: "workstation_audit"
  owner: "MANWE"
  status: "implemented_phase_6_native_acceptance_pending"
  reviewed: "2026-08-16"
---

> 🜏 Soterion: 📜 Routing and Communications workstation audit | owner: MANWE | status: Phase 6 implemented, native acceptance pending | reviewed: 2026-08-16

# Lower Monitor 04 — Routing and Communications

## Audit status

> Implementation update: Phase 6 replaced the audited generic composition with the canonical directed-flow Routing owner and aligned the frontend refresh path to the registered CHARON command. See [`ROUTING_COMMUNICATIONS.md`](ROUTING_COMMUNICATIONS.md). The audit evidence below records the pre-implementation state and remains useful as provenance.

- Investigation date: 2026-08-16.
- Authority standard: current code, persisted slot state, current projections, adapters, and registered Tauri commands.
- Documentation is context only until verified.
- This was a current-state audit at capture time, not an approved redesign.
- No application code or runtime state was changed by the audit itself.

## Physical surface and route

```text
boardroom.lower.right_inner
  -> view_desk_r
  -> routing_and_comms
  -> Routing + Communications Workstation
```

The active first-level profile and scene-slot template request:

1. `systems`
2. `operations_and_packages`

The opened workstation therefore uses the same two live modules as Fleet and Backbone.

## What currently opens

### Tab 1 — Systems

The tab is the same `SystemsModule.tsx` used by Monitor 2. Its routing-relevant parts are:

- Routing Action Contract;
- Routing Ownership;
- Lane Headroom;
- Lane Fitness;
- Routable Providers;
- Manwe Capability/health panel;
- runtime drift where model/context mismatches are available.

It also displays unrelated Fleet, storage, automation, setup, audit, operator-cockpit, knowledge, task, agent, and operating-plan panels.

### Tab 2 — Operations and Packages

This is the same generic module used by Fleet. It contains package, storage, governance, output, Paperclip, escalation, operator-action, accounting, and maintenance material. It is not a Routing and Communications-specific second tab.

## Declared source contract

Current `routing_and_comms` sources:

| Source | Role | Current state | Generated timestamp | Display behavior |
|---|---|---|---|---|
| `core/state/operator_runtime_status.json` | Primary | Present, 39,624 bytes | 2026-06-24 | Actively supplies routes, providers, headroom, fitness, and health |
| `core/state/manwe_router.json` | Primary | Missing | — | Preferred router projection absent; fallback used |
| `core/state/hermes_command.json` | Supplemental | Present, 45,793 bytes | 2026-07-14 | Declared as evidence but not the source of live routing panels |

Additional action/evidence sources:

| Source | Current state | Generated timestamp | Use |
|---|---|---|---|
| `core/state/provider_intelligence.json` | Present, 10 providers | 2026-07-13 | Refresh action evidence/status; not the current provider view model |
| `core/state/provider_token_usage.json` | Missing | — | Listed by action contract |
| `core/state/chronos_runtime.json` | Missing | — | Receipt target for provider checks |

## Actual data path

```text
core/state/operator_runtime_status.json
  -> bundle.operatorRuntimeStatus
  -> createArdaFleetViewModel()
  -> laneOwnership / laneHeadroom / laneFitness / providers
  -> SystemsModule
```

Despite this being the Routing workstation, there is no separate Routing view model. It reuses `FleetViewModel`.

### Displayed fields

From operator runtime:

- interactive, execution, and background lane routes;
- provider and model per lane;
- route class and reason in the view model, although the compact ownership cards omit the reason text and show route class;
- provider soft caps;
- per-lane provider headroom;
- average latency, success count, and failure count;
- routable provider list and active connections;
- fleet health summary and offline targets.

Current projection characteristics:

- three lane routes;
- three routable-provider records;
- generated 2026-06-24;
- six unexpected offline targets in the associated fleet summary.

The current surface therefore shows an old routing snapshot as though it were current operational routing.

## Router fallback behavior

`createArdaFleetViewModel()` prefers providers from:

```text
bundle.manweRouter.provider_pressure
```

When absent, it falls back to:

```text
bundle.operatorRuntimeStatus.routable_providers
```

Because `core/state/manwe_router.json` is missing, the current provider model comes from the fallback. The router source is listed missing, but the populated fallback can still make the screen appear complete.

The source references mark operator runtime `fresh` using the bundle load timestamp rather than the document's actual 2026-06-24 generation time. This is not a reliable freshness statement.

## Live Manwe/Charon stream

A real five-second adapter exists:

```text
useManweLiveSnapshot(5000, viewMode !== 'boardroom')
```

It queries:

- `/healthz`, falling back to `/health`;
- `/providers/capabilities`;
- `/provider_candidates`.

In Tauri it can use the allowlisted `read_charon_json` backend command. In browser mode it fetches the configured `:7171` endpoint.

The lower workstation remains in `boardroom` mode, so the adapter is disabled there. The Manwe Capability panel therefore does not receive this live snapshot in the actual lower-monitor workstation.

## Existing controls

### Read-only provider checks

`RoutingActionContractPanel` exposes two nominally safe actions:

1. `arda.chronos_run_provider_checks`
2. `arda.manwe_refresh_provider_intelligence`

The contract explicitly states:

```text
Provider reroute: not exposed
route mutation requires a separate approval contract
```

That is an appropriate safety boundary.

### CHRONOS provider checks

Frontend command:

```text
run_chronos_provider_checks
```

The Tauri backend registers this command. It runs the `arda-chronos` provider checks and writes/reads `core/state/chronos_runtime.json`. This path exists in source, although it was not executed during this no-mutation audit.

### Provider-intelligence refresh mismatch

Frontend `systemActionBus.ts` invokes:

```text
run_manwe_provider_intelligence_refresh
```

with action ID:

```text
arda.manwe_refresh_provider_intelligence
```

The Tauri backend instead registers:

```text
run_charon_provider_intelligence_refresh
```

and its handler accepts:

```text
charon.refresh_provider_intelligence
```

No registered `run_manwe_provider_intelligence_refresh` command was found. The frontend unit test mocks the unregistered frontend name, so it does not prove the Tauri integration works. This action is currently an end-to-end wiring defect.

### Route Selector button

The center console's ROUTE button only opens this workstation. It does not perform route selection. No governed route-mutation control is exposed here.

## Communications content

The section is titled “Routing and Communications,” and `hermes_command.json` contains communication/subcomponent information. However, the live tabs are dominated by provider routing and generic system operations. No dedicated communications surface was found for:

- user-facing channel health;
- gateway/session connectivity;
- message delivery state;
- agent communication interruptions;
- inbox/event routing;
- current communications incidents.

The first-level source panel IDs mention `boardroom`, `inference_router`, and `interrupts`, but the adapted module pair does not preserve those as distinct views.

## Current truth classification

| Feed/control | Classification |
|---|---|
| Lane routes/headroom/fitness | Old snapshot, actively displayed |
| Providers | Fallback snapshot from operator runtime |
| `manwe_router.json` | Missing |
| Live Manwe/Charon API | Real adapter, disabled in boardroom mode |
| Provider intelligence | Snapshot present; not the displayed provider model |
| Hermes command/communications | Snapshot present; mostly unused by this workstation |
| CHRONOS check | Registered read-only action, not run in this audit |
| Provider-intelligence refresh | Frontend/backend command and action-ID mismatch |
| Route mutation | Intentionally not exposed |
| Communications controls/data | No coherent dedicated surface found |

## Why Fleet and Routing currently look duplicated

1. Both lower slots request exactly `systems` and `operations_and_packages`.
2. Both receive the same module instances from the shared registry.
3. Both use `FleetViewModel` for health, providers, routes, and lane metrics.
4. Both show storage, setup, audit, queue, and general operations material.
5. Routing has no dedicated view model or focused workstation composition.
6. The declared `boardroom`, `inference_router`, and `interrupts` source panels collapse into the generic pair.

## Intended information responsibility to retain

This domain appears to require:

- current lane-to-provider/model ownership;
- route reasons and policy class;
- provider capabilities and compatibility receipts;
- latency, success/failure, headroom, cooldown, and budget pressure;
- fallback and reroute history;
- communications/gateway/channel health if “Communications” remains in scope;
- clear observation versus selection versus governed mutation boundaries;
- fresh/stale/missing/disabled source truth.

Hardware/node inventory and physical connectivity should likely remain with Fleet. Final ownership waits for cross-reference.

## Interaction implications to retain, not implement yet

- A route/provider list can select a provider, lane, or incident and open detail beside it.
- Current route, candidate route, and approved route must be distinct.
- Live capability receipts should not be replaced by old snapshot labels.
- A route selector must either perform a real governed selection flow or be named as navigation only.
- Communications may need a structurally different view from inference routing rather than one shared card stack.

## Cross-reference candidates

Do not resolve until all records exist:

- Provider/model inventory with Fleet.
- Route mutation approval with Governance.
- Interrupts and urgent routing state with Command Core.
- Hermes communications data with agent canvases and Human/Personal communications.
- Generic Operations and Packages duplication.
- MANWE versus Charon naming and command ownership.
- Source profile `routing_and_providers` inside the panel versus section ID `routing_and_comms`.

## Evidence anchors

- `core/state/arda_boardroom_slots.json`
- `core/state/arda_source_map.json`
- `apps/arda-hud/src/App.tsx`
- `apps/arda-hud/src/components/arda/modules/SystemsModule.tsx`
- `apps/arda-hud/src/components/arda/modules/systems/RoutingOwnershipPanel.tsx`
- `apps/arda-hud/src/components/arda/modules/systems/RoutingActionContractPanel.tsx`
- `apps/arda-hud/src/scene/workstations/adapters/ardaAdapter.ts`
- `apps/arda-hud/src/components/arda/hooks/useManweLiveSnapshot.ts`
- `apps/arda-hud/src/lib/manweLive.ts`
- `apps/arda-hud/src/lib/systemActionBus.ts`
- `apps/arda-hud/src-tauri/src/lib.rs`
- source files listed above

## Verification required after later approved changes

- A dedicated route/communications view-model test suite.
- Actual source timestamps carried through freshness UI.
- Boardroom-mode live-feed tests.
- Tauri integration tests using registered command names and accepted action IDs.
- Missing router/fallback fixtures.
- Proof that any future mutation is separately governed and receipted.
- Native list/detail and visual acceptance tests.
- Focused Vitest, TypeScript, Rust, and Tauri build gates.

## Phase 10 closeout status

Routing implementation, CHARON command-registration, and focused contracts are green within the 142-file, 576-test Phase 10 suite, and the optimized Tauri build passed. Native lane/provider selection, provider refresh receipt, source/timestamp, missing-state, hidden-polling, keyboard, reduced-motion, screenshot, and frame-rate checks remain blocked because the current release exposed no controllable native window. See [`ACCEPTANCE_MATRIX.md`](ACCEPTANCE_MATRIX.md) and [`VERIFICATION_CLOSEOUT.md`](VERIFICATION_CLOSEOUT.md).
