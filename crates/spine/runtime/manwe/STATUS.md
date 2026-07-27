# manwe — Current Status

Crate: `crates/spine/runtime/manwe`
Reviewed: 2026-07-27
State: active; foundation baseline complete and maintained

## Verification performed in the 2026-07-27 foundation closure

| Command | Result |
|---|---|
| `cargo check -p manwe --all-targets --all-features` | PASS |
| `cargo clippy -p manwe --all-targets --all-features -- -D warnings` | PASS |
| `cargo test -p manwe --all-features` | PASS: 278 library + 29 binary tests |
| `cargo fmt -p manwe -- --check` | PASS |
| `python crates/spine/runtime/manwe/tests/process_smoke.py` | PASS: static and full governed adaptive processes |
| `python crates/spine/runtime/manwe/tests/check_docs.py` | PASS: 6 Markdown files, 27 local links, 12 source-index entries |
| `cargo test -p arda-engine` | PASS: 8 library + 1 integration test |

The closure audit repaired three regressions from the latest Manwe commit: the
new mutation handlers now return one concrete response type, the unused
top-level static-config API-key field was removed instead of advertising an
unwired config contract, and mutation authorization now allows compatibility
mode only when `ARDA_MANWE_API_KEY` is unset while requiring an exact bearer
token when configured. A focused regression test covers disabled, missing,
incorrect, and matching authorization states.

The process smoke starts temporary static and adaptive Manwe processes plus a
local mock OpenAI upstream. It verifies health/models/capabilities/chat,
governed headers, config provenance and generation, malformed/missing config
fallbacks, canonical path ownership, and state/governance receipts without
calling external providers.

Direct observation on 2026-07-27 found the operator-managed governed runtime
listening on `0.0.0.0:5110`, reporting `runtime: full_governed`, 21 catalog
providers, and 5 ready/healthy providers. This deployment port does not replace
the registered default `127.0.0.1:7171` contract. The live catalog and canonical
provider TOML both report `edge_carnice` disabled; older claims that it is an
active route are historical service-repair evidence, not current enrollment.

## Current capabilities

- OpenAI-compatible `/v1/chat/completions` and `/v1/models` surface.
- Health, provider/state, capabilities, and Prometheus metrics endpoints.
- Fleet provider discovery from `config/fleet.toml`, startup probing, and
  60-second refresh.
- Static explicit model/provider routing, plus a separately selected full
  governed adaptive runtime with policy chains, Echo Gate evaluation, quotas,
  fallback selection, provider drivers, persistence, and observability.
- Per-resource-group serialization with configurable positive concurrency and
  queue-timeout values. Fleet providers can declare
  `resource_group_concurrency`; saturated adaptive selections prefer an
  equivalent eligible provider in another resource group before queueing.
- Static `manwe.toml` fallback with startup config validation.
- Credential-free active config paths/sources and catalog generations on health
  and capabilities surfaces.
- Stream-request handling and non-streaming proxy responses with JSONL route
  receipts. The binary's explicit stream contract is buffered SSE, advertised
  by `x-manwe-streaming-mode: buffered`; it does not provide live pass-through.
- Deterministic task-class benchmark receipts for exact-match expectations;
  benchmark IDs are bounded and no judge model is invoked on edge nodes.
- Canonical `manwe_*` Prometheus metrics with base-unit latency and bounded
  `provider_id`/`model`/`route_class` labels; generated `charon_*` aliases are
  retired.
- `edge_carnice` remains configured at Beelink `:1234` but is currently disabled
  in `config/manwe.providers.toml`. Enrollment is runtime configuration, not a
  crate-foundation completion condition. Provider health and operational-state
  responses remain the authority for current route eligibility.
- Rich adaptive policy/service library behind `adaptive`.
- Realm/action policy evaluation on every adaptive preview and selected route, with typed
  scorer, reload, and runtime-blocking receipts.
- Optional tonic gRPC services behind `grpc` plus the `--grpc` runtime flag.

## Runtime contract

- Default HTTP bind: `127.0.0.1:7171`
- gRPC bind when enabled: `MANWE_GRPC_PORT`, default `0.0.0.0:50051`
- Routing mode: static by default; `--adaptive` or
  `MANWE_ROUTING_MODE=adaptive` when compiled with `adaptive`
- Resource-group controls:
  `ARDA_MANWE_RESOURCE_GROUP_CONCURRENCY` (default `1`) and
  `ARDA_MANWE_RESOURCE_GROUP_QUEUE_TIMEOUT_SECONDS` (default `30`), with
  per-provider `resource_group_concurrency` values in `config/fleet.toml`
- Local-only model alias: `local/auto`
- Static ownership: `--config` owns forwarding providers;
  `ARDA_MANWE_FLEET_CONFIG`, legacy alias
  `ANNUNIMAS_CHARON_FLEET_CONFIG`, then `$ARDA_ROOT/config/fleet.toml` owns fleet
  discovery. Fleet failures produce an empty fleet catalog without replacing
  forwarding providers.
