# Providers

Manwe has one governed provider model. Local, free-cloud, subscription, and paid
providers use the same enrollment, health, capability, quota, fitness, and
selection records.

## Configuration precedence

1. `ARDA_MANWE_PROVIDER_CONFIG`
2. compatibility alias `ANNUNIMAS_CHARON_PROVIDER_CONFIG`
3. `$ARDA_ROOT/config/manwe.providers.toml`
4. governed bootstrap defaults

Each `[[provider]]` defines identity, `base_url`, optional `api_key_env`, access
tier, limits, driver selection, and nested `[[provider.model]]` entries. Model
entries carry context and capability truth. The legacy configured `healthy`
field is ignored because live probes own readiness.

The optional fleet bootstrap overlay resolves from `ARDA_FLEET_BOOTSTRAP_STATE`
then `$ARDA_ROOT/core/state/fleet_bootstrap.json`. Provider and tool-fit
intelligence overlays have dedicated environment overrides.

## Runtime state and evidence

Mutable state resolves from `ARDA_MANWE_STATE_DIR`, `ARDA_MANWE_HOME`,
`$ARDA_ROOT/data/manwe`, compatibility `$ARDA_HOME/data/manwe`, then the
build-derived workspace root.

The service maintains:

- provider runtime/probe state;
- provider capability receipts;
- task-class and tool-schema route outcomes;
- sanitized tool-fit observations;
- lane-fitness and bandit learning state;
- route and governance event ledgers.

Selection consumes this evidence together with request task class, context,
tool schema, origin/privacy, cost policy, resource pressure, cooldown, and quota
state. `/providers`, `/providers/capabilities`, `/provider_candidates`,
`/observability`, route headers, and persisted receipts expose why a candidate
was admitted, rejected, or selected.

## Capability and mutation boundary

A provider may be configured but ineligible because it is disabled, unhealthy,
missing its named credential, over quota, in cooldown, blocked by governance,
or lacks a required model capability. Status output contains credential names
and bounded diagnostics, never secret values.

Provider-result, model-streaming-validation, and provider-config reload routes
are mutation surfaces. If `ARDA_MANWE_API_KEY` is set, they require the exact
bearer token. If unset, local compatibility remains available. The HTTP server
rejects non-loopback bind addresses; remote access belongs behind an
authenticated reverse proxy.

## Stable endpoint contract

The only supported process serves the governed HTTP/OpenAI API on port `7171`.
`manwe.toml` supplies bind/port compatibility values only; it is not a second
provider catalog. The former static fleet/forwarding process and standalone
gRPC process have been retired.

`ARDA_ROUTE_*` remains a shared policy namespace because Manwe, Varda, and Aule
consume it; it is not a stale private runtime selector.
