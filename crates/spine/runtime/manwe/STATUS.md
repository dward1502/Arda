# manwe — Current Status

Crate: `crates/spine/runtime/manwe`
Reviewed: 2026-07-23
State: active; canonical process, governed adaptive runtime, and telemetry contract aligned

## Verification performed in this review

| Command | Result |
|---|---|
| `cargo check -p manwe` | PASS |
| `cargo test -p manwe` | PASS: 1 library + 22 binary tests |
| `cargo check -p manwe --all-targets --features adaptive` | PASS |
| `cargo test -p manwe --features adaptive` | PASS: 268 library + 23 binary tests |
| `python crates/spine/runtime/manwe/tests/process_smoke.py` | PASS: static and full governed adaptive processes |
| `cargo check -p manwe --features grpc` | PASS |
| `cargo fmt -p manwe -- --check` | PASS |
| `cargo check -p manwe --all-targets --all-features` | PASS |
| `cargo test -p manwe --all-features` | PASS: 268 library + 24 binary tests |
| `cargo test -p arda-aule --features telemetry --test telemetry_surface` | PASS: public telemetry contract |
| `python crates/spine/runtime/manwe/tests/check_docs.py` | PASS: 8 Markdown files, 25 local links, 12 source-index entries |

The passing commands emitted only the workspace warning that the launcher
package's non-root Cargo profile is ignored plus pre-existing Manwe dead-code
warnings. `arda_aule::telemetry` is now a feature-gated public module with a
supported event emitter, tracing-layer builder, schema constant, and shutdown
function. The active adaptive service emits state, governance, and memory
events through that API. Manwe installs the OTLP layer when an endpoint is
configured and flushes the provider at process exit. Trace/log destination
routing and serialized attribute preservation have focused contract coverage.
Event `delivery` attributes distinguish event-writer acceptance,
Mnemosyne encoding, and telemetry-only memory delivery from durable persistence.
Its old unattached `service_events.rs` duplicate was
removed with the stale `observability::tracer`, `service::telemetry`, and
misspelled `ardea_aule` references it contained.

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
  receipts. The binary's explicit stream contract is buffered SSE, advertised
  by `x-manwe-streaming-mode: buffered`; it does not provide live pass-through.
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
  `ARDA_MANWE_FLEET_CONFIG` owns fleet
  discovery. Fleet failures produce an empty fleet catalog without replacing
  forwarding providers.
- Adaptive ownership: `ARDA_MANWE_PROVIDER_CONFIG` owns governed providers,
  falling back to `$ARDA_ROOT/config/charon.providers.toml` and then governed
  defaults. `ARDA_MANWE_STATE_DIR`, then `ARDA_MANWE_HOME`, owns mutable state;
  the fallback is `$ARDA_HOME/data/manwe` and then `./data/manwe`.

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
2. Remaining legacy-named policy/tuning variables need canonical Manwe aliases;
   path ownership variables now use the Manwe names documented above.
3. The direct `arda-hud` HTTP-consumer claim in older docs was not confirmed by
   this source audit; HUD currently has broader Manwe state/action projections.

## Source-graph state

The source graph is reconciled. Seven unattached root migration shims, eleven
incomplete parallel files directly under `src/adaptive/`, and the undeclared
`adaptive/service/fleet_persistence.rs` were removed after workspace-wide
consumer searches found no live users. `src/types.rs` remains the canonical
public domain model. `adaptive/transport/http.rs` is the active governed HTTP
runtime started by `--adaptive`; its daemon/IPC code remains a compiled
compatibility surface and is not advertised as part of the canonical binary.

## Next action

Keep provider/config documentation covered by the lightweight documentation
validator. Do not change port 7171 independently of engine, launcher, and
registry consumers.

## Registry/process-owner evidence

The canonical supervised process is now
`cargo run -p manwe -- --config manwe.toml`; its registered health endpoint is
`http://127.0.0.1:7171/healthz`. `arda-engine` parses the actual singular
`[[service]]` command/cwd schema and rejects an empty registry instead of
silently supervising nothing. On 2026-07-22, `cargo test -p arda-engine` passed
4 tests and `cargo check --bin arda` passed.