arda-engine crate status
=======================

local verification
------------------

- cargo check -p arda-engine: passed
- cargo test -p arda-engine: 6 passed, 0 failed
  - registry::tests::empty_registry_is_rejected_instead_of_silently_supervising_nothing
  - registry::tests::workspace_registry_declares_canonical_manwe_process
  - registry::tests::no_ui_keeps_manwe_and_drops_ui_services
  - registry::tests::missing_command_for_optional_service_drops_service_with_no_error
  - registry::tests::missing_command_for_required_service_is_reported_as_error
  - supervisor::tests::supervises_and_reaps_child_on_shutdown
- cargo check -p arda-engine --all-features: passed
- cargo test -p arda-engine --all-features: 6 passed, 0 failed

health summary
--------------

active: arda-engine v0.1.0
last reviewed: 2026-07-17

signals
-------

- process supervision + restart/backoff: supervisor.rs
- declarative service discovery: registry.rs
- harness HTTP tap-in: harness.rs @127.0.0.1:7878
- `/v1/models` proxy to manwe: harness.rs with explicit reqwest timeout + optional bearer forward
- spine re-exports: lib.rs / manwe.rs

open risks
----------

- boot() is currently a placeholder
- harness `/v1/models` proxy is an unauthenticated `reqwest::Client::get()`; no timeout, no URI validation, no fallback if manwe is down
- 2s PID mirror poll loop is coarse for tight monitoring windows
- dev-dependency on tonic/prost is unused by this crate surface and may distract from actual test surface

plan
----

see PLAN.md
