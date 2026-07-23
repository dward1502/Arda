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
Tests: 81 integration/unit tests as of last review; 0 doc tests

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
- lazy in-memory `DigestIndex` with 300s TTL, warm/invalidate wired
- query scoring on rich Phase-2 fields weighted additive

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

### A2 async/buffered Books JSONL writes — OPEN
Replace per-record `OpenOptions::append().open() + writeln! + sync` in ledger writers with shared async/buffered append logic. Modules: `ingest/io.rs`, `ingest/policy.rs`, `ingest/deep.rs`.
Touch: possibly introduce shared append helper, then adopt here.
Verify: bulk-ingest wall time drops; fsync count drops.

### B4 importer/crawler shared reqwest Client — OPEN
Audit HTTP call sites for per-call `Client::new()`. Thread shared client with read_timeout/connect defaults aligned to workspace.
Touch: `ingest/importers.rs`, `ingest/crawl.rs`.
Verify: connection/tls reuse visible in logs; fewer fd churn.

### B1 crawl concurrency cap — OPEN
Add bounded crawl fanout via semaphore.
Touch: `ingest/crawl.rs`.
Default: 8. Verify: rate to a single host stays under cap and queue does not stall.

### B2 in-memory digest index — DONE
`Arc<RwLock<Option<DigestIndex>>>`, warm/invalidate on ingest/deep write paths.

## Tier C — resilience/quality

### D1 source classification cache — OPEN
Cache classify verdicts by content hash + source kind to avoid re-classify on re-crawl.
Touch: `ingest/source.rs`.

### D2 scholarly fallback/retry — OPEN
Retry budget + offline re-enrichment queue when upstream metadata service fails.
Touch: `ingest/scholarly.rs`.

### D3 interceptor contract docs — OPEN
Add module docstring explaining veto conditions and event contract.
Touch: `ingest/interceptor.rs`.

### D4 malformed policy-readiness schema validation — DONE
Malformed record counter rendered and surfaced in IPC/HTTP.

## Tier D — observability

### C1 Prometheus metrics — DONE
`AthenaMetrics` with ingest/query/deep/policy counters, HTTP `/metrics`, IPC `metrics` command.

### C2 pipeline correlation IDs — OPEN
Mint `pipeline_id` at ingest entry and thread through crawl->importer->scholarly->policy->deep emit.
Touch: `ingest.rs`, `ingest/observability.rs`, plus downstream ledger writers.

### C3 live status/activity surface — PARTIALLY DONE
`/status` exists and refreshes derived gauges. Richer live activity remains open.

### E1 source freshness gauges — OPEN
Track `last_full_refresh_utc` per source; expose `athena_source_age_seconds{source_id}` and refresh fields in `/status`.
Touch: `ingest/source.rs`, status aggregation, metrics renderer.

## Tier E — features

### E3 govern-on-ingest quarantine gate — OPEN
Use `record_bacon_lite` result at ingest to gate landing; heavy failure -> `quarantine: true`.
Touch: `ingest.rs`, `ingest/policy.rs`.

### E4 deep-analysis cache — OPEN
Content-addressed cache keyed on query + relevant_doc_ids + model_id.
Touch: `ingest/deep.rs`, cache module.

### E5 streaming query results — OPEN
SSE endpoint that emits scored hits as scored instead of buffered.
Touch: `transport/http.rs`, `ingest/query.rs`.

### E2 query result citations — OPEN
Structured `citations: [{source_id, doc_id, span}]` on query match.
Touch: `ingest/query.rs`.

---

# 6. Design assessment fidelity gaps

### P0 retrieval fidelity
Replace naive `.contains()` substring scorer with BM25 or small HNSW/hybrid lexical+semantic field-normalized scorer. This is the highest-value change and is independent of other migrations.

### P1 make confidence honest
Until related governance gates are real, avoid deep tie-break outranking solid lexical match. Option A: gate tie-break on real gates; option B: mark confidence `provisional` in URI/CLI. Prefer option A first.

### P2 persist/share index
Persist `DigestIndex` to disk and rebuild incrementally on append instead of full lazy rebuild on mtime.

### P3 normalize ingest text
Stem/lemmatize tokens at ingest time so `running`/`ran` and `auth`/`authentication` collapse. Cheap; complements BM25.

### P4 surface deep-status in query
Return a flag when matches are shallow-only so callers know answer rests on metadata, not extracted knowledge.

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

- [ ] A2 async/buffered Books JSONL writes
- [ ] B4 shared reqwest Client in importers/crawlers
- [ ] B1 crawl concurrency cap
- [ ] D1 source classification cache
- [ ] D2 scholarly fallback/retry + re-enrichment queue
- [ ] D3 interceptor contract docs
- [ ] C2 pipeline correlation IDs end-to-end
- [ ] E1 source freshness gauges + /status fields
- [ ] E3 govern-on-ingest quarantine gate
- [ ] E4 deep-analysis content-addressed cache
- [ ] E5 SSE streaming query results
- [ ] E2 query result citations
- [ ] P0 replace substring scorer with BM25/hybrid lexical+semantic scorer
- [ ] P1 gate deep tie-break on real governance readiness OR mark provisional
- [ ] P2 persist + share DigestIndex across restarts
- [ ] P3 stem/lemmatize tokens at ingest time
- [ ] P4 return shallow-only flag when matches lack extracted knowledge
- [ ] Unify duplicated layout roots into single WorkspaceLayout owner
- [ ] Document sync-blocking regions or make `AthenaStore` async
- [ ] Add schema-version migration for evolving JSONL stores
- [ ] Group `AthenaStore` path fields into typed structs
- [ ] Add HTTP SSE stream for deep-analysis queue events

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
- Highest-value gap to close first: retrieval fidelity. `arda-varda` has the governance and durability layer; it needs BM25/hybrid lexical+semantic retrieval, persisted/shared index state, and normalized tokenization to match public systems while retaining its distinct trust model.

Suggested future-iteration themes after this plan closes
- persistence + benchmark provenance for `DigestIndex`
- token normalization and BM25/hybrid retrieval
- explicit memory consolidation semantics without losing append-only auditability
- interoperability bridges where `arda-varda` memory artifacts can be read by or synchronized to systems such as MemX/Zep/Graphiti
