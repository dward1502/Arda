# arda-governance implementation plan

Source: `crates/spine/governance/arda-governance`, `README.md`, `BREAKDOWN.md`, live code, consumers in `arda-varda`, `arda-mandos`, `arda-orome`.

Execution status: Phase 9 closeout evidence is canonical in
[`FIRST_CLASS_CHECKLIST.md`](FIRST_CLASS_CHECKLIST.md). This summary does not track separate
completion state.

## Baseline evidence

Live implementation exists and is active in workspace consumers:
- `triad_validate`, `evaluate_governance_chain`, `assess_governance_evidence`, and `assess_governance_evidence_typed` form typed deterministic gates with no I/O on hot paths.
- `load_governance_chain[_from_str]`, `load_philosopher_profiles[_from_str]`, and `GovernancePaths` provide explicit path-based filesystem access with preserved read/parse/validation error classes.
- `calculate_resonance*` functions cover governed and policy-absent ecosystems with normalized `[0,255]` outputs; subject to evidence-quality/source-maturity constraints.
- `evaluate_love_dynamics`, `profile_joulework`, `interpret_alignment`, and `default_governance_readiness_report` are deterministic advisory functions.
- `enqueue_bacon_lite` implements bounded async persistence to process-global paths resolved
  from `ARDA_ROOT`/the working directory plus explicit environment overrides;
  `record_bacon_lite_to` is the cold-path adapter with caller-supplied paths.
- `BaconLiteLogPaths` exposes explicit override paths instead of inferred paths from `CARGO_MANIFEST_DIR`.
- `global_governance_metrics().snapshot()` provides bounded-label in-process metrics; no server is started.
- `build_governance_status_report` produces read-only operator projection preserving `default_autonomy_ready = false`.
- `GameTheory::select_agent_with_policy` uses explicit fallback policy and reason.
- `collect_environmental_signals` pools bounded async NOAA/audio/vision futures; unavailable sources return neutral advisory evidence.
- `GovernanceSignalEnvelope` carries timestamp/freshness/confidence/quality/health as typed evidence.
- `AthenaStore::ingest_batch_with_environment` inserts environmental evidence into Varda receipts without acceptance changes.
- Compatibility contract is enforced via `tests/fixtures/public_api_v1.json` and `tests/public_api_compat.rs`.
- Deprecation contract: synthetic `calculate_resonance` paths are deprecated; new code routes through Triad/chain results or calls `calculate_resonance_without_governance` explicitly.

## Canonical public path

- Crate root re-exports: governance chain evaluation, config/path loaders, resonance/love/JouleWork scorers, evidence schema, Bacon-Lite persistence, metrics snapshot, status report, environmental signals, game-theory selection, and fallback rules.
- Primary integrations: `arda-varda` task/receipt governance; `arda-mandos` policy/outcome authority; `arda-orome` dispatch/governance hooks.

## Canonical contracts

- Governance configuration loaders use caller-owned paths. The process-global Bacon-Lite
  writer resolves its documented defaults from runtime environment/current-directory state,
  never from the build-time manifest path.
- Prometheus transport is caller-owned; `arda-aule` renders scrape-compatible text surface.
- Metrics labels are intentionally closed; raw strings never become labels.
- Compatibility contract: breaking changes require explicit new consumer impact review; new enum variants and fields are additive and backward-compatible.

## Implemented release-gate work

1. Bacon-Lite unit fixtures cover bounded-writer burst, saturation/latency, concurrent
   producers, restart recovery, malformed ledgers, rotation, and disk failure.
2. Love compatibility and JouleWork source/provenance boundaries are versioned, tested,
   and documented in `GOVERNANCE_PROVENANCE.md`; the project creator approved the narrow
   public algorithmic adaptations on 2026-07-25.
3. `arda-aule` exposes `governance-metrics` and `governance-status` in human and JSON forms;
   its `full-cli` all-target check, tests, and strict Clippy pass with process-level operator
   contract coverage.
4. Environmental fixtures cover quiet/storm/stale/unavailable inputs, NOAA parse/timeout/
   cache behavior, and advisory quality/freshness semantics.

## Retained compatibility boundaries

- Metrics transport and Prometheus exposition remain caller-owned.
- The human release approval applies only to the narrow algorithmic adapters documented in
  `GOVERNANCE_PROVENANCE.md`; it does not authorize copying upstream expression.
- Deprecated `calculate_resonance*` paths will be removed; consumers must migrate before 0.3.0.

## Status

Canonical public paths and release evidence are intact. Phase 9 is complete, as recorded in
`FIRST_CLASS_CHECKLIST.md` and `STATUS.md`.
