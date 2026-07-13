---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: REPAIR
---
crate: annunimas-charon
kind: routing
agent: charon
realm: command
capabilities:
  - provider-routing
  - priority-policy-routing
  - quota-tracking
  - health-state
  - provider-result-circuit-breaker
  - active-inference-probes
  - catalog-reconciliation
  - route-observability
  - ipc-http-transport
status: active-mvp
search_tags: [charon, inference, routing, quota, providers]
---

# annunimas-charon

Provider-agnostic inference routing daemon for CITADEL agents.

## Purpose
Track provider health/quota/catalog state and route model requests to the best currently available provider/model for each task shape.

Charon is now the Annunimas LLM operations control plane, not only a simple OpenAI-compatible proxy. It combines local, edge, cloud, subscription, and Hermes-backed routes; remembers provider/model failures; reconciles provider catalogs; exposes active probe and observability surfaces; and keeps production default models separate from cheap health-probe models. Route governance is deterministic and local: `service/echo_gate.rs` handles regex pre-route risk/evidence checks, while `service/route_policy.rs` consumes `annunimas-governance::evaluate_governance_chain` for `single`, `triad`, and `chain` method metadata and forwards live chain, Resonance, Love Dynamics, Triad Philosopher, and JouleWork source fields into `RouteDecision.governance`.

## What's in this crate
- `service.rs`: service construction, public state/path APIs, provider snapshots, and route orchestration.
- `service/route_selection.rs`, `service/route_policy.rs`, `service/route_scoring.rs`: provider/model eligibility, execution profiles, scoring, free-pool policy, learned route bias, and live governance-chain route metadata.
- `service/state_mutation.rs`, `service/runtime_state.rs`, `service/state_io.rs`: durable provider/model health, cooldown, latency, and failure memory.
- `service/catalog_reconciliation.rs`: live `/models` reconciliation, stale configured-model detection, default/probe model selection, and repair of catalog-missing quarantines.
- `service/observability.rs`: operator rollups for failures, slow providers, free-pool health, fallback chains, recent routes, and catalog reconciliation.
- `service/hermes_cli_driver.rs`, `service/hermes_proxy_driver.rs`, `service/codex_responses_driver.rs`: subscription/Hermes bridge paths and pooled direct driver support.
- `transport/`: IPC + optional HTTP/SSE daemon interfaces.
- `types.rs`: request envelope, provider/model state, route decision schemas.

## Owns
- provider pool assembly and default/config merge
- route selection and proxy execution
- provider health and cooldown state
- provider/model failure memory and model quarantine
- catalog reconciliation and probe-model/default-model repair
- active inference probe receipts
- operator observability rollups
- fleet bootstrap interpretation for edge-backed inference lanes
- route receipts and state persistence

## Reads
- [`config/charon.providers.toml`](/var/home/mythos/Annunimas/config/charon.providers.toml)
- [`config/governance/chains.toml`](/var/home/mythos/Annunimas/config/governance/chains.toml)
- [`config/fleet.toml`](/var/home/mythos/Annunimas/config/fleet.toml)
- [`core/edge/targets.toml`](/var/home/mythos/Annunimas/core/edge/targets.toml)
- environment variables for provider credentials and runtime controls

## Writes
- [`data/charon/`](/var/home/mythos/Annunimas/data/charon)
- [`core/state/`](/var/home/mythos/Annunimas/core/state)

## Main Interfaces
- CLI: [`annunimas-cli charon ...`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/commands/charon.rs)
- HTTP: [`src/transport/http.rs`](/var/home/mythos/Annunimas/crates/annunimas-charon/src/transport/http.rs)
- IPC: [`src/transport/ipc.rs`](/var/home/mythos/Annunimas/crates/annunimas-charon/src/transport/ipc.rs)

