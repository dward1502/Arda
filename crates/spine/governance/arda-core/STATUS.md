---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine_status"
  owner: "ARDA-CORE / HADES"
  status: "active"
  last_reviewed: "2026-07-28"
---
# arda-core status

Crate: `crates/spine/governance/arda-core`
Sigil: 📜 SCROLL

## Build
- `cargo check -p arda-core` -> OK
- `cargo test -p arda-core --all-features` -> 111/111 passing
  - 110 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- `cargo clippy -p arda-core --all-targets --all-features -- -D warnings` -> OK
- `cargo check -p arda-core --no-default-features` -> OK
- `cargo test -p arda-core --no-default-features -- --test-threads=1` -> 111/111 passing
- `cargo check -p arda-core --all-targets --all-features` -> OK
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-core --no-deps --all-features` -> OK

## Source classification
- Production/default: 45 files.
- Production/feature-gated: 0 files (the manifest declares no features).
- Generated include: 0 files.
- Test-only standalone source: 0 files.
- Integration test/build script: 1 integration test and 0 build scripts.
- Unwired: 0 files.
- Latent file-vs-directory module-root collisions: 0.

The exhaustive path list is maintained in `BREAKDOWN.md`.

## Runtime / env knobs
- `ARDA_PRESSURE_ADMISSION_*` env knobs may tune pressure-aware
  bounded background/sync execution in `background.rs`.
- Soterion index path can be overridden from the environment; watcher
  uses that override before defaults.
- governance gates load policy modes and per-action overrides from YAML.
- affordability policy hooks are exposed through
  `dispatch_full_with_affordability(...)`; compatibility callers
  retain allow-all behavior.
- GEN3 observability knobs:
  - `ARDA_LOOP_ECONOMY_SNAPSHOTS` enables economy snapshot writes
  - `ARDA_LOOP_LATENCY_PROBES` enables latency probe sampling
  - `ARDA_LOOP_MAX_LATENCY_SAMPLES` bounds retained probe samples

## Evidence paths
- `src/loop_engine.rs` dispatch path with gate/ledger/joule contracts
- `src/governance_gates.rs` triage by `DecisionClass` and action class
- `src/learning.rs` routing bias and best-agent selection
- `src/background.rs` bounded execution for sync/async/background
- `src/ledger.rs` append-only JSONL output with Soterion enrichment
- `src/service_registry/registry.rs` duplicate registration rejection
- `src/soterion_watcher.rs` markdown index watcher with persistence
- `src/state.rs` episodic/semantic memory split and plan/goal round trip
- `src/service_registry/registry.rs` snapshot/from_snapshot tests added for
  duplicate skip and round-trip preservation
- `src/loop_observability.rs` GEN3 env-toggled observability config + latency probes
- `src/learning_adapter.rs` GEN3 learning-to-domain adaptation + ledger receipt
- `src/aipkg.rs` schema-aligned manifest/preflight enforcement and fail-closed
  signed receipt-chain validation over explicit governance evidence

## AIPKG v0.1 status
- Foundation contract complete as of 2026-07-27.
- Runtime dispatch blocks malformed attached manifests before allocation.
- Receipt chains reject expired/unsigned/mismatched/failed evidence rather than
  inferring outcomes from manifest declarations.
- Machine-readable authorities:
  `core/state/aipkg_contract.json`,
  `core/state/aipkg_marketplace_separation_contract.json`, and
  `spec/aipkg/v0.1/receipt.schema.json`.
- Signing keys and executor-specific evidence collection remain profile-owned
  operational concerns, not hidden defaults in `arda-core`.

## Known follow-ups
- `cargo check -p arda-core` and strict all-target/all-feature Clippy emit no crate warnings.
  The only command output is
  the workspace-level non-root profile warning from `arda-launcher`.
- `ServiceRegistry::from_snapshot` intentionally skips records rejected by
  `upsert_contract`; duplicate-skip behavior is covered by a unit test.
- Remaining GEN3 questions are tracked in `docs/interop/landscape.md`.
- The completed foundation `PLAN.md` was retired on 2026-07-28 after this status and the other
  canonical documents absorbed its durable decisions and verification evidence.

## Retired source
- `src/alerts.rs` was removed on 2026-07-25. It was malformed, had never been
  exported from `src/lib.rs`, and had no repository consumer.
- `src/loop_alerts.rs` is the canonical compiled alert surface and is consumed
  by `arda-aule` for append-only Warden alert emission.

## GEN2 closeout
- Baseline verified: `cargo test -p arda-core` -> 99/99 passing
  - 98 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- Coverage paths verified present per PLAN section 4:
  - governance, loop engine, learning, background gate, state/message,
    service_registry, soterion (+watcher)
- Crate boundary stable; no public API growth from GEN2.
- No `arda-core` compiler warnings; do not expand scope without consumer evidence.

## Foundation designation
- Foundation baseline: complete as of 2026-07-25.
- GEN1 documentation alignment and GEN2 robustness work are closed.
- Implemented GEN3 observability/learning interop and AIPKG receipt-chain law
  are additive and covered by the current 111-test baseline.
- The crate remains active and maintained; “complete” means this stabilization
  plan is closed, not that future evidence-backed features are prohibited.

## Owner notes
- `arda-engine` re-exports `arda_core::service_registry`.
- `arda-engine` re-exports `arda_core::loop_observability` and exposes
  `EngineObservabilityStatus` as the higher-level aggregator for loop
  observability + learning interop.
- Crate boundary is intentionally stable; do not split or rename without
  consumer migration evidence.
- STATUS evidence was refreshed against the current `manwe` branch source of
  truth on 2026-07-25 after the test baseline had drifted from 91 to 99.
- GEN3 interop moved from deferred to evidence-backed via the `arda-engine`
  aggregation surface; `arda-aule` consumes other core service/runtime contracts. Remaining
  interop work is documented in `docs/interop/landscape.md`.
