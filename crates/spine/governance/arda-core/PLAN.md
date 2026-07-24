---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine"
  owner: "ARDA-CORE / HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---

# arda-core plan

Crate: `crates/spine/governance/arda-core`
Owner surface: governance spine primitives + contracts + loop + registry
Current baseline: `cargo check -p arda-core` OK; `cargo test -p arda-core` 91/91 passing

## 1. Objective
Make `arda-core` the stable, documented, receipt-backed foundation for
task/agent/governance/loop/ml/service-registry state in Arda. Nothing
upstream should reach around it for canonical types or execution policy.

## 2. Generation boundary
GEN1: existing live surface, evidence collection, and doc alignment.
GEN2: correctness/robustness work that does not grow public API.
GEN3: broader learning/observability/interop that waits until GEN2 is closed.

## 3. GEN1 — existing surface, evidence, docs
- Enumerate public surface in `src/lib.rs` and reconcile against
  `INDEX.md`, `README.md`, `BREAKDOWN.md`.
- Collect evidence paths for every major module.
- Capture known warnings/follow-ups from `BREAKDOWN.md` and confirm each
  with current code.
- Outcome: README/BREAKDOWN/STATUS describe reality, not wish list.

## 4. GEN2 — correctness and robustness
- Governance gate coverage: unknown intent fallback via
  `policy_for`/`policy_for_action_class` default policy and parse-error
  handling covered in `governance_gates::tests::non_json_payload_returns_parse_error`.
- Loop engine coverage: bounded dispatch (`dispatch_cap_zero_dispatches_nothing`,
  `dispatch_cap_limits_dispatched_task_count`), market-collapse behavior
  (`dispatch_records_market_collapse_when_no_bidders`, alert synthesis in
  `loop_alerts`), budget exhaustion (`dispatch_blocks_when_goal_budget_exhausted`,
  `dispatch_blocks_budget_when_estimator_reports_high_joule_cost`),
  triad veto/record-only split (`dispatch_records_triad_veto_without_blocking`,
  `dispatch_blocks_triad_veto_when_policy_requires_it`), alert emission
  (`analyze_tick` coverage in `loop_alerts::tests`).
- Learning coverage: routing bias update (`learning::tests::routing_bias_reflects_observed_success_rate`),
  best-agent selection (`learning::tests::picks_best_agent_for_type`),
  ledger round trip (`learning::tests::round_trip`).
- Background gate coverage: poison-recovery (`gate_registry_recovers_from_poisoned_mutex`,
  `pressure_cache_recovers_from_poisoned_mutex`), scaled limit floor
  (`scaled_limit_never_drops_below_one`), async and sync cap behavior
  (`bounded_async_gate_runs_work`, sync cap via `try_run_bounded`).
- Ledger/message boundaries: malformed input tolerance in
  `state::tests::list_skips_non_json_and_tmp`, envelope metadata defaults
  and round trip in `message::tests::message_defaults_are_emitted_and_survive_round_trip`.
- Service registry: upsert result handling, duplicate rejection
  (`service_registry::registry::tests::duplicate_registration_rejected`),
  snapshot/from_snapshot round-trip and invalid-name rejection
  (`service_registry::registry::tests::empty_service_name_is_rejected`).
- Soterion: signature rendering determinism/build_persist_index
  round trip (`soterion::tests::scan_directory_load_and_persist_round_trip_index`,
  `soterion::tests::persist_if_changed_only_writes_newer_index`,
  `soterion::tests::render_signature_concatenates_without_intermediate_glyph_vec`),
  index persistence recovery and watcher resilience covered in `soterion_watcher::tests`.
- Status: baseline tests green; GEN1/ GEN2 coverage added in place,
  preserving crate boundary stability.

## 4. GEN2 status
Closed.
- Baseline: `cargo test -p arda-core` 91/91 passing
  - 90 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- Coverage confirmed present in `governance_gates`, `loop_engine`,
  `loop_alerts`, `learning`, `background`, `state`, `message`,
  `service_registry`, `soterion`, `soterion_watcher`.
- Known warnings unchanged; no public API expansion from GEN2.

## 5. GEN3 — learning/observability/interop
- Evaluate public learning/memory systems for concepts that can be
  adapted into Arda governance semantics without breaking append-only
  auditability.
- Add observability knobs for loop economy and decision latency.
- Consider interoperability views for `arda-core` contracts consumed
  by external tooling.
- Status: evidence-backed.
  - `loop_observability` provides env-toggled economy/latency knobs.
  - `learning_adapter` provides learning-to-domain adaptation + ledger receipt.
  - `arda-aule` consumes both via `LoopCommands::Observability` and
    `LearningCommands::Ledger`.
  - `arda-engine` exposes `EngineObservabilityStatus` as the aggregator.
- Remaining interop scenarios are captured in `docs/interop/landscape.md`.

## 6. Execution order
1. Reconcile docs to current code and write STATUS evidence.
2. Add missing tests for uncovered GEN2 behavior paths.
3. Fix small correctness issues from tests, smallest first.
4. Record GEN3 landscape and defer interop until needed.
5. Keep crate boundary stable across all steps.