## HTTP Surfaces
- `GET /health`: provider readiness summary and recent route counters.
- `GET /providers`: provider inventory with routing metadata; supports `ids`, `provider_ids`, `compact`, and `include_models`.
- `POST /probe`: cheap end-to-end inference probe with structured attempts, marker validation, route receipt, and provider-aware throttling.
- `POST /reconcile_catalogs`: compare configured and live catalogs, persist default/probe model choices, and clear stale catalog-missing quarantines when repaired.
- `GET /observability`: operator rollups for top failures, slow active providers, best observed model per task, free-provider pool health, billing/quota risk, fallback chains, catalog reconciliation, and recent routes.
- `GET /v1/models`, `POST /v1/chat/completions`: OpenAI-compatible model catalog and proxy paths.
- `GET /metrics`, `GET /route_history`, `POST /reload_config`: Prometheus, route correlation, and config reload surfaces.

## Routing Notes
- Tool/code requests default to `tool_oriented` on the `execution` lane.
- Bare OpenAI `tools` schemas with `tool_choice=auto` are treated as available
  tools, not mandatory tool execution. Charon records `tools_available` and
  `tool_schema_count`, then strips `tools`/`tool_choice` before proxying plain
  chat/status turns. Explicit `tool_use_required=true`, required `tool_choice`,
  tool history, agentic metadata, or regex-detected execution intent still
  routes as tool execution.
- Large-context agentic execution, default threshold 64K tokens via `ANNUNIMAS_CHARON_LARGE_TOOL_CONTEXT_THRESHOLD`, prefers external tool-capable candidates when policy allows cloud routing. This keeps low-power local models available as fallback while preventing Hermes/Ares-style oversized tool-call turns from being forced through `edge_backbone_coder`.
- OpenRouter `:free` and `/free` models are not default candidates for agentic tool execution, even when runtime metadata advertises tool support. Operators can still opt into that pool explicitly with `allow_free_tool_pool=true` or `tool_pool_strategy=free_first`.
- Local routes still win when privacy requires local execution, `origin_preference=local`, or no external tool-capable candidate fits the request context.

## Common Commands
```bash
cargo run -p annunimas-cli -- charon status
cargo run -p annunimas-cli -- charon providers
cargo run -p annunimas-cli -- charon route --task-type chat --priority normal
curl -sS http://127.0.0.1:5110/health
curl -sS http://127.0.0.1:5110/observability
curl -sS -X POST http://127.0.0.1:5110/probe
```

## Debug Path
- routing bug:
  start with `src/service/route_selection.rs`, `src/service/route_policy.rs`, `src/service/route_scoring.rs`, then `src/types.rs` and `config/charon.providers.toml`
- provider/model health bug:
  start with `src/service/state_mutation.rs`, `src/service/runtime_state.rs`, `src/service/status.rs`, then `data/charon/`
- catalog/probe-model bug:
  start with `src/service/catalog_reconciliation.rs`, then `src/transport/http.rs`
- observability bug:
  start with `src/service/observability.rs`, then `src/service/service_events.rs`
- edge/fleet health bug:
  start with `src/service/bootstrap_runtime.rs`, then `config/fleet.toml`, then `core/edge/targets.toml`
- HTTP/API bug:
  start with `src/transport/http.rs`

## Current Operational Baseline
Reviewed after the June 2026 router hardening pass:

- Release build and service restart succeeded through the runtime build environment.
- `cargo test -p annunimas-charon` passed with 169 tests.
- Live Charon reported 21 total providers, 15 ready/healthy providers, and 6 disabled providers.
- `/probe` returned a marker-verified health-probe route through a cheap probe model.
- `/observability` reported a widened free-provider pool covering OpenRouter, NVIDIA, Groq, Cerebras, Google, and OpenCode with no skip reasons in the final validation snapshot.
- `/reconcile_catalogs` checked 21 providers, found 9 live catalogs, repaired Google model-ID prefix handling, and cleared stale catalog-missing quarantine when live models matched configured models.

See [`CURRENT_PLAN.md`](/var/home/mythos/Annunimas/crates/annunimas-charon/CURRENT_PLAN.md) and [`docs/plans/2026-06-03-charon-router-improvement-plan.md`](/var/home/mythos/Annunimas/docs/plans/2026-06-03-charon-router-improvement-plan.md) for the current runbook and next improvement plan.
