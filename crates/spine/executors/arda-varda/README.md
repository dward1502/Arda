---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
crate: arda-varda
kind: agent
agent: athena
realm: knowledge
sigil: "𓁿"
capabilities:
  - ingest
  - research
  - code
  - decision
  - general
status: operational
search_tags: [agent, athena, llm, knowledge]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: operational | reviewed: 2026-07-25

# arda-varda

Knowledge ingest, query, deep-analysis, and synthesis agent.

## Purpose
Provide local knowledge ingest, corpus query, deep-analysis queue execution,
and provider-routed research support for Athena workflows.

Operationally, the crate now includes:
- append-only digest and per-source Books JSONL ingest persistence through
  shared, per-path buffered appenders with interval-batched durability syncs
- local BM25 query results with normalized tokens, typed source spans,
  shallow-only signaling, and governance-gated confidence
- an atomic schema-v1 digest index shared across restarts/live stores and
  incrementally refreshed per changed source
- deterministic governance/accounting scaffolding
- Unix socket IPC and HTTP/SSE transport surfaces
- CLI-facing daemon/service integration

Current scoped verification: `cargo test -p arda-varda` passes 109 tests.

## Runtime bounds and hook contract

- Crawl4AI and Scrapling share the global `athena_crawl` admission gate;
  `ARDA_ATHENA_CRAWL_MAX_CONCURRENCY` overrides its default capacity of 8.
- Crawl4AI, Scrapling, GitHub, scholarly metadata, and router HTTP calls reuse
  process-wide reqwest connection pools with explicit timeout defaults.
- JSONL writes reuse per-path 64 KiB buffered handles. Complete records flush
  before unlocking; `ARDA_ATHENA_JSONL_SYNC_INTERVAL_MS` controls the
  `sync_data` interval (250 ms by default), and final drop synchronizes writers.
- Scholarly metadata fetches retry within a configurable budget and append
  exhausted fetches to `scholarly_reenrichment.jsonl`. Operators can process
  pending/failed records with IPC `scholarly_reenrich` or HTTP `POST
  /scholarly_reenrich`; successful recovery appends an enriched shallow book
  version and refreshes knowledge views.
- Deep-analysis results are cached under `cache/deep_analysis/` by normalized
  query, relevant document IDs, and model ID; evidence changes invalidate
  affected entries. `POST /query/stream` emits scored, citation-bearing SSE
  matches and a terminal completion event without changing `POST /query`.
- Query and index tokens share lightweight stemming/domain normalization.
  Ranking uses corpus-aware, field-weighted BM25; deep confidence contributes
  only after triad and policy-ready gates pass, and every match reports whether
  it is `shallow_only`.
- `digest-index-v1.json` is atomically written under a cross-process lock,
  loaded at startup, shared by live store instances, and updated per source on
  ingest, deep-analysis, and scholarly append paths.
- Every standalone ingest and crawl mints an `athpl_<uuid>` pipeline ID. The ID
  is returned in crawl/import receipts and persisted through scholarly,
  shallow/deep book, policy-readiness, queue, knowledge-view, triage, and
  interceptor outputs. Use `AthenaStore::ingest_with_pipeline_id` when handing
  a crawl/import result into ingest so the upstream ID remains end-to-end.
- Ingest records persist `last_full_refresh_utc`. `/status` returns sorted
  per-source refresh timestamps and ages plus the oldest source age, while
  `/metrics` exports `athena_source_age_seconds{source_id}`. Older digest
  records use `processed_at_utc` as their compatibility fallback.
- `AthenaStore` crawl wrappers publish currently active Crawl4AI/Scrapling work
  with correlated IDs, redacted URLs, start times, and elapsed ages. `/status`,
  IPC `status`, and SSE `/events` also expose the newest eight unique completed
  pipelines and latest crawl/deep/scholarly error; malformed ledger lines are
  skipped and cancellation cannot leave phantom activity.
- Default interceptors run in order: Hades, Warden, Mnemosyne.
- Interceptors cannot veto core ingest work. Their `after` hooks run after the
  corresponding durable ATHENA write and are best-effort side effects.

Non-ingest task types still route through the configured LLM provider.

## What's in this crate
- `lib.rs`: Athena agent implementation, ingest/query/deep routing, model route selection, and LLM execution flow.
- `ingest.rs`: top-level ingest orchestration plus test coverage for local storage, query, deep-analysis, and policy/event behavior.
- `ingest/`: extracted helper surfaces including scholarly metadata, deep recovery, policy, observability, query, source classification, routing, and views.
- `transport/`: daemon transports (`ipc.rs` and feature-gated `http.rs`) and daemon config/startup wiring.
