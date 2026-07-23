---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: status
  owner: HADES
  status: active
  last_reviewed: 2026-07-22
---

# arda-vaire — status

Verified: cargo test -p arda-vaire 28/28 passing on the local Hermes cutover branch

## health

Current state is active-mvp with working episodic memory, adaptive significance scoring, contract dual-write, and public transport coverage.

## signals

- main path: encode, recall_recent, recall_relevant, identity_state, consolidate, stats, sync_obsidian, status
- signal path: JouleWork + Love Equation + Triad validation + bacon-lite confidence; adaptive overload/repetition/novelty adjustments
- governance path: significance gates store membership; noise writes to noise ledger; contract_memory_root dual-writes v0.1 MemoryRecord
- transport path: IPC + optional HTTP/SSE daemon on 127.0.0.1:5115

## test evidence

Full suite: 24 unit tests + 2 knowledge-delta integration tests + 2 public-flow integration tests. All pass.

## open risks

- retrieval fidelity is below comparable public systems
- observability for runtime effectiveness is still loose
- missing receiver/generic fallback coverage for transport edge cases
- no recall fidelity benchmark against BM25/hybrid

## open tasks

See CHECKLIST.md for authorship. Ownership remains with HADES.
