---
soterion:
  sigil: "SCROLL"
  glyph: "𓊝"
  role: "inference_gateway"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---

# manwe

Local OpenAI-compatible inference gateway and runtime hault for the Arda
charon/adaptive inference surface. Provides a frozen static root at
`127.0.0.1:7171` plus a feature-gated adaptive subtree.

Owner: hades | Sigil: 🜏 SCROLL | Status: active

## Summary

`manwe` replaces the legacy hosted multi-process `annunimas-charon` runtime
with a single local gateway root. The default surface is intentionally thin:
static TOML-backed provider catalog, upstream forwarding, and admin-style
endpoints. Active adaptive logic lives under `src/adaptive/`, but is
currently deferred for baseline rebuild because the service tree does not
compile cleanly.

## Where it lives

- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/runtime/manwe`
- Config: `manwe.toml` next to binary/config roots; embedded default goes to
  local Ollama (`http://127.0.0.1:11434/v1`)
- Runtime bind: `127.0.0.1:7171`
- Endpoints: `GET /healthz`, `GET /v1/models`, `POST /v1/chat/completions`

## Verification status

- `cargo check -p manwe`: PASS
- `cargo test -p manwe`: PASS
- `cargo check -p manwe --features adaptive`: PASS
- `cargo test -p manwe --features adaptive`: PASS
- `cargo fmt -p manwe -- --check`: PASS

Behavioral evidence to date: see `BREAKDOWN.md` + `STATUS.md`
`2026-07-21/22` validation.

## Adaptive routing notes

The `adaptive` service core passes compile/test and is now documented with
behavioral evidence tests. Verified runtime coverage includes:

- adaptive hard filters cover health, modality, context, and resource-receipt
  gates, then task-fit/score ordering
- local inference surface preference is active for `execution` and `background`
  lanes via `retain_preferred_local_surface_candidates`; supported values are:
  `mesh`, `llamacpp`, `hybrid`; anything else is treated as `hybrid`
- adaptive filter pipeline rejects unhealthy providers for code/tool routes
- static fallback behavior verified for missing provider (`503`), upstream
  non-JSON (`502`), and unreachable upstream (`502`)

Operators should treat `STATUS.md` as the timestamped runtime-evidence source
because behavior can evolve as adaptive policy stack matures.

## Architecture / layout decision

- Keep the merged lib + binary surface in `manwe` for now; no separate daemon
  crate introduced. `arda-engine` and `arda-aule` depend on this crate's public
  types, so a split needs a separator migration step before retiring this shell.
- gRPC wiring is explicitly **inactive by default**: it compiles only behind
  `--features grpc` and requires the binary flag `--grpc`. Without that pair,
  `manwe` only serves HTTP on `127.0.0.1:7171`. When active, it binds
  `MANWE_GRPC_PORT` or `0.0.0.0:50051` and exposes `HealthModelService` +
  `RouteGovernanceService`.

## Consumer wiring

- `arda-engine`: supervises manwe process, re-exports types, proxies `/v1/models`
- `arda-hud`: operator dashboard consumes `/v1/models` and `/healthz`
- `arda-launcher`: hardcodes downstream-side assumptions on manwe `:7171`
- Service registry: registers `manwe` as gateway in `services.toml`

## Stable module layout

| Module | Role |
|--------|------|
| `lib.rs` | Public re-exports; feature gates `adaptive::*` + `service` |
| `main.rs` | Binary entry, CLI, Axum router, chat completions handler |
| `config.rs` | TOML config, provider resolution, embedded default |
| `gateway.rs` | `SpannedManweGateway`, `ProviderRecord` |
| `provider.rs` | `ProviderDefinition`, `ProviderCatalog` states/registry |
| `transport.rs` | Transport traits + adaptive HTTP/IPC stub shells |
| `route.rs` | Authority trait stubs (`ManweCore`, `ManweGovernance`, `ManweMnemosyne`, `CharonPlutus`) |
| `resource_limits.rs` | Resource-group concurrency/queue timeout limits |
| `routing_adapter.rs` | `AdaptiveRoutingAdapter` shim, currently returns not-wired error |
| `charon_remote.rs` | Legacy charon bridge models |
| `service.rs` | `ManweService` stub when `adaptive` is disabled |
| `grpc.rs` | Feature-gated tonic `HealthModelService` + `RouteGovernanceService` |

