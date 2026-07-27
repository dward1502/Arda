# arda-orome index

## Maintained documentation

- `INDEX.md` — this deterministic navigation file.
- `README.md` — public scope, integration guidance, consumers, and documentation map.
- `STATUS.md` — dated build evidence and current stability findings.
- `BREAKDOWN.md` — production/test-only/unwired module inventory and invariants.
- `PLAN.md` — active source-tree stabilization work and future proposals.
- `OWNERSHIP.md` — owned and non-owned authority boundaries.

## Direct crate children

- `Cargo.toml` — package, feature, dependency, and build-dependency declarations.
- `build.rs` — protobuf generation into `src/grpc/`.
- `proto/` — health-model and route-governance protobuf contracts.
- `src/` — compiled, test-only, generated, and currently unwired Rust sources.
- `tests/` — provider orchestration and governance ledger integration tests.

## Cross-crate verification surfaces

- `crates/engine/src/orome.rs` and `crates/engine/tests/orome_smoke.rs`.
- `crates/spine/runtime/manwe/src/grpc.rs` behind Manwe's `grpc` feature.
- `crates/spine/observability/arda-aule/src/prometheus/autopilot/a2h.rs` behind Aule's
  `full-cli` feature.

Purpose: deterministic crate navigation. See `BREAKDOWN.md` before treating any file under `src/`
as compiled behavior.
