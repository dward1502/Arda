---
soterion:
  sigil: "FORGE"
  role: "phase_record"
  owner: "HERMES"
  status: "implemented"
  reviewed: "2026-08-16"
---

# Phase 6 — Routing + Communications Flow Workstation

> 🜏 Soterion: Routing now owns CHARON lane/provider/model flow, route fitness and budget evidence, communication receipts, and read-only provider checks. Fleet remains the sole owner of Service Health.

## Scope

Phase 6 replaces Routing's generic `Systems` and `Operations and Packages` stack with one directed-flow focused workstation. It promotes existing CHARON/MANWË/operator projections through a typed Routing view model; it does not create a new routing producer or mutation authority.

Implemented surfaces:

- canonical `routing_and_comms` `systems` module rendered by `RoutingFocusedWorkstationView`;
- semantic lane selector and selected-route detail;
- provider health, access tier, model count, active-connection, headroom, fitness, and route-reason evidence;
- communication-path receipts derived only from loaded Hermes message/gateway projections;
- read-only CHRONOS provider checks and CHARON provider-intelligence refresh actions;
- compact lower routing instrument driven by provider/lane pressure rather than Fleet node health.

## Focused information architecture

The workstation has five bounded regions:

1. route posture and budget pressure;
2. directed lane-to-provider flow selector;
3. selected route detail for provider, model, class, reason, capacity, latency, successes, and failures;
4. provider and communication fields;
5. read-only actions plus independent source truth.

Fleet hardware, package inventory, setup, audit, and Service Health do not render in Routing. `routing_and_comms` resolves to one focused `systems` owner module while Fleet retains its separate topology implementation.

## Source truth

| Family | Canonical source | Rendering contract |
|---|---|---|
| operator route runtime | `core/state/operator_runtime_status.json` | lane ownership, headroom, fitness, recent route outcomes, and budget pressure |
| CHARON router | `core/state/manwe_router.json` | snapshot/stale/missing/unavailable state; does not override operator evidence when absent |
| provider intelligence | `core/state/provider_intelligence.json` | provider metadata with independent source state |
| provider token usage | `core/state/provider_token_usage.json` | independent source state; no invented consumption data |
| CHRONOS runtime | `core/state/chronos_runtime.json` | provider-check receipt state |
| Hermes communications | loaded message/gateway receipt projections | `projected` only when records exist; otherwise `unavailable` |

Persisted projections are labelled snapshot/projected rather than live. Missing or disconnected communication data never generates activity.

## Native command repair and action authority

The frontend action ID is now `charon.refresh_provider_intelligence` and invokes the registered Tauri command `run_charon_provider_intelligence_refresh`. The prior frontend-only `run_manwe_provider_intelligence_refresh` mismatch is removed.

A registration-aware contract test verifies all three links together:

- frontend system-action descriptor;
- backend accepted action ID;
- command presence in the Tauri `generate_handler!` registration.

`arda.chronos_run_provider_checks` remains read-only. No provider reroute or route mutation callback is invented. A dispatched refresh remains pending until the native result supplies its receipt/result path; the workstation does not show route changes as completed from button intent alone.

## Tests and verification

Phase 6 adds contracts for:

- Routing view-model derivation and independent source truth;
- lane selection and selected-route evidence;
- communication unavailable/projection behavior;
- exclusion of Fleet and package content;
- action dispatch through the registered CHARON command;
- canonical Routing composition;
- keyboard/native-button semantics and selection fallback.

Verification on 2026-08-16:

- focused Routing/action/composition tests: 42 passed;
- standalone TypeScript project build: passed;
- full HUD Vitest suite: 138 files, 562 tests passed;
- production frontend build: passed; 2,617 modules transformed;
- HUD lint: passed with pre-existing warnings only;
- Tauri Rust library tests: 69 passed, 0 failed, 2 Chromium-dependent tests ignored;
- documentation local-link audit: 64 links checked, 0 broken;
- native desktop enumeration: zero windows; no screenshot or pointer-interaction claim made.

Evidence logs:

- [`evidence/phase6-vitest-20260816.log`](evidence/phase6-vitest-20260816.log)
- [`evidence/phase6-build-20260816.log`](evidence/phase6-build-20260816.log)
- [`evidence/phase6-links-20260816.md`](evidence/phase6-links-20260816.md)

## Qualification

No fresh native screenshot, native pointer interaction, or newly launched Tauri binary is claimed. Native command registration and Rust behavior are verified by source-aware frontend contracts plus the Tauri library suite; visual acceptance remains limited to deterministic semantic component tests and the production build in this environment.
