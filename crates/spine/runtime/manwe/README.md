# manwe

`manwe` is Arda's single governed OpenAI-compatible inference gateway. The
binary always starts `ManweService` and the HTTP transport from
`src/adaptive/transport/http.rs`; there is no selectable static or gRPC process.

## Canonical runtime

From the workspace root:

```text
cargo run -p manwe -- --config manwe.toml --bind 0.0.0.0 --port 7171
```

`--adaptive` remains accepted as a hidden compatibility no-op while installed
launchers converge. It does not select another runtime. `--grpc` fails with an
explicit retirement message.

The root daemon owns the supported production launch through `services.toml`:

```text
cargo run -p arda -- --no-ui
```

That profile supervises Manwe, probes `http://127.0.0.1:7171/healthz`, and
publishes lifecycle/readiness state from the root harness at
`http://127.0.0.1:7878/v1/status`. The normal profile also includes the launcher
and HUD.

## HTTP surface

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/healthz`, `/health`, `/status` | Governed runtime and provider health |
| `GET` | `/providers`, `/state` | Provider state and routing eligibility |
| `GET` | `/providers/capabilities` | Capability receipts |
| `GET` | `/provider_candidates` | Current route candidates |
| `GET` | `/observability` | Route, lane, and tool-fit evidence |
| `GET` | `/metrics` | Prometheus metrics |
| `GET` | `/v1/models` | OpenAI-compatible model catalog |
| `GET` | `/v1/capabilities` | Runtime/capability contract |
| `POST` | `/v1/chat/completions` | OpenAI-compatible governed routing |

The coordinated HTTP contract is port `7171`. Local callers use
`127.0.0.1:7171`; the canonical workstation service binds `0.0.0.0:7171` so
approved Tailscale consumers retain access. Change the port or bind only with
engine, HUD, monitoring, Hermes bridge, and offsite-operator consumer updates.

## Provider and state ownership

Provider configuration resolves in this order:

1. `ARDA_MANWE_PROVIDER_CONFIG`
2. compatibility alias `ANNUNIMAS_CHARON_PROVIDER_CONFIG`
3. `$ARDA_ROOT/config/manwe.providers.toml`
4. governed bootstrap defaults

Mutable state resolves from `ARDA_MANWE_STATE_DIR`, `ARDA_MANWE_HOME`,
`$ARDA_ROOT/data/manwe`, compatibility `$ARDA_HOME/data/manwe`, then the
build-derived workspace root. Important evidence includes:

- `state.jsonl` and `governance_events.jsonl`
- `tool_fit_ledger.jsonl`
- `provider_runtime_state.json`
- `provider_capability_receipts.json`
- `lane_fitness.json` and `bandit.json`

Provider identity, access tier, health, quotas, model capabilities, lane fitness,
and route outcomes feed one selection model for local, free-cloud, and paid
providers. Credentials are referenced by environment-variable name and are not
emitted by status surfaces.

The HTTP listener is loopback-only; expose it remotely only through an
authenticated reverse proxy. Mutation routes retain local compatibility when
`ARDA_MANWE_API_KEY` is unset. When configured, callers must provide the exact
bearer token.

## Cargo features

| Feature | Effect |
|---|---|
| default (`adaptive`) | Builds the only supported governed runtime |
| `telemetry` | Adds `arda_aule` OpenTelemetry emission and shutdown flushing |

Building the binary with `--no-default-features` fails at compile time rather
than silently restoring a smaller router.

## Public library surface

`src/lib.rs` exports `config`, `error`, `routing_adapter`, `types`, and the
governed `adaptive` tree, plus the principal public request/provider/route
models. `src/types.rs` remains the canonical domain model.

## Verification

```text
cargo check -p manwe --all-targets --all-features
cargo clippy -p manwe --all-targets --all-features -- -D warnings
cargo test -p manwe --all-features -- --test-threads=1
cargo fmt -p manwe -- --check
python crates/spine/runtime/manwe/tests/process_smoke.py
python crates/spine/runtime/manwe/tests/check_docs.py
cargo test -p arda-engine --all-targets -- --test-threads=1
cargo test --test root_daemon -- --test-threads=1
```

See [`STATUS.md`](STATUS.md), [`BREAKDOWN.md`](BREAKDOWN.md), and
[`PROVIDERS.md`](PROVIDERS.md) for current evidence and boundaries.