## Adaptive module layout

| Module | Role |
|--------|------|
| `adaptive/mod.rs` | Module list |
| `adaptive/types.rs` | Adaptive types/errors |
| `adaptive/error.rs` | Adaptive error types |
| `adaptive/routing_adapter.rs` | Chat completion routing adapter wiring |
| `adaptive/service/mod.rs` | Service module index |
| `adaptive/service/types.rs` | `ManweService` spine, runtime state shape |
| `adaptive/service/runtime_state.rs` | Runtime state containers |
| `adaptive/service/bootstrap*.rs` | Bootstrapping defaults/runtime/overlay |
| `adaptive/service/provider_admin.rs` | Provider admin surfaces |
| `adaptive/service/capabilities.rs` | Provider capabilities model |
| `adaptive/service/http_clients.rs` | Reqwest client caching/binding |
| `adaptive/service/health_probe.rs` | Upstream health checking |
| `adaptive/service/echo_gate.rs` | Echo/conformance optional behavior |
| `adaptive/service/state_io.rs` | State persistence/io |
| `adaptive/service/state_mutation.rs` | State mutation APIs |
| `adaptive/service/route_policy.rs` | Route policy definitions |
| `adaptive/service/route_scoring.rs` | Scoring/lane fitness estimates |
| `adaptive/service/route_selection.rs` | Selection/policy logic |
| `adaptive/service/route_sessions.rs` | Session/history tracking |
| `adaptive/service/route_candidate_cache.rs` | Candidate cache/snapshot |
| `adaptive/service/route_policy_tests.rs` | Policy unit tests |
| `adaptive/service/bandit.rs` | Multi-armed bandit state |
| `adaptive/service/agent_quotas.rs` | Agent quota buckets/windows |
| `adaptive/service/catalog_reconciliation.rs` | Config/state reconciliation |
| `adaptive/service/status.rs` | Status shaping for HTTP/admin |
| `adaptive/service/observability.rs` | Metrics/tracing scaffolding |
| `adaptive/service/metrics.rs` | Prometheus-style metrics |
| `adaptive/service/event_writer.rs` | Async event writer background task |
| `adaptive/service/adaptive_routing.rs` | Routing core behavior |
| `adaptive/service/codex_responses_driver.rs` | Codex response shaping |
| `adaptive/service/hermes_cli_driver.rs` | Hermes CLI driver integration |
| `adaptive/service/hermes_proxy_driver.rs` | Hermes proxy/spawn path |
| `adaptive/service/paths.rs` | Path constants for state/config |
| `adaptive/service/proxy.rs` | Upstream call/response conversion |
| `adaptive/service/error.rs` | Service-level error taxonomy |
| `adaptive/service/bootstrap_defaults.rs` | Default provider bootstrapping |
| `adaptive/service/bootstrap_overlay.rs` | Runtime state overlays |
| `adaptive/service/bootstrap_runtime.rs` | Runtime bootstrap sequence |
| `adaptive/transport/mod.rs` | Transport module index |
| `adaptive/transport/http.rs` | Axum routes for `/status`, `/providers/candidates`, proxy SSE |
| `adaptive/transport/ipc.rs` | Unix-domain socket transport |

## Improvement ideas

1. Restore adaptive baseline as a separate quiet pass: bring back a
   known-good `service_events.rs`, `route_policy.rs`, `route_selection.rs`,
   `route_scoring.rs`, `adaptive_routing.rs`, and related visibility/type
   fixes before re-enabling adaptive compilation
2. Defer fleet/bootstrap config loading entirely to the adaptive baseline
   restoration pass, or keep the current static-only config surface until
   then rather than reintroducing it in `config.rs`
3. Replace `local_placeholder` bootstrap with a real local provider or
   configurable null/mesh provider for tests
4. Replace `Arc<AdaptiveRoutingAdapter>` placeholder error with a real scoring
   pipeline or gate it behind another feature until implemented
5. Add compile-time validation for provider credentials/bind to avoid runtime
   surprises; surface through `/healthz` for both static and adaptive modes
6. Split daemon runtime from crate library so `manwe` can be a pure gateway
   type crate with lightweight binary shell
7. Add integration tests for static forward path covering missing-provider,
   malformed model, upstream non-JSON, and upstream unreachable branches
