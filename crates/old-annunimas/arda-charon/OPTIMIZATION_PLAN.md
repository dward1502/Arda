---
soterion:
  symbol: "🪙"
  codepoint: "U+1FA99"
  hex: "0x0001FA99"
  domain: "plutus/charon routing economics"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Charon Optimization & Feature Plan

Drafted 2026-05-07 after the streaming-pinning incident (`nemotron-3-super-free` ate every Hermes call). Three invariant fixes already landed in this session — see `project_charon_streaming_timeout.md`. This plan covers the next layer.

Items are tagged **P0** (do next), **P1** (do soon), **P2** (nice to have). Each item names files to touch and the observable signal that proves it's working.

---

## A. Routing quality

### A1. Per-task-type spread tuning. **DONE 2026-05-08**
`HybridRoutePolicy` now carries `spread_score_band` and `spread_top_cap`, resolved per task type with request-level overrides (`route_spread_score_band`, `route_spread_top_cap`). Defaults:

| Task type                     | Band | Cap |
| ----------------------------- | ---: | --: |
| `chat` / `summary`            | 10%  |   5 |
| `code` / `reasoning` / `research` | 3% |   3 |
| `monitoring` / `background`   | 2%   |   2 |
| fallback                      | 5%   |   4 |

`select_route_candidate` uses those policy values instead of the old fixed 5%/top-4 pool.

### A2. Failure-aware score decay. **DONE 2026-05-07**
`service/route_scoring.rs::provider_score` now applies a multiplicative `score *= 0.88^min(consecutive_failures, 8)` at the end of scoring (skipped if score is already non-positive to avoid sign-flipping near zero). The existing linear `-10 * consecutive_failures` was getting canceled by lane bias / cost bonuses for "favored" providers; the geometric multiplier compounds with the spread band so a flaky-but-not-dead provider naturally drifts out of the weighted-random pool without a binary cooldown.

### A3. Latency-aware scoring. **DONE 2026-05-07**
`provider_score` previously only used `avg_latency_ms` when `policy.latency_sla_ms` was explicitly set. Added a soft latency penalty when no SLA is set, with a per-lane floor and cap:

| Lane                          | Lane floor | Max penalty |
| ----------------------------- | ----------:| -----------:|
| `interactive` / `monitoring`  |     2,000ms|     up to 18|
| `orchestrator` / `execution`  |    10,000ms|     up to 8 |
| `background`                  |    30,000ms|     up to 4 |

Penalty grows linearly past the floor, capped at the lane max. Reasoning-heavy upstreams (OpenRouter, etc.) now naturally lose interactive/chat traffic to fast direct providers without operators having to set explicit SLAs; thinking-heavy execution work isn't starved.

### A4. Sticky-session option (opt-in). **DONE 2026-05-09**
`request.options.session_affinity = "sticky"` now pins subsequent routes for the same `(agent_id, session_id|conversation_id|thread_id|task_type)` to the first selected provider/model for a bounded TTL (`session_affinity_ttl_minutes`, default 15, max 240). Explicit `force_provider_id` / `force_model_id` still wins over affinity.

### A5. Graceful capability filtering. **DONE 2026-05-09**
`ModelState` and `[[provider.model]]` now support a `capabilities` object (`tools`, `streaming`, `structured_output`). Route policy filters models pre-flight for tool use, streaming, and structured output. Runtime `streaming_validated=false` remains an additional streaming gate.

---

## B. Performance / hot-path

### B1. Reduce `providers.read().await` lock churn. **DONE 2026-05-07** (first pass)
Two structural changes:

1. **`route()` no longer holds the providers write lock through the post-decision bookkeeping.** The lock is released as soon as the provider state mutation + decision build is complete; bacon-lite recording, `append_state_event`, `append_governance_event`, `emit_work_signal_background`, `emit_relationship_signal_background`, `emit_memory_event`, and `metrics.observe_route_pick` now all run lock-free. Even with B3's async event writer, we'd still have been serializing every concurrent route through the bacon-lite + mnemosyne emits — that ends here.

2. **New `route_and_resolve()` returns `(RouteDecision, ProviderState)` in one lock window.** The proxy retry loops in `proxy_openai_streaming` and `proxy_openai_request` previously did `route()` (write lock) → `providers.read()` (separate lock acquisition) per attempt to look up the matched provider's connection metadata. Now they get both back from one call. With max_attempts up to ~14 per request, that's a meaningful reduction. `route()` is preserved as a thin wrapper that returns just the decision.

