arda-engine
===========

Single dependency surface for the `arda` daemon to reach system services.

what it does
------------

- process supervision with restart and exponential backoff
- declarative service discovery from `services.toml`
- harness HTTP control surface on `127.0.0.1:7878`
- `/v1/models` proxy to `manwe` with explicit reqwest timeout + optional bearer forwarding
- bind address override via `ARDA_HARNESS_BIND_ADDR`
- public spine re-exports for `manwe` and `arda-core::service_registry`

public surface
--------------

- `arda_engine::Registry` / `Registry::load(path)` / `Registry::resolve(root, no_ui)`
- `arda_engine::Supervisor` / `Shutdown`
- `arda_engine::harness::serve(addr, state, shutdown)`
- `arda_engine::harness::DEFAULT_MANWE_PROXY_TIMEOUT`
- `arda_engine::manwe::{...}`
- `arda_engine::service_registry`
- `arda_engine::boot()`

build / test
-----------

- cargo check -p arda-engine
- cargo test -p arda-engine

verification evidence
---------------------

- cargo check -p arda-engine: passed
- cargo test -p arda-engine: 6 passed, 0 failed
  - registry::tests::empty_registry_is_rejected_instead_of_silently_supervising_nothing
  - registry::tests::workspace_registry_declares_canonical_manwe_process
  - registry::tests::no_ui_keeps_manwe_and_drops_ui_services
  - registry::tests::missing_command_for_required_service_is_reported_as_error
  - registry::tests::missing_command_for_optional_service_drops_service_with_no_error
  - supervisor::tests::supervises_and_reaps_child_on_shutdown

runtime notes
-------------

- harness emits `/v1/models` with optional `Authorization` bearer forwarding
- proxy timeout is 5s unless `HarnessState::manwe_proxy_timeout` is set
- bind address default is `127.0.0.1:7878`; env override is `ARDA_HARNESS_BIND_ADDR`

connections
-----------

- runtime: proxies `/v1/models` to `manwe` at `127.0.0.1:7171`
- compile time: depends on `arda-core`, `manwe`, `tokio`, `axum`, `reqwest`, `serde`, `toml`

docs
----

See STATUS.md for build/test evidence and open risks.
See PLAN.md for current improvement backlog.