- Adaptive ownership: `ARDA_MANWE_PROVIDER_CONFIG` owns governed providers,
  with `ANNUNIMAS_CHARON_PROVIDER_CONFIG` retained as the lower-precedence
  legacy environment alias, then `$ARDA_ROOT/config/manwe.providers.toml`, and
  finally governed defaults. `ARDA_MANWE_STATE_DIR`, then `ARDA_MANWE_HOME`,
  then `$ARDA_ROOT/data/manwe`, and then the compatibility `$ARDA_HOME/data/manwe`
  root owns mutable state. Without an environment root, Manwe uses its
  build-derived Arda workspace root rather than the process working directory.
- Realm-policy ownership: `$ARDA_ROOT/config/governance/realm_policies.toml`; the optional
  `ARDA_GOVERNANCE_BLOCKING_ENABLED` operator request remains subordinate to scoped
  readiness, independent-review, rollback, and operator-disable authority gates.

## Adaptive runtime boundary

Static mode continues through `provider::ProviderCatalog`. When adaptive mode
is requested, `src/main.rs` constructs `adaptive::service::ManweService`, loads
the adaptive provider catalog, and starts `adaptive::transport::http` on the
same configured bind. Adaptive gRPC combination is rejected explicitly until
the governed gRPC transport is defined.

Fresh controlled-process evidence returned `runtime: full_governed`, selected
the `smoke/smoke-model` route, returned `MANWE_FULL_SMOKE_OK`, emitted
`x-manwe-route-id`, provider/model/class/lane headers, and wrote both
`route_selected` state and governance receipts. Manwe owns an explicitly
rooted Bacon-Lite writer: machine evidence is written to
`$ARDA_ROOT/data/governance/bacon_lite.jsonl` and the operator projection to
`$ARDA_ROOT/docs/operator/library/governance/bacon_lite.md`; crate-local
`data/` and `docs/operator/library/` output trees are retired.

## Historical runtime evidence retained

A prior 2026-07-21/22 temporary-port run reported seven configured fleet
providers, four healthy/model-confirmed providers, successful health/models/
capabilities/chat calls, a successful `MANWE_OK` adaptive request, route receipt
generation, and serialization of two requests sharing one physical resource
group. That process was stopped after validation.

This is useful regression evidence, but it was not rerun during this
documentation review.

## Open risks and gaps

1. The binary and rich adaptive service have parallel routing/config/transport
   paths. Capability claims must stay scoped until they are unified.
2. `ARDA_ROUTE_*` policy/tuning variables are intentionally retained as a
   shared contract consumed by Manwe, Varda, and Aule rather than renamed as
   Manwe-private variables.
3. The direct `arda-hud` HTTP-consumer claim in older docs was not confirmed by
   this source audit; HUD currently has broader Manwe state/action projections.

## Source-graph state

The source graph is reconciled. In addition to the previously retired seven
root migration shims, eleven incomplete files directly under `src/adaptive/`,
and undeclared `adaptive/service/fleet_persistence.rs`, this review removed 33
undeclared parallel files directly under `adaptive/service/`. The service module
is defined by `service/mod.rs` including `full_service.rs`; every corresponding
implementation is attached explicitly from `adaptive/service/full/`. The
parallel files were therefore unreachable from both crate roots and had no live
workspace consumers. Active external path records now point at the `full/`
implementations.

`src/types.rs` remains the canonical public domain model.
`adaptive/transport/http.rs` is the active governed HTTP runtime started by
`--adaptive`; its daemon/IPC code remains a compiled compatibility surface and
is not advertised as part of the canonical binary. Two tracked Python bytecode
artifacts under `tests/__pycache__/` were also retired, and the test directory
now ignores regenerated Python caches.

## Foundation designation

The crate's foundation baseline is complete: package ownership, default and
governed runtime boundaries, feature contracts, process smoke coverage,
configuration precedence, source ownership, indexes, and operator documentation
are aligned with the compiled implementation. This designation does not freeze
Manwe or claim that the intentionally parallel static and governed routing
architectures have been unified; those remain bounded evolution work rather
than incomplete baseline repair.

## Maintenance posture

Keep provider/config documentation covered by the lightweight documentation
validator. Do not change port 7171 independently of engine, launcher, and
registry consumers. Future implementation work should preserve the verified
default, adaptive, gRPC, telemetry, and process-smoke gates recorded above.

## Registry/process-owner evidence

The canonical supervised process is now
`cargo run -p manwe -- --config manwe.toml`; its registered health endpoint is
`http://127.0.0.1:7171/healthz`. `arda-engine` parses the actual singular
`[[service]]` command/cwd schema and rejects an empty registry instead of
silently supervising nothing. On 2026-07-27, `cargo test -p arda-engine` passed
8 library tests and 1 integration test.