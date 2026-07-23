arda-contract-registry
======================

Canonical contract registry data model for Arda governance artifacts.

what it does
------------

- schema-pinned contract manifest model for tracks/source modules/evidence classes
- public API for `ContractRegistry` and `TrackDefinition`
- live verification tests proving declared schema versions exist in source/surface modules

public surface
--------------

- `arda_contract_registry::registry::ContractRegistry`
- `arda_contract_registry::registry::TrackDefinition`
- `ContractRegistry::track_ids()`

build / test
-----------

- cargo check -p arda-contract-registry
- cargo test -p arda-contract-registry

verification evidence
---------------------

- cargo check -p arda-contract-registry: passed
- cargo test -p arda-contract-registry: 3 passed, 0 failed
  - registry_smoke::registry_schema_version_is_pinned
  - registry_smoke::every_track_has_source_modules
  - registry_smoke::every_track_schema_version_is_present_in_a_source_or_surface_module

runtime notes
-------------

- registry schema is curated in repo state and verified by smoke tests.

connections
-----------

- compile time: `serde`, `serde_json`, `thiserror`
- test time: `tempfile`, `walkdir`

docs
----

See STATUS.md for build/test evidence and open risks.
See PLAN.md for current improvement backlog.
