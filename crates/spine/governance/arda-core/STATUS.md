---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine_status"
  owner: "ARDA-CORE / HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---
# arda-core status

Crate: `crates/spine/governance/arda-core`
Sigil: 📜 SCROLL

## Build
- `cargo check -p arda-core` -> OK
- `cargo test -p arda-core` -> 91/91 passing
  - 90 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests

## Runtime / env knobs
- `ARDA_PRESSURE_ADMISSION_*` env knobs may tune pressure-aware
  bounded background/sync execution in `background.rs`.
- Soterion index path can be overridden from the environment; watcher
  uses that override before defaults.
- governance gates load policy modes and per-action overrides from YAML.
- affordability policy hooks are exposed through
  `dispatch_full_with_affordability(...)`; compatibility callers
  retain allow-all behavior.

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

## Known warning/follow-ups
- `service_registry/registry.rs:37` ignores `upsert_contract(...)` result.
- `tool_contract/service.rs:5` has 1 unused import.
- GEN3 interop is deferred; see `docs/interop/landscape.md`.

## GEN2 closeout
- Baseline verified: `cargo test -p arda-core` -> 91/91 passing
  - 90 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- Coverage paths verified present per PLAN section 4:
  - governance, loop engine, learning, background gate, state/message,
    service_registry, soterion (+watcher)
- Crate boundary stable; no public API growth from GEN2.
- Known warnings preserved; do not expand scope without consumer migration.

## Owner notes
- `arda-engine` re-exports `arda_core::service_registry`.
- Crate boundary is intentionally stable; do not split or rename without
  consumer migration evidence.
- STATUS evidence was refreshed against the current `manwe` branch source of
  truth after doc/code drift was found.
