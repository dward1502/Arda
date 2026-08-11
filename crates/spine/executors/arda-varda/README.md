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
- one typed `WorkspaceLayout` owner for store, human/machine library, and
  external queue paths
- schema-v2 deep-queue and policy-readiness records with read-time migration
  for unversioned legacy JSONL
- deterministic governance/accounting scaffolding
- Unix socket IPC and HTTP/SSE transport surfaces
- CLI-facing daemon/service integration
- advisory-only Rúmil receipt evaluation; partial, stale, rejected-provider, and missing-evidence receipts require review, and no Rúmil result authorizes execution

Current scoped verification: `cargo test -p arda-varda` passes 120 tests.

Documentation ownership: current crate behavior and the completed P0-P2
decisions live in this README and `BREAKDOWN.md`. Completed plans and status
snapshots are retired after live reconciliation.
Historical assessments and generated validation evidence live under
`docs/archive/arda-varda/`; there is intentionally no crate-local `docs/` tree.

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
  affected entries. Deep-queue batches use a bounded worker pool configured by
  `ARDA_ATHENA_DEEP_WORKERS` (default 2); deep queue/deep-analysis admission is
  bounded by `ARDA_ATHENA_DEEP_QUEUE_MAX_CONCURRENCY` (default 2).
  `POST /query/stream` emits scored, citation-bearing SSE
  matches and a terminal completion event without changing `POST /query`.
- `GET /deep/events?after=<line-id>` follows append-only deep-queue records as
  schema-v2 SSE events. Event IDs are durable JSONL line cursors, so reconnecting
  clients can resume without replaying earlier queue events.
- `AthenaStore` is intentionally synchronous: construction, ledger/query IO,
  ingest/crawl, index mutation, and deep/policy processing are documented
  blocking regions. Long-lived async transports isolate these calls with
  `tokio::task::spawn_blocking`; in-memory metric access remains direct.
- Query and index tokens share lightweight stemming/domain normalization.
  Ranking uses corpus-aware, field-weighted BM25; deep confidence contributes
  only after triad and policy-ready gates pass, and every match reports whether
  it is `shallow_only`.
- `digest-index-v1.json` is atomically written under a cross-process lock,
  loaded at startup, shared by live store instances, and updated per source on
  ingest, deep-analysis, and scholarly append paths.
- `cargo run -p arda-varda --bin arda-varda-benchmark` runs the checked-in
  provenance-bearing fixture and emits machine-readable Recall@1, citation
  correctness, shallow-only rate, latency, and classification-cache profile.
  The benchmark uses an isolated store layout and does not write into the live
  operator library, machine index, governance ledger, or side-effect queues.
- `arda-cli athena-status --root <path>` projects the canonical synthesis queue
  and governance/learning counters without owning a second ledger or queue.
- HTTP and IPC status expose advisory-only stale-source alerts. Configure the
  threshold with `ATHENA_STALE_SOURCE_THRESHOLD_SECONDS` (default seven days).
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
- Bacon-Lite evidence from the canonical store lands in Arda's root
  `data/governance/` and `docs/operator/library/governance/` paths. A store rooted
  outside the workspace uses that store root for both outputs, which keeps tests
  and isolated deployments from recreating crate-local documentation trees.
- Default planning tasks, learning deltas, and the Athena IPC socket resolve from
  `ARDA_ROOT` (or the detected workspace root), never from the process working
  directory. Running Varda from its crate directory therefore cannot recreate
  crate-local `core/` or `data/` trees.

Non-ingest task types still route through the configured LLM provider.

## Governed external-source decisions

- Reddit is the sole external-source receipt pilot. A versioned canonical
  receipt must validate before task promotion; incomplete receipts fail closed.
- NotebookLM remains a non-authoritative, read/query-only synthesis lane. It is
  blocked from task promotion and from auth, mutation, cleanup, and audio tools
  without explicit user approval.
- Persistent classification caching is deferred: the checked-in benchmark
  profile found uncached classification far below the 100 µs decision threshold,
  so disk persistence would add complexity and I/O without measured benefit.
- Hybrid semantic retrieval is deferred until the benchmark corpus contains a
  reproducible BM25 miss. The current baseline is Recall@1 1.0 with citation
  correctness 1.0, so no embedding/vector dependency was added.
- A shared ledger trait is deferred because the live repository has only one
  concrete external-source ledger consumer. The current JSONL owner remains
  authoritative until a second non-speculative consumer exists.

## What's in this crate
- `lib.rs`: Athena agent implementation, ingest/query/deep routing, model route selection, and LLM execution flow.
- `ingest.rs`: top-level ingest orchestration plus test coverage for local storage, query, deep-analysis, and policy/event behavior.
- `ingest/`: extracted helper surfaces including typed layout ownership, JSONL
  schema migration, scholarly metadata, deep recovery, policy, observability,
  query, source classification, routing, and views.
- `transport/`: daemon transports (`ipc.rs` and feature-gated `http.rs`) and daemon config/startup wiring.
