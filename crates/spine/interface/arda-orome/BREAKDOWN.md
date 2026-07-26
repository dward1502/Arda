---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  role: "interface_layer"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-25"
---

# arda-orome breakdown

Interface layer for Arda agents: typed comms, routing, provider dispatch, governance recording, intent classification, MCP, service events, context enrichment, edge policy, and runtime integration.

## Current role

`arda-orome` owns the spine's human/agent interface contracts and the bounded provider dispatch boundary. It does not own provider selection strategy or inference policy; those remain Manwe responsibilities.

## Implemented surfaces

- A2H and A2A messages, envelopes, TTL, queueing, and delivery status
- Agent registry and message router with retry, expiry, queueing, and dead-letter behavior
- Three-tier intent classification
- MCP protocol, tools, and governance validation
- Boardroom, council, approval, interruption, completion, comms, and status events
- Mnemosyne-backed context enrichment with bounded async cache
- Provider registry, adapters, streaming sessions, and typed runtime configuration
- Bounded provider timeout/retry orchestration and metrics
- Typed direct/fanout routing with parallel shared-transport fanout
- Fleet communication scope policy with explicit external approval
- Ledger-backed typed task approval and interruption envelopes
- Discord/slash/CLI relay and optional HTTP/SSE surfaces
- Engine-level no-network smoke dispatch via `arda_engine::orome`

## Canonical provider path

| Surface | Location | Responsibility |
|---|---|---|
| Provider types/runtime | `src/provider/runtime.rs` | Provider inventory and observable receipt model |
| Adapter contracts | `src/provider/adapter.rs` | Provider capabilities and adapter errors |
| Streaming | `src/provider/streaming.rs` | Typed chunks, endings, and sessions |
| Orchestration | `src/provider/orchestration.rs` | Timeout, retry, expiry, fanout, fleet policy, and metrics |
| Registry | `src/provider/registry.rs` | Adapter registration and capability lookup |
| Engine package | `crates/engine/src/orome.rs` | Compiled deterministic smoke integration |

## Dispatch invariants

- Every dispatch target must be known to `ProviderRuntime`.
- Fanout is rejected above `DispatchPolicy::max_fanout`.
- Each provider attempt is bounded by `timeout_ms`.
- Retry count is bounded by `max_attempts`; only retryable adapter errors retry.
- Expired requests never reach a transport.
- External fleet scope is rejected by default and can require explicit approval when enabled.
- Attempts, retries, successes, failures, timeouts, fanout targets, and streaming chunk counts are observable.

## Governance invariants

- `GovernanceHooks` reads action-class behavior from `arda_core::GovernanceGates`.
- Observe/record modes map to `PolicySafe`.
- Escalation/independent-receipt modes map to `RequiresOperatorReview`.
- Block mode maps to `PolicyBlocked`.
- Task approval and interruption envelopes include schema version, decision, UTC timestamp, and ledger write path.
- Records are appended through `arda_core::Ledger`; callers do not write ad hoc governance files.

## HUD and plan surfaces

ARDA HUD already consumes both plan authorities:

- `apps/arda-hud/src/lib/ardaSource.ts::derivePlanMap` inventories `docs/plans` and `core/projects/Plans`.
- `apps/arda-hud/src/lib/reviewGateDerivation.ts::getPlanShelf` projects human/core roots and linked plan paths.
- The engine smoke report identifies `provider_metrics`, `human_plan`, and `governance_receipts` as its operator-facing surfaces.

## Test inventory

- 14 crate unit tests
- 5 provider orchestration integration tests
- 2 governance/ledger integration tests
- 1 engine integration test for the compiled Orome smoke path

## Verified commands

- `cargo test -p arda-orome` — 21 passed, 0 failed
- `cargo test -p arda-engine --test orome_smoke` — 1 passed, 0 failed
- `cargo fmt -p arda-orome -p arda-engine -- --check`

## Residual boundaries

Production provider clients remain transport implementations and deployment configuration. They must implement `ProviderTransport` and retain the runtime's timeout, retry, scope, governance, and receipt contracts. Manwe remains the owner of provider selection and inference routing policy.
