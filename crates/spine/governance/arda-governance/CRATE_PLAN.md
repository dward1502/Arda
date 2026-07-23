# arda-governance implementation plan

Source: `crates/spine/governance/arda-governance`, `README.md`, `BREAKDOWN.md`, live code, consumers in `arda-varda`, `arda-mandos`, `arda-orome`.

## Baseline evidence

Live implementation exists and is active in workspace consumers:
- `triad_validate`, `evaluate_governance_chain`, `assess_governance_evidence`, and `assess_governance_evidence_typed` form typed deterministic gates with no I/O on hot paths.
- `load_governance_chain[_from_str]`, `load_philosopher_profiles[_from_str]`, and `GovernancePaths` provide explicit path-based filesystem access with preserved read/parse/validation error classes.
- `calculate_resonance*` functions cover governed and policy-absent ecosystems with normalized `[0,255]` outputs; subject to evidence-quality/source-maturity constraints.
- `evaluate_love_dynamics`, `profile_joulework`, `interpret_alignment`, and `default_governance_readiness_report` are deterministic advisory functions.
- `enqueue_bacon_lite` and `record_bacon_lite_to` implement bounded async/cold-path persistence with caller-owned paths.
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

- Governance evaluation is caller-owned configuration and path; library does not infer repository root from current process state.
- Prometheus transport is caller-owned; `arda-aule` renders scrape-compatible text surface.
- Metrics labels are intentionally closed; raw strings never become labels.
- Compatibility contract: breaking changes require explicit new consumer impact review; new enum variants and fields are additive and backward-compatible.

## Implementation work

1. Add evidence-bearing integration coverage for producer-rate/backpressure conditions under async bounded writer saturation.
2. Validate Love Equation unit multipliers and JouleWork source-provenance invariants across long-running sessions in coordination with `arda-economics`.
3. Add operator-level export commands for `governance-metrics` and `governance-status` via `arda-aule` as explicit human/JSON/JSON paths.
4. Add environmental-source health/freshness/measurement-quality regression fixtures so degraded/unavailable states remain reproducible across NOAA failures.

## Open risk items

- Metrics transport and Prometheus exposition remain caller-owned; visual verification requires production-consumer validation before governance status drops assume unpromoted metrics.
- Integration tests for failed hardware backends and estimator fallback combinations exist in adjacent crates; sharing or calling them from governance services needs explicit test wiring.
- Deprecated `calculate_resonance*` paths will be removed; consumers must migrate before 0.3.0.

## Status

Canonical public path is intact; implementation evidence is active and works continues on environmental/regression coverage and operator stream export.
