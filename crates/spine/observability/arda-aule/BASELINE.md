# arda-aule Release Baseline — 2026-07-25

## Supported surface

`arda-aule` exposes observability contracts, governance metrics rendering, optional telemetry,
and the separately compiled `arda-cli` operator binary. The binary exposes only commands with
live implementations: telemetry schema, receipt rendering, governance metrics/status, and
Plutus export.

Imported CEO, Prometheus-daemon, and internal CLI module trees remain as migration evidence but
are not attached to the library graph and are not compatibility promises.

## Verification

- `cargo check -p arda-aule --features full-cli --all-targets`: passing
- `cargo test -p arda-aule --features full-cli`: 14 tests passed
- `cargo test -p arda-aule --features full-cli`: 2 doctests passed
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`: passing
- `cargo fmt --all -- --check`: passing as part of the workspace release gate

Process-level integration coverage runs `governance-metrics --json` and
`governance-status --path <ledger> --json`, parses their output, and verifies the documented
machine-readable contracts.

## Closeout decision

The prior `full-cli` failure was caused by exposing aspirational command variants and importing
stale monolith modules with retired dependencies. Those paths were detached rather than
presented as supported functionality. Any future reactivation requires a separately approved
migration with live dependencies, implementations, tests, and updated contract documentation.