**Verified:** 8 parallel `/v1/chat/completions` calls land cleanly; route_id flows; no error/warn lines beyond the routine echo-gate logging.

**Not done in this pass (deferred to a follow-up with benchmarks):**
- Splitting `route()`'s single write into a brief-write (refresh_provider_windows) → read (candidate selection) → brief-write (reservation bump) sequence. The candidate-selection phase still runs under the write lock, which means concurrent routes still serialize there. The split is straightforward but introduces an A/B race between phases (provider list could be reloaded mid-route) — wants a benchmark + chosen reload-coordination strategy before shipping. Tracked as B1.next.
- Caching `proxy_max_attempts(providers.len())` so the loop doesn't take a separate read lock just to count providers. Trivial; only fires once per request so low priority.

### B2. Cache scored candidates per (task_type, priority, options-hash) for ~500ms. **DONE 2026-05-09**
`CharonService` now keeps a short-lived scored-candidate cache keyed by task type, priority, strictness, forced provider/model, options hash, and the derived route profile (`route_class`, `execution_lane`, `context_window_target`). The default TTL is 500ms and can be tuned with `ANNUNIMAS_CHARON_ROUTE_CANDIDATE_CACHE_MS` (capped at 5s).

The cache stores pre-governance scored provider/model candidates. On cache hit, each candidate is revalidated against the current provider list, eligibility/quota state, exclusions, capability filters, and model health before the existing Echo Gate, policy, lane-cap, sort, and weighted spread logic runs. If all cached entries went stale, Charon recomputes and refreshes the cache.

**Verified:** `cargo test -p annunimas-charon` passes (107 tests). The bursty route path now avoids repeated lane-fitness reads and provider score recomputation for identical short-window request shapes.

### B3. Drop blocking I/O off the proxy hot path. **DONE 2026-05-07**
New `service/event_writer.rs` owns one bounded `tokio::sync::mpsc::Sender<String>` per JSONL file (capacity 4096). One spawned writer task per file holds an open file handle and drains the channel; fsync is coalesced — every 64 lines or every 100ms, whichever hits first — instead of per-line.

`append_state_event` / `append_governance_event` now serialize the JSON (cheap) and `try_send` (cheap). The hot path no longer takes the fs2 lock or fsyncs.

**Failure modes handled:**
- No tokio runtime present (early unit tests) → senders are `None`, every append falls back to the original sync `append_jsonl`. No test code change needed.
- Channel saturated (writer fell behind) → caller falls back to sync append on its own thread, with a `WARN` log so operators can see it. Events are never dropped.
- Writer task fails to open the file at startup → drains the channel and drops; sync fallback handles real writes.
- Service shutdown → senders drop, writer flushes any buffered lines and exits.

**Verified live:** 13 mixed `/route` + `/v1/chat/completions` calls produced 22 events in `state.jsonl`, zero `saturated`/`fsync failed` warnings in the journal. Hot-path latency now bounded by the upstream provider, not by local fsync stalls.

### B4. Reuse reqwest clients per (provider, mode, lane). **DONE 2026-05-07**
`CharonService` now carries `http_clients: Arc<RwLock<HashMap<HttpClientKey, Arc<reqwest::Client>>>>` keyed by `(provider_id, is_stream, execution_lane)`. New helper `http_client_for(provider_id, is_stream, execution_lane)` does double-checked locking and lazily builds clients with the right timeout shape (streaming → connect_timeout + read_timeout; non-streaming → connect_timeout + total .timeout()), preserving the timeout invariant that fixed the original SSE incident. Both `proxy_openai_request` and `proxy_openai_streaming` now fetch from the cache; per-call `reqwest::Client::builder()...build()` is gone.

Bounded set: ~3 lanes × 2 modes × N providers. nvidia's `http1_only` flag is preserved.

### B5. Split `service.rs` per the README's own refactor note. **DONE 2026-05-09**
Focused split landed without behavior changes:

