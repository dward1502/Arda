# arda-orome HERMES Execution Checklist

Source: combined from `BREAKDOWN.md` and `docs/plans/HERMES.md`.

Items with [ ] pending; [~] in progress; [x] done.

## Baseline (verified)
- [x] Read live source/crate state
- [x] `cargo check -p arda-orome` passes
- [x] `cargo test -p arda-orome` passes (21 tests: 14 unit + 7 integration; 0 failures)
- [x] Inspect actual `arda-orome` workspace consumers
- [x] Resolve crate documentation cross-references
- [x] Keep canonical provider/runtime module registration coherent

## Hermes tasks from docs/plans/HERMES.md
- [x] Richer provider adapters and live streaming surfaces
- [x] Strengthen fanout and routing orchestration
- [x] Expand edge-worker and fleet communication policy
- [x] Broaden ARDA HUD consumption of core and human-plan surfaces

Evidence:
- `src/provider/orchestration.rs` adds bounded timeout/retry dispatch, expiry rejection, typed direct/fanout routing, streaming receipts, metrics, and a deterministic manual transport.
- `EdgeCommunicationPolicy` and `FleetScope` bound local, trusted-fleet, and externally approved dispatch.
- `apps/arda-hud/src/lib/ardaSource.ts` derives both `docs/plans` and `core/projects/Plans`; `reviewGateDerivation.ts::getPlanShelf` consumes both roots.

## Breakdown priorities
- [x] Add unit/integration coverage: router retry/expiry, intent classification, MCP/provider governance
- [x] Unify duplicate message abstractions/canonical types across crate
- [x] Replace static string labels with typed enums where required by routing contracts
- [x] Make registry/router state sharable via core runtime traits
- [x] Persist `MessageQueue`/agent registry state
- [x] Remove broken test imports and async test compile failures
- [x] Clean crate-local unused imports
- [x] Normalize governance hooks centrally
- [x] Add typed approval/interruption envelopes backed by ledger writes
- [x] Replace static context storage with bounded async cache
- [x] Wire one interface package into the engine as a live smoke path

## Repair and closeout evidence
- [x] `src/provider/runtime.rs` owns canonical provider and dispatch receipt types
- [x] `src/provider/orchestration.rs` owns dispatch policy, transport contract, fanout, edge policy, and metrics
- [x] `src/governance.rs` maps central `GovernanceGates` action policies into ledger-backed typed envelopes
- [x] `tests/provider_orchestration.rs` covers retry, timeout, expiry, fanout bounds, observability, and external approval
- [x] `tests/governance_ledger.rs` verifies typed approval/interruption ledger records and decisions
- [x] `crates/engine/src/orome.rs` provides the no-network `manual_smoke_dispatch` engine path
- [x] `cargo test -p arda-engine --test orome_smoke` passes (1 test; 0 failures)

## Completion
All checklist items are complete. Production provider credentials and network transports remain deployment configuration, not crate-plan work.
