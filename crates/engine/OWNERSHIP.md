# arda-engine ownership

## Engine-owned authority

`arda-engine` owns reusable daemon mechanics:

- parsing and resolving declarative `services.toml` entries;
- rejecting unresolved required services and filtering optional/UI services;
- child process supervision, restart/backoff, PID publication, and shutdown;
- the local harness router, listener lifecycle, Manwe proxy, and Warden scout proxy;
- bounded engine-level observability projections;
- the supported Manwe, Orome, and core-spine re-export surface.

## Root-daemon-owned authority

The root `arda` package owns process startup policy:

- CLI argument parsing;
- workspace root and registry path selection;
- the order in which registry validation, smoke exit, signal handling,
  supervision, and harness startup occur;
- environment/config discovery and construction of `HarnessState`;
- operator-facing startup and failure messages.

`--once` is a root-daemon smoke policy. It must call the real engine registry
loader/resolver, then exit before spawning children or binding the harness.

## Explicit non-ownership

- Manwe owns inference gateway behavior and port `7171`.
- Aule owns Prometheus exposition and telemetry aggregation.
- Orome owns provider dispatch behavior.
- Governance/core crates own policy and service-registry schemas.
- `arda-engine` does not own a hidden global `boot()` lifecycle and must not
  perform ambient mutation merely because it was linked.

## Change rule

New startup behavior belongs in the root daemon unless it is reusable process,
registry, harness, or observability mechanics. Every engine API change requires
focused crate tests and a root `arda` consumer check.
