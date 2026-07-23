# manwe — Current Status

Crate: `crates/spine/runtime/manwe`
Reviewed: 2026-07-23
State: active; canonical process and full governed adaptive runtime aligned; telemetry broken

## Verification performed in this review

| Command | Result |
|---|---|
| `cargo check -p manwe` | PASS |
| `cargo test -p manwe` | PASS: 0 library + 19 binary tests (prior default-only baseline) |
| `cargo check -p manwe --all-targets --features adaptive` | PASS |
| `cargo test -p manwe --features adaptive` | PASS: 264 library + 21 binary tests |
| `python crates/spine/runtime/manwe/tests/process_smoke.py` | PASS: static and full governed adaptive processes |
| `cargo check -p manwe --features grpc` | PASS |
| `cargo fmt -p manwe -- --check` | PASS |
| `cargo check -p manwe --all-features` | FAIL: 14 telemetry-path compile errors |
| `cargo test -p manwe --all-features` | FAIL during the same telemetry compilation |

The passing commands emitted only the workspace warning that the launcher
package's non-root Cargo profile is ignored. The all-feature commands fail in
`adaptive/service/service_events.rs`: `arda_aule::telemetry` is not exported,
`observability::tracer` and `service::telemetry` do not exist, and two references
misspell `arda_aule` as `ardea_aule`.

The process smoke test starts controlled temporary static and adaptive Manwe
processes plus a local mock OpenAI upstream. It verifies `/healthz`,
`/v1/models`, `/v1/capabilities`, `/v1/chat/completions`, governed response
headers, config provenance/catalog generations, missing/malformed/partial
static startup, missing/malformed fleet startup, and adaptive
state/governance receipts without calling external providers.

## Current capabilities

- OpenAI-compatible `/v1/chat/completions` and `/v1/models` surface.
- Health, provider/state, capabilities, and Prometheus metrics endpoints.
- Fleet provider discovery from `config/fleet.toml`, startup probing, and
  60-second refresh.
- Static explicit model/provider routing, plus a separately selected full
  governed adaptive runtime with policy chains, Echo Gate evaluation, quotas,
  fallback selection, provider drivers, persistence, and observability.
- Per-resource-group serialization with configurable positive concurrency and
  queue-timeout values.
- Static `manwe.toml` fallback with startup config validation.
- Credential-free active config paths/sources and catalog generations on health
  and capabilities surfaces.
- Stream-request handling and non-streaming proxy responses with JSONL route
  receipts. The current binary buffers a stream response before returning it;
  it does not provide live SSE pass-through.
- Rich adaptive policy/service library behind `adaptive`.
- Optional tonic gRPC services behind `grpc` plus the `--grpc` runtime flag.

## Runtime contract

- Default HTTP bind: `127.0.0.1:7171`
- gRPC bind when enabled: `MANWE_GRPC_PORT`, default `0.0.0.0:50051`
- Routing mode: static by default; `--adaptive` or
  `MANWE_ROUTING_MODE=adaptive` when compiled with `adaptive`
- Resource-group controls:
  `ARDA_MANWE_RESOURCE_GROUP_CONCURRENCY` (default `1`) and
  `ARDA_MANWE_RESOURCE_GROUP_QUEUE_TIMEOUT_SECONDS` (default `30`)
- Local-only model alias: `local/auto`
- Static ownership: `--config` owns forwarding providers;
  `ARDA_MANWE_FLEET_CONFIG` (legacy `ARDA_MANWE_FLEET_CONFIG`) owns fleet
  discovery. Fleet failures produce an empty fleet catalog without replacing
  forwarding providers.
- Adaptive ownership: `ARDA_MANWE_PROVIDER_CONFIG` (legacy
  `ARDA_MANWE_PROVIDER_CONFIG`) owns governed providers;
  `ARDA_MANWE_STATE_DIR` (legacy `ARDA_MANWE_HOME`) owns mutable state. Their
  defaults are `$ARDA_HOME/config/charon.providers.toml` and
  `$ARDA_HOME/data/manwe` respectively.

## Adaptive runtime boundary

Static mode continues through `provider::ProviderCatalog`. When adaptive mode
is requested, `src/main.rs` constructs `adaptive::service::ManweService`, loads
the adaptive provider catalog, and starts `adaptive::transport::http` on the
same configured bind. Adaptive gRPC combination is rejected explicitly until
the governed gRPC transport is defined.

Fresh controlled-process evidence returned `runtime: full_governed`, selected
the `smoke/smoke-model` route, returned `MANWE_FULL_SMOKE_OK`, emitted
`x-manwe-route-id`, provider/model/class/lane headers, and wrote both
`route_selected` state and governance receipts.

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
2. Remaining `ARDA_MANWE_*` policy/tuning variables need canonical
   `ARDA_MANWE_*` aliases; path ownership variables now have canonical-first
   precedence.
3. `ManweService::read_lane_fitness_snapshot` currently returns `None`, so the
   adaptive scoring hook exists but does not consume persisted lane fitness.
4. The binary buffers responses marked as streaming before returning them, so
   latency and backpressure differ from true SSE pass-through.
5. Current tests are strong at unit/policy level but do not provide a maintained
   gRPC process-level integration harness.
6. The direct `arda-hud` HTTP-consumer claim in older docs was not confirmed by
   this source audit; HUD currently has broader Manwe state/action projections.
7. The `telemetry` feature is not buildable, so all-feature CI cannot currently
   pass even though default, adaptive-only, and gRPC-only paths do.

## Source-graph state

The source graph is reconciled. Seven unattached root migration shims, eleven
incomplete parallel files directly under `src/adaptive/`, and the undeclared
`adaptive/service/fleet_persistence.rs` were removed after workspace-wide
consumer searches found no live users. `src/types.rs` remains the canonical
public domain model. `adaptive/transport/http.rs` is the active governed HTTP
runtime started by `--adaptive`; its daemon/IPC code remains a compiled
compatibility surface and is not advertised as part of the canonical binary.

## Next action

Close the lane-fitness, buffered-streaming, and gRPC process-test gaps in
[`CHECKLIST.md`](CHECKLIST.md), then repair the telemetry feature contract. Do
not change port 7171 independently of engine, launcher, and registry consumers.

## Registry/process-owner evidence

The canonical supervised process is now
`cargo run -p manwe -- --config manwe.toml`; its registered health endpoint is
`http://127.0.0.1:7171/healthz`. `arda-engine` parses the actual singular
`[[service]]` command/cwd schema and rejects an empty registry instead of
silently supervising nothing. On 2026-07-22, `cargo test -p arda-engine` passed
4 tests and `cargo check --bin arda` passed.