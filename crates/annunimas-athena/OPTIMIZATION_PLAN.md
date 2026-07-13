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

# Athena Optimization & Feature Plan

Drafted 2026-05-07. P0 / P1 / P2 priorities, file pointers, verification signals.

Scope: `crates/annunimas-athena/` — knowledge-ingest, query, deep-analysis, and synthesis agent. `lib.rs` (agent entrypoint, model routing, LLM execution), `ingest.rs` + `ingest/` (13 submodules: crawl, importers, interceptor, io, scholarly, source, policy, routing, query, views, deep, remediation, observability, layout), `transport/` (IPC + optional HTTP/SSE).

Athena's hot path is the ingest pipeline (network + disk-heavy) and the deep-analysis queue (LLM-bound, runs through the configured `LlmProvider` which currently routes to Charon via the core router). Storage is append-only Books JSONL per-source plus a digest index.

---

## A. Correctness / wiring gaps

### A1. `reqwest = "0.11"` (Cargo.toml). **P0**
Charon and the rest of the workspace are on reqwest 0.12. Athena pulls 0.11 in alongside it, doubling the dependency tree (two TLS stacks, two HTTP clients in the binary). Also: 0.11 lacks `read_timeout` per-Client which means any streaming path here can't apply the same fix that resolved the Charon SSE incident.

- Touch: `crates/annunimas-athena/Cargo.toml`, any reqwest call sites that use 0.11-only APIs.
- Signal: `cargo tree | grep reqwest` shows a single version; binary size drops.

### A2. Synchronous ingest writes. **P0**
`ingest/io.rs` and the various Books-JSONL writers do per-record `OpenOptions::new().append(true).open()` + `writeln!` + `sync_data()`-equivalent. During a bulk crawl this is per-page disk-bound. Same pattern Charon's B3 fixed — adopt the same async writer module (or factor it out of charon into a shared `annunimas-core::jsonl_writer`).

- Touch: `ingest/io.rs`, possibly new shared module in `annunimas-core`.
- Signal: bulk-ingest of a 1k-page corpus completes in noticeably less wall time, iostat shows fewer fsyncs.

### A3. AthenaStore initialization is fail-fast on permission errors. **P1**
`AthenaStore::from_default_or_workspace_fallback` is called inside `AthenaAgent::new`. If the primary path is unreachable (e.g., readonly mount), the agent fails to boot. Mirror Charon's `from_default_or_fallback` pattern: try the default, fall back to a workspace-local data dir on permission errors.

- Touch: `ingest/layout.rs` (path resolution), `AthenaStore::from_default_or_workspace_fallback`.
- Signal: Athena boots even when `data/athena/` is unwritable.

---

## B. Performance / hot-path

### B1. Crawl concurrency is unbounded or unspecified. **P1**
`ingest/crawl.rs` — verify whether crawls fan out without a concurrency cap. An uncapped crawl on a large source (e.g., a 10k-page doc site) can saturate outbound connections, get the source IP rate-limited, and stall the whole ingest queue. Add a configurable concurrency cap (default 8) via `tokio::sync::Semaphore`.

- Touch: `ingest/crawl.rs`.
- Signal: inbound rate to a target host stays under the cap; large crawls don't block other agents' network calls.

### B2. Query path re-reads the digest from disk per call. **P1**
`ingest/query.rs` — likely re-opens the books JSONL or digest on every `query()` invocation. For repeat queries (which the deep-analysis loop emits at high frequency) that's avoidable I/O. Add an in-memory index with a TTL or invalidate on append.

- Touch: `ingest/query.rs`, possibly `AthenaStore` to hold a `RwLock<DigestIndex>`.
- Signal: query latency p50 drops from disk-bound (~ms) to memory-bound (~µs) on warm queries.

### B3. Deep-analysis queue serialization. **P2**
`ingest/deep.rs` — confirm whether the queue is processed serially or with a worker pool. A long deep-analysis (LLM with reasoning) can head-of-line-block other queued items. If serial, add a small `tokio::task::JoinSet` with a configurable worker count (default 2).

- Touch: `ingest/deep.rs`.

### B4. Importers don't share an HTTP client. **P2**
`ingest/importers.rs` — same lesson as Charon B4. One `reqwest::Client` cached per-importer, not per-call.

---

## C. Observability

### C1. No Prometheus metrics. **P0**
Athena emits `observability.rs` events (likely to a JSONL log) but no scrape surface. Add:

