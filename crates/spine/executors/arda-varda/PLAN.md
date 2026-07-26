---
soterion:
  sigil: "AEGIS"
  glyph: "◈"
  code_point: "U+25C8"
  role: "knowledge_governance_plan"
  owner: "ARDA-VARDA / HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---
# arda-varda Plan + Checklist

Merged from:
- `crates/spine/executors/arda-varda/OPTIMIZATION_PLAN.md`
- `crates/spine/executors/arda-varda/DESIGN_ASSESSMENT.md`
- `crates/spine/executors/arda-varda/BREAKDOWN.md`
- `crates/spine/executors/arda-varda/STATUS.md`
- `docs/plans/ATHENA.md`
- `docs/plans/original-human-plan-narration/ATHENA.md`

Verification commands
- `cargo check -p arda-varda`
- `cargo test -p arda-varda`
- `cargo check -p arda-varda --features http`

---

# 1. Identity and scope

`arda-varda` is Arda's knowledge executor. ingest, query, digest, deep-analysis,
policy-readiness promotion, interceptor pipeline, IPC+HTTP transport, and
learning emissions for the sovereign corpus.

Crate root: `crates/spine/executors/arda-varda`
Data/core roots: `data/athena/*`, `core/state/*`, env-overridable
Tests: 109 integration/unit tests as of last review; 0 doc tests

Storage surfaces:
- ingest books JSONL per-source, digest/global records
- deep queue, deep graph, policy readiness, planning-task receipts, crawl receipts, uncertainty selections
- append-only, idempotent, malformed-line tolerant

Transport:
- IPC Unix-socket + optional HTTP/SSE daemon, bounded by `try_run_bounded`
- metrics: HTTP `/metrics` + IPC `metrics` command

Runtime notes:
- LLM-backed deep analysis routes to Charon `/v1` by default; env configurable
- IPC io-timeout raised to 120s for LLM extraction; override via `ARDA_VARDA_IPC_IO_TIMEOUT_SECS`

---

# 2. Current runtime/evidence snapshot

- `cargo test -p arda-varda`: pass
- `cargo check -p arda-varda`: OK
- `cargo check -p arda-varda --features http`: pass
- `reqwest v0.12.28` is the sole reqwest line; dependency tree is unified
- Prometheus metrics surface lands over HTTP/IPC
- Malformed policy-readiness lines are counted and surfaced

---

# 3. Agentic-OS abstractions

Ingest pipeline
- source classification, shallow analysis, dedup by source_id
- deterministic local parse; LLM not on ingest hot path
- interceptor stack: Hades, Warden, Mnemosyne hooks

Source taxonomy
- GitHub repo/file, scholarly link, documentation, news, government doc, note
- code snippet, PDF, X/post/bookmark, chat export

Deep analysis
- queued extraction, implementation brief synthesis, scholarly title generation
- uncertainty sampling; `extraction_status` tracks llm_complete/parse_failed/no_llm/no_material

Knowledge store
- `AthenaStore` owns paths above
- schema-v1 `DigestIndex` persisted atomically to `digest-index-v1.json`, shared
  across live stores/restarts, and refreshed incrementally per appended source
- field-normalized BM25 query scoring over normalized shallow/deep tokens

Governance
- triad, bacon-lite, resonance, love equation, joule
- `confidence_self_report` is present; governance/plutus/mnemosyne readiness varies by crate maturity

Transport
- Unix-socket IPC server + optional HTTP/SSE
- bounded task runtime via arda-core

Learning
- `KnowledgeDelta` TTL emissions with schema validation

---

# 4. Hardening contract

- workstation is canonical deep-ingest executor surface
- source provenance survives ingest, policy-ready promotion, task emission
- memory lanes bounded across episodic/source-book/policy-ready/implementation-ready
- task emission deterministic, idempotent, receipt-backed
- runtime observable via /status, metrics, governance receipts

---

# 5. Executable plan

## Tier A — correctness/stabilization

