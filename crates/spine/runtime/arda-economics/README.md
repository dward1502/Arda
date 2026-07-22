# arda-economics

`arda-economics` is the Plutus runtime substrate for Arda's resource accounting.
It tracks provider spend, JouleWork, account balances, relationship continuity,
and governance evidence, then exposes the combined state over IPC and optional
HTTP transports.

## Main surfaces

- `EconomicsEngine`: provider cost models, daily budget state, and threshold alerts.
- `EnergyMeter`: asynchronous measured/estimated joule sampling with tariff fallback.
- `JouleWorkTracker`: per-agent work measurements and summaries.
- `PlutusLedger`: in-memory balances and transfer history.
- `LoveEquation`: weighted relationship continuity scores.
- `PlutusService`: validated mutations and atomic `runtime_status.json` persistence.
- `PlutusDaemon`: Unix socket transport plus optional HTTP API.

See [INDEX.md](INDEX.md) for module navigation and [BREAKDOWN.md](BREAKDOWN.md)
for architecture, maturity, and remaining improvements.

## Runtime state

By default, Plutus stores state under `data/plutus` at the discovered Arda
workspace root. Override this with `ARDA_PLUTUS_HOME`. Workspace discovery uses
`ARDA_ROOT`, then the legacy `ANNUNIMAS_ROOT`, then walks upward to the root
`[workspace]` manifest.

The persisted schema is versioned. Version 1 snapshots migrate to version 2 on
load, preserving all governance records available in the old snapshot. Unknown
future schema versions are rejected rather than silently rewritten.

Successful mutations are also written to the append-only
`runtime_events.jsonl` ledger using the shared
`arda_core::ledger::AppendOnlyLedger` contract.

## Operator export

```text
cargo run -p arda-aule --bin arda-cli -- plutus export
cargo run -p arda-aule --bin arda-cli -- plutus export --json
cargo run -p arda-aule --bin arda-cli -- plutus export --path /path/to/runtime_status.json
```

The human view summarizes budget pressure, providers, accounts, governance,
JouleWork, relationships, and append-only event count without requiring direct
JSON inspection.

## Verification

```text
cargo test -p arda-economics
cargo clippy -p arda-economics --all-targets -- -D warnings
```
