# arda-orome stabilization plan

Crate: `crates/spine/interface/arda-orome`
State: completed
Reviewed: 2026-07-27

## Purpose

This file records the completed source-tree stabilization decision. Current truth and command evidence live in `STATUS.md`; future capability expansion belongs in a reviewed product or owning-crate plan.

## Completed decisions

### P0 — source classification

- Preserved the resident service, MCP, context, resident protocol, and Discord contract sources behind the explicit opt-in `service-runtime` feature.
- Added `service/provider_compat.rs` to adapt the historical service API to current bounded provider orchestration.
- Retired six unsupported/no-consumer files: `contract.rs`, `edge.rs`, `formatter.rs`, `relay.rs`, `serenity_bot.rs`, and `slash.rs`.
- Result: every Rust file is production-compiled, test-only, generated, or absent; unwired count is zero.

### P1 — feature contract

- Removed the no-op default `http` feature and unused `axum`, `tower`, and `tokio-stream` dependencies.
- Added `service-runtime` with only the dependencies required by its compiled closure.
- Kept default behavior minimal and backward-compatible for direct consumers.

### P1 — manifest and API boundary

- Removed dependencies used only by retired files.
- `intent` and `registry` compile for tests and `service-runtime`.
- `message_retry_expiry`, `router`, and `provider/tests.rs` remain unit-test-only.
- The service feature is public and testable; its compatibility path remains no-network.

### P2 — concrete transport and fleet policy

- Added opt-in `HttpJsonTransport` with redirect denial, bounded response reads, explicit provider receipts, and optional stream chunks.
- Added `TransportOutcome` and projected provider message IDs into `DispatchReceipt` and outbound records.
- Made `Local` the sole default fleet scope.
- Required `TrustedFleet` targets to be explicitly allowlisted by provider ID.
- Proved allowed and denied trusted-fleet paths over a real loopback TCP endpoint.

## Acceptance result

All producer, strict Clippy, rustdoc, engine, Manwe gRPC, and Aule full-CLI gates listed in `STATUS.md` pass.

## Future proposals

No crate-owned implementation task remains from the HERMES plan. Additional provider protocols, credential lifecycle, remote deployment, or outpost topology require a new owning-system plan and concrete consumer.
