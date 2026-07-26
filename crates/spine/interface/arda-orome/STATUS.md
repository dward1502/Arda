# arda-orome status

Crate: `crates/spine/interface/arda-orome`
Current state: active; checklist complete
Branch: `manwe`
Last verified: 2026-07-25
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `INDEX.md`.

## Runtime state

- Provider runtime supports bounded timeout/retry, request expiry, streaming receipts, typed direct/fanout routing, metrics, and explicit fleet-scope policy.
- Central `GovernanceHooks` records typed approval/interruption decisions through `arda_core::Ledger`.
- `arda-engine::orome::manual_smoke_dispatch` compiles and executes a deterministic no-network dispatch path.
- ARDA HUD derives and consumes both human-plan and core-plan roots.

## Verification

- `cargo test -p arda-orome`: 21 passed, 0 failed.
- `cargo test -p arda-engine --test orome_smoke`: 1 passed, 0 failed.
- Scoped Rust formatting check passes for `arda-orome` and `arda-engine`.

## Operational boundary

Real provider credentials/endpoints remain deployment concerns. External fleet dispatch is disabled by default and remains approval-gated when explicitly enabled. Manwe owns provider selection and inference routing policy.
