# Root `arda` daemon package

Status: canonical composition root verified locally on 2026-08-04
Package: workspace root `arda` binary
Entrypoint: [`src/main.rs`](../src/main.rs)
Direct workspace dependency: [`arda-engine`](../crates/engine/README.md)

## Purpose

The root package is deliberately a composition boundary, not a second runtime
implementation. It discovers the repository, loads and validates the service
registry, projects endpoint configuration into the engine harness, starts the
engine-owned harness and supervisor, and coordinates shutdown.

## Startup sequence

1. Parse `--once`, `--no-ui`, and `--harness-addr`.
2. Discover the nearest ancestor containing `services.toml`, falling back to the
   workspace manifest directory only when no ancestor marker exists.
3. Load `services.toml` from that discovered root. Registry parsing, command
   resolution, required/optional failures, and UI-tag filtering are delegated to
   `arda-engine`.
4. Fail before startup when any required service cannot resolve.
5. If `--once` is set, report the validated service count and return before
   constructing the harness or supervisor and before spawning any child.
6. Discover the Warden scout URL and create the read-only harness state.
7. Start the harness and resolved service supervisor.
8. On SIGINT/Ctrl-C, notify both components, reap supervised children, await the
   harness task, and exit.

## Endpoint discovery

Warden scout precedence is:

1. nonempty `ARDA_WARDEN_SCOUT_URL`;
2. `scout_url` on the first `config/fleet.toml` node whose ID contains
   `warden` (case-insensitive);
3. unavailable (`null`) when neither source exists.

The root harness projects the coordinated Manwe endpoint
`http://127.0.0.1:7171`. The root-owned Manwe process binds `0.0.0.0:7171` so
verified local and fleet consumers share one contract.

## CLI semantics

| Option | Contract |
|---|---|
| `--once` | Load, parse, validate, resolve, and log the registry; start no harness, supervisor, or child process. |
| `--no-ui` | Exclude services tagged `ui` before required-service error accounting; headless services remain required. |
| `--harness-addr <ADDR>` | Bind the engine harness to the supplied address; default is `127.0.0.1:8088`. |

There is no supported harness-only ownership profile. The removed
`--harness-only` flag is covered by a CLI rejection test.

Maintained configuration smoke:

```text
cargo build -p arda
./target/debug/arda --once --no-ui
```

The verified workspace registry resolves exactly one required headless service,
`manwe`. Without `--once`, the root daemon owns that process for its full
lifecycle.

## Source breakdown

| Path | Responsibility |
|---|---|
| [`src/main.rs`](../src/main.rs) | CLI, root discovery, composition, endpoint projection, startup ordering, and shutdown coordination. |
| [`tests/root_daemon.rs`](../tests/root_daemon.rs) | CLI-level composition tests using isolated registries, scripts, ports, and fleet fixtures. |
| [`services.toml`](../services.toml) | Declarative required/optional process registry; currently owns required Manwe. |
| [`config/fleet.toml`](../config/fleet.toml) | Fleet-owned fallback source for Warden scout discovery; unchanged by Packet 8. |
| [`crates/engine/src/registry.rs`](../crates/engine/src/registry.rs) | Registry schema, validation, command resolution, and `--no-ui` filtering. |
| [`crates/engine/src/harness.rs`](../crates/engine/src/harness.rs) | HTTP status/model/scout projection and graceful harness shutdown. |
| [`crates/engine/src/supervisor.rs`](../crates/engine/src/supervisor.rs) | Child spawn, monitoring, shutdown, and reaping. |

## Ownership boundary

### Owned by the root package

- Composition order and fail-before-spawn behavior.
- Repository-root-relative registry loading.
- CLI semantics and harness bind selection.
- Warden environment/fleet precedence projection.
- Signal fan-out and task joining.

### Delegated to `arda-engine`

- Registry parsing and service-resolution policy.
- Harness routes and response behavior.
- Process spawn, watch, termination, and reaping.
- Shared shutdown/status primitives.

### Not owned here

- Service declarations and executable details (`services.toml`).
- Fleet inventory truth (`config/fleet.toml`).
- Manwe routing implementation or endpoint migration.
- Launcher/HUD process behavior.
- Scout, queue, approval, or governance authority.

## Verification evidence

Package-local:

- 5 root integration tests pass serially.
- Tests prove ancestor-root registry discovery, invalid-registry rejection,
  required-service failure, `--no-ui`, no spawn under `--once`, live harness
  startup, environment-over-fleet Warden precedence, SIGINT shutdown, and child
  reaping.
- `cargo check -p arda --all-targets --all-features` passes.
- `cargo clippy -p arda --all-targets --all-features -- -D warnings` passes.
- Warning-denied package Rustdoc passes.
- The maintained real-registry smoke passes with one resolved headless service.

Direct dependency:

- `arda-engine` all-target/all-feature check passes.
- 25 engine unit tests plus integration suites pass, including readiness,
  bounded restart state, supervisor reaping, and root harness forwarding.
- Strict combined U1 Clippy currently stops on an unrelated pre-existing
  `clippy::unnecessary_to_owned` warning in
  `crates/engine/src/harness/research.rs:601`; the focused builds and test
  suites pass.

U1 cross-package evidence:

- `cargo build -p arda -p manwe --all-features` passes.
- `cargo test -p manwe --all-features -- --test-threads=1` passes: 281 library
  and 3 binary tests.
- `cargo test --test root_daemon -- --test-threads=1` passes 5/5.
- Manwe single-process smoke and documentation validation pass.
- Formatting and `git diff --check` pass.

## Canonical process ownership

`config/systemd/arda.service` starts the root daemon. `arda-manwe.service` is an
alias to that unit rather than an independent service definition. The source
topology therefore has one owner for Manwe startup, health, restart, shutdown,
and recovery. Installation over a currently running legacy user session is U4
installation/recovery scope.

## Packet 8 repair (historical)

The daemon already discovered an ancestor repository root but loaded bare
`services.toml` relative to the process working directory. Packet 8 changed the
load to `root.join("services.toml")`. Running `arda --once` from a nested
repository directory is now covered and passes. No changes to `services.toml`
or `config/fleet.toml` were required.
