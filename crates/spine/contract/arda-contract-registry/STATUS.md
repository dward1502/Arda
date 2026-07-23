arda-contract-registry crate status
===================================

local verification
------------------
- cargo check -p arda-contract-registry: passed
- cargo test -p arda-contract-registry: 3 passed, 0 failed
  - registry_smoke::registry_schema_version_is_pinned
  - registry_smoke::every_track_has_source_modules
  - registry_smoke::every_track_schema_version_is_present_in_a_source_or_surface_module
- cargo check -p arda-contract-registry --all-features: passed
- cargo test -p arda-contract-registry --all-features: 3 passed, 0 failed

health summary
--------------
active: arda-contract-registry v0.1.0
last reviewed: 2026-07-23

signals
-------
- schema-pinned contract manifest model: registry.rs
- live filesystem verification against declared source modules/surfaces: tests/registry_smoke.rs

open risks / notes
-----------------
- smoke tests depend on `core/state/contract_registry.json`; registry will not verify until Phase A artifact exists.
- tests also assert `source_modules` paths exist on disk; if those paths move, registry_smoke must be updated.
- this crate is not wired into a runtime path beyond governance/docs; it is a contract/schema source of truth.

