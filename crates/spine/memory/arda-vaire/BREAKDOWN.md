---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: memory_service
  owner: HADES
  status: active
  last_reviewed: 2026-07-27
---

# arda-vaire breakdown

## Scope

`arda-vaire` owns local event scoring, persistence, recall, identity-state
synthesis, consolidation, promotion receipts, Obsidian indexing, and optional
IPC/HTTP delivery for the Mnemosyne memory surface.

It does not own global governance policy, consumer-specific task authority,
the Prometheus backend, or the separate
`core/state/mnemosyne_continuity.json` operator projection.

## Compiled source map

- `src/lib.rs` — module declarations and crate-root re-exports
- `src/error.rs` — typed memory and IPC errors
- `src/schema.rs` — durable episodic and continuity schema identifiers
- `src/retrieval_eval.rs` — equivalent-dataset contracts, adapter boundary,
  baseline ranker, and retrieval quality evaluation
- `src/significance.rs` — deterministic significance classification
- `src/service.rs` — public data contracts, orchestration, observability state,
  knowledge-seed bridging, and crate-local tests
- `src/service/store.rs` — data-root resolution, episodic/noise persistence,
  contract dual-write, record decoding, and scope derivation
- `src/service/retrieval.rs` — recent/scoped/relevant recall, lexical ranking,
  knowledge-seed recall, and identity synthesis
- `src/service/promotion.rs` — consolidation, semantic/procedural promotion,
  receipts, and Obsidian synchronization
- `src/service/status.rs` — statistics, status, recent ledgers, and path reports
- `src/transport/mod.rs` — daemon configuration and transport orchestration
- `src/transport/ipc.rs` — Unix-socket JSON command server/client
- `src/transport/http.rs` — default-feature HTTP/SSE routes

Every Rust source file under `src/` is reachable from `src/lib.rs`; there is no
unwired source subtree or module-root collision.

## Data and behavior boundaries

- `encode` evaluates significance and writes either episodic or noise data.
- Episodic records include source, scope, confidence, trust, and chain metadata.
- `recall_relevant` uses bounded lexical ranking with protected-scope weighting;
  it is not a BM25, vector, or hybrid index.
- `consolidate` promotes eligible repeated procedural patterns and emits
  promotion receipts that retain source memory IDs.
- `sync_obsidian` requires an explicit vault path and indexes Markdown content
  into a separate JSONL surface; it does not make human notes canonical machine
  truth.
- Observability reports recall counts/fidelity/latency, IPC queue latency,
  consolidation depth, and receipt totals. Configured services atomically
  persist those snapshots for `arda-aule` consumption.
- HTTP is feature-gated; IPC is compiled independently of HTTP. Neither
  transport is required for direct library consumers.

## Live integrations

- `arda-varda`: unconditional encode consumer
- `arda-outpost-scout`: unconditional scoped encode/recall consumer
- `arda-orome`: `service-runtime`-feature identity and relevant-recall consumer
- `arda-aule`: `full-cli`-feature status consumer
- `manwe`: `adaptive`-feature encode consumer

## Completed review work

The former implementation checklist is complete and was retired into Git
history after this reconciliation. Verified capabilities include:

- default and no-default feature test coverage
- HTTP contract/status/encode/recall and SSE payload coverage
- IPC round-trip, unreachable-socket, malformed-response, and local-default
  behavior
- knowledge-delta coverage for boardroom, human-context, edge-runtime, and
  system-continuity scopes
- confidence/trust disclosure in recall results
- receipt-backed promotion and duplicate/overload/novelty regressions
- recall, IPC, consolidation, and promotion observability
- controlled Hit@1/latency regression benchmark
- equivalent-dataset adapter contract with Hit@1, Recall@K, and MRR evidence
- atomic durable observability/status/statistics export and bounded-label
  `arda-aule` Prometheus ingestion
- append/consolidation soak coverage with malformed-record and restart recovery
- bounded fallback work-signal execution through one shared Tokio runtime,
  verified by the 512-event operator-scale soak
- explicit episodic/continuity schema versions, legacy read migration, and
  unsupported-future-schema disclosure
- canonical `<arda-root>/data/mnemosyne` config/socket defaults with direct
  library use independent of transport daemons

## Verification evidence

Verified 2026-07-27:

- `cargo check -p arda-vaire` — pass
- `cargo test -p arda-vaire --all-features` — 30 unit + 13 integration tests
  pass
- `cargo test -p arda-vaire --no-default-features` — 27 unit + 13 integration
  tests pass
- benchmark target completes 600 queries at Hit@1 `1.000`
- equivalent-dataset gate — 6 queries at Hit@1/Recall@3/MRR `1.000`
- recovery test — 128 append attempts, 8 consolidation cycles, 3 malformed
  durable surfaces, and restart recovery
- explicit operator-scale soak — 512 append attempts and 8 consolidation cycles
- `cargo test -p arda-aule --all-features --test metrics_exporter_cli` — pass
- strict rustdoc initially found invalid `<root>/<id>` markup in `service.rs`;
  the comment was corrected and the strict documentation gate now passes

## Documentation supersession

| Retired path | Current authority | Reason |
| --- | --- | --- |
| `CHECKLIST.md` | `BREAKDOWN.md` completed review work | All implementation items were verified |
| `CRATE_PLAN.md` | `BREAKDOWN.md` completed review work | Completed implementation packet duplicated maintained evidence |
| `STATUS.md` | `README.md` verification + this file | Status and risk prose duplicated maintained docs |
| `OWNERSHIP.md` | Scope and data/behavior boundaries above | Ownership claims were stale and incomplete |
| `docs/plans/MNEMOSYNE.md` | `README.md` + this file | All hardening and comparative gates were completed; active-plan copy retired |