- `athena_ingest_documents_total{source_kind, outcome}` — outcome ∈ ok/skip/fail
- `athena_ingest_bytes_total{source_kind}`
- `athena_ingest_latency_seconds{source_kind}` (histogram)
- `athena_query_total{kind}` (kind ∈ keyword/semantic/policy)
- `athena_query_latency_seconds`
- `athena_deep_queue_depth` (gauge — how far behind the deep worker is)
- `athena_deep_runs_total{outcome}`
- `athena_policy_readiness_promotions_total`

Athena's transport already has feature-gated HTTP — add a `/metrics` endpoint there. For non-HTTP deployments expose the same counters via a `metrics()` IPC method.

- Touch: new `metrics.rs`, hooks in `ingest/io.rs`, `ingest/query.rs`, `ingest/deep.rs`, `ingest/policy.rs`.
- Signal: Grafana shows ingest throughput by source; deep-queue depth alarm wires up.

### C2. No correlation ID for ingest pipelines. **P1**
A single ingest can fan out to crawl → importer → scholarly enrichment → policy gate → deep analysis. Each step writes to its own ledger but there's no shared id linking them. Mint a `pipeline_id` at ingest entry, thread it through every event.

- Touch: `ingest.rs` entrypoint + every step.

### C3. No surface for "what is athena doing right now". **P2**
Charon has `/status` and `/state` endpoints. Athena should too — current crawl progress, queue depth, last-N completed pipelines, last error.

---

## D. Resilience / quality

### D1. Source classification doesn't cache verdicts. **P1**
`ingest/source.rs` classifies every document on every ingest call. For a re-crawl of an existing source, every doc gets re-classified. Cache by content hash + source kind.

- Touch: `ingest/source.rs`.

### D2. Scholarly metadata lookup has no fallback. **P1**
`ingest/scholarly.rs` — if the upstream metadata service is down, scholarly enrichment fails closed and the doc gets ingested without metadata (or worse, the ingest fails). Add a retry budget + offline degradation mode where the doc is enqueued for re-enrichment instead of failing.

### D3. Interceptor gate isn't documented in code. **P1**
`ingest/interceptor.rs` — what does an interceptor do, when does it veto, what's the contract? Module needs a docstring at minimum.

