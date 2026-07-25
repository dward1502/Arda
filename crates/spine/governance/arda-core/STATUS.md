---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine_status"
  owner: "ARDA-CORE / HADES"
  status: "active"
  last_reviewed: "2026-07-25"
---
# arda-core status

Crate: `crates/spine/governance/arda-core`
Sigil: 📜 SCROLL

## Build
- `cargo check -p arda-core` -> OK
- `cargo test -p arda-core` -> 99/99 passing
  - 98 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- `cargo clippy -p arda-core --all-targets --all-features -- -D warnings` -> OK

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

## Known follow-ups
- `cargo check -p arda-core` and strict all-target/all-feature Clippy emit no crate warnings.
  The only command output is
  the workspace-level non-root profile warning from `arda-launcher`.
- `ServiceRegistry::from_snapshot` intentionally skips records rejected by
  `upsert_contract`; duplicate-skip behavior is covered by a unit test.
- Remaining GEN3 questions are tracked in `docs/interop/landscape.md`.

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
- Implemented GEN3 observability/learning interop is additive and covered by
  the 99-test baseline.
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
- GEN3 interop moved from deferred to evidence-backed via `arda-aule`
  loop/learning consumers and `arda-engine` aggregation surface; remaining
  interop work is documented in `docs/interop/landscape.md`.
