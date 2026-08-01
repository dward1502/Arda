# arda-orome index

## Maintained documentation

- `INDEX.md` — deterministic navigation.
- `README.md` — public/default and opt-in contracts.
- `STATUS.md` — dated verification evidence and boundaries.
- `BREAKDOWN.md` — production/test-only/retired classification.
- `OWNERSHIP.md` — authority boundaries.

## Direct crate children

- `Cargo.toml` — package, `service-runtime` feature, dependencies, and build dependencies.
- `build.rs` — protobuf generation into `src/grpc/`.
- `proto/` — health-model and route-governance protobuf contracts.
- `src/` — default, opt-in, test-only, and generated Rust sources; no unwired Rust files remain.
- `tests/` — provider orchestration, live HTTP/fleet policy, and governance-ledger integration tests.

## Cross-crate verification

- `crates/engine/src/orome.rs` and `crates/engine/tests/orome_smoke.rs`.
- `crates/spine/runtime/manwe/src/grpc.rs` behind Manwe `grpc`.
- `crates/spine/observability/arda-aule/src/prometheus/autopilot/a2h.rs` behind Aule `full-cli`.
