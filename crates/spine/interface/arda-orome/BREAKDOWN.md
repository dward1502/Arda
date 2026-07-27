---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "interface_implementation_map"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-26"
---

> 🜏 Soterion: ⟁ interface_implementation_map | owner: HADES | status: active | reviewed: 2026-07-26

# arda-orome breakdown

Canonical implementation map for the Arda communication and provider interface crate.

## Where it lives

- Crate: `crates/spine/interface/arda-orome`
- Library root: `src/lib.rs`
- Provider implementation: `src/provider/`
- Protobuf sources: `proto/`
- Generated gRPC Rust: `src/grpc/`
- Integration tests: `tests/`

## Build topology

The source inventory contains 54 Rust files:

| Class | Count | Meaning |
|---|---:|---|
| Production-compiled | 14 | Reachable from `src/lib.rs`, including generated gRPC files |
| Unit-test-only | 5 | Reachable only under `cfg(test)` |
| Unwired | 35 | Not reachable from the crate root in production or tests |

Cargo success applies only to the first two classes. File presence is not implementation evidence.

## Production modules

| File/module | Responsibility |
|---|---|
| `src/lib.rs` | Public module declarations and selected crate-root re-exports |
| `src/comm.rs` | A2H protocol, queue, channel, priority, attachment, and response types |
| `src/governance.rs` | Policy-to-ledger approval and interruption hooks |
| `src/grpc.rs` | Generated tonic surface re-exports |
| `src/grpc/arda.orome.health_model.rs` | Generated health-model client/server/messages |
| `src/grpc/arda.orome.route_governance.rs` | Generated route-governance client/server/messages |
| `src/message.rs` | A2A messages, TTL, threads, signatures, and hops |
| `src/provider/adapter.rs` | Adapter contract, capability description, and adapter errors |
| `src/provider/orchestration.rs` | Timeout/retry, expiry, routing, fanout, fleet policy, and metrics |
| `src/provider/registry.rs` | Adapter registration and capability lookup |
| `src/provider/runtime.rs` | Provider inventory, configuration, and dispatch receipt types |
| `src/provider/streaming.rs` | Streaming events, sessions, and accounting surface |
| `src/provider/mod.rs` | Provider public exports and module wiring |
| `src/types.rs` | Shared boardroom, council, approval, interruption, and operator schemas |

## Unit-test-only modules

- `src/intent.rs`
- `src/message_retry_expiry.rs`
- `src/registry.rs`
- `src/router.rs`
- `src/provider/tests.rs`

The first four are declared behind `#[cfg(test)]` in `lib.rs`; provider tests are declared behind
`#[cfg(test)]` in `provider/mod.rs`. External and integration-test consumers cannot import them.

## Unwired source inventory

| Group | Unwired files |
|---|---|
| Root interface/runtime | `agent.rs`, `context_cache.rs`, `context_enrichment.rs`, `contract.rs`, `edge.rs`, `mnemosyne_integration.rs`, `protocol.rs` |
| Discord/presentation | `discord_health.rs`, `discord_safe_message.rs`, `formatter.rs`, `relay.rs`, `serenity_bot.rs`, `slash.rs` |
| MCP | `mcp/browser.rs`, `channel.rs`, `external_sources.rs`, `mod.rs`, `protocol.rs`, `server.rs`, `tools.rs` |
| Service root | `service.rs` |
| Service children | `classification.rs`, `comms_event.rs`, `council.rs`, `decision.rs`, `inbound.rs`, `interrupts.rs`, `outbound.rs`, `queue_state.rs`, `runtime.rs`, `semantic_channel.rs`, `status.rs`, `subagent_completion.rs`, `support.rs`, `task_approval.rs` |

None of these files should be described as a supported runtime surface until it is deliberately
wired and passes producer/consumer gates. `src/service/README.md` and `src/service/INDEX.md` record
the service subtree's current unwired status.

## Provider dispatch invariants

- Every dispatch target is known to `ProviderRuntime`.
- Fanout cannot exceed `DispatchPolicy::max_fanout`.
- Each attempt is bounded by `timeout_ms`.
- Retry count is bounded by `max_attempts`, and only retryable errors retry.
- Expired requests do not reach transport.
- External fleet dispatch is denied by default and may require explicit approval.
- Attempts, retries, outcomes, timeouts, fanout targets, and streaming chunks are observable.
- `ManualTransport` is deterministic and no-network.

## Governance invariants

- `GovernanceHooks` reads behavior from `arda_core::GovernanceGates`.
- Observe/record maps to `PolicySafe`.
- Escalation/independent receipts maps to `RequiresOperatorReview`.
- Block-on-fail maps to `PolicyBlocked`.
- Typed records include schema version, decision, UTC timestamp, and ledger path.
- Records are appended through `arda_core::Ledger`.

## Feature and generation contract

- Default feature: `http`.
- Current compiled use of `http`: none; it only activates optional dependencies.
- `build.rs` compiles `proto/health_model.proto` and `proto/route_governance.proto` into
  source-managed files under `src/grpc/`.
- `DOCS_RS` skips protobuf generation, so checked-in generated sources remain required for docs.

## Direct consumers

| Consumer | Feature/path | Use |
|---|---|---|
| `arda-engine` | default dependency | Provider type re-exports and manual smoke dispatch |
| Manwe | `grpc` | Health-model and route-governance gRPC implementation |
| `arda-aule` | `full-cli` | A2H messages, priority, and response action |

## Change boundaries

- Manwe retains provider/model-selection and inference-routing authority.
- `src/service.rs` is the only canonical service root; do not recreate a parallel
  `src/service/mod.rs`.
- Do not make unwired source public by adding broad module declarations without resolving the
  service dependency closure and adding contract tests.
- Do not prune dependencies until each unwired source group has a wire, migrate, or retire decision.
- Preserve gRPC wire compatibility or coordinate Manwe changes in the same batch.
- Preserve provider dispatch bounds and governance receipt semantics for engine consumers.

## Verification

Current command evidence and remaining stability findings live in `STATUS.md`. Active resolution
work and acceptance criteria live in `PLAN.md`.
