# Providers

Manwe has two selectable HTTP runtimes with separate provider inputs. Do not
treat their TOML files or persisted state as interchangeable.

## Static runtime (default)

The default binary combines two catalogs:

1. `--config <path>` (default `manwe.toml`) owns fallback forwarding
   providers. A valid non-empty file is used. A missing, unreadable, malformed,
   or provider-empty file selects the embedded local Ollama configuration.
2. `ARDA_MANWE_FLEET_CONFIG`, legacy alias
   `ANNUNIMAS_CHARON_FLEET_CONFIG`, then
   [`$ARDA_ROOT/config/fleet.toml`](../../../../config/fleet.toml), owns fleet discovery.
   Missing or malformed fleet input produces an empty fleet catalog; it does
   not replace the forwarding providers from `--config`.

Fleet providers are probed at startup and the catalog is reloaded and probed
every 60 seconds. Requests first try an eligible fleet route and then the
static forwarding catalog. Explicit `provider/model`, catalog model IDs,
`auto`, and the local-only `local/auto` alias are supported.

The static runtime exposes `/providers`, `/state`, `/v1/models`,
`/v1/capabilities`, and `/v1/chat/completions`. It writes best-effort route
receipts to `$ARDA_ROOT/data/manwe/route_receipts.jsonl`.

Static startup validates the selected forwarding configuration's provider
presence, bind address, endpoint shape, and non-noise API keys. Provider reachability
and model availability remain runtime probe/forwarding concerns.

## Full governed adaptive runtime

Compile with `adaptive` and select it with `--adaptive` or
`MANWE_ROUTING_MODE=adaptive`. This starts
[`adaptive/transport/http.rs`](src/adaptive/transport/http.rs) and
`ManweService`; it does not consume `manwe.toml` or `config/fleet.toml` as its
provider catalog.

Provider configuration resolves as follows:

1. `ARDA_MANWE_PROVIDER_CONFIG`.
2. Legacy environment alias `ANNUNIMAS_CHARON_PROVIDER_CONFIG`.
3. `$ARDA_ROOT/config/manwe.providers.toml`. If `ARDA_ROOT` is unset, the
   build-derived Manwe source ancestor is used.
4. Governed built-in providers when the file is missing; governed defaults
   after a malformed or provider-empty file is rejected.

The optional fleet bootstrap overlay resolves from
`ARDA_FLEET_BOOTSTRAP_STATE`, then
`$ARDA_ROOT/core/state/fleet_bootstrap.json`. Provider and tool-fit
intelligence overlays have their own `ARDA_PROVIDER_INTELLIGENCE_PATH` and
`ARDA_TOOL_FIT_MODEL_INTELLIGENCE_PATH` overrides.

Each `[[provider]]` may define identity, `base_url`, `api_key_env`, limits,
driver selection, and nested `[[provider.model]]` records. A configured
`api_key_env` is checked by name; credentials are not stored in status output.
The legacy `healthy` field is ignored because live probes own health.

Adaptive mutable state resolves from `ARDA_MANWE_STATE_DIR`, then
`ARDA_MANWE_HOME`, then `$ARDA_ROOT/data/manwe`, then the compatibility
`$ARDA_HOME/data/manwe` root, and finally the build-derived Arda workspace root.
The service root contains:

- `state.jsonl` and `governance_events.jsonl`
- `tool_fit_ledger.jsonl`
- `provider_runtime_state.json`
- `provider_capability_receipts.json`
- `lane_fitness.json` and `bandit.json`

Persisted runtime state overlays probe/model memory onto configured provider
identity; it does not own endpoints or credentials. The governed HTTP surface
adds `/providers/capabilities` and `/provider_candidates` (singular
`provider`, underscore), plus probe, reconciliation, routing, observability,
path, event, and metric endpoints. There is no `/providers/candidates` route.

## Operational boundary

- A provider can be configured but ineligible because it is disabled,
  unhealthy, missing its named credential, over quota, in cooldown, or lacks a
  required model capability.
- Static and governed status/capability responses report credential-free config
  provenance and catalog generation.
- Buffered SSE is the static binary's documented stream contract; it is not
  live incremental pass-through. See [`README.md`](README.md).
- `ARDA_ROUTE_*` remains the canonical shared route-policy namespace because
  Manwe, Varda, and Aule consume it; it is not a legacy Manwe naming surface.
