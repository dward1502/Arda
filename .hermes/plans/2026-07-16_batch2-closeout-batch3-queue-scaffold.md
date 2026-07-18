# Arda Batch 2 Closeout + Batch 3 Queue Scaffold

## Batch 2 closeout
- Extracted executor ownership surfaces are proven compile-safe:
  - `arda-human/tests/canary.rs` — green
  - `arda-learning/tests/canary.rs` — green
  - `arda-transport/tests/canary.rs` — green
  - `arda-service-registry/tests/canary.rs` — green
- `arda-varda/src/lib.rs` keeps `AthenaAgent`/`ingest` ownership; source-only cleanup to remove ownership of `human`, `learning`, `transport`, `service_registry` is complete.
- `arda-varda/tests/learning_contract_test.rs` and `arda-varda/tests/local_harness.rs` are archived to `docs/archive/arda-varda-tests/` with `.archived-2026-07-16` suffix.
- `cargo check -p arda-human -p arda-learning -p arda-transport -p arda-service-registry -p arda-varda` passes.

## Batch 3 queue scaffold
Goal: migrate remaining test layout to reduce executor-local context without removing coverage.

### Batch 3.1 — archive remaining executor-local tests
1. Confirm no other `tests/` directories remain under `crates/spine/executors/arda-varda/`.
2. If any appear, archive to `docs/archive/arda-varda-tests/` with date suffix.

### Batch 3.2 — add thin canary tests where missing
1. Add `tests/canary.rs` to any executor crate lacking one.
2. Make each canary reference the exported public surface only.

### Batch 3.3 — workspace test sweep
1. Run `cargo test --workspace` filtered to `canary` to prove no crate-local test regressions.
2. Document any crate that fails and needs deferred test migration.

### Validation
- `cargo test -p <each executor crate> canary` green
- `cargo test -p arda-varda` compile-safe in check mode