- New `service/route_candidate_cache.rs` owns the B2 scored-candidate cache, cache keys, TTL handling, cached candidate revalidation, and initial candidate scoring.
- New `service/route_sessions.rs` owns sticky session affinity helpers and the `/route_history` ring buffer types/methods.
- New `service/http_clients.rs` owns B4 reqwest client pooling.
- New `service/route_selection.rs` owns candidate filtering, Echo Gate routing effects, policy narrowing, weighted spread selection, and cooldown-bypass fallback.
- New `service/service_events.rs` owns async JSONL event appends, Mnemosyne emission, and Plutus background signal helpers.
- `service.rs` now stays focused on service construction, public state/path APIs, route orchestration, provider snapshots, and the remaining model-probe helpers.

**Result:** `service.rs` dropped from ~1322 lines after the first B5 pass to ~687 lines after the second pass, with the hot routing path split across focused modules.

**Verified:** `cargo test -p annunimas-charon` passes (109 tests).

---

## C. Observability

### C1. First-class Prometheus exporter for routing decisions. **DONE 2026-05-07**
Counters/gauges/histogram emitted on `/metrics`:
- `charon_route_decisions_total{provider, model, task_type, lane}` ✓
- `charon_provider_failures_total{provider, reason_class}` ✓ (reason_class binned via `classify_failure_reason`: `streaming_chunk_decode`, `rate_limited`, `upstream_5xx`, `upstream_4xx`, `timeout`, `connect_failed`, `preflight_blocked`, `hermes_cli_exit`, `other`)
- `charon_streaming_chunk_errors_total{provider, model}` ✓ — directly catches the SSE regression that triggered this work
- `charon_route_score{provider, model}` ✓ (gauge of last score per pick)
- `charon_proxy_latency_seconds{provider, lane}` ✓ (histogram, buckets 0.1/0.25/0.5/1/2/5/10/30/60/120/300s + `+Inf`)

**Implementation:** new `service/metrics.rs` (hand-rolled, no `prometheus` crate dep) with a `Mutex<HashMap>` store. Wired in:
- `service.rs::route` increments `route_decisions_total` + sets `route_score` on every successful pick.
- `service/state_mutation.rs::mark_provider_result` increments `provider_failures_total` on the failure branch (with `classify_failure_reason` binning).
- `transport/http.rs::streaming_upstream_chunk_to_downstream_chunk_with_feedback` increments `streaming_chunk_errors_total` once per dead stream (atomic-guarded).
- `service/proxy.rs::proxy_openai_request` records `proxy_latency_seconds` on success.

**Verified:** 5 chat completions through `/v1/chat/completions` produced `charon_route_decisions_total{...} 5`, `charon_route_score{...} 223.375`, `charon_proxy_latency_seconds_count{...} 4` (1 of the 5 was streaming, which doesn't go through the non-streaming latency hook — that's expected). `charon_streaming_chunk_errors_total` stays at 0 thanks to the read_timeout fix; if a misbehaving provider regresses, this counter will spike before users notice.

**Follow-ups:**
- Add a streaming-specific latency histogram (time-to-first-token, total stream duration) — separate from `charon_proxy_latency_seconds` because stream duration is not a meaningful "latency" without separating it from generation cost.
- Add provider/model labels to `charon_proxy_latency_seconds` (currently provider+lane only) — useful when one provider serves multiple model tiers with different latency profiles.
- Wire metrics into the Beelink Prometheus scrape config (`config/monitoring-setup/prometheus-central.yml`). The `charon` scrape target is declared there, but live validation on 2026-05-21 shows `100.78.138.113:5110/metrics` currently refuses connections, so the remaining work is service-side metrics exposure/restart rather than another local Prometheus file.

### C2. Route receipts: stable IDs and end-to-end correlation. **DONE 2026-05-07**
- `RouteDecision` now carries `route_id: String` (16 hex chars from `rand::random::<u64>()` — no uuid dep).
- `CharonService::route` mints the ID at the top and stamps the decision with it; flows into `state.jsonl`/`governance_events.jsonl` via the existing `route_selected` event serializer.
- `attach_charon_route_metadata` includes `route_id` in the `_charon_route` response body field for non-streaming calls.
- Streaming responses surface it as the `x-charon-route-id` HTTP header (verified live: `curl -D-` shows the header on `/v1/chat/completions?stream=true`).
- `StreamingProxyOutcome` carries `route_id` so the http transport can emit the header without re-querying state.

