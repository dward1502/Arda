# arda-engine status

Crate: `crates/engine`
State: stable first-class foundation
Last verified: 2026-08-08
Branch: `visual/hud-boardroom-convergence`

## Current contract

- All 27 production Rust files and 17 integration test targets are wired.
- The crate has no declared features; no-default and all-feature gates therefore
  exercise the same supported graph.
- The root `arda` package is the only direct Cargo consumer.
- The former no-op `boot()` boundary is removed.
- `arda --once` now loads and resolves `services.toml`, reports required-service
  errors, honors `--no-ui`, and exits before supervision or harness startup.
- `services.toml` declares four capability authorities; engine projections are
  derived from the mutable canonical registry and begin fail-closed until live
  health evidence arrives.
- Harness Manwe calls use the state-owned client, explicit five-second default
  timeout, and optional bearer forwarding.
- `ArdaEngineGovernanceEnforcer` is the canonical runtime decision owner. It
  consumes normalized advisory evaluator receipts, persists a digest-bound
  verdict/transition lineage, and prevents workers or projections from lowering
  that verdict.
- The global append-only resource ledger preserves observed versus default
  provenance, supports late observed-usage reconciliation without rewriting
  history, and feeds route/execution spend, JouleWork, provider, and pressure caps.
- `OromeOperatorRuntime` is the engine-owned entry point for durable,
  transport-neutral Hermes operator-session ingestion, approval validation,
  canonical response correlation, and transport health projection.

## Verification evidence

- `cargo fmt --all -- --check`: passed for the workspace.
- `cargo test -p arda-engine`: 122 passed, 2 ignored, 0 failed.
- Strict all-target engine-only Clippy (`--no-deps`) reaches two unrelated
  pre-existing findings: `harness/research.rs` (`unnecessary_to_owned`) and
  `tests/harness_personal_ops.rs` (`len_zero`). Allowing exactly those two
  lints passes with `-D warnings` for the remaining graph.
- Including dependencies also reaches the unrelated pre-existing
  `too_many_arguments` finding in `arda-outpost-protocol`.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-engine --no-deps --all-features`:
  passed.
- `cargo check -p arda --all-targets`: passed.

## Remaining posture

No active crate-local implementation plan remains. Future additions must retain
the engine/root ownership boundary and add focused tests plus consumer evidence.