### A1 reqwest unification — DONE
Single `reqwest = 0.12.x` across workspace; no 0.11 TLS/http stack.
Evidence: `cargo tree -p arda-varda | grep reqwest` -> one line.

### A3 boot fallback — DONE
`AthenaStore::from_default_or_workspace_fallback` falls back to workspace data dir on permission errors.

## Tier B — hot-path

### A2 async/buffered Books JSONL writes — DONE
`AthenaStore` clones share a per-path JSONL appender with reusable 64 KiB
`BufWriter<File>` handles. Writes remain append-only, are serialized per path,
retain the cross-process file lock, and flush complete records before unlock so
read-after-write behavior is unchanged. `sync_data` occurs on the first write
and then at a configurable interval (`ARDA_ATHENA_JSONL_SYNC_INTERVAL_MS`,
default 250 ms), with a final flush/sync on appender drop. Independent paths do
not share an I/O mutex. The production helper is used by Books, digest, deep,
policy, scholarly, crawl, and view ledgers.
Touch: `ingest/io.rs`, `ingest.rs`.
Verify: an eight-thread/400-record test produces valid persistent JSONL while
opening one handle and issuing one timed sync instead of 400 opens/syncs.

### B4 importer/crawler shared reqwest Client — DONE
Process-wide async and blocking `reqwest` clients are initialized through
`OnceLock` and reused by Crawl4AI, Scrapling, GitHub, scholarly metadata, and
router HTTP calls. Clients set a 5-second connect timeout, 20-second read
timeout, bounded total request timeout, and 90-second idle pool timeout; GitHub
retains its API-specific user agent and headers.
Touch: `ingest/http_client.rs`, `ingest/crawl.rs`, `ingest/github.rs`,
`ingest/scholarly.rs`, `ingest/routing.rs`.
Verify: pointer-identity coverage proves repeated callers receive the same
async/blocking pools; production crawler and scholarly/GitHub tests use them.

### B1 crawl concurrency cap — DONE
The Crawl4AI async path and Scrapling sync path share the named
`athena_crawl` bounded admission gate. `ARDA_ATHENA_CRAWL_MAX_CONCURRENCY`
configures the global cap; the default is 8. Saturation is returned explicitly
instead of starting unbounded crawl work.
Touch: `ingest/crawl.rs`, `ingest.rs`.

### B2 shared persistent digest index — DONE
`Arc<RwLock<Option<DigestIndex>>>` fronts an atomic, lock-coordinated schema-v1
index on disk. Ingest, deep-analysis, and scholarly writes refresh only the
touched source while retaining unrelated entries; startup and stale live stores
load the shared artifact before considering a full rebuild.

## Tier C — resilience/quality

### D1 source classification cache — DONE
`ClassificationCache` stores deterministic source-kind verdicts by full
SHA-256 content hash and is shared by cloned `AthenaStore` handles. Repeated
inputs and re-crawls reuse the in-process verdict without changing source IDs
or persisted contracts.
Touch: `ingest/source.rs`.

### D2 scholarly fallback/retry — DONE
Scholarly metadata fetches now use a configurable bounded retry budget
(`ARDA_ATHENA_SCHOLARLY_RETRY_BUDGET`, default 3) and delay
(`ARDA_ATHENA_SCHOLARLY_RETRY_DELAY_MS`, default 200 ms). Exhausted upstream
fetches append durable `pending` records to `scholarly_reenrichment.jsonl`
without discarding an available offline fixture. The queue processor retries
pending/failed records, appends terminal status events, persists recovered
metadata as a new shallow book version, refreshes knowledge views, and is
available through IPC `scholarly_reenrich` and HTTP `POST
/scholarly_reenrich`. Queue pending/failed counts and the queue path are
included in status output.
Touch: `ingest/scholarly.rs`, `ingest/source.rs`, `ingest/deep.rs`,
`ingest/observability.rs`, `transport/ipc.rs`, `transport/http.rs`.

