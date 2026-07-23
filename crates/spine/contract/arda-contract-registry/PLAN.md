arda-contract-registry plan closeout
====================================
last_reviewed: 2026-07-23

summary
-------
This repo did not yet have live crate-doc artifacts for `arda-contract-registry`.
Initial gap: no `BREAKDOWN.md`, `STATUS.md`, `README.md`, `INDEX.md`,
`PLAN.md`, or `CHECKLIST.md` existed at top level.

completed work
--------------
- create crate-doc hygiene loop
  - add `BREAKDOWN.md`, `STATUS.md`, `README.md`, `INDEX.md`,
    `PLAN.md`, `CHECKLIST.md`
  - model purpose/layout/dependencies and verification evidence
  - document risk that smoke tests depend on `core/state/contract_registry.json`
- add to workspace
  - add `crates/spine/contract/arda-contract-registry` to root `Cargo.toml` `members`
- verify
  - add `cargo check -p arda-contract-registry`: passed
  - add `cargo test -p arda-contract-registry`: 3 passed
  - add `cargo check/test --all-features`: passed

verification evidence
---------------------
✅ cargo check -p arda-contract-registry
✅ cargo test -p arda-contract-registry
   - registry_schema_version_is_pinned
   - every_track_has_source_modules
   - every_track_schema_version_is_present_in_a_source_or_surface_module
✅ cargo check -p arda-contract-registry --all-features
✅ cargo test -p arda-contract-registry --all-features

remaining risk / notes
-----------------------
- smoke tests will hard-fail just with stable `core/state/contract_registry.json`.
- `tests/registry_smoke.rs` depends on `walkdir` and `tempfile` dev-dependencies.
