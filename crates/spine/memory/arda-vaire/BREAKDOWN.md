---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "memory_service"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
---

# arda-vaire

Memory service for Arda agents: significance-weighted episodic records,
identity state, recall/recent/relevant, knowledge-seed recall, consolidation,
Obsidian sync, and daemon transport.
Owner: hades | Sigil: 📜 SCROLL | Status: active
Verified: `cargo check -p arda-vaire` OK; default tests 29 unit + 5 integration; no-default tests 27 unit + 5 integration

## Summary
`arda-vaire` is the continuous memory/identity persistence layer for the
Mnemosyne agent. It stores significance-weighted memory events with hash-
chained episodic integrity, provides scoped recall, synthesizes identity
state, and exposes an optional HTTP/SSE + Unix-socket daemon interface.

- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/memory/arda-vaire`
- Data roots: `data/mnemosyne/*`, env-overridable via `ARDA_MNEMOSYNE_HOME`
- Contract dual-write opt-in: `ARDA_CONTRACT_MEMORY_ROOT`
- Public exports: `InformantEvent`, `MnemosyneService`

## Future iterations context: public memory and learning systems

## Public Systems Overview
- Mem0 — scalable long-term personalization, Python-native
- Zep/Graphiti — temporal knowledge graph with validity timestamps
- Letta/MemGPT — explicit memory block management with tiered storage
- LangMem — LangGraph-native long-term memory store
- MemX — Rust, libSQL-backed, local-first; retrieval <90 ms, 100k+ records
- LlamaIndex Memory — document-heavy RAG memory, retrieval-centric
- MemU — structured memory evolution and consolidation

## Where arda-vaire Differs

- arda-vaire is an executor within a governance stack, not a generic RAG framework
- It provides provenance, receipts, policy-readiness promotion, uncertainty sampling, and governance scoring
- Transport is built-in (IPC + optional HTTP/SSE) with unified ownership boundaries
- A controlled local scaffold now measures Hit@1 and latency; no equivalent-dataset public comparison has been run

## Where Public Tooling Differs

- Governance-first ingestion (receipts + promotion) is unique to arda-vaire
- JSONL append-only with malformed-line tolerance is production-ready in arda-vaire
- Single IPC/HTTP/SSE transport simplifies cross-component orchestration
- Explicit love/joule/resonance scoring and triad validation form a distinct trust model

## arda-vaire's Unique Strengths

- Sovereign local AI memory with governance guarantees
- Deterministic receipting and bounded runtime execution
- Provenance-preserving, idempotent storage

## Public Systems Benchmarks

The local fixture scaffold reports Hit@1 `1.000` over 600 queries at 63.46 µs/query on the verification host. This result is a regression baseline only and must not be compared directly with published public-system results until dataset, hardware, and metric definitions are equivalent.

## Recommendations for arda-vaire

1. Harden governance-first promotion so no recalled task enters without receipts
2. Fix retrieval fidelity by integrating BM25/hybrid search
3. Improve observability to measure runtime effectiveness
4. Make store boundaries and ownership explicit
5. Make confidence/trust explicit for downstream agents

## Public Memory & Learning Systems Reference

- **Mem0** — production memory layer, episodic + semantic with vector (+ graph) retrieval
- **Zep/Graphiti** — temporal knowledge graph with validity timestamps
- **Letta/MemGPT** — explicit memory block management with tiered storage
- **LangMem** — LangGraph-native long-term memory store
- **MemX** — Rust, libSQL-backed, local-first retrieval performance
- **MemU** — structured memory evolution and consolidation
- **LlamaIndex Memory** — document-heavy RAG memory, retrieval focus

## Action Items

- Conduct an equivalent-dataset comparative retrieval benchmark before selecting BM25/vector/hybrid changes.
- Export the current process-local observability snapshot to the canonical durable metrics backend when an operational consumer is selected.