### D3 interceptor contract docs — DONE
The module documentation now defines registration order, the non-vetoing
`before` contract, post-persistence `after` events, best-effort failure
isolation, and at-least-once consumer expectations.
Touch: `ingest/interceptor.rs`.

### D4 malformed policy-readiness schema validation — DONE
Malformed record counter rendered and surfaced in IPC/HTTP.

## Tier D — observability

### C1 Prometheus metrics — DONE
`AthenaMetrics` with ingest/query/deep/policy counters, HTTP `/metrics`, IPC `metrics` command.

### C2 pipeline correlation IDs — DONE
Standalone ingest and crawl entry points mint `athpl_<uuid>` identifiers. Crawl
captures retain the crawl identifier, and `ingest_with_pipeline_id` lets an
importer or crawl handoff preserve it rather than minting a disconnected ID.
The identifier is carried by batch/import receipts, ingest records, shallow and
deep book versions, scholarly re-enrichment events, deep queue events,
policy-readiness records, knowledge views/graph events, triage entries, and
Hades/Warden/Mnemosyne interceptor emissions. Persisted record fields use serde
defaults so ledgers written before C2 remain readable. Production-path tests
assert correlation across crawl-to-ingest handoff, ingest-to-scholarly recovery,
and ingest-to-deep-queue/book-to-policy continuations.
Touch: `ingest.rs`, `ingest/crawl.rs`, `ingest/deep.rs`,
`ingest/interceptor.rs`, `ingest/io.rs`, `ingest/scholarly.rs`,
`ingest/views.rs`.

### C3 live status/activity surface — DONE
`AthenaStore` now tracks Crawl4AI and Scrapling operations that are actually
active in the current process. Status reports their correlated pipeline ID,
provider, redacted URL, start timestamp, and elapsed seconds; a drop guard
removes cancelled/early-return work. Durable digest and crawl-receipt ledgers
provide the newest eight unique completed pipelines, while failed deep and
scholarly events plus process-local crawl failures provide the newest valid
`last_activity_error`. Malformed/legacy lines and invalid timestamps are
ignored. Existing deep and scholarly pending counts remain the durable queue
depth surface. HTTP `/status`, IPC `status`, and SSE `/events` serialize the
same status snapshot. Production-path tests cover an actively blocked crawl,
completion, failure/redaction, cancellation cleanup, bounded/deduplicated
history, malformed ledgers, HTTP, IPC, and SSE payloads.
Touch: `ingest/activity.rs`, `ingest.rs`, `ingest/crawl.rs`,
`ingest/observability.rs`, `transport/http.rs`, `transport/ipc.rs`.

### E1 source freshness gauges — DONE
Each new ingest record persists `last_full_refresh_utc`. Status aggregation
selects the latest refresh per source and publishes `source_freshness_total`,
`oldest_source_age_seconds`, and sorted per-source `source_freshness` entries
containing the timestamp and computed age. `/metrics` exports
`athena_source_age_seconds{source_id}` and replaces the complete gauge set on
each status refresh so removed sources do not leave stale series. Pre-E1 digest
records remain visible by falling back to their `processed_at_utc` timestamp;
invalid timestamps and malformed JSONL lines are ignored. HTTP contract,
production status, metric replacement, and legacy-record tests cover the path.
Touch: `ingest.rs`, `ingest/source.rs`, `ingest/observability.rs`,
`ingest/metrics.rs`, `transport/http.rs`.

## Tier E — features

### E3 govern-on-ingest quarantine gate — DONE
Ingest now evaluates and records Bacon-Lite before landing. A heavy governance
failure is the explicit conjunction of a failed Bacon-Lite result, a failed
underlying triad, and a failed Bacon evidence gate. Those records remain in the
append-only digest with `digest_status: "quarantine"`, `quarantine: true`, and a
versioned reason, but do not enter Books, derived knowledge views, or the triage
registry. Advisory Bacon failures whose triad still passes preserve the existing
ingest path; recorder failures are logged and do not masquerade as governance
verdicts. Production-path coverage proves both quarantine isolation and the
ordinary ingest regression boundary.
Touch: `ingest.rs`, `ingest/policy.rs`.

