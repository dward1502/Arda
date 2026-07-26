# arda-varda status

- status: active; `PLAN.md` execution resumed
- latest check: `cargo test -p arda-varda`
- feature check: `cargo check -p arda-varda --all-features`
- evidence: 2026-07-25

## validation evidence
- `cargo fmt --check -p arda-varda` passes
- `cargo test -p arda-varda` — 109 passed, 0 failed; 0 doc tests
- `cargo check -p arda-varda --all-features` passes
- removed stale Python packaging claims; `maturin` is not installed in this environment and `/mnt/cryptothor/Arda` is not present, so the earlier site-packages installation path is not evidence of a working Python packaging flow for this workspace snapshot

## summary
This crate builds and tests cleanly. The combined plan checklist was reconciled
against live code rather than inferred from older status prose. B1 crawl
admission is complete with a configurable global gate and default capacity of
8. D1 source classification now reuses process-local verdicts by full content
hash. D2 scholarly enrichment now has bounded configurable retries, a durable
append-only re-enrichment queue, queue processing over IPC/HTTP, recovered-book
persistence, knowledge-view refresh, and status counts. D3 is complete with an
explicit interceptor lifecycle/order contract. C2 is complete: `athpl_<uuid>`
pipeline IDs now correlate crawl handoffs, importer/batch receipts, scholarly
recovery, ingest/book records, policy readiness, deep queue/book emissions,
knowledge views, triage, and interceptor ledgers while retaining compatibility
with older JSONL records. E1 is complete: ingest records persist full-refresh
timestamps, `/status` reports per-source timestamps/ages and the oldest source,
and Prometheus exports `athena_source_age_seconds{source_id}`. Pre-E1 records
fall back to their processed timestamp. C3 is complete: process-local crawl
guards expose truthful active work and clean up cancellation, durable ledgers
yield the newest eight unique completed pipelines and newest deep/scholarly
error, and crawl failures are retained in process with redacted URLs. HTTP
`/status`, IPC `status`, and SSE `/events` share the snapshot; existing deep and
scholarly pending counts remain its queue-depth fields. A2 is complete through
a shared per-path buffered JSONL appender: handles are reused, full records are
flushed under the cross-process lock, and `sync_data` is interval-batched with a
drop-time durability boundary. B4 is complete through process-wide pooled async
and blocking reqwest clients shared by crawler, GitHub, scholarly, and router
HTTP paths with explicit connect/read/request/pool-idle limits.
E3 is complete: Bacon-Lite is evaluated before ingest landing, and only the
heavy-failure conjunction (`passed == false`, `triad_passed == false`, Bacon
outcome `Fail`) quarantines content. Quarantined receipts remain auditable in
the digest with a versioned reason while Books, derived knowledge views, and
triage emission are skipped. Advisory Bacon failures with a passing triad retain
the established ingest path.
E4 is complete with a persistent SHA-256 content-addressed deep-analysis cache
keyed by normalized query, canonical relevant document IDs, and model ID.
Cache hits avoid duplicate durable side effects, while opposition-evidence
updates invalidate affected entries before re-analysis. E2 is complete with
typed source/document citations and field/byte-range/text spans derived from
the fields that contributed to each query score. E5 is complete through
`POST /query/stream`, which emits `athena.query.v1` SSE `match` events as index
entries are scored and ends with `complete`; the existing sorted `/query`
response remains backward compatible.
P0-P4 retrieval fidelity is complete. Query ranking now uses corpus-aware,
field-normalized BM25 over consistently normalized index/query tokens, including
the documented `run` and `auth` collapses. Self-reported confidence can only
break lexical ties after real triad and policy-ready gates pass. Query matches
add a default-compatible `shallow_only` signal over Rust, buffered HTTP, and SSE
surfaces. The finalized schema-v1 digest index is atomically persisted,
cross-process locked, loaded across restarts/live stores, and incrementally
merged for ingest, deep-analysis, and scholarly source updates.
