# arda-economics module index

| Module | Responsibility |
|---|---|
| `economics.rs` | Provider cost models, spend accumulation, ROI, and budget alerts |
| `meter.rs` | Async energy-meter contract, hardware probes, tariff loading, estimator registry |
| `joule_work.rs` | JouleWork records, summaries, measurement provenance, snapshot restore |
| `ledger.rs` | Plutus account balances, transfers, and snapshot restore |
| `love_equation.rs` | Relationship scoring, ranking, and timestamp-preserving restore |
| `service.rs` | Validated operations, governance evidence, schema migration, atomic snapshots, append-only runtime events |
| `transport/ipc.rs` | Unix-domain socket command server/client |
| `transport/http.rs` | Optional Axum HTTP routes and event stream |
| `transport/mod.rs` | Daemon configuration and concurrent transport supervision |
| `transport/finance_stream.rs` | Finance metrics, budget pressure, snapshot freshness, and transport latency |
| `error.rs` | Crate-specific error types |
| `lib.rs` | Public exports |

Related documentation:

- [README.md](README.md): operator and integration overview
- [BREAKDOWN.md](BREAKDOWN.md): current-state architecture and source classification
- [STATUS.md](STATUS.md): exact closeout evidence
- [OWNERSHIP.md](OWNERSHIP.md): producer/consumer authority
