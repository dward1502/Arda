---
soterion:
  sigil: "REPAIR"
  role: "crate_breakdown"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-04"
---

# manwe — Crate Breakdown

Path: `crates/spine/runtime/manwe`

## Purpose

Manwe is Arda's single inference boundary. It keeps provider enrollment,
capability truth, health, quotas, resource ownership, selection rationale, and
route evidence behind one OpenAI-compatible port.

## Package shape

`Cargo.toml` defines the `manwe` library and binary. The default `adaptive`
feature is mandatory for the binary. `telemetry` optionally adds Aule/OTLP
emission. The former selectable static and tonic gRPC runtime features were
retired during single-runtime convergence.

## Active binary graph

`src/main.rs` is a thin process shell:

1. parse bind, port, and compatibility flags;
2. validate the endpoint configuration;
3. construct `adaptive::service::ManweService`;
4. reload the governed provider catalog; and
5. start `adaptive::transport::http`.

The removed root modules `provider.rs`, `receipts.rs`, `resource_limits.rs`, and
`grpc.rs` belonged only to the retired parallel process. Their live governed
counterparts remain under `adaptive/service/full/` and the adaptive transport.

## Active library graph

| Module | Role |
|---|---|
| `config.rs` | Public endpoint/config compatibility model and state-root helpers |
| `error.rs` | Public error taxonomy |
| `routing_adapter.rs` | Stable adapter facade |
| `types.rs` | Canonical request, provider, model, route, and governance types |
| `adaptive/` | Governed runtime, drivers, selection, evidence, and transports |

The active service implementation is attached by
`adaptive/service/full_service.rs` and grouped under
`adaptive/service/full/`:

- bootstrap/state: catalog bootstrap, runtime state, state I/O and mutation;
- routing: classification, scoring, candidate selection, sessions and caches;
- execution: provider drivers, health probes, quotas and resource limits;
- evidence: capability receipts, lane fitness, bandit state, route receipts,
  tool-fit observations, metrics and operator diagnostics;
- governance: realm policy, Echo Gate and typed route-governance receipts.

## Canonical process ownership

`services.toml` declares Manwe as the required gateway at port `7171`.
`arda-engine::registry` resolves that declaration and the root `arda` supervisor
owns its PID, readiness probe, bounded restart, and shutdown. The root harness
publishes required/optional, lifecycle, PID, restart count, backoff, and detail.

The repository `arda.service` unit launches the root daemon, not a second Manwe
process. Installing it creates `arda-manwe.service` only as a compatibility
alias to the same root owner.

## Consumers

Coordinated consumers include the root harness, HUD/Tauri backend, Aule
Prometheus topology, central Prometheus target, Hermes bridge, offsite operator
profile, SELinux runtime contract, local profile template, and production smoke
probe. Canonical source defaults use `7171`; historical reports retain prior
`5110` observations as evidence rather than launch authority.

## Persisted state

| Surface | Location |
|---|---|
| Provider catalog | `config/manwe.providers.toml` or configured override |
| Runtime state | `data/manwe/` or configured state root |
| Route/governance events | `state.jsonl`, `governance_events.jsonl` |
| Capability/tool evidence | `provider_capability_receipts.json`, `tool_fit_ledger.jsonl` |
| Fitness learning | `lane_fitness.json`, `bandit.json` |

## Maintained gates

The mandatory gates are all-feature check/clippy/test, formatting, the single
process smoke, documentation validation, Arda engine tests, and root-daemon
integration tests. See [`STATUS.md`](STATUS.md) for the latest observed counts.
