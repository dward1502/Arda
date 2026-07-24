arda-engine plan closeout
========================
last_reviewed: 2026-07-23

summary
-------
The initial mismatch was between docs/surface and live code.
Live review showed the harness proxy path had no explicit timeout, no bearer
forwarding path, and stale breakpoint/docs references.

completed work
--------------
- align harness proxy path
  - add explicit reqwest timeout + optional bearer forwarding
  - surface `ARDA_HARNESS_BIND_ADDR` override with documented default
- close crate-doc hygiene loop
  - update CHECKLIST.md with verification evidence
  - update STATUS.md/README.md after edits
  - remove/completed items in PLAN.md into this closeout note
- fix build/test tooling
  - fix edition lint issue + remove temporary TODO/test scaffold workaround
  - verify with `cargo check/test` + feature-scoped `check/test`
- add gen3 observability/interop surface
  - add `src/observability.rs` with `EngineObservabilityStatus`
  - re-export `arda-core::loop_observability` from `lib.rs`
  - wire arda-aule loop/learning interop consumers

verification evidence
---------------------
✅ cargo check -p arda-engine
✅ cargo check -p arda-engine --all-features
✅ cargo test -p arda-engine
✅ cargo test -p arda-engine --all-features
   - 6 passed; 0 failed
   - empty_registry_is_rejected_instead_of_silently_supervising_nothing
   - missing_command_for_required_service_is_reported_as_error
   - missing_command_for_optional_service_drops_service_with_no_error
   - workspace_registry_declares_canonical_manwe_process
   - no_ui_keeps_manwe_and_drops_ui_services
   - supervises_and_reaps_child_on_shutdown

remaining risk / notes
----------------------
- harness.rs had pre-existing axum/handler-macro edition friction; resolved via
  2024 edition bump and removing the temporary no-op test scaffold.
- boot()/client contract docs were stale; no active remaining callers were
  found, and the docs were aligned to current surface.
- arda-core interop is now evidence-backed through arda-aule consumers and
  arda-engine aggregation; remaining scenarios are in docs/interop/landscape.md.
