# arda-economics status

Crate: `crates/spine/runtime/arda-economics`
Current state: active
Branch: `manwe`
Test evidence: `cargo test`
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md`.

Current signature: IPC/HTTP transport surfaces active; runtime economics engine owns mutation, valuation, and energy-meter estimates.
Open evidence: finance metrics export for `PlutusRuntimeEvent` streams and long-running JouleWork/Mandos-validated invariant regression tests remain incomplete.
Governance metric stream export requires caller-side verification from `arda-aule`.

Operational expectation: verify with `cargo test` and JouleWork pluto-stream audit before production use.

See `CRATE_PLAN.md` and `OWNERSHIP.md` for implementation priorities.
