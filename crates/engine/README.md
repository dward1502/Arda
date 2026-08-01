# arda-engine

`arda-engine` is the process-supervision and local harness library used by the
root `arda` daemon. It owns declarative service resolution, child supervision,
the operator tap-in HTTP surface, and narrow spine re-exports. The root daemon
owns startup sequencing and supplies runtime configuration.

## Public surface

- `registry::Registry`: load and resolve `services.toml`, including required,
  optional, and `--no-ui` service behavior.
- `supervisor::{Supervisor, Service, Shutdown}`: spawn, monitor, restart, and
  stop resolved child processes.
- `harness::{serve, HarnessState}`: local health/status, Manwe model proxy, and
  bounded Warden scout proxy routes.
- `observability::EngineObservabilityStatus`: aggregate loop/learning status.
- `orome` and `manwe`: supported provider/gateway integration surfaces.
- `arda_core::{loop_observability, service_registry}` re-exports.

There is intentionally no `boot()` function. The former function only logged
that dependencies linked and performed no initialization. The root daemon now
validates and resolves the real service registry before `--once` exits.

## Runtime configuration

- Harness default: `127.0.0.1:7878`.
- Harness override: `--harness-addr` or `ARDA_HARNESS_BIND_ADDR`.
- Manwe default supplied by the daemon: `http://127.0.0.1:7171`.
- Optional Manwe bearer: `ARDA_MANWE_PROXY_BEARER`.
- Warden scout: `ARDA_WARDEN_SCOUT_URL`, then `config/fleet.toml` discovery.
- Manwe proxy timeout: five seconds by default and explicitly owned by
  `HarnessState`.

## Verification

The 2026-07-28 first-class closeout passed formatting, no-default and
all-feature check/test, strict all-target Clippy, strict rustdoc, and root
`arda` consumer compilation. The suite contains 10 unit tests and 1 integration
test. See [STATUS.md](STATUS.md) for exact commands and [BREAKDOWN.md](BREAKDOWN.md)
for the complete source graph.
