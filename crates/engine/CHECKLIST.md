CHECKLIST
=========
last_reviewed: 2026-07-23

completed
---------
- [x] bring BREAKDOWN.md verification status in line with live test evidence
      evidence: arda-engine `cargo test --all-features` passes 6/6; see STATUS.md
- [x] add `/v1/models` proxy timeout/auth path
      evidence: `HarnessState` now carries `client`, `manwe_proxy_timeout`,
      and optional `manwe_proxy_bearer`; `/v1/models` uses reqwest timeout +
      optional `Authorization` forwarding.
- [x] add README + harness docs for bind env override
      evidence: README.md and harness.rs doc updated; `ARDA_HARNESS_BIND_ADDR`
      documented with default `127.0.0.1:7878`.
- [x] fix edition/axum lint issue after adding handler-macro workaround/no-op cleanup
      evidence: `edition = "2024"`; `cargo check --all-features` clean.
- [x] remove edition-alignment TODO and restore any commented test scaffolding
      evidence: temporary TODO removed in harness.rs.
- [x] final verification pass before crate-doc hygiene closeout
      evidence: `cargo check/test` and `cargo check/test --all-features` both green.

deferred / not pursued
-----------------------
- review dev-dependency footprint: engine Cargo.toml dev-deps are currently
  limited to harness test surface (`serde_json`, etc.); no extra inferred deps.
- update `boot()` usage contract: `boot()` was removed from the surface under
  this review; no remaining callers to document.