### E4 deep-analysis cache — DONE
Deep analysis now uses a persistent content-addressed cache under
`cache/deep_analysis/`, keyed by a length-delimited SHA-256 digest of the
normalized deep query, canonical relevant document-ID set, and active/default
model ID. Cache hits return the original result without duplicating Book,
queue, digest, governance, or view writes. Opposition-evidence harvests
invalidate entries referencing the changed source so policy-readiness
transitions are recomputed rather than hidden by stale cache state.
Touch: `ingest.rs`, `ingest/deep_cache.rs`, `ingest/policy.rs`.

### E5 streaming query results — DONE
`POST /query/stream` scores index entries on a blocking worker and emits each
positive hit immediately as an `athena.query.v1` SSE `match` event, including
rank, score, and citations, followed by a terminal `complete` event. The
existing sorted/buffered `POST /query` contract remains unchanged.
Touch: `transport/http.rs`, `ingest/query.rs`.

### E2 query result citations — DONE
Every query match now carries structured
`citations: [{source_id, doc_id, span: {field, start, end, text}}]` entries
derived from the actual title, summary, tag, or extracted-knowledge fields
that contributed to its score. Existing persisted/serialized matches remain
compatible through a defaulted empty citation list.
Touch: `ingest.rs`, `ingest/index.rs`, `ingest/query.rs`.

---

# 6. Design assessment fidelity gaps

### P0 retrieval fidelity — DONE
The substring scorer was replaced by corpus-aware, field-weighted BM25 with
document-frequency IDF, term-frequency saturation, length normalization, exact
token boundaries, and scoring across shallow and extracted deep fields.

### P1 make confidence honest — DONE
Self-reported deep confidence contributes only when the persisted deep record
has both `triad_passed` and `policy_readiness == "policy_ready"`. Ungoverned
confidence cannot break an otherwise equal lexical tie.

### P2 persist/share index — DONE
`DigestIndex` schema v1 is atomically persisted under a cross-process lock,
loaded across restarts, shared by concurrently live stores, and merged per
source after ingest, deep-analysis, and scholarly append paths.

### P3 normalize ingest text — DONE
The finalized index stores normalized tokens. Lightweight stemming collapses
`running`/`ran`/`runs` to `run`, and domain aliasing collapses
`authentication` to `auth`; query terms use the same normalization.

### P4 surface deep-status in query — DONE
Each default-compatible `QueryMatch` returns `shallow_only`, derived from the
presence of extracted deep knowledge. Buffered Rust/HTTP responses and SSE
`match` events expose the same signal.

---

# 7. Suggested execution order

1. Workspace hygiene + boot: A1(reqwest), A3(fallback)
2. Hot-path: A2(async writes), B4(shared client), B1(crawl cap)
3. Resilience: D1(source cache), D2(scholarly fallback), D3(docs)
4. Observability: C2(pipeline IDs), E1(freshness gauges), C3(live status)
5. Retrieval fidelity: P0 replacement scorer
6. Features: E3(quarantine), E4(deep cache), E2(citations), E5(streaming)
7. Honesty/ops: P1 confidence gating, P2 persisted index, P3/P4

---

# 8. Combined checklist

- [x] A2 async/buffered Books JSONL writes
- [x] B4 shared reqwest Client in importers/crawlers
- [x] B1 crawl concurrency cap
- [x] D1 source classification cache
- [x] D2 scholarly fallback/retry + re-enrichment queue
- [x] D3 interceptor contract docs
- [x] C2 pipeline correlation IDs end-to-end
- [x] E1 source freshness gauges + /status fields
- [x] C3 live status/activity surface
- [x] E3 govern-on-ingest quarantine gate
- [x] E4 deep-analysis content-addressed cache
- [x] E5 SSE streaming query results
- [x] E2 query result citations
- [x] P0 replace substring scorer with field-normalized BM25 scorer
- [x] P1 gate deep tie-break on real governance readiness
- [x] P2 persist + share DigestIndex across restarts
- [x] P3 stem/normalize indexed and query tokens
- [x] P4 return shallow-only flag when matches lack extracted knowledge
- [ ] Unify duplicated layout roots into single WorkspaceLayout owner
- [ ] Document sync-blocking regions or make `AthenaStore` async
- [ ] Add schema-version migration for evolving JSONL stores
- [ ] Group `AthenaStore` path fields into typed structs
- [ ] Add HTTP SSE stream for deep-analysis queue events

