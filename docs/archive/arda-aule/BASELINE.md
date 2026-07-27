# arda-aule Release Baseline — 2026-07-25

## Supported surface

`arda-aule` is the consolidated observability and operator-control crate. Its default build keeps
the stable contract, council, governance-metric, service, and telemetry surfaces. `full-cli`
additionally attaches the CEO modules and the migrated Prometheus core/service/transport graph,
CEO autopilot, and execution intents, then builds the single `arda-cli` operator binary.

The current tree contains 82 Rust files and 27,611 lines across `src/` and `tests/`. Detached copied
CLI, stale fleet-pipeline, retired Apollo bridge, and duplicate-root source has been removed after
the supported replacements and ownership boundaries were verified.

## Verification

- `cargo check -p arda-aule`: passing
- `cargo test -p arda-aule`: 5 tests passed
- `cargo test -p arda-aule`: 2 doctests passed
- `cargo check -p arda-aule --features full-cli --all-targets`: passing
- `cargo test -p arda-aule --features full-cli --lib --tests`: passing
- `cargo test -p arda-aule --all-features --lib --tests`: passing serially
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- `cargo fmt -p arda-aule -- --check`: passing
- Process smoke checks for autopilot status/read-only execution and execution-intent listing: passing
- Workspace-wide `cargo fmt --all -- --check` remains blocked by pre-existing formatting
  drift in the modified launcher Tauri source, outside this crate's scope.

Process-level integration coverage exercises governance commands and the migrated Prometheus
surface. Unit coverage includes CEO loading/routing, autopilot execution, Prometheus orders,
thoughts, planning, council gates, projections, execution intents, service behavior, IPC, and
optional HTTP transport.

## Closeout decision

The consolidation is complete. Apollo-dependent autopilot execution now hands pending work to the
canonical task queue for the active core loop/executor, provider/fleet routing belongs to Manwe
through explicit intents, and Aule-owned CLI capabilities are reachable through the one supported
`arda-cli` binary. The copied
global CLI tree was retired because its command groups belong to their respective canonical crates,
not to observability.
