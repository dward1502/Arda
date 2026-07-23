---
soterion:
  sigil: "REPAIR"
  role: "crate_breakdown"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---

# manwe — Crate Breakdown

Path: `crates/spine/runtime/manwe`

## Purpose

Manwe is Arda's local inference boundary. It gives callers one
OpenAI-compatible gateway while provider enrollment, health, model identity,
resource ownership, and routing policy evolve behind it.

The crate contains two intentionally selectable runtime modes:

1. A production-shaped binary gateway in `src/main.rs`, backed by
   `config/fleet.toml` and a lightweight deterministic provider catalog.
2. A feature-gated full governed service under `src/adaptive/service/`, with
   policy, governance, quotas, persistence, provider drivers, observability,
   and the adaptive HTTP transport.

The Cargo `adaptive` feature compiles both; `--adaptive` selects and
instantiates the full governed service while the default remains static.

## Package shape

`Cargo.toml` defines both:

- library: `manwe` from `src/lib.rs`
- binary: `manwe` from `src/main.rs`

Features:

| Feature | Dependencies / surface |
|---|---|
| default | HTTP gateway plus core public types/config |
| `adaptive` | `arda-core`, `arda-governance`, `arda-economics`, `arda-vaire`; full adaptive library tree |
| `grpc` | `arda-orome`; tonic HTTP-adjacent services |
| `telemetry` | Intended `arda-aule/telemetry` service events; currently fails to compile |

## Active binary graph

`src/main.rs` directly includes only:

| Module | Role |
|---|---|
| `config.rs` | `manwe.toml`, embedded Ollama fallback, provider resolution, startup validation |
| `provider.rs` | Fleet TOML parsing, provider probes, eligibility, adaptive-lite selection diagnostics |
| `receipts.rs` | JSONL route receipts and quality/throughput extraction |
| `resource_limits.rs` | Per-resource-group concurrency and queue timeout |
| `grpc.rs` | Optional tonic server, included only by `grpc` |

The binary starts with a `config/fleet.toml` catalog, probes providers, refreshes
that catalog every 60 seconds, and falls back to `ManweConfig` forwarding when
the fleet catalog cannot satisfy a request.

## HTTP request path

1. Parse `model`; infer `chat`, `code`, or `vision` task shape and a context
   estimate.
2. Resolve an eligible fleet provider. Adaptive-lite mode allows automatic
   task/context/capability selection and emits rejection diagnostics.
3. Acquire the provider's physical resource-group lease.
4. Forward to `<base_url>/chat/completions`, injecting a configured API key.
5. Return provider/model/resource/routing headers and append a route receipt.
6. If no fleet provider matches, attempt the static `manwe.toml` provider path;
   otherwise return a structured 502/503 response.

## Active library graph

`src/lib.rs` includes:

| Module | Role |
|---|---|
| `config.rs` | Public `ManweConfig` |
| `error.rs` | Public error taxonomy and adaptive `ArdaError` conversion |
| `routing_adapter.rs` | Stable adapter name; stub without `adaptive`, real re-export with it |
| `types.rs` | Canonical request, provider, model, route, and governance types |
| `adaptive/` | Rich service tree when `adaptive` is enabled |

The adaptive library's active service tree groups into:

- bootstrap/state: `bootstrap*`, `runtime_state`, `state_io`, `state_mutation`
- routing: `route_policy`, `route_scoring`, `route_selection`,
  `adaptive_routing`, `route_sessions`, `route_candidate_cache`
- provider execution: `proxy`, `health_probe`, `http_clients`,
  `catalog_reconciliation`, `provider_admin`, `capabilities`
- alternate drivers: `codex_responses_driver`, `hermes_cli_driver`,
  `hermes_proxy_driver`
- control/evidence: `agent_quotas`, `bandit`, `echo_gate`, `event_writer`,
  `metrics`, `observability`, `service_events`, `status`

`src/types.rs` is the canonical domain model; adaptive code re-exports it
through `src/adaptive/types.rs`.

## Source-graph reconciliation