### D4. Policy readiness writer has no schema validation. **P2**
`ingest/policy.rs` consumes a `policy_readiness.jsonl`. Each line is parsed leniently — bad records are skipped silently. Add `count_malformed_records` (mirroring Charon's pattern) and surface as a metric.

---

## E. New features

### B2. In-memory digest index for query path. **P1 — done 2026-05-11**
`AthenaStore::query` no longer re-scans `books/` on every call. New module `ingest/index.rs` builds a flat `DigestIndex` of every source's shallow record + latest deep `extracted_knowledge` once, then serves queries from RAM. Invalidated by books-dir mtime change OR explicit invalidate-on-write OR a 300s TTL.

Scoring now searches the rich Phase-2 extraction fields, not just title/summary/tags:
- title match: 2.5
- concept exact: 2.0
- novel_idea: 1.8
- summary: 1.5 (applicability: 1.5)
- pattern: 1.4
- deep_summary: 1.2 (integration_hook: 1.2)
- tag: 1.0
- comparable_system: 0.8
- deep+high-confidence bonus: 0.5 × confidence (only when text score > 0)

Multi-word queries tokenize through a stopword-stripped pipeline AND the full phrase, so "agent context protocol" hits both individual term matches and exact-phrase matches in concept strings.

New `QueryMatch` fields: `concepts_hit` (which concepts matched), `extraction_status`, `confidence_self_report` — all `skip_serializing_if` so legacy clients aren't broken.

Live verified: `athena query "agent context protocol"` ranks `udapy/rust-agentic-skills` first (27.32, 3 concepts matched) over generic agent repos; `athena query "RAG hybrid search BM25"` ranks `ksimback/hermes-ecosystem` first (21.77) by surfacing its exact concept strings.

`AthenaStore::warm_digest_index()` exposed as a public method for callers (e.g. daemon startup) to pre-warm the index. Invalidation is wired into ingest and deep_analyze write paths.

### E0b. Deep-analysis LLM extraction (Library of Alexandria). **P0 — done 2026-05-11**
`AthenaStore::deep_analyze` previously emitted `"deterministic scaffold complete"` — pure template, no LLM. Now runs real LLM-driven knowledge extraction when an LLM provider is attached.

- New module `crates/annunimas-athena/src/ingest/extraction.rs` with strict-JSON system prompt, `ExtractedKnowledge` schema (concepts, patterns, novel_ideas, applicability_to_annunimas, integration_hooks, comparable_systems, risks_or_concerns, confidence_self_report, summary_one_paragraph), and brace-balanced JSON extractor that tolerates code-fenced or preambled responses.
- `AthenaStore` now holds an optional `Arc<dyn LlmProvider>`; daemon attaches it at startup via `with_llm`. CLI local fallback also attaches the LLM so both paths produce real digests.
- Default LLM endpoint is **Charon's `/v1` model router** (`http://127.0.0.1:5110/v1`, model `auto`). Overrides: `ANNUNIMAS_ATHENA_LLM_BASE_URL`, `ANNUNIMAS_ATHENA_LLM_MODEL`, `ANNUNIMAS_ATHENA_LLM_API_KEY_ENV`, or `ANNUNIMAS_ATHENA_LLM_USE_CONFIG=1` to fall back to `config/default.toml`.
- Bumped default IPC io-timeout from 15s → 120s in `transport/ipc.rs` so the daemon path can complete LLM extraction (override via `ANNUNIMAS_ATHENA_IPC_IO_TIMEOUT_SECS`).
- Deep entry `extraction_status` field signals downstream consumers: `llm_extraction_complete`, `llm_extraction_parse_failed`, `llm_extraction_failed`, `no_llm_attached`, `no_extractable_material`. Deep-queue event `reason` mirrors this.
- 6 new unit tests + live verification with both `AdamStrojek/rust-agentai` and `udapy/rust-agentic-skills` producing structured `concepts`/`patterns`/`integration_hooks` keyed to specific Annunimas crates.

### E0. GitHub repo shallow extractor. **P0 — done 2026-05-10**
For `SourceType::GithubRepo` / `GithubFile` URLs, ingest now fetches via the GitHub REST API and populates structured shallow fields (title, summary, language, license, key_dependencies, stars, README excerpt, topics) instead of writing a URL-only placeholder. Auth via `ANNUNIMAS_GITHUB_TOKEN` (raises 60→5000 req/hr). Workspace `Cargo.toml` with no root deps now descends into members (inline and glob) to surface real dependencies. Offline-deterministic via `ANNUNIMAS_ATHENA_FORCE_OFFLINE_GITHUB_METADATA=true`.

- New module: `crates/annunimas-athena/src/ingest/github.rs` (parser, REST client, manifest parsers for cargo/npm/pyproject/requirements/gomod).
- Schema: `ShallowAnalysis.github_metadata: Option<GithubMetadata>` added (skip-serialized when None).
- Tests: 9 new unit tests + 2 existing repo-brief tests now exercise the github path.
- Verified live: `AdamStrojek/rust-agentai` and `udapy/rust-agentic-skills` ingest with real title/license/language/deps populated.

### E1. Per-source SLA / freshness tracking. **P1**
"This corpus was last refreshed N hours ago" is a question operators want to answer. Track `last_full_refresh_utc` per source, expose via `/status` and as `athena_source_age_seconds{source_id}` gauge.

### E2. Query result citations. **P2**
Query returns matching content. Add a structured `citations: [{source_id, doc_id, span}]` field so downstream agents can quote rather than paraphrase. Reduces hallucination in deep-analysis output.

### E3. Govern-on-ingest gate. **P2**
Today the ingest pipeline runs `record_bacon_lite` on the ingest task itself, but doesn't gate on the result. A heavy bacon-lite fail (e.g., source contradicts existing corpus) should at minimum mark the doc `quarantine: true` for review, not silently land it in the corpus.

- Touch: `ingest.rs`, `ingest/policy.rs`.

### E4. Deep-analysis caching. **P2**
A deep-analysis run consumes one or more LLM calls. If the same (corpus_state, query) tuple is asked twice, re-run is wasteful. Add a content-addressed cache (key = hash(query, relevant_doc_ids, model_id)).

### E5. Streaming query results. **P2**
Athena's HTTP transport doesn't currently stream large query results — they buffer. SSE endpoint that emits hits as they're scored would help when the corpus is huge.

---

## Suggested execution order

1. **Workspace hygiene + boot:** A1 (reqwest unify), A3 (boot fallback).
2. **Hot-path:** A2 (async ingest writes), B2 (query memo), C1 (metrics).
3. **Resilience:** D1 source-classify cache, D2 scholarly fallback.
4. **Observability:** C2 pipeline IDs, E1 source freshness gauges.
5. **Features:** E3 govern-on-ingest, E4 deep cache, E2 citations.

A1+A3+C1 is one focused session; that's the P0 stabilization tier.

### Status snapshot

Verified 2026-05-21 against source and targeted Cargo tests.

| Item | Status | Verified evidence |
|------|--------|-------------------|
| A1   | done   | `cargo tree -p annunimas-athena` shows only `reqwest v0.12.28`; workspace duplicate 0.11 users in core/governance were bumped. |
| A2   | open   | JSONL append/write path still needs a dedicated async or shared buffered writer review. |
| A3   | done   | `AthenaStore::from_default_or_workspace_fallback` in `ingest/layout.rs` falls back to workspace `data/athena` on permission errors. |
| B1   | open   | Crawl concurrency cap still needs focused verification/implementation in `ingest/crawl.rs`. |
| B2   | done   | `AthenaStore` owns `digest_index: Arc<RwLock<Option<DigestIndex>>>`; warm/invalidate paths are wired. |
| B3   | open   | Deep-analysis worker-pool behavior still needs focused verification. |
| B4   | open   | Importer HTTP-client reuse still needs focused verification/implementation. |
| C1   | done   | `AthenaMetrics` renders Prometheus text; HTTP `/metrics` and IPC `metrics` command export it; targeted HTTP/IPC tests pass. |
| C2   | open   | Pipeline/correlation id threading is not yet implemented end-to-end. |
| C3   | partial | `/status` exists and now refreshes status-derived gauges, but a richer live activity surface remains open. |
| D1   | open   | Source classification cache still needs implementation. |
| D2   | open   | Scholarly retry/offline re-enrichment queue still needs implementation. |
| D3   | open   | Interceptor contract documentation still needs implementation. |
| D4   | done   | Malformed policy-readiness JSONL lines are counted, exposed in status, and exported as `athena_policy_readiness_malformed_records`. |
| E0   | done   | GitHub shallow extractor previously completed. |
| E0b  | done   | Deep-analysis LLM extraction previously completed. |
| E1   | open   | Source freshness gauges/status fields still need implementation. |
| E2   | open   | Query result citations still need implementation. |
| E3   | open   | Govern-on-ingest quarantine gate still needs implementation. |
| E4   | open   | Deep-analysis cache still needs implementation. |
| E5   | open   | Streaming query results still need implementation. |

### Verification notes — 2026-05-21

Commands run for the C1/D4/A1 stabilization slice:

```bash
cargo fmt -p annunimas-athena
cargo check -p annunimas-athena
cargo test -p annunimas-athena malformed_policy_readiness_gauge_renders_and_snapshots -- --nocapture
cargo test -p annunimas-athena http_contract_metrics_endpoint_exports_prometheus_text -- --nocapture
cargo test -p annunimas-athena ipc_round_trip_ingest_query_status -- --nocapture
source scripts/runtime_build_env.sh && cargo tree -p annunimas-athena | grep 'reqwest v' | sort -u
```

Observed results:

- Formatting completed successfully.
- `cargo check -p annunimas-athena` completed successfully.
- Malformed policy-readiness gauge/status snapshot test passed.
- HTTP `/metrics` Prometheus export contract test passed.
- IPC round-trip now exercises the `metrics` command and passed.
- Dependency tree reported a single reqwest line: `reqwest v0.12.28`.

Implementation evidence:

- `ingest/metrics.rs` owns Athena counters/gauges and Prometheus rendering.
- `transport/http.rs` exposes `/metrics` and refreshes status-derived gauges before render.
- `transport/ipc.rs` exposes the same rendered Prometheus text through the `metrics` command.
- `ingest/observability.rs` counts malformed policy-readiness records and updates the gauge during status aggregation.
- `ingest.rs` includes `policy_readiness_malformed_records` in `AthenaStatus`.

### Next execution slice

Recommended next slice: **A2 + B4**.

1. A2: audit all Books/ledger JSONL append paths in `ingest/io.rs`, `ingest/policy.rs`, `ingest/deep.rs`, and adjacent modules; replace per-record open/write/sync hot paths with a shared buffered/async append helper where safe.
2. B4: audit importer and crawler HTTP call sites for per-call `reqwest::Client` construction; thread a shared client through the importer/crawler path with timeout/read-timeout defaults matching the rest of the workspace.
3. Verification target: add unit coverage for append helper behavior and shared-client configuration, then run `cargo fmt -p annunimas-athena`, `cargo check -p annunimas-athena`, and focused Athena tests.

Keep C2/E1 as the following observability slice after the hot-path I/O and client reuse work lands.
