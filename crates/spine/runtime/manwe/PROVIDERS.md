# Providers

`manwe` works with OpenAI-compatible upstream providers via a static TOML catalog plus optional runtime reconciliation.

## Config locations

- `config/governance/charon.providers.toml` — primary provider config
- `core/state/fleet_bootstrap.json` — optional bootstrap state
- runtime overlay/snapshot paths under the service root:
  - `provider_runtime_state.json`
  - `provider_capability_receipts.json`
  - `lane_fitness.json`
  - `bandit_state.json`

## Connection behavior

- The gateway binds `127.0.0.1:7171` and exposes `/v1/chat/completions` and `/v1/models`.
- Requests are forwarded to upstream providers using reqwest clients cached by provider/mode/lane.
- `openai_compat` drivers can be reconciled against live `/models` catalogs when supported.
- Other drivers fall back to the configured model catalog.

## Operational notes

- If no providers are configured or none are healthy/enabled, inference returns 503.
- Credential/bind errors are runtime-only; there is no compile-time validation.
- Some capability evidence is persisted via receipts; the capability view is exposed through `/providers/candidates` and `/providers/capabilities`.
