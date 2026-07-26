---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: HADES
  status: active
  last_reviewed: 2026-07-25
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-07-25

# arda-vaire

Continuous memory and identity persistence service for Mnemosyne.

## Verified surface

- Encode: `encode(InformantEvent)` -> Optional `RecallRecentEntry`
- Recall: `recall_recent(hours, crate_filter)`, `recall_relevant(query, hours, crate_filter, scope, limit)`
- Maintenance: `consolidate(hours)`, `stats()`, `status()`
- Identity: `identity_state()`
- Human bridge: `sync_obsidian(vault_path, max_files)`
- Transport: IPC + optional HTTP/SSE daemon
- Governance path: significance-gated store membership and contract dual-write via `with_contract_memory_root`
- Recall reports: explicit source `confidence` and governance-derived `trust`
- Promotion: semantic/procedural records carry source memory IDs and append a receipt to `archive/promotion_receipts.jsonl`
- Observability: recall request/result totals, last recall fidelity/latency, IPC queue latency, consolidation depth, and promotion receipt totals through `observability_snapshot()` and `stats()`

## Ownership and store boundaries

- `service.rs` owns public contracts and orchestration; `significance.rs` owns deterministic governance scoring.
- `service/store.rs` owns append-only episodic and optional contract-memory writes.
- `service/retrieval.rs` owns scoped lexical ranking and recall-fidelity observation.
- `service/promotion.rs` owns semantic/procedural derivation and promotion receipts.
- `transport/ipc.rs` owns Unix-socket framing and client forwarding; `transport/http.rs` owns HTTP/SSE routing. Transports do not own scoring or persistence policy.

## Verified evidence

Verified 2026-07-25:

- `cargo check -p arda-vaire` — pass
- `cargo test -p arda-vaire` — 29 unit + 5 integration tests pass
- `cargo test -p arda-vaire --no-default-features` — 27 unit + 5 integration tests pass
- `cargo bench -p arda-vaire --bench recall_fidelity` — 600 controlled fixture queries, Hit@1 `1.000`, 63.46 µs/query on this host

The benchmark is a reproducible local fidelity scaffold, not a cross-system result. Public-system comparison requires equivalent datasets, hardware, and scoring definitions.

## Live status

See STATUS.md for current health signals, open risks, and ownership.

## Work queue

See CHECKLIST.md for authorship, ownership, and implementation tracking.
