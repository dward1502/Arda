# manwe

`manwe` is Arda's local OpenAI-compatible inference gateway. It owns the
stable HTTP entry point on `127.0.0.1:7171`, discovers local fleet providers,
selects an eligible upstream, forwards chat-completion requests, and records
route receipts.

Status: active with its foundation baseline complete as of 2026-07-27. The
default and full governed `adaptive` runtimes have maintained process smoke
coverage; the `telemetry` and all-feature contracts pass their focused checks.
See [`STATUS.md`](STATUS.md) for verification evidence and bounded continuing
risks.

## Runtime surface

The binary in `src/main.rs` serves:

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/healthz`, `/health`, `/status` | Runtime, config, and provider health |
| `GET` | `/providers`, `/state` | Current fleet-provider view |
| `GET` | `/metrics` | Prometheus text metrics |
| `GET` | `/v1/models` | OpenAI-compatible model catalog |
| `GET` | `/v1/capabilities` | Routing mode and resource-group state |
| `POST` | `/v1/chat/completions` | OpenAI-compatible inference proxy |

Defaults:

- HTTP bind: `127.0.0.1:7171`
- Static config: `manwe.toml`, with an embedded local Ollama fallback
- Fleet catalog: `$ARDA_ROOT/config/fleet.toml`, probed at startup and every 60 seconds
- Model references: explicit `provider/model`, a catalog model ID, `auto`, or
  `local/auto`
- Receipts: `$ARDA_ROOT/data/manwe/route_receipts.jsonl`

Configuration ownership and precedence:

- Static forwarding config is owned by `--config` (default `manwe.toml`). A
  valid file wins; a missing, unreadable, malformed, or provider-empty file
  selects the embedded Ollama fallback and reports the exact fallback reason.
- Static fleet discovery is independent of forwarding config and is owned by
  `ARDA_MANWE_FLEET_CONFIG`, legacy alias
  `ANNUNIMAS_CHARON_FLEET_CONFIG`, then `$ARDA_ROOT/config/fleet.toml`. Missing
  or malformed fleet input produces an empty fleet catalog; it does not replace
  static forwarding providers.
- Full governed adaptive mode does not consume either static catalog. Its
  provider source is `ARDA_MANWE_PROVIDER_CONFIG`, legacy environment alias
  `ANNUNIMAS_CHARON_PROVIDER_CONFIG`, then
  `$ARDA_ROOT/config/manwe.providers.toml`. Missing or invalid provider input
  selects governed defaults.
- Adaptive mutation routes are open for local compatibility when
  `ARDA_MANWE_API_KEY` is unset. When it is set to a non-empty value,
  provider-result, model-streaming-validation, and config-reload mutations
  require an exact `Authorization: Bearer <value>` header.
- Adaptive runtime state is owned by `ARDA_MANWE_STATE_DIR`, then
  `ARDA_MANWE_HOME`, then `$ARDA_ROOT/data/manwe`, then the compatibility
  `$ARDA_HOME/data/manwe` root. If no environment root is supplied, Manwe uses
  its build-derived Arda workspace root; it never derives mutable state from the
  process working directory. Provider runtime state overlays configured provider
  identity; it does not own endpoint or credential configuration.

`/healthz` and `/v1/capabilities` expose credential-free `config_source`,
config paths, and `catalog_generation`. Generation starts at `1` after startup
and increments after each successful catalog reload. No API key values are
included in these diagnostics.

## Build and run

From the workspace root:

```text
cargo run -p manwe -- --config manwe.toml
cargo run -p manwe --features adaptive -- --adaptive
cargo run -p manwe --features grpc -- --grpc
```

Useful flags are `--bind`, `--port`, `--config`, `--adaptive`, and `--grpc`.
`MANWE_ROUTING_MODE=adaptive` is equivalent to `--adaptive`. Requesting an
adaptive or gRPC mode without compiling its Cargo feature fails fast.

The gRPC server is opt-in. It binds `MANWE_GRPC_PORT`, defaulting to
`0.0.0.0:50051`, and runs alongside HTTP only when both `--features grpc` and
`--grpc` are used.

## Cargo features

| Feature | Current effect |
|---|---|
| default | Builds the public types/config library and the fleet-backed HTTP gateway |
| `adaptive` | Compiles the full governed `ManweService`; `--adaptive` starts its policy, quotas, persistence, provider drivers, and HTTP transport |
| `grpc` | Compiles the tonic services and permits `--grpc` |
| `telemetry` | Emits adaptive state, governance, and memory events through the feature-gated `arda_aule::telemetry` API |

With `telemetry`, Manwe installs the `arda_aule` OpenTelemetry layer at process
startup. Set `ARDA_OTLP_ENDPOINT` or `OTEL_EXPORTER_OTLP_ENDPOINT` to enable
OTLP trace export; the provider is flushed on process exit. Telemetry event
attributes are serialized without loss into the selected trace/log destination.

`--adaptive` and `MANWE_ROUTING_MODE=adaptive` select the full governed
`ManweService`. `/v1/capabilities` identifies this runtime as
`full_governed` with `policy_authority: manwe_service`, `governance: true`, and
`quota_mesh: true`. Static mode retains the smaller fleet-policy gateway.

Adaptive previews and route selections load `config/governance/realm_policies.toml`,
resolve `governance_realm` and `governance_action_class` request metadata (defaulting to
`routing`/`provider_selection`), and attach the typed realm-policy verdict, scorer receipts,
and runtime-blocking decision to `RouteGovernance`. Policy load failure falls back to the
validated non-blocking default. `ARDA_GOVERNANCE_BLOCKING_ENABLED` is only an operator request;
the governance authority still requires scoped autonomy-ready evidence before it can block.

### Streaming contract

The fleet-backed binary accepts OpenAI requests with `stream=true` and preserves
the upstream SSE content type and bytes, but it intentionally buffers the full
upstream body before returning it. It attempts to persist the final route receipt
before the caller consumes the response and marks the response with
`x-manwe-streaming-mode: buffered`. This surface does not promise live SSE
pass-through, incremental latency, or streaming backpressure. Clients that need
those properties must not treat this binary path as a true streaming transport.
As on non-streaming routes, receipt persistence is best-effort and a write
failure is logged without replacing an otherwise valid upstream response.

Run both maintained process-level contracts with:

```text
python crates/spine/runtime/manwe/tests/process_smoke.py
```

The `grpc` feature also has an in-process runtime smoke test that binds an
ephemeral listener, connects generated tonic clients, and exercises both the
health/model and route-governance services.

## Public library surface

`src/lib.rs` currently exports:

- `config`, `error`, `routing_adapter`, and `types`
- `adaptive` when the `adaptive` feature is enabled
- `ManweConfig`, `ManweRequestEnvelope`, `ModelState`, `ProviderState`, and
  `RouteDecision`

The gateway binary has private modules for provider discovery, receipts,
resource limits, and optional gRPC. The source graph has been reconciled: the
governed service implementations are attached explicitly from
`src/adaptive/service/full/`, and obsolete parallel source copies are retired.
[`BREAKDOWN.md`](BREAKDOWN.md) records the evidence and retained boundaries.

## Workspace integration

- `arda-engine` depends on and re-exports the `manwe` library, and its harness
  proxies `/v1/models` to a configured Manwe URL.
- `arda-launcher` discovers `MANWE_BASE_URL` / `ARDA_MANWE_BASE_URL` during
  onboarding.
- `services.toml` registers the required canonical gateway as
  `cargo run -p manwe -- --config manwe.toml`, with
  `http://127.0.0.1:7171/healthz` as its health contract.
- `arda-engine` deserializes the manifest's singular `[[service]]` command,
  arguments, working directory, tags, and health metadata. In `--no-ui` mode,
  it drops launcher/HUD entries and continues to supervise Manwe.

## Verification

The current 2026-07-27 foundation closure includes:

```text
cargo check -p manwe --all-targets --all-features
cargo clippy -p manwe --all-targets --all-features -- -D warnings
cargo test -p manwe --all-features
cargo fmt -p manwe -- --check
python crates/spine/runtime/manwe/tests/process_smoke.py
python crates/spine/runtime/manwe/tests/check_docs.py
cargo test -p arda-engine
```

These commands pass. The all-feature suite contains 278 library and 29 binary
tests; `STATUS.md` records downstream and live-runtime evidence.

## Documentation

- [`STATUS.md`](STATUS.md) — current evidence, health, boundaries, and risks
- [`BREAKDOWN.md`](BREAKDOWN.md) — crate shape, active module graph, and consumers
- [`PROVIDERS.md`](PROVIDERS.md) — static and governed provider/config contract
- [Archived CHARON foundation plan](../../../../docs/archive/CHARON.md)
- [Archived foundation checklist](../../../../docs/archive/MANWE_FOUNDATION_CHECKLIST.md)