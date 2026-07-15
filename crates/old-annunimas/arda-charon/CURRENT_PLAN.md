---
soterion:
  symbol: "🪙"
  codepoint: "U+1FA99"
  hex: "0x0001FA99"
  domain: "plutus/charon routing economics"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Charon Routing Layer — Current Plan

Updated: 2026-06-03
Status: Live router and LLM operations control plane. This file supersedes the May 2026 plan that described the pre-`/probe`, pre-`/observability`, pre-catalog-reconciliation posture.

## 1. Live State Summary

CHARON is becoming Uinen Tolkien inspired masters of the seas/paths 

Charon is running as the Arda OpenAI-compatible routing layer and provider control plane on port 5110.

Verified endpoints:

| Endpoint | Status | Notes |
| --- | ---: | --- |
| `GET /v1/models` | 200 | Returns the configured/visible OpenAI-compatible model catalog. |
| `GET /metrics` | 200 | Prometheus metrics are live. |
| `POST /reload_config` | 200 | Reloads `config/charon.providers.toml` and fleet bootstrap overlay. |
| `GET /health` | 200 | Dedicated Charon health contract; returns provider summary and recent route counters. |
| `GET /providers` | 200 | Provider inventory with routing metadata, filters, compact mode, catalog visibility, probe eligibility, and Hermes bridge metadata. |
| `POST /probe` | 200 | Native marker-validated inference probe with structured attempts, route receipt, probe throttling, and provider-aware failure classification. |
| `POST /reconcile_catalogs` | 200 | Live catalog reconciliation, stale model detection, default/probe model selection, and repair of catalog-missing quarantines. |
| `GET /observability` | 200 | Operator rollups for failures, slow providers, best observed routes, free-provider pool, billing/quota risk, fallback chains, catalog reconciliation, and recent routes. |
| `GET /route_history` | 200 | In-memory recent route receipt ring. |

Provider metrics observed after the June router hardening pass:

| Metric | Value |
| --- | ---: |
| Total providers | 21 |
| Ready/healthy providers | 15 |
| Disabled providers | 6 |
| Native probe result | Passed with marker validation and `route_class = "health_probe"` |
| Catalog reconciliation | 21 checked, 9 live catalogs observed |
| Charon crate tests | 169 passed |

Representative model/provider posture:

- `auto` remains the correct client-facing model for Charon-owned selection.
- Local/edge providers still expose Qwen-family local models where enabled.
- Cloud/free-pool providers include OpenRouter, NVIDIA, Groq, Cerebras, Google, and OpenCode when their metadata and runtime state remain healthy.
- Production default models and cheap health-probe models are now separate provider fields.
- Google catalog matching accepts live IDs with the `models/` prefix and clears prior catalog-missing quarantine when the configured model is recognized live.

## 2. Architecture

Config sources are merged from:

1. `config/charon.providers.toml` — primary provider definitions.
2. `core/state/fleet_bootstrap.json` — fleet overlay for discovered edge nodes, observed models, and health.
3. `core/state/provider_intelligence.json` — runtime quota/usage/health intelligence.

Routing flow:

`route_preview()` → `select_route_candidate()` → `build_route_decision()` → provider execution

Routing considers priority, strict mode, forced provider/model, excluded providers, hybrid route policy, task type, runtime signals, sticky sessions, learned/bandit scoring, lane fitness, request shape, tool/structured/streaming requirements, free-pool policy, provider/model cooldown, catalog-missing quarantine, and probe/default model separation.

Driver types currently in use:

- `openai_compat` — standard OpenAI-compatible HTTP providers.
- `hermes_agent_cli` — subscription-backed routes through the local Hermes CLI; now avoided on fast lanes unless explicitly allowed.
- `hermes_proxy` — persistent local Hermes proxy bridge where configured.
- `codex_responses` — pooled HTTP path for Codex Responses-compatible routes.

Key state/repair loops:

- Runtime provider/model health is persisted and merged on reload instead of being overwritten by static config.
- Failure memory distinguishes account/billing/quota/rate-limit/model-not-found/unsupported-model/proxy failures and applies provider or model cooldown/quarantine where appropriate.
- Catalog reconciliation compares configured and live `/models` catalogs, persists default and probe model choices, and emits reconciliation receipts.
- Native `/probe` uses a cheap health-probe execution profile (`route_class = "health_probe"`, small context target, `stream=false`) without contaminating production model defaults.
- Learned routing now keys by task plus request shape: tools, structured output, and streaming status.

## 3. Provider Route Status

### Healthy / Usable Routes

Local and edge routes:

- `edge_core` — local workstation route exposes LiquidAI `LFM2.5-8B-A1B-Q4_K_M` as the governed core lane at 128K context; Qwen3.5-9B remains a rollback artifact outside the active edge_core catalog.
- `edge_worker_light` — light worker route exposes Qwen-family local models where the worker endpoint is healthy.
- `edge_guardhouse` — Pi5 guardhouse route, slow but useful for light/background work.
- `edge_backbone` — Threadripper Qwen3.6-35B route is configured but currently disabled in active provider config; keep as optional/backbone capacity until intentionally re-enabled and route-smoked.

Cloud/aggregator/free-pool routes known to be usable when quota and upstream account state allow:

- `openrouter`
- `nvidia`
- `cerebras`
- `groq`
- `google`
- `opencode` (subject to rate limits)

LiteLLM route:

- `litellm_gateway` is enabled in `config/charon.providers.toml`.
- Direct unauthenticated LiteLLM `/v1/models` correctly returns 401.
- Direct authenticated LiteLLM `/v1/models` returns 200 with 4 models:
  - `litellm-router`
  - `claude-sonnet-4-6`
  - `claude-opus-4-6`
  - `claude-haiku-4-5`
- Charon-routed non-streaming inference against `litellm_gateway/litellm-router` returned 200, confirming Charon-to-LiteLLM auth and proxying work.
- Charon health reports `providers_total=18`, `providers_enabled=14`, `providers_ready=14`, confirming the enabled provider pool is loaded and routable after restart.

Service role split:

- `mesh-llm.service` is the local workstation model server on port 9337. Charon's `edge_core` provider points at it through the Tailscale address `http://100.78.138.113:9337/v1`; as of 2026-06-03 the governed edge_core catalog is the single LiquidAI `LFM2.5-8B-A1B-Q4_K_M` model at 128K context. The raw mesh endpoint can still expose peer-discovered mesh inventory, but Charon's provider intelligence override keeps edge_core pinned to the local core model.
- `arda-litellm.service` is a normalized/authenticated gateway on port 4000 for subscription/cloud-style routing through `litellm_gateway`; it is not the local mesh backend for `edge_core`.
- Keep both enabled: Charon is the front-door router on port 5110, mesh-llm backs local edge capacity, and LiteLLM backs normalized subscription/cloud capacity.

### Degraded / Needs Follow-up

- Provider state buckets currently report 15 ready/healthy and 6 disabled after service restart and route-smoke validation. Catalog visibility still does not guarantee inference success; use `/probe` and route-specific inference checks before marking a route fully operational.
- LiteLLM requires the configured API key; 401 without auth is expected, not a service outage.
- Cloud providers can be blocked by upstream quota/API-key state independently of Charon health.
- `edge_laptop` is disabled in the active provider config and should remain optional until the host is intentionally brought back online and route-tested.
- `hermes_agent_cli` remains useful as a subscription bridge, but it is intentionally kept out of fast interactive/execution/planning lanes unless requested because subprocess startup latency is too high for the default hot path.

## 4. Operational Commands

Reload Charon config after provider edits:

```bash
curl -sS -X POST http://127.0.0.1:5110/reload_config
```

List models through Charon:

```bash
curl -sS http://127.0.0.1:5110/v1/models
```

Inspect Prometheus metrics:

```bash
curl -sS http://127.0.0.1:5110/metrics
```

Run native probe:

```bash
curl -sS -X POST http://127.0.0.1:5110/probe
```

Inspect provider metadata:

```bash
curl -sS 'http://127.0.0.1:5110/providers?compact=true'
curl -sS 'http://127.0.0.1:5110/providers?ids=google,groq,nvidia&include_models=true'
```

Run catalog reconciliation:

```bash
curl -sS -X POST http://127.0.0.1:5110/reconcile_catalogs
```

Inspect operator rollups:

