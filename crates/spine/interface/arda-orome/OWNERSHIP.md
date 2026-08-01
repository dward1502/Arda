# arda-orome ownership

Crate: `crates/spine/interface/arda-orome`
Owner: HADES / interface layer
Status: active
Reviewed: 2026-07-27

## This crate owns

- A2H/A2A message and envelope contracts.
- Provider adapter, registry, bounded dispatch, fleet-scope, streaming, metrics, receipt contracts, and the opt-in HTTP JSON transport.
- Health-model and route-governance protobuf definitions and generated Rust surfaces.
- Typed interface/operator payloads and ledger-backed governance record creation.
- The opt-in `service-runtime` resident-service, MCP, context, Discord-safety, and projection contracts.

## This crate does not own

- Provider/model selection, route fitness, or inference policy; Manwe owns them.
- Daemon supervision, provider credentials, deployment secrets, or exclusive process binding.
- Provider credentials, endpoint discovery, or live status merely because a provider is configured; a receipt proves one delivery, not continuing health.
- Governance or ledger implementations consumed from central crates.
- ARDA HUD consumer implementation; that belongs to `apps/arda-hud`.

## Change authority

- gRPC schema changes require Manwe checks.
- provider-contract changes require `arda-engine` checks.
- A2H changes require `arda-aule --features full-cli` checks.
- `service-runtime` changes require all-feature tests, strict Clippy, and explicit no-network/live-transport truthfulness.
- trusted-fleet policy changes require denied-before-network and receipt-backed live-socket tests.
- Do not restore retired Serenity/presentation modules without a concrete owner, consumer, and tested transport contract.