A single request can now be traced gateway → charon → upstream by grepping the route_id across `state.jsonl`, `governance_events.jsonl`, and any client-side log that captured the response header / `_charon_route.route_id`.

### C3. `/route_history` endpoint. **DONE 2026-05-09**
`CharonService` now keeps an in-memory bounded ring of recent successful route picks (`ANNUNIMAS_CHARON_ROUTE_HISTORY_LIMIT`, default 256). HTTP `GET /route_history` returns the latest route IDs, agent/task, provider/model, lane/class, and pick score.

---

## D. Resilience features

### D1. Active health probes, not just passive failure tracking. **DONE 2026-05-07**
New `service/health_probe.rs` runs an in-process loop every 60s (with a 15s startup stagger) that:

- Snapshots the providers list under a brief read lock.
- For each enabled, HTTP-driver provider with a `base_url`, GETs `{base_url}/models` with a 5s timeout. Cheapest endpoint that every OpenAI-compat upstream (and llama.cpp) supports. Each provider probed in its own `tokio::spawn` so a slow one can't block the sweep.
- Records `charon_provider_probes_total{provider, outcome}` and `charon_provider_probe_latency_ms{provider}`. `outcome ∈ {ok, fail}`. 401s are counted as `ok` (endpoint reachable; auth-rotation events shouldn't pollute the liveness gauge).
- Spawned from `transport::http::run_http_server` at daemon start; idempotent guarded by callers.

**Deliberately does NOT mutate provider state** (consecutive_failures, in_cooldown, etc.). The live failure-feedback path already handles real user-traffic failures; poisoning state from probe blips would create a feedback loop where a flaky network path takes down good providers. That's a job for D4 (quota preemption from response headers) and the existing external `annunimas-charon-inference-probe.timer` (which runs heavier end-to-end tests at 10-minute granularity). This loop is **connection warmer + liveness metric**.

**Verified live:** 30 seconds after restart, all 14 enabled providers reported `outcome="ok"`. Latency profile lined up cleanly with the network topology — edge_backbone 3ms, edge_worker_light 13ms (LAN), litellm_gateway 37ms (loopback), then cloud providers groq 155 / nvidia 182 / cerebras 214 / opencode 267 / zai 491 ms. No journal warnings.

### D2. Per-model SSE format validation. **DONE 2026-05-08**
`ModelState` now has `streaming_validated: Option<bool>`. Unknown (`null`) stays backward-compatible and routable; explicit `false` blocks streaming routes in `model_supports_request`.

The existing `charon probe` command now runs an SSE probe per tested provider/model, validates up to the first 5 non-empty SSE lines as either `data: {...}`, `data: [DONE]`, or `: comment`, reports `streaming_validated` / `streaming_chunks_validated`, and updates daemon/local model state through the new `model_streaming_validation` IPC/HTTP command.

### D3. Circuit breaker tiers. **DONE 2026-05-09**
Cooldown now has a half-open recovery tier inferred from provider runtime state. When a provider's cooldown expires, Charon clears `in_cooldown` but preserves `consecutive_failures >= 3`; that state is treated as half-open. Half-open providers are eligible for only one probe route per `ANNUNIMAS_CHARON_HALF_OPEN_PROBE_STRIDE` attempts (default 10). A confirmed successful upstream result via `mark_provider_result(... ok=true ...)` clears the failure streak and fully reopens the provider; a failed probe advances the failure streak and re-enters cooldown with the normal exponential backoff.

`route_and_resolve` no longer clears a half-open provider's failure streak at reservation time. The provider only fully recovers after the actual proxy result reports success, preventing a freshly recovered provider from receiving a herd of requests before it has proven healthy.

**Verified:** `cargo test -p annunimas-charon` passes (109 tests), including focused coverage for the half-open probe gate and the reservation-vs-confirmed-success behavior.

### D4. Quota-aware preemption. **DONE 2026-05-08**
`apply_provider_rate_limit_hints` now parses remaining + reset headers for request/minute/day windows. Reset hints align Charon's local quota window with upstream timing, and critically low remaining capacity (`<=1` request or `<=5%`) marks the window exhausted before the next route selection. Charon emits `provider_rate_limit_pressure` when it preemptively exhausts a provider window, so operators can verify rebalancing before 429s appear.

---

## E. New features

### E1. Cost-aware routing. **DONE 2026-05-09**
`ModelState` and `[[provider.model]]` config now support optional `cost_per_million_tokens_in` and `cost_per_million_tokens_out`. `cost_target` is accepted as an alias for the existing cost policy (`cheap` → `low`, `premium` → `high`), and `provider_score` now uses declared model cost when scoring cheap/premium routes. Cheap routes penalize high declared token cost; premium routes can slightly favor higher-cost model tiers without affecting balanced routing.

### E2. Speculative dual-routing. **P2**
For `priority=high` agentic tool calls, fire the same request to top-2 candidates simultaneously, return the first that completes, cancel the other. Costs ~2x but cuts tail latency in half.

- Touch: `service/proxy.rs` (new path).

### E3. Streaming response transformation. **DONE 2026-05-09**
Streaming OpenAI shim requests can set `transform.strip_reasoning = true` (or under `extra_body.transform`) to strip `reasoning`, `reasoning_content`, and `reasoning_details` from parseable SSE `data:` JSON chunks before forwarding. Opaque or malformed chunks are passed through unchanged; `transform` is stripped from upstream request payloads.

### E4. Tool-call fidelity guard. **DONE 2026-05-09**
`normalize_openai_response` now auto-repairs known malformed tool-call response shapes:

- Legacy `message.function_call` is promoted to OpenAI-compatible `message.tool_calls[]`.
- Missing tool call `id` values get stable `call_charon_*` IDs.
- Missing `type` becomes `function`.
- Non-string `function.arguments` values are JSON-serialized.
- `finish_reason=function_call` or `stop` is normalized to `tool_calls` when tool calls are present.

This keeps Hermes/client tool loops on the modern `tool_calls` shape even when an upstream emits legacy or incomplete function-call payloads.

### E5. Multi-tenant API keys (per-agent quotas). **DONE 2026-05-09**
Charon now tracks per-agent quota windows per provider so one runaway agent cannot consume all available capacity for a shared provider. The enforcement layer is conservative by default: limits are inactive unless configured by request options or env.

Supported controls:
- Request options: `agent_requests_per_minute`, `agent_requests_per_day` (aliases: `per_agent_requests_per_minute`, `per_agent_requests_per_day`)
- Env defaults: `ANNUNIMAS_CHARON_AGENT_REQUESTS_PER_MINUTE`, `ANNUNIMAS_CHARON_AGENT_REQUESTS_PER_DAY`
- Provider-limit fractions: `ANNUNIMAS_CHARON_AGENT_MINUTE_QUOTA_FRACTION`, `ANNUNIMAS_CHARON_AGENT_DAY_QUOTA_FRACTION`

The gate runs in normal candidate selection and cooldown-bypass selection; reservations increment the provider/agent minute/day window. Existing global provider quotas remain unchanged.

**Verified:** `cargo test -p annunimas-charon` passes (111 tests), including a focused test proving an exhausted agent is blocked without blocking another agent on the same provider.

### E6. Persistent learned routing (light bandit). **DONE 2026-05-09**
Added a lightweight beta-Bernoulli routing learner in `service/bandit.rs`, persisted to `bandit.json` under the Charon runtime root. Charon records the selected `(task_type, provider, model)` at route reservation time, then `mark_provider_result` observes success/failure and updates the corresponding arm.

Scoring integration is deliberately modest: candidate scoring adds a small posterior-mean bonus/penalty after enough observations (`ANNUNIMAS_CHARON_BANDIT_MIN_OBSERVATIONS`, default 3). The bonus weight is tunable with `ANNUNIMAS_CHARON_BANDIT_SCORE_WEIGHT` (default 8), so learned routing biases the existing policy scorer without replacing it.

**Verified:** `cargo test -p annunimas-charon` passes (111 tests), including a focused persistence/score-bonus test.

### E7. Native provider metadata endpoint. **DONE 2026-06-03**
`GET /providers` now exposes provider inventory as an operator surface instead of forcing callers to infer route posture from `/health`, `/v1/models`, and state files.

Supported query controls:

- `ids` / `provider_ids`: comma-separated provider filter.
- `compact=true`: keeps routing metadata while dropping large catalog/base URL/API key/model payload details.
- `include_models=true`: includes per-model state when detailed inspection is needed.

Rows include provider ID, driver, Hermes provider name, enabled/healthy/operational state, streaming support, probe eligibility, probe model/profile, last success/failure timestamps, failure class, model count, and Hermes bridge latency strategy.

### E8. Native `/probe` inference receipt. **DONE 2026-06-03**
`POST /probe` now runs a cheap marker-validated inference check through the same routing/proxy stack as user traffic, while forcing a health-probe execution profile:

- `stream=false`
- `prefer_probe_model=true`
- `context_window_target=1024`
- `route_class="health_probe"`

Probe attempts are structured with route, provider/model, status, marker result, outcome class, throttling information, and inferred provider failure when a proxy path fails before a route receipt can be returned. Probe results are persisted through `record_probe_result`.

Health mutation deliberately treats HTTP-2xx marker misses differently from upstream transport/provider failures. A provider can return a malformed/non-marker answer without being marked dead as if it had failed to serve the request.

### E9. Catalog reconciliation job and endpoint. **DONE 2026-06-03**
`service/catalog_reconciliation.rs` and `POST /reconcile_catalogs` now make provider catalog drift a first-class repair loop.

The reconciliation pass:

- fetches live `/models` catalogs where supported
- compares live and configured model IDs
- handles Google `models/` ID prefixes as equivalent to configured bare IDs
- marks configured models stale when missing from live catalogs
- clears previous catalog-missing quarantine when a model is recognized live again
- persists default model, probe model, and probe profile selections
- emits `provider_catalog_reconciled` and `provider_catalog_reconciliation_complete` events

Providers without a live catalog still get a configured-catalog probe choice persisted, so probe routing does not depend on every upstream offering reliable catalog discovery.

### E10. Widened volatile free-provider pool. **DONE 2026-06-03**
The default free-provider pool now covers OpenRouter, NVIDIA, Groq, Cerebras, Google, and OpenCode. Pool selection is based on configured provider IDs and current metadata rather than a single legacy provider name.

Routing/observability skip providers with recent account, quota, cooldown, or repeated-probe failures. `/observability.free_provider_pool` reports pool membership and skip reasons so operators can see why a nominally free provider is not receiving traffic.

### E11. Probe model separate from production default. **DONE 2026-06-03**
Provider state now carries `probe_model` and `probe_profile` separately from the production default model.

Route policy honors `prefer_probe_model` for probe-shaped requests, and scoring biases health probes toward cheap/probe-safe models rather than large production defaults. This prevents routine health checks from consuming premium models or mutating production defaults just to make a provider probeable.

### E12. Operator observability rollups. **DONE 2026-06-03**
`GET /observability` now returns a compact operational view:

- `top_failures`
- `slowest_active_providers`
- `best_model_per_task_observed`
- `free_provider_pool`
- `providers_in_billing_or_quota_risk`
- `recent_fallback_chains`
- `recent_legacy_route_failures`
- `recent_catalog_reconciliations`
- `recent_routes`

This is separate from Prometheus. Prometheus remains the time-series path; `/observability` is the operator/readiness snapshot.

### E13. Subscription bridge latency reduction. **DONE 2026-06-03**
Hermes-backed routes now have a clearer latency strategy:

- `hermes_agent_cli` readiness caches repeated unsupported model failures.
- `hermes_proxy_driver` supports persistent local proxy process metadata/readiness.
- `codex_responses_driver` uses Charon's pooled HTTP client path instead of spawning Hermes CLI subprocesses.
- Fast interactive/execution/planning lanes avoid `hermes_agent_cli` unless request options explicitly allow it.
- `/providers` exposes `hermes_bridge` metadata so operators can identify slow CLI-backed routes.

### E14. Request-shape-aware learned routing. **DONE 2026-06-03**
Bandit learning now keys by task plus request shape:

```text
task_type | tools={bool} | structured={bool} | stream={bool}
```

A provider/model that succeeds for tool calls no longer inherits the same learned bonus for plain chat by task type alone. This keeps learned routing useful without letting one successful request shape over-bias unrelated workloads.

---

## Suggested execution order

1. **Stabilize what we just shipped:** ~~C1 metrics~~ **DONE**, ~~E8 `/probe`~~ **DONE**, ~~E9 catalog reconciliation~~ **DONE**, ~~E12 `/observability`~~ **DONE**. Next: stand up the Grafana/ARDA panels and scheduled smoke receipts so operators actually look at the data.
2. **Hot-path:** ~~B3 drop blocking I/O~~ **DONE**, ~~B4 reqwest reuse~~ **DONE**, ~~B1 lock coalesce (first pass)~~ **DONE** — B1.next (read/write split for the candidate-selection phase) deferred until benchmarked.
3. **Resilience:** ~~D1 active probes~~ **DONE**, ~~D2 streaming-format validation~~ **DONE**, ~~D4 quota preemption~~ **DONE**.
4. **Routing quality:** ~~A2 score decay~~ **DONE**, ~~A3 latency penalty~~ **DONE**, ~~A1 per-task tuning~~ **DONE**.
5. **Features:** ~~E4 tool-call fidelity~~ **DONE**, ~~E1 cost-aware~~ **DONE**, ~~E5 per-agent quotas~~ **DONE**, ~~E6 learned routing~~ **DONE**. E2 remains deliberately open/opt-in only because speculative dual-routing doubles spend.

Cross-cutting infra: ~~C2 route IDs~~ **DONE** — every routing decision now carries a stable 16-hex correlation ID surfaced in events and as `x-charon-route-id` / `_charon_route.route_id`. Use this when writing the Grafana panels and when adding D1/D2 probes; the probes should mint their own route_ids and emit them so probe results show up alongside live traffic in the same trace tooling.

### Status snapshot after this session

| Item | Status |
| ---- | ------ |
| C1 Prometheus metrics                       | DONE 2026-05-07 |
| C2 Route IDs (correlation)                  | DONE 2026-05-07 |
| B1 lock coalesce (first pass)               | DONE 2026-05-07 |
| B3 async event writer                       | DONE 2026-05-07 |
| B4 reqwest client pooling                   | DONE 2026-05-07 |
| A2 failure-aware score decay                | DONE 2026-05-07 |
| A3 latency-aware scoring                    | DONE 2026-05-07 |
| A1 per-task spread tuning                   | DONE 2026-05-08 |
| D1 active health probes                     | DONE 2026-05-07 |
| D2 per-model SSE validation                 | DONE 2026-05-08 |
| D4 quota-aware preemption                   | DONE 2026-05-08 |
| E1 cost-aware routing                       | DONE 2026-05-09 |
| E4 tool-call fidelity guard                 | DONE 2026-05-09 |
| A4 sticky-session affinity                  | DONE 2026-05-09 |
| A5 model capability filtering               | DONE 2026-05-09 |
| C3 `/route_history` endpoint                | DONE 2026-05-09 |
| E3 streaming strip-reasoning transform      | DONE 2026-05-09 |
| E5 per-agent quotas                         | DONE 2026-05-09 |
| E6 persistent learned routing               | DONE 2026-05-09 |
| E7 `/providers` metadata endpoint           | DONE 2026-06-03 |
| E8 native `/probe` inference receipt        | DONE 2026-06-03 |
| E9 catalog reconciliation                   | DONE 2026-06-03 |
| E10 widened volatile free-provider pool     | DONE 2026-06-03 |
| E11 probe model separate from default       | DONE 2026-06-03 |
| E12 operator observability rollups          | DONE 2026-06-03 |
| E13 subscription bridge latency reduction   | DONE 2026-06-03 |
| E14 request-shape-aware learned routing     | DONE 2026-06-03 |
| Streaming proxy timeout fix (pre-plan)      | DONE 2026-05-07 |
| Routing spread (weighted-random) (pre-plan) | DONE 2026-05-07 |
| Stream chunk error feedback (pre-plan)      | DONE 2026-05-07 |
| E2 speculative dual-routing                 | OPEN / not default; verified 2026-05-20 with source search that no routing-level top-2 race path is implemented |
| Remaining follow-ups                        | OPEN: B1.next candidate-selection lock split/benchmark, streaming latency metrics, Grafana panel validation, dynamic model discovery, per-model liveness surface, and E2 opt-in design |

2026-05-20 status check: B3, D1, D2, D4, and the listed A/C/E completion rows above are implemented. E2 remains intentionally open because speculative dual-routing doubles upstream spend and needs explicit trigger rules, cancellation semantics, and metrics before activation. The old `Everything else` row was too broad after the 2026-05-07 through 2026-05-09 completion passes.
