---
soterion:
  sigil: "SCROLL"
  glyph: "𓁿"
  role: "executor"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
---

# arda-varda
Knowledge executor for Arda agents: ingest/query/deep-analysis, source
classification, crawl/github/scholarly extraction, policy-readiness
promotion, human-knowledge scanning, IPC+HTTP transport, uncertainty
deltas, and Athena agent runtime.
Owner: hades | Sigil: 🜏 SCROLL | Status: active

## Summary
`arda-varda` is the largest and best-tested executor crate in the Arda
spine. It implements an opinionated local-knowledge OS: JSONL-backed
digest/books/deep-queue/policy-readiness store, deterministic governance
scaffolding, a pipelined interceptor stack, and both Unix-socket and
HTTP/SSE transport. The `AthenaAgent` front-loads `ingest`/`query`/
`deep_analyze` as local storage paths and falls back to the configured
LLM provider for research/code/decision/general tasks.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/executors/arda-varda`
- Configs/data: `data/athena/*`, `core/state/*`, env-overridable paths
- Tests: 109 passing integration/unit tests + 0 doc tests

## Verification status
- `cargo check -p arda-varda --all-features`: OK
- `cargo test -p arda-varda`: 109 passed, 0 failed
- Coverage highlights: ingest pipeline, crawlers, GitHub/scholarly
  extraction, query, deep-analysis queue, policy promotion, IPC/HTTP
  transport, interceptor pipeline, human ingestion scanner

## Agentic-OS abstractions
- **Ingest pipeline**: source classification, shallow analysis,
  deduplication, digest JSONL, books JSONL, interceptor pipeline with
  Hades/Warden/Mnemosyne hooks
  - Crawl4AI and Scrapling use a shared global admission gate configured by
    `ARDA_ATHENA_CRAWL_MAX_CONCURRENCY` (default 8)
- **Source taxonomy**: GitHub repo/file, scholarly link, documentation,
  news, government doc, note, code snippet, PDF, X post/bookmark,
  chat export
- **Deep analysis**: queued deep processing, extraction, implementation briefs,
  scholarly title generation, uncertainty sampling, and persistent
  query+document+model-addressed result caching with evidence invalidation
- **Query delivery**: normalized, field-weighted BM25 matches carry typed
  source/document spans and `shallow_only` status; confidence tie-breaking is
  gated by triad plus policy-ready governance. HTTP supports both the existing
  sorted JSON response and incremental `athena.query.v1` SSE events
- **Knowledge store**: `AthenaStore` owns:
  - `digest.jsonl`, `books/`, `deep_queue.jsonl`, `deep_graph.jsonl`
  - `scholarly_reenrichment.jsonl` with pending/failed/completed events and
    a bounded queue processor that persists recovered shallow metadata
  - `policy_readiness.jsonl`, `planning_task_receipts.jsonl`
  - `crawl_receipts.jsonl`, `uncertainty_selections.jsonl`
  - atomic schema-v1 `digest-index-v1.json`, shared across processes/restarts
    and incrementally merged for each changed source
  - one shared per-path buffered JSONL appender across cloned stores; handles
    are reused, record visibility remains immediate, and durability syncs are
    interval-batched rather than repeated for every append
  - `athpl_<uuid>` pipeline IDs spanning crawl/import receipts, scholarly
    recovery, ingest/book, policy/deep, view, triage, and interceptor records;
    older JSONL records remain readable through defaulted fields
- **Policy-readiness surface**:
  - gate evaluation, promotion, regression tracking
  - opposition viewpoint harvesting
  - evidence-driven planning-task generation
  - primary/synthetic policy-ready and reference-only counters
- **Human ingestion scanner**:
  - contract `arda.human_ingestion_result.v1`
  - frontmatter validation, status/authority/source-type inference,
    conflict/candidate detection, contradiction candidates JSONL
- **Interceptor pipeline**:
  - `HadesQueueInterceptor`, `WardenQueueInterceptor`,
    `MnemosyneInterceptor`
  - typed `DigestEvent` lifecycle
  - ordered, non-vetoing `before` hooks and post-persistence, best-effort
    `after` hooks
- **Governance/telemetry hooks**:
  - triad validation, bacon-lite logging, resonance/love/joule scoring
  - `AthenaMetrics` snapshots, deep-queue status counts
  - persisted full-refresh timestamps, `/status` source freshness summaries,
    and `athena_source_age_seconds{source_id}` Prometheus gauges
  - process-local active crawl guards plus durable newest-eight completed
    pipeline and latest-error summaries shared by HTTP, IPC, and SSE status
- **Transport**:
  - `AthenaDaemon` runs IPC Unix-socket server + optional HTTP/SSE
  - bounded by `try_run_bounded` from `arda-core/background.rs`
  - configurable timeouts via env
- **Inference routing**:
  - sync bridge to Charon/router via IPC or HTTP
  - env-configurable socket/URL with fallback
  - shared async/blocking reqwest pools also serve crawl, GitHub, and scholarly
    importer traffic with explicit timeout and idle-pool bounds
- **Learning/uncertainty**:
  - `KnowledgeDelta` TTL records emitted to JSONL
  - confidence/uncertainty couple with schema validation

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | `AthenaAgent` implementing `arda-core::Agent` |
| `ingest.rs` | Store, types, pipeline orchestration |
| `ingest/*` | activity, crawl, deep, extraction, github, http_client, importers, index, interceptor, io, layout, metrics, observability, policy, query, remediation, routing, scholarly, source, uncertainty_sampler, views |
| `human.rs` | Human-root scanner + frontmatter contract |
| `learning.rs` | `KnowledgeDelta` TTL emissions |
| `transport/` | IPC + optional HTTP/SSE daemon transport |
| `README.md` | Sigil/metadata/capabilities overview |

## Consumer wiring
- Acts as executor target for Athena workflows
- Depends on `arda-core`, `arda-governance`, `arda-vaire`,
  `arda-economics`, `arda-mandos`
- Indirectly wired into boardroom/council/Hades via interceptor
  pipeline and planning-task receipts

## Ideas for improvement
1. Persist/share the source-classification cache across process restarts; the
   current cache is process-local and keyed by a full content hash.
2. Add configurable stale-source thresholds and alerts atop the source-age
   gauges; freshness metadata and pipeline correlation are complete.
3. Normalize duplicated layout roots: `ingest/layout.rs` +
   `human/library_root` + `machine/library_root` should share one
   `WorkspaceLayout`
4. Make `AthenaStore` async or clearly document sync-blocking regions;
   current sync/async mix shows up in `run_async_for_sync` bridge
5. Promote the shared JSONL appender into an append-only ledger trait if
   non-Athena crates need the same buffering and durability contract.
6. Unify `ingest/` and `ingest/ingest/` directories—repo has both
7. Add HTTP SSE stream for deep-analysis queue events instead of only
   polling JSONL
8. Expose `KnowledgeVault` synthesis queue through `engine`/CLI so it’s
   telemetry-visible
9. Add schema-version migration for `deep_queue.jsonl` /
   `policy_readiness.jsonl` so upgrades don’t break old records
10. Reduce `AthenaStore` field count by grouping paths into typed structs
11. Wire governance/learning signals into `engine` metrics or dashboard
12. Split `policy_readiness` into its own crate if it grows further
