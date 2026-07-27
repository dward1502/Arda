# arda-orome ownership

Crate: `crates/spine/interface/arda-orome`
Owner: HADES / interface layer
Status: active
Reviewed: 2026-07-26

## This crate owns

- A2H/A2A message and envelope contracts exposed from the crate root.
- Provider adapter, registry, dispatch, fleet-scope, streaming, metrics, and receipt contracts.
- Health-model and route-governance protobuf definitions and generated Rust surfaces.
- Typed interface/event payload schemas.
- Ledger-backed task-approval and interruption record creation.

## This crate does not own

- Provider or model selection, route fitness, or inference policy; Manwe owns those decisions.
- Transport-exclusive process binding or daemon supervision.
- Provider credentials, endpoints, or deployment secrets.
- Governance policy definitions; it consumes `arda_core::GovernanceGates`.
- Ledger implementation; it consumes `arda_core::Ledger`.
- Behavior in currently unwired source files until a reviewed wire/migrate decision is completed.

## Change authority

- gRPC schema changes require coordinated Manwe compatibility checks.
- Provider public-contract changes require `arda-engine` checks.
- A2H contract changes require `arda-aule --features full-cli` checks.
- Governance record changes require central governance/ledger review.
- Unwired source retirement or exposure requires the evidence and gates in `PLAN.md`.

## Canonical consumer path

Consumers should import crate-root re-exports or the public `provider`/`grpc` modules. They must not
depend on files that are absent from the `lib.rs` module graph.
