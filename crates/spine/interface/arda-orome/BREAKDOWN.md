---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  code_point: "U+27C1"
  role: "interface_implementation_map"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-27"
---

# arda-orome breakdown

Canonical implementation map for `crates/spine/interface/arda-orome`.

## Build topology

The source inventory contains 50 Rust files.

| All-feature class | Count | Meaning |
|---|---:|---|
| Production-compiled | 47 | Reachable from `src/lib.rs`, including generated gRPC and `service-runtime` modules |
| Unit-test-only | 3 | `message_retry_expiry`, `router`, and `provider/tests.rs` |
| Unwired | 0 | No Rust source is left outside a declared production/test/generated boundary |

The default build keeps the stable six-family interface surface. `service-runtime` adds the preserved resident-service closure explicitly.

## Default modules

- `comm.rs`, `governance.rs`, `grpc.rs`, generated `grpc/*.rs`, `message.rs`, and `types.rs`;
- `provider/{adapter,orchestration,registry,runtime,streaming}.rs` and `provider/mod.rs`.

## Opt-in `service-runtime` modules

- Root/runtime: `agent.rs`, `context_cache.rs`, `context_enrichment.rs`, `mnemosyne_integration.rs`, `protocol.rs`, and `provider/http.rs`;
- Discord contracts: `discord_health.rs`, `discord_safe_message.rs`;
- MCP: all seven files under `src/mcp/`;
- service: `service.rs`, its 14 established children, and `service/provider_compat.rs`;
- support promoted from the default test boundary: `intent.rs` and `registry.rs`.

`provider_compat.rs` adapts the preserved service API to current provider orchestration. Resident-service compatibility dispatch is no-network `ManualTransport`; polling returns no inbound messages and providers remain offline without health evidence. `provider/http.rs` separately supplies policy-gated live HTTP JSON dispatch with bounded responses and provider-message receipt proof.

## Retired residue

The following unsupported files were deleted after module-graph, consumer, and history review:

- `contract.rs` — unused generated-root shim;
- `edge.rs` — superseded edge/outpost model with no consumer;
- `formatter.rs`, `relay.rs`, `serenity_bot.rs`, `slash.rs` — unattached presentation and Serenity-era runtime residue.

## Provider invariants

- targets are configured;
- fanout, attempts, and timeout are bounded;
- expired requests never reach transport;
- only local dispatch is allowed by default;
- trusted-fleet dispatch requires an allowed scope and an explicit target-provider allowlist;
- external dispatch is denied unless policy and approval permit it;
- attempts, retries, outcomes, timeouts, targets, and stream chunks are observable;
- service-owned governance, memory, and resource evidence is rooted in the configured service project, with explicit environment overrides retained;
- `ManualTransport` never proves live delivery;
- HTTP success without a non-empty provider message ID is not delivery proof.

## Features and generation

- Default features: none.
- `service-runtime`: resident service/MCP/context/Discord-contract dependency closure.
- `build.rs` generates both protobuf surfaces into `src/grpc/`; checked-in generated sources support docs builds.

## Consumers and boundaries

- `arda-engine` consumes provider contracts.
- Manwe consumes gRPC contracts and retains routing/model authority.
- `arda-aule` consumes A2H contracts.
- `service-runtime` is preserved and tested migration input, not a daemon-lifecycle or credential owner.
