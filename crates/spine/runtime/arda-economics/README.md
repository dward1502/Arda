---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: HADES
  status: active
  last_reviewed: 2026-07-22
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-22

# arda-economics

Runtime economics substrate for Arda: provider spend accounting, JouleWork measurement provenance, LoveEquation relationship scoring, energy metering/tariffs, `PlutusLedger`, and a persistent appending-events `PlutusService` daemon over IPC and optional HTTP/SSE.

## Verified surface

- `EconomicsEngine`, `LinearCostModel`, `ROIMetrics`
- `JouleWorkTracker` with measurement-source provenance and per-agent summaries
- `LoveEquation` with configurable weights, relationships, snapshot/restore
- `EnergyMeter` + `EstimatorMeter` + `HardwareMeter` + `MeterRegistry` + `TariffTable`
- `PlutusLedger` balances + append-only `runtime_events.jsonl`
- `PlutusService` with atomic snapshot persistence, v1-to-v2 migration, and governed mutations
- `PlutusDaemon` Unix socket + optional HTTP/SSE with configurable timeouts

## Verified evidence

Build/test proofpoint: cargo check -p arda-economics + cargo test -p arda-economics 26/26 passing.

## Runtime state

By default, runtime state lives under `data/plutus`. Override with `ARDA_PLUTUS_HOME`. Workspace discovery uses `ARDA_ROOT`, then `ANNUNIMAS_ROOT`, then upward workspace manifest walk.

## Live status

See STATUS.md for current health signals, open risks, and ownership.

## Work queue

See CHECKLIST.md for authorship and implementation tracking.

## Operator export

ARDACLIOrCurrentEquivalent path: plutus export / plutus export --json / plutus export --path override.