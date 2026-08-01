---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: HADES
  status: stable
  last_reviewed: 2026-07-28
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: stable | reviewed: 2026-07-28

# arda-economics

Runtime economics substrate for Arda: provider spend accounting, JouleWork measurement provenance, LoveEquation relationship scoring, energy metering/tariffs, `PlutusLedger`, and a persistent appending-events `PlutusService` daemon over IPC and optional HTTP/SSE.

## Verified surface

- `EconomicsEngine`, `LinearCostModel`, `ROIMetrics`
- `JouleWorkTracker` with measurement-source provenance and per-agent summaries
- `LoveEquation` with configurable weights, relationships, snapshot/restore
- `EnergyMeter` + `EstimatorMeter` + `HardwareMeter` + `MeterRegistry` + `TariffTable`
- typed backend failure, ordered estimator fallback, finite-sample validation,
  and tariff freshness hooks
- `PlutusLedger` balances + append-only `runtime_events.jsonl`
- `PlutusService` with atomic snapshot persistence, v1-to-v2 migration, and governed mutations
- `PlutusDaemon` Unix socket + optional HTTP/SSE with configurable timeouts
- finance export with budget pressure, snapshot freshness, and IPC/HTTP latency
  aggregates

## Verified evidence

Closeout proofpoint: all-feature suite 34 passed, 1 operator-scale test ignored by
default; the 10,000-event operator-scale provenance test passed separately.

## Runtime state

By default, runtime state lives under `data/plutus`. Override with `ARDA_PLUTUS_HOME`. Workspace discovery uses `ARDA_ROOT`, then `ANNUNIMAS_ROOT`, then upward workspace manifest walk.

## Live status

See STATUS.md for current health signals, open risks, and ownership.

## Ownership

See [OWNERSHIP.md](OWNERSHIP.md). No active crate-local plan remains.

## Operator export

ARDACLIOrCurrentEquivalent path: plutus export / plutus export --json / plutus export --path override.