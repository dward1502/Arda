# arda-economics status

Crate: `crates/spine/runtime/arda-economics`
State: stable first-class runtime economics substrate
Last verified: 2026-07-28

## Closeout result

- All 12 Rust files are wired: 11 unconditional production files and 1
  `http`-feature production file; no generated, standalone test, integration,
  build-script, or unwired files exist.
- Meter errors are typed. `MeterRegistry::estimate` skips failed/non-finite
  hardware samples and deterministically falls back to the estimator.
- Tariff tables expose load timestamp and deterministic staleness checks.
- Runtime snapshots and finance export expose budget pressure, snapshot age,
  and IPC/HTTP request latency aggregates.
- A 10,000-event operator-scale test proves unit multipliers, observed/default
  provenance totals, and confidence aggregation.
- Direct Cargo consumers are `arda-mandos`, `arda-vaire`, and `arda-varda`.
  `arda-aule`, `arda-governance`, and Manwe are not direct consumers.

## Verification evidence

- No-default check: passed.
- No-default suite: 33 passed, 1 ignored; 0 failed.
- All-target/all-feature check: passed.
- All-feature suite: 34 passed, 1 ignored; 0 failed.
- Operator-scale ignored test executed separately: 1 passed; 10,000 events,
  total 10,600, observed 5,300, default fallback 5,300.
- Strict all-target Clippy and strict rustdoc: passed.
- Direct consumer all-target/all-feature checks for `arda-mandos`, `arda-vaire`,
  and `arda-varda`: passed.
- Full all-feature suite changed no files inside the crate.

No active crate-local plan remains. The launcher profile warning is a workspace
manifest warning external to this crate.
