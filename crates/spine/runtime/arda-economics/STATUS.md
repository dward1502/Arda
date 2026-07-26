# arda-economics status

Crate: `crates/spine/runtime/arda-economics`
Current state: active
Branch: `manwe`
Test evidence: `cargo check`, `cargo test`
Known rustc warning: `profiles for the non root package will be ignored, specify profiles at the workspace root:` from `apps/arda-launcher/src-tauri/Cargo.toml` — not from `arda-economics`.
Documentation set: `README.md`, `BREAKDOWN.md`, `CHECKLIST.md`, `CRATE_PLAN.md`, `OWNERSHIP.md`, `STATUS.md`.

Current signature: IPC/HTTP transport surfaces active; runtime economics engine owns mutation, valuation, and energy-meter estimates.
Completed: additive `transport/finance_stream.rs` finance metrics export for `PlutusRuntimeEvent` streams; `PlutusLedger` enriched with credit totals and last-credit provenance.
Deferred: failed hardware/estimator fallback integration coverage and long-running JouleWork measurement-source invariant regression; see `CRATE_PLAN.md` for triggers.
Open evidence: caller-side verification from `arda-aule` for governance metric stream export remains unfinished.

Operational expectation: verify with `cargo test` and JouleWork pluto-stream audit before production use.

See `CRATE_PLAN.md` and `OWNERSHIP.md` for implementation priorities.