Live reconciliation (2026-07-25): the crate already has an in-memory digest
index with TTL/mtime invalidation, extracted-knowledge-aware scoring,
confidence-bearing deep results, Prometheus metrics, bounded ingest/deep/HTTP
admission, malformed policy-readiness accounting, shallow/deep status labels,
and typed interceptor events. End-to-end correlation IDs, source freshness,
truthful live/durable activity status, the E3 heavy-failure quarantine gate,
content-addressed deep caching, query citations, scored-hit SSE query streaming,
field-normalized BM25 retrieval, governance-gated confidence, normalized tokens,
shared persistent incremental indexing, and shallow-only result signaling are
now complete. Semantic/vector retrieval remains a possible future complement,
not a prerequisite for this checklist.

---

# 9. References

- Crate: `crates/spine/executors/arda-varda`
- Plan narrative: `docs/plans/ATHENA.md`
- Archive narration: `docs/plans/original-human-plan-narration/ATHENA.md`
- Prior optimization Plan: `crates/spine/executors/arda-varda/OPTIMIZATION_PLAN.md`
- Design assessment: `crates/spine/executors/arda-varda/DESIGN_ASSESSMENT.md`
- Breakdown: `crates/spine/executors/arda-varda/BREAKDOWN.md`
- Status: `crates/spine/executors/arda-varda/STATUS.md`

---
# 10. Future iterations context: public memory and learning systems

After this plan is complete, future iterations should consider the following public
memory/learning landscape when extending `arda-varda` beyond governance-first local
knowledge into broader retrieval, consolidation, benchmarking, and interoperability.

Public systems
- Mem0 — scalable long-term memory for agents; Python-native; strong personalization and managed memory semantics.
- Zep / Graphiti — temporal knowledge graph with `valid_at` / `invalid_at` timestamps and hybrid retrieval; Python; strong time-aware recall.
- Letta / MemGPT — explicit memory-block management with working/long-term tiering.
- LangMem — LangGraph-native long-term memory store; Python/LangChain ecosystem.
- memU — structured memory evolution, consolidation, and production persistence semantics.
- LlamaIndex Memory — retrieval-centric memory tied to document-heavy RAG.
- MemX — Rust, local-first, libSQL-backed; claimed sweet spot: 100k+ records, end-to-end recall under 90 ms, Hit@1 ≈ 91.3%, Hit@5 = 51.6%, MRR ≈ 0.380.

Comparison notes for `arda-varda`
- Where public tooling is stronger today: embed/store/search polish, benchmark provenance, agent-memory lifecycle semantics, and consolidation automation.
- Where `arda-varda` is stronger or differentiated: governance-first ingest, append-only provenance and receipts, explicit uncertainty sampling and confidence reporting, built-in IPC+HTTP/SSE transport, and intrinsic hook surfaces for triad/bacon-lite/resonance/love/joule scoring.
- Remaining retrieval opportunity: benchmark the completed BM25 path and add a
  semantic/vector complement only where measured recall warrants its cost.

Suggested future-iteration themes after this plan closes
- benchmark provenance and schema migration strategy for `DigestIndex`
- optional hybrid semantic retrieval measured against the BM25 baseline
- explicit memory consolidation semantics without losing append-only auditability
- interoperability bridges where `arda-varda` memory artifacts can be read by or synchronized to systems such as MemX/Zep/Graphiti
