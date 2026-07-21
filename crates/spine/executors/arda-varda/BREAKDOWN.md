---
soterion:
  sigil: "SCROLL"
  glyph: "𓁿"
  role: "executor"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
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
- Tests: 81 passing integration/unit tests + 0 doc tests

## Verification status
- `cargo check -p arda-varda`: OK, only upstream `arda-core` warnings
- `cargo test -p arda-varda`: 81 passed, 0 failed
- Coverage highlights: ingest pipeline, crawlers, GitHub/scholarly
  extraction, query, deep-analysis queue, policy promotion, IPC/HTTP
  transport, interceptor pipeline, human ingestion scanner

## Agentic-OS abstractions
- **Ingest pipeline**: source classification, shallow analysis,
  deduplication, digest JSONL, books JSONL, interceptor pipeline with
  Hades/Warden/Mnemosyne hooks
- **Source taxonomy**: GitHub repo/file, scholarly link, documentation,
  news, government doc, note, code snippet, PDF, X post/bookmark,
  chat export
- **Deep analysis**: queued deep processing, extraction,
  implementation briefs, scholarly title generation, uncertainty sampling
- **Knowledge store**: `AthenaStore` owns:
  - `digest.jsonl`, `books/`, `deep_queue.jsonl`, `deep_graph.jsonl`
  - `policy_readiness.jsonl`, `planning_task_receipts.jsonl`
  - `crawl_receipts.jsonl`, `uncertainty_selections.jsonl`
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
- **Governance/telemetry hooks**:
  - triad validation, bacon-lite logging, resonance/love/joule scoring
  - `AthenaMetrics` snapshots, deep-queue status counts
- **Transport**:
  - `AthenaDaemon` runs IPC Unix-socket server + optional HTTP/SSE
  - bounded by `try_run_bounded` from `arda-core/background.rs`
  - configurable timeouts via env
- **Inference routing**:
  - sync bridge to Charon/router via IPC or HTTP
  - env-configurable socket/URL with fallback
- **Learning/uncertainty**:
  - `KnowledgeDelta` TTL records emitted to JSONL
  - confidence/uncertainty couple with schema validation

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | `AthenaAgent` implementing `arda-core::Agent` |
| `ingest.rs` | Store, types, pipeline orchestration |
| `ingest/*` | crawl, deep, extraction, github, importers, index,
               interceptor, io, layout, metrics, observability,
               policy, query, remediation, routing, scholarly,
               source, uncertainty_sampler, views |
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
1. Normalize duplicated layout roots: `ingest/layout.rs` +
   `human/library_root` + `machine/library_root` should share one
   `WorkspaceLayout`
2. Make `AthenaStore` async or clearly document sync-blocking regions;
   current sync/async mix shows up in `run_async_for_sync` bridge
3. Replace manual JSONL appends with shared append-only ledger trait
4. Unify `ingest/` and `ingest/ingest/` directories—repo has both
5. Add HTTP SSE stream for deep-analysis queue events instead of only
   polling JSONL
6. Expose `KnowledgeVault` synthesis queue through `engine`/CLI so it’s
   telemetry-visible
7. Add schema-version migration for `deep_queue.jsonl` /
   `policy_readiness.jsonl` so upgrades don’t break old records
8. Reduce `AthenaStore` field count by grouping paths into typed structs
9. Wire governance/learning signals into `engine` metrics or dashboard
10. Split `policy_readiness` into its own crate if it grows further
