# arda-orome implementation plan

Source: `crates/spine/interface/arda-orome`, `CHECKLIST.md`, and verified workspace consumers.

## Canonical public path

- Crate-root and `provider` re-exports expose `ProviderRuntime`, `ProviderConfig`, `ProviderType`, `DispatchReceipt`, routing intent, transport, policy, and metrics types.
- `GovernanceHooks` is the central approval/interruption recording surface.
- `arda-engine::orome` is the compiled engine integration and deterministic smoke path.

## Canonical contracts

- `ProviderType` is the bounded provider family.
- `ProviderRuntime` owns provider inventory plus dispatch, edge, and metrics state.
- `RoutingIntent` expresses direct or bounded fanout targets.
- `ProviderTransport` isolates network/provider implementations from orchestration.
- `DispatchReceipt` and `DispatchMetricsSnapshot` provide operator-observable outcomes.
- `GovernanceHooks` maps action-class policy through `arda_core::GovernanceGates` and appends typed records to `arda_core::Ledger`.

## Completed implementation

1. Added bounded timeout/retry behavior, request expiry checks, streaming receipt accounting, and metrics.
2. Added typed direct/fanout routing with bounded parallel dispatch for shared transports.
3. Added explicit `FleetScope` and `EdgeCommunicationPolicy`; external dispatch can require prior approval.
4. Added deterministic `ManualTransport` and `arda_engine::orome::manual_smoke_dispatch`.
5. Added central ledger-backed task approval and interruption hooks.
6. Verified HUD derives and consumes both human-plan and core-plan roots.
7. Added integration tests for failure, timeout, expiry, fanout, edge policy, governance decisions, ledger writes, and engine wiring.

## Verification evidence

- `cargo test -p arda-orome`: 21 passed, 0 failed (14 unit + 7 integration).
- `cargo test -p arda-engine --test orome_smoke`: 1 passed, 0 failed.
- `cargo fmt -p arda-orome -p arda-engine -- --check`: required and recorded at closeout.

## Residual operational boundaries

- Real provider credentials, endpoints, and transport-specific clients are deployment configuration.
- External fleet communication is denied by the default edge policy; callers must opt in to the scope and provide approval where required.
- `ManualTransport` is intentionally no-network and exists for deterministic engine/CLI/HUD probes, not production dispatch.

## Status

Implementation plan complete. Future provider integrations should implement `ProviderTransport` without bypassing runtime timeout, retry, fanout, edge, governance, or receipt contracts.
