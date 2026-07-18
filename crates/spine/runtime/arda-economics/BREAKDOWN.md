---
soterion:
  sigil: "REPAIR"
  glyph: "⚡"
  role: "runtime_economics"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
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
- `cargo test -p arda-economics`: 14 passed, 0 failed
- Doc tests: 0
- No consumers found in `arda-engine` or `apps`; used by `arda-governance`,
  `arda-orome`, `arda-varda`, `arda-vaire` through `PlutusService`

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
| `error.rs` | Canonical error type |

## Consumer wiring
- Used by `arda-governance` MCP runtime governance for background work
  tracking
- Used by `arda-orome` context_enrichment and service layer
- Used by `arda-varda` interceptor pipeline and planning-task receipts
- Used by `arda-vaire` tests and service emit paths
- Depends on: `arda-core`, `arda-governance`

## Ideas for improvement
1. Add crate-level `INDEX.md`/`README.md` if missing; current docs are sparse
2. Replace duplicated `arda_root()` / `annunimas_root()` path logic with a
   shared layout helper under `arda-core`
3. Make `EnergyMeter::estimate()` async or document why sync is sufficient;
   hardware sampling may need async sysfs reads
4. Add schema-version migration for `runtime_status.json` so state upgrades
   don’t lose history
5. Expose `EconomicsEngine::can_afford()` as a policy hook in `arda-core`
   governance gates instead of local-only checks
6. Add explicit tests for `ROIMetrics`, `LoveEquation` restore, and tariff
   reload edge cases
7. Consider splitting transport into a feature-gated `arda-plutus-transport`
   crate if daemon surface grows
8. Add budget alerting hooks when `budget_usage_percent()` crosses thresholds
9. Make `PlutusService` implement a shared `AppendOnlyLedger` trait from
   `arda-core` for unified ledger semantics
10. Add operator-facing `plutus export` command or HUD section so economics
   state is visible without JSON inspection
