# arda-orome index

## Crate artifacts

- `Cargo.toml` — crate manifest
- `README.md` — public overview and integration guidance
- `BREAKDOWN.md` — module boundaries, invariants, and verified architecture
- `CHECKLIST.md` — completed HERMES implementation checklist
- `CRATE_PLAN.md` — implementation contracts and residual boundaries
- `STATUS.md` — current runtime and verification status
- `OWNERSHIP.md` — ownership constraints
- `src/` — crate implementation
- `tests/provider_orchestration.rs` — retry, timeout, expiry, fanout, fleet-policy tests
- `tests/governance_ledger.rs` — typed governance and ledger tests

## Cross-crate integration

- `crates/engine/src/orome.rs` — deterministic engine smoke package
- `crates/engine/tests/orome_smoke.rs` — compiled integration proof
- `apps/arda-hud/src/lib/ardaSource.ts` — human/core plan inventory
- `apps/arda-hud/src/lib/reviewGateDerivation.ts` — plan shelf projection

Purpose: canonical artifact and evidence index for `arda-orome`.

Review cadence is quarterly unless ownership, routing policy, or provider contracts change.
