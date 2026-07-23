# arda-mandos status

Crate: `crates/spine/runtime/arda-mandos`
Current state: active
Branch: `manwe`
Test evidence: 52 passing tests under all-features/no-default-features; crate-local strict Clippy passes
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md`.

Current signature: Oracle engine evaluates once per normalized query, writes auditable verdicts before exposure, and exposes typed evidence provenance and reasoning context.
Open evidence: HTTP status semantics, Unicode-safe notifications, persistence authority, and escalation disposition docs remain pending persistence/transport follow-ups.
Prometheus exposition remains caller/`arda-aule` owned.

Operational expectation: verify with `cargo test` before production use.
See `CRATE_PLAN.md` and `OWNERSHIP.md` for implementation priorities.