Workspace-wide symbol and module-declaration searches found no consumers for
the seven formerly unattached root files. Each was removable migration residue:

| Removed file | Classification evidence |
|---|---|
| `charon_remote.rs` | Superseded remote/gateway bridge; depended on undeclared `ManweTransport` and gateway records |
| `gateway.rs` | Superseded gateway record model; no consumer outside its paired dead bridge |
| `grpc_types.rs` | Superseded state adapter; the live gRPC server is `src/grpc.rs` |
| `route.rs` | Placeholder authority traits whose default behavior was `NotImplemented` |
| `routing.rs` | Old adaptive feature shim; superseded by root `routing_adapter.rs` and the governed service |
| `routing_types.rs` | State types used only by removed `grpc_types.rs` |
| `support.rs` | Old facade referencing an undeclared root transport module |

The parallel files formerly directly under `src/adaptive/` (`adapters.rs`,
`admin.rs`, `bandit.rs`, `drivers.rs`, `fallback.rs`, `policy.rs`, `provider.rs`,
`quota.rs`, `selector.rs`, `session.rs`, and `state.rs`) were also removable
residue. Several referenced missing `candidate` or `score` modules, and none was
declared by `adaptive/mod.rs`; their implemented counterparts live under
`adaptive/service/full/`. The direct `adaptive/error.rs`,
`adaptive/routing_adapter.rs`, and `adaptive/types.rs` files remain active
boundary modules. `adaptive/types.rs` continues to re-export the canonical
public model from `src/types.rs`.

`adaptive/transport/` is not a future or obsolete tree: it is attached by
`adaptive/mod.rs`, and `src/main.rs` starts its governed HTTP server for
`--adaptive`. Its HTTP routes are therefore active runtime endpoints. The
daemon/IPC pieces compile as an intentionally retained compatibility surface,
but are not advertised as endpoints of the canonical binary.

`adaptive/service/fleet_persistence.rs` was removable residue. Its snapshot and
admission-receipt API had no consumer, was not declared by the service, and
duplicated persistence responsibilities now owned by the live bootstrap,
runtime-state, and event paths. It was removed rather than exposing an unused
second persistence contract.

## Configuration and persisted state

| Surface | Current location |
|---|---|
| Static fallback config | CLI `--config`, default `manwe.toml` |
| Binary fleet catalog | `config/fleet.toml` |
| Binary receipts | `data/manwe/route_receipts.jsonl` |
| Adaptive provider config | `ARDA_MANWE_PROVIDER_CONFIG` or adaptive defaults |
| Adaptive state root | `ARDA_MANWE_HOME` or service root |

The static and fleet catalogs are separate inputs. Their precedence and
operator-facing relationship need one canonical configuration contract.

## Consumers and operational wiring

- `crates/engine/Cargo.toml` depends on `manwe`.
- `crates/engine/src/manwe.rs` re-exports the library.
- `crates/engine/src/harness.rs` proxies its `/v1/models` route to Manwe.
- `apps/arda-launcher` discovers the Manwe base URL and reports a missing
  onboarding gate when none is configured.
- `services.toml` reserves Manwe as a required gateway at port 7171 and starts
  the canonical process with `cargo run -p manwe -- --config manwe.toml`.
- `arda-engine::registry::Registry` consumes the manifest's singular
  `[[service]]` command/cwd schema and its supervisor launches the resolved
  command from the declared working directory.

Port `7171` remains a coordinated contract. Change it only with engine,
launcher, service-registry, and operator configuration updates.

## Architecture decision

Keep the library and binary in one crate for now. Existing consumers depend on
the library types, and no separate deployment shell has a complete migration
path. Before considering a split, first unify or explicitly separate the
binary's lightweight router and the adaptive service runtime.

## Current engineering priorities

1. Close the known lane-fitness, streaming, and gRPC process-test gaps.
2. Repair the telemetry feature contract so all-feature CI can pass.
3. Refresh the remaining stale provider and source-index documentation.

Execution details and completion criteria are in [`CHECKLIST.md`](CHECKLIST.md).