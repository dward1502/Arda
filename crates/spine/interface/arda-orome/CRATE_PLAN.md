# arda-orome implementation plan

Source: `crates/spine/interface/arda-orome`, `CHECKLIST.md`, workspace consumers in `arda-aule`, `arda-manwe`, `arda-varda`, `arda-orome`, `arda-vaire`.

## Canonical public path

- Crate root re-exports: `ProviderRuntime`, `ProviderConfig`, `ProviderType`, `DispatchReceipt`
- Primary integrations: `arda-manwe` adaptive routing; `arda-varda` dispatch receipts; `arda-aule` provider surfaces; `arda-vaire` ambient context caching.

## Canonical contracts

- `ProviderType` is the bounded dispatch target class.
- `ProviderRuntime` is the shared runtime context passed through dispatch and caching.
- `DispatchReceipt` is the observable outcome of a provider interaction used by operator metrics.
- Provider adapters can be enriched without changing existing interface consumers because dispatch happens behind typed models.

## Implementation work

1. Add implementation coverage for provider adapter retry/expiry behavior (`src/provider/runtime.rs`, `src/provider/tests.rs`).
2. Expand router retry/expiry tests and expose bounded dispatch metrics for operator observation (`src/provider/runtime.rs`, `src/context_cache.rs`).
3. Strengthen fanout and routing orchestration through typed routing intent so `arda-manwe` and `arda-aule` observe explicit dispatch outcomes.
4. Wire one interface package into the engine/CLI as a live smoke path using explicit `Manual::test()` dispatch with recorded metrics.
5. Normalize governance hooks centrally through typed approval/interruption envelopes backed by ledger writes in coordination with `arda-governance`.

## Open risk items

- Provider adapter timeout/retry policy remains implicit; without explicit timeout behavior, operators observe unbounded waits.
- Fanout orchestration coverage excludes failure-path parallelism.
- Live interfaces exist, but no recorded implementation evidence in `CHECKLIST.md`.

## Status

Canonical public path is intact; implementation evidence is incomplete.
