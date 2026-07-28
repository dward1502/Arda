---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: HADES
  status: active
  last_reviewed: 2026-07-27
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-27

# arda-vaire

`arda-vaire` is Arda's first-class Mnemosyne memory library. It records
significance-weighted episodic events, provides scoped recall and identity
state, consolidates eligible events into semantic or procedural records, and
supports an optional Obsidian index.

## Public boundary

The crate root re-exports `InformantEvent`, `MnemosyneService`, and the durable
schema identifiers. Additional typed results are available through
`arda_vaire::service`; equivalent-dataset retrieval evaluation is available
through `arda_vaire::retrieval_eval`.

Primary service operations:

- write: `encode`
- recall: `recall_recent`, `recall_recent_scoped`, `recall_relevant`, and
  `recall_knowledge_seeds`
- state: `identity_state`, `stats`, `status`, `paths`, and
  `observability_snapshot`
- maintenance: `consolidate` and `sync_obsidian`; Obsidian synchronization
  requires an explicit vault path in library, IPC, and HTTP calls
- optional contract-store copy: `with_contract_memory_root`
- durable runtime export: `with_metrics_root` and `export_runtime_snapshots`

`transport::ipc` is always compiled. The default `http` feature adds the Axum
HTTP/SSE transport. Consumers can use the library without running either
transport server.

## Persistence contract

- Default root: `<arda-root>/data/mnemosyne`
- Root override: `ARDA_MNEMOSYNE_HOME`
- Episodic records: month-partitioned JSONL with chain metadata and schema
  `arda.mnemosyne.episodic.v1`
- Low-significance records: `noise.jsonl`
- Derived stores: `semantic/` and `procedural/`
- Promotion evidence: `archive/promotion_receipts.jsonl`
- Optional contract dual-write: `ARDA_CONTRACT_MEMORY_ROOT`; the value `auto`
  selects `<arda-root>/core/state/memory`

`core/state/mnemosyne_continuity.json` is an operator projection, not a file
written by this crate's persistence implementation. Its explicit projection
schema is `arda.mnemosyne.continuity.v1`. Unversioned episodic records migrate
in memory on read; unsupported future schemas are disclosed in statistics and
skipped rather than interpreted as current evidence.

## Retrieval and observability

`tests/fixtures/retrieval_equivalence_v1.json` is the shared corpus/query
contract for lexical, BM25, vector, or hybrid adapters. The public
`RetrievalAdapter` trait and `evaluate_adapter` function produce Hit@1,
Recall@K, and mean reciprocal rank without coupling the crate to a vendor.

Configured services atomically persist schema-versioned `observability.json`,
`stats.json`, and `status.json` under
`core/metrics/by_crate/mnemosyne`. `arda-aule` consumes observability through
fixed metric families. Its only label values are the bounded sets
`signal={recall_requests,recall_results,queue_observations,promotion_receipts}`
and `operation={recall,queue}`; user, query, source, and tag values never become
Prometheus labels.

## Consumers

Unconditional dependencies:

- `arda-varda` writes completed ingestion events.
- `arda-outpost-scout` writes and recalls scoped observations.

Feature-gated dependencies:

- `arda-orome` uses Mnemosyne under its `service-runtime` feature.
- `arda-aule` reads memory status and exports durable Mnemosyne metrics under
  `full-cli`.
- `manwe` writes events under `adaptive`.

## Verification

Verified 2026-07-27 after first-class hardening:

- `cargo check -p arda-vaire` — pass
- `cargo test -p arda-vaire --all-features` — 30 unit and 13 integration tests
  pass
- `cargo test -p arda-vaire --no-default-features` — 27 unit and 13 integration
  tests pass
- equivalent-dataset lexical baseline — 6 queries, Hit@1/Recall@3/MRR `1.000`
- append/consolidation recovery test — 128 append attempts across 8
  consolidation cycles, malformed episodic/noise/archive lines, and a service
  restart
- explicit operator-scale soak — 512 append attempts across 8 consolidation
  cycles; pass after bounding fallback work-signal execution to one shared Tokio
  runtime
- `RUSTDOCFLAGS='-D warnings' cargo doc -p arda-vaire --no-deps` — pass after
  correcting invalid rustdoc path markup
- `cargo test -p arda-aule --all-features --test metrics_exporter_cli`
  — durable Mnemosyne metric ingestion passes
- all five direct/feature-gated consumer compile gates — pass, including the
  scout bridge's canonical `<arda-root>/data/mnemosyne` resolution

## Documentation

- `BREAKDOWN.md` — implementation map, ownership boundaries, completed review,
  and plan supersession record
