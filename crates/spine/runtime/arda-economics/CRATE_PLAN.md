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

## Completed work

1. Added additive `transport/finance_stream.rs` with finance metric export and
   `PlutusRuntimeEvent` stream coverage for `runtime_status.json` +
   `runtime_events.jsonl`, re-exported from crate root.
2. Enriched `PlutusLedger` snapshots with credit totals, event counts, and
   last-credit provenance.

## Implementation work

3. Add integration coverage for failed hardware backend/estimator fallback
   combinations (`src/runtime.rs`).
4. Add observability hooks for budget pressure, tariff table staleness, and
   IPC/HTTP queue latency.
5. Validate JouleWork unit multipliers and source-provenance invariants across
   long-running sessions in coordination with `arda-governance`.

## Deferred next steps

6. Integration: measurement-provider fallback matrix with `Mandos` orbital/hardware
   validation. Triggers: `arda-mandos` transport-matrix plan stabilized; operator
   hardware minimums owned; deterministic timeout vendors established.
7. Regression: long-running pluto-stream session for `JouleWorkMeasurementSource`
   occupant invariants. Triggers: multi-hour runner harness owned; governance
   invariant checklist exists; source-provenance fetch policy validated.
8. Operator audit trail: caller-side verification from `arda-aule` for governance
   metric stream export.

## Open risk items

- Finance metrics export is implemented for streamed `PlutusRuntimeEvent` types;
  caller-side verification remains incomplete.
- Failed hardware/estimator fallback matrix is only partially covered; deferred
  until `arda-mandos` orbital/matrix integration is ready.
- Long-running JouleWork session invariants are not yet regression-tested;
  deferred to dedicated runtime harness.

## Status

Public path is intact; finance/metrics coverage shipped as additive module.
Hardware/estimator fallback and long-running invariant regression are deferred
to dedicated integration planning.
