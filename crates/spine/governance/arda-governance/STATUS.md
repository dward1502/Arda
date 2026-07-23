# arda-governance status

Crate: `crates/spine/governance/arda-governance`
Current state: active
Branch: `manwe`
Test evidence: `cargo test`
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md` (missing).
Governance and memory integration scenario: `arda-governance`/`arda-vaire` test exists and asserts expected.

Current signature: Triad/chain evaluation and typed evidence assessment active; backwards-compatible compatibility contract enforced via `tests/fixtures/public_api_v1.json`.
Open evidence: environmental-source regression fixtures and producer-rate/backpressure tests remain incomplete.
Deprecated `calculate_resonance*` paths will be removed before 0.3.0; migration required in consumers.

Operational expectation: verify with `cargo test` and `tests/fixtures/public_api_v1.json` compatibility check before production use.

See `CRATE_PLAN.md` and `OWNERSHIP.md` for implementation priorities.
