---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: status
  owner: HADES
  status: active
  last_reviewed: 2026-07-22
---

# arda-economics — status

Verified: cargo test -p arda-economics 26/26 passing on the local Hermes cutover branch

## health

Active-mvp runtime substrate. Public surface compiles cleanly, state persistence is schema-migrated and append-only, and transport tests cover IPC + optional HTTP/SSE. Improvement passes marked complete in `BREAKDOWN.md`.

## signals

- economics path: `EconomicsEngine` + `LinearCostModel` + `ROIMetrics` + daily budget + threshold alerts
- joule work path: `JouleWorkTracker` with unit multipliers, measurement-source provenance, confidence tracking
- love path: `LoveEquation` with configurable weights, relationship recording, top-N retrieval, snapshot/restore
- meter path: `EnergyMeter` trait with hardware backends (`Rapl`, `Nvml`, `PowerMetrics`, `Pi5Rails`) + `EstimatorMeter` tariff fallback
- ledger path: `PlutusLedger` balances + `AppendOnlyLedger` runtime events via `runtime_events.jsonl`
- service path: `PlutusService` orchestrates mutations + atomic snapshot persistence + v1-to-v2 migration
- governance path: `PlutusGovernanceRecord` with triad/bacon-lite/resonance scores on every service action
- transport path: `PlutusDaemon` Unix socket + optional HTTP/SSE with configurable timeouts

## test evidence

Full suite: 26 unit tests across economics/love/meter/service/transport. No doc tests.

## open risks

- no direct compiled-in consumers in `arda-engine` or `apps`; indirect usage through governance/interface/planner crates
- hardware meter tests are limited to happy-path and estimator fallback; missing failed/back-end coverage
- finance metrics stream not exposed to external observability stack

## open tasks

See CHECKLIST.md for authorship and remaining validation work.
