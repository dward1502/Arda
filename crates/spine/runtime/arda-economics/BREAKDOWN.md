---
soterion:
  sigil: "REPAIR"
  glyph: "⚡"
  role: "runtime_economics"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-28"
---

# arda-economics
Runtime economics crate for Arda: cost modeling, joule work tracking,
love-equation relationships, energy metering, ledger, and IPC/HTTP
daemon transport for the Plutus service.
Owner: hades | Sigil: ⚡ REPAIR | Status: active

## Summary
`arda-economics` is the cleanest runtime crate in the Arda spine.
It owns the economics layer behind autonomous decision making:
provider cost models, joule-work tracking with measurement provenance,
love-equation relationship scoring, energy metering with tariff-based
estimation and hardware probing, an account ledger, and a fully
persistent Plutus service with governance history.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/runtime/arda-economics`
- Config: `config/governance/joule_tariffs.toml`
- Data: `data/plutus/*`, env-overridable via `ARDA_PLUTUS_HOME`

## Verification status
- `cargo check -p arda-economics`: OK
- `cargo test -p arda-economics --all-features`: 34 passed, 1 ignored, 0 failed
- Doc tests: 0
- Direct Cargo consumers: `arda-mandos`, `arda-vaire`, and `arda-varda`

## Agentic-OS abstractions
- **Economics engine**: `EconomicsEngine` with daily budget, provider-
  specific `CostModelConfig`, linear cost calculation, spend recording,
  `can_afford()` gating, ROI metrics
- **Joule work tracker**: `JouleWorkTracker` with unit multipliers
  (`Compute 1.0`, `Network 0.5`, `Storage 0.3`, `Attention 1.5`,
  `Reasoning 2.0`), measurement-source provenance, confidence tracking,
  per-agent/unit/source summaries, snapshot restore
- **Love equation**: configurable weighted score from resonance/attention/
  reciprocity, relationship recording and top-N retrieval, snapshot/restore
- **Energy metering**: `EnergyMeter` trait with hardware backends
  (`Rapl`, `Nvml`, `PowerMetrics`, `Pi5Rails`) and `EstimatorMeter`
  tariff fallback; `WorkProfile::Cloud` and `WorkProfile::Local`
- **Tariff table**: TOML-loadable per-provider/model rates with defaults
  and reload support
- **Ledger**: `PlutusLedger` account credit/balance with snapshot/restore
- **Plutus service**: `PlutusService` is the runtime orchestrator tying
  economics, joulework, love, ledger, and governance history together;
  atomic snapshot persistence via tmp+rename; `from_default_or_workspace_fallback()`
- **Governance history**: every service action emits a `PlutusGovernanceRecord`
  with triad, bacon-lite, and resonance scores
- **Transport**: `PlutusDaemon` runs IPC Unix-socket + optional HTTP/SSE;
  configurable timeouts; tests cover IPC/HTTP round-trips

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Public exports for all subsystems |
| `economics.rs` | `CostModel`, `LinearCostModel`, `EconomicsEngine`, `ROIMetrics` |
| `joule_work.rs` | `JouleWork`, `JouleWorkUnit`, `JouleWorkTracker`, summaries |
| `love_equation.rs` | `LoveEquation`, `LoveConfig`, `LoveScore`, relationships |
| `meter.rs` | `EnergyMeter` trait, `EstimatorMeter`, `HardwareMeter`, `MeterRegistry`, `TariffTable` |
| `ledger.rs` | `PlutusLedger` account balances |
| `service.rs` | `PlutusService`: runtime orchestration + persistence |
| `transport/mod.rs` | Daemon config + runner |
| `transport/ipc.rs` | Unix socket server |
| `transport/http.rs` | Optional HTTP/SSE server |
| `transport/finance_stream.rs` | Finance, budget-pressure, freshness, and transport-latency export |
| `error.rs` | Canonical error type |

## Consumer wiring
- Used directly by `arda-mandos`, `arda-vaire`, and `arda-varda`
- `arda-aule`, `arda-governance`, Manwe, and launcher are not direct Cargo consumers
- Depends on: `arda-core`, `arda-governance`

## Supported source classification

| Classification | Count | Paths |
|---|---:|---|
| Production/unconditional | 11 | `src/lib.rs`, seven other top-level modules, `transport/mod.rs`, `transport/ipc.rs`, `transport/finance_stream.rs` |
| Production/feature-gated | 1 | `transport/http.rs` (`http`, enabled by default) |
| Generated include | 0 | None |
| Standalone test-only source | 0 | Tests are inline |
| Integration test/build script | 0 / 0 | None |
| Unwired | 0 | None |

The all-feature suite has 34 passing tests plus one ignored operator-scale test.
The ignored 10,000-event provenance/multiplier test passed when executed directly.

## Ideas for improvement
Completed in the first improvement pass:

1. [x] Added crate-level `README.md` and `INDEX.md`.
2. [x] Added `arda_core::layout::arda_root_from()` and replaced this crate's
   duplicated fixed-depth root discovery. `ARDA_ROOT` is canonical and
   `ANNUNIMAS_ROOT` remains a compatibility fallback.
3. [x] Made `EnergyMeter::estimate()` async while retaining a private sync
   estimator path for the existing synchronous `JouleEstimator` contract.
4. [x] Added v1-to-v2 `runtime_status.json` migration, rejection of unknown
   future schemas, and full governance-record persistence in v2.
5. [x] Added explicit ROI, LoveEquation restore, tariff validation/reload, input
   validation, schema migration, and budget-threshold tests.
6. [x] Added warning (80%) and critical (100%) budget threshold hooks and status
   reporting, including finite zero-budget behavior.

Completed in the second improvement pass:

7. [x] Added the dependency-neutral `AffordabilityPolicy` contract and
   `AffordabilityDecision` under `arda-core` governance gates. `EconomicsEngine`
   implements the policy, and `dispatch_full_with_affordability()` enforces it
   before market selection while compatibility dispatch remains allow-all.
8. [x] Kept transport in this crate after reviewing its current three-module,
   single-consumer surface. HTTP is already feature-gated, and extraction now
   has explicit triggers: a second runtime consumer, an independent release
   lifecycle, or transport dependencies materially inflating non-daemon builds.
9. [x] Added the shared `arda_core::ledger::AppendOnlyLedger` contract.
   `PlutusService` implements it through `runtime_events.jsonl`, and every
   successful economic mutation appends a durable `arda.plutus.event.v1` entry.
10. [x] Added the live `arda-cli plutus export` command with human-readable and
    `--json` output, explicit `--path` override, missing-state handling, and
    append-only event counts.

All improvement items are resolved. Completed trackers were retired after strict
crate, operator-scale, and direct-consumer gates.
