# arda-governance status

Crate: `crates/spine/governance/arda-governance`
Version: `0.1.0`
State: **stable for current scope**
Reviewed: 2026-07-26
Required crate-local work: **none**

## Current implementation

- Deterministic Triad/chain evaluation and versioned structured evidence assessment.
- Async-first local and optional provider-neutral LLM scorer receipts.
- Validated realm/action policy with atomic reload audit.
- One readiness/review/rollback/operator-gated runtime blocking authority.
- Resonance, Love task-value compatibility, canonical Love Dynamics, JouleWork,
  Nonconformist Bee, Empirical Distrust, philosopher arbitration, and game-theory selection.
- Bounded Bacon-Lite persistence, recovery, ledger summaries, metrics, and operator reports.
- Typed audio/vision/solar environmental evidence with advisory-only semantics.
- Public wire compatibility guarded by `tests/fixtures/public_api_v1.json`.

## Stability assessment

- No unfinished crate-local checklist or known failing gate remains.
- Default governance readiness is deliberately **not autonomy-ready**. Blocking remains off
  without independent review receipts, scoped evidence, rollback proof, and operator control.
  This is the intended safety contract, not an incomplete implementation.
- `calculate_resonance` and `calculate_resonance_basic` remain deprecated compatibility paths;
  their eventual 0.3.0 removal is future migration work, not a current stability blocker.
- Production persistence should use `enqueue_bacon_lite`; synchronous recording is for tests,
  migrations, and explicitly cold paths.

## Verification evidence

Passed from the workspace root on 2026-07-26:

- `cargo fmt -p arda-governance -- --check`.
- `cargo test -p arda-governance --all-features`: 117 passed
  (67 unit, 47 integration, 3 doctests).
- `cargo clippy -p arda-governance --all-targets --all-features -- -D warnings`.
- `cargo doc -p arda-governance --no-deps --all-features`.

Cargo emitted only the workspace's existing informational warning that a non-root package
profile in `apps/arda-launcher/src-tauri/Cargo.toml` is ignored. It did not affect this crate.

## Direct consumers

`manwe`, `arda-aule`, `arda-varda`, `arda-mandos`, `arda-orome`, `arda-economics`, and
`arda-vaire` declare direct dependencies. Consumer-specific historical release evidence is
preserved in git history; this file reports only the current crate-local verification run.

## Documentation contract

- Start with `README.md` for the public boundary and API map.
- Use `BREAKDOWN.md` for implementation structure and integration ownership.
- Use `PLAN.md` for future discussion and proposal decisions.
- Use `GOVERNANCE_PROVENANCE.md` for source/adaptation provenance.
- Use `OWNERSHIP.md` for authority boundaries.
