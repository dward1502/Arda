# arda-aule status

Crate: `crates/spine/observability/arda-aule`
Version: `0.1.0`
State: **source-classified, feature-complete for current scope, and verified**
Reviewed: 2026-07-28
Required crate-local stabilization work: **complete**

## Current state

- All 85 Rust files are classified: 5 default production, 72 feature-gated production, 8 integration tests, and zero generated-include, standalone test-only, build-script, or unwired files.
- The feature-gated production split is 4 `telemetry`, 66 `full-cli`, and 2 `http` files.
- No detached source tree or latent `foo.rs` plus `foo/mod.rs` collision exists.
- `full-cli` compiles the CEO, Prometheus, autopilot, service, transport, and operator-binary closure.
- `http` adds the metrics exporter and HTTP/node-metrics closure.
- The pre-existing Vaire metrics work is preserved: the exporter consumes `arda.mnemosyne.observability.v1`, and the continuity projection emits its distinct `arda.mnemosyne.continuity.v1` contract.
- A stale all-feature assertion that still expected the generic core-state schema for the Mnemosyne continuity projection was corrected to verify the live specific schema.

## Remaining boundaries

- Provider/model selection and route fitness remain Manwe authority.
- Task execution remains executor/core-loop authority; Aule writes governed queue and intent records.
- Memory truth remains Vaire/Mnemosyne authority; Aule projects and exports bounded observations.
- Existing `annunimas_*` metrics remain a coordinated migration boundary, not a crate-local rename task.
- No active crate-local `PLAN.md` is required. Archived foundation plans remain historical evidence and must not be resurrected as commitments.

## Verification evidence

Passed from the workspace root on 2026-07-28:

- `cargo fmt -p arda-aule -- --check`.
- `cargo check -p arda-aule --no-default-features`.
- `cargo test -p arda-aule --no-default-features -- --test-threads=1`: 5 unit/integration tests and 2 doctests passed.
- `cargo check -p arda-aule --all-targets --all-features`.
- `cargo test -p arda-aule --all-features -- --test-threads=1`: 187 unit/integration tests and 2 doctests passed.
- `cargo clippy -p arda-aule --all-targets --all-features -- -D warnings`.
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-aule --no-deps --all-features`.
- `cargo check -p manwe --features telemetry`.
- `cargo test -p manwe --features telemetry --no-run`.

Cargo emitted only the existing workspace warning about the ignored non-root launcher profile.
