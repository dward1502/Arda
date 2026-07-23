# arda-economics implementation plan

Source: `crates/spine/runtime/arda-economics`, `CHECKLIST.md`, workspace consumers in `arda-varda`, `arda-manwe`, `arda-governance`, `arda-mandos`, `arda-aule`.

## Canonical public path

- Crate root re-exports `EconomicsEngine`, runtime traits, HTTP/IPC transport surfaces, `PlutusService`, ledger events, and tariff/configuration loaders.
- Primary integrations: `arda-varda` dispatch/budget enforcement; `arda-manwe` adaptive constraint hooks; `arda-aule` operator metrics.

## Canonical contracts

- `EconomicsEngine` owns service mutation and budget/roi/tariff valuation; it receives explicit transport timeouts.
- `PlutusService` emits `arda.plutus.event.v1` on successful mutation events.
- IPC/HTTP transports share one canonical runtime contract; transport tests verify same configurable timeout and Unix socket/HTTP daemon behavior.
- `EnergyMeter::estimate()` is async-capable while preserving a sync estimator path.

## Implementation work

1. Add integration coverage for failed hardware backend/estimator fallback combinations (`src/runtime.rs`).
2. Add observability hooks for budget pressure, tariff table staleness, and IPC/HTTP queue latency.
3. Add finance metrics export for `PlutusRuntimeEvent` streams emitted by `arda-aule` and consumed by operator UI.
4. Validate JouleWork unit multipliers and source-provenance invariants across long-running sessions in coordination with `arda-governance`.

## Open risk items

- Finance metrics export is missing for streamed `PlutusRuntimeEvent` types.
- Failed hardware/estimator fallback matrix is only partially covered.
- Long-running JouleWork invariants are not yet regression-tested.

## Status

Public path is intact; finance/metrics and long-running invariant coverage are incomplete.