```bash
curl -sS http://127.0.0.1:5110/observability
```

Check service status:

```bash
systemctl --user status arda-charon.service --no-pager -n 30
```

Check LiteLLM directly with auth loaded from `config/.env`:

```bash
set -a
source config/.env
set +a
curl -sS -H "Authorization: Bearer ${LITELLM_API_KEY}" http://127.0.0.1:4000/v1/models
```

Check edge backbone:

```bash
curl -sS http://127.0.0.1:8080/health
systemctl --user status arda-edge-backbone.service --no-pager -n 30
```

## 5. Current Gaps

1. `/probe` is live, but still operator-triggered for the strongest receipt.
   - Next: add a low-rate scheduled probe matrix that records per-tier results without exhausting free providers.

2. `/observability` is live, but dashboards/alerts should be made first-class.
   - Next: expose the same rollups through Grafana/ARDA HUD and alert on rising billing/quota risk, repeated fallback chains, and free-pool shrinkage.

3. Catalog reconciliation repairs model drift, but rollout policy is still human-operated.
   - Next: connect reconciliation output to the model lifecycle registry and keep config mutation behind explicit operator approval.

4. Failure memory is durable enough for routing, but postmortem inspection still requires reading JSONL/state files for full context.
   - Next: add a compact provider/model failure timeline endpoint or CLI view keyed by provider, model, and route ID.

5. `hermes_agent_cli` latency is mitigated, not eliminated.
   - Next: prefer persistent Hermes proxy/direct HTTP drivers for subscription routes and keep CLI startup as a fallback bridge.

## 6. Recommended Next Actions

Reviewed 2026-06-03 against live Charon (`/health`, `/probe`, `/providers`, `/observability`, `/reconcile_catalogs`) and the optimization roadmap.

Immediate:

1. Keep route smoke validation current: catalog (`/v1/models`), health (`/health`), metrics (`/metrics`), config reload (`/reload_config`), probe (`/probe`), provider inventory (`/providers`), observability (`/observability`), reconciliation (`/reconcile_catalogs`), and one cheap completion per dependable provider tier.
2. Keep `litellm_gateway` enabled and monitored. It remains a routed provider; only treat it as failed when authenticated requests or authenticated probes fail.
3. Keep `edge_laptop` disabled unless it is intentionally online and route smoke passes.
4. External docs/status pages should no longer describe Charon as missing `/v1/models`, `/metrics`, `/reload_config`, `/route_history`, `/providers`, `/probe`, `/observability`, or `/reconcile_catalogs`; those surfaces are live and should remain in the smoke contract.
5. Preserve the split between production defaults and probe models. Do not "fix" probe routing by changing a provider's production default.

Short term:

1. Turn the manual smoke set into a repeatable script/report that records pass/fail by provider tier and preserves route IDs for trace correlation.
2. Add dashboard panels/alerts from `charon_provider_state_count{state=...}`, `charon_provider_probes_total{provider,outcome}`, `charon_route_decisions_total{provider,model,task_type,lane}`, `/observability.free_provider_pool`, and `/observability.providers_in_billing_or_quota_risk`; avoid overlapping healthy/down derivations.
3. Use `/reload_config` for provider catalog changes when possible, followed by reload, probe, reconciliation, and provider-state checks.
4. Benchmark the deferred B1.next routing-lock split before changing the candidate-selection lock strategy.
5. Route Hermes subscription traffic through persistent proxy/direct drivers where possible; reserve `hermes_agent_cli` for explicit or fallback use.

Longer term:

1. Promote catalog reconciliation into a model lifecycle registry flow with human-gated config mutation and rollback receipts.
2. Add per-model health/liveness indicators in `/v1/models` or a companion endpoint, extending `streaming_validated`, catalog presence, last success/failure, and quarantine state into a broader operator-facing surface.
3. Refine automatic failover for quota/auth/rate-limit/account failures, building on quota-aware preemption and the proxy retry/failure-feedback path.
4. Keep E2 speculative dual-routing as an explicit opt-in/P2 design until cost controls, cancellation behavior, and observability are specified.
5. Consider compatibility-test coverage against LiteLLM-style routers, OpenRouter-style marketplaces, and Portkey-style gateway expectations if Charon becomes an external-facing product.
