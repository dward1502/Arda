---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-28"
---

# arda-core

Shared primitives and contracts for the Arda governance spine.

## Purpose
Provide the canonical data model, traits, execution policy, and receipt
semantics so every upper-layer crate speaks the same protocol instead of
rolling its own governance surface.

## What this crate provides
- contracts: `Decision`, `DecisionClass`, `Plan`, `Goal`, `Reflection`,
  `MemoryRecord`, `TriadOutcome`, `PhilosopherVerdict`
- execution: loop dispatcher, reflector, joule market, council billing,
  HALT file, pressure-aware bounded background/sync work
- governance: `GovernanceGates`, policy modes, per-class/action overrides
- AIPKG: schema-aligned manifest/preflight law and fail-closed signed receipt
  chains over explicit governance evidence
- learning: routing bias, best-agent selection, loop economy snapshots
- messaging/audit: append-only JSONL `Ledger`, `Message`, Soterion metadata
- operations: `SystemdClient`, service registry state, contract/service
  record types
- runtime: `LlmProvider`, OpenAI-compatible HTTP client, capability routing

## Verified state
- `cargo check -p arda-core` -> OK
- `cargo test -p arda-core --all-features` -> 111/111 passing
  - 110 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- README/BREAKDOWN/STATUS/INDEX/OWNERSHIP aligned to current source
- Foundation stabilization plan complete as of 2026-07-25; the crate remains
  active for evidence-backed maintenance and additive evolution.

## Primary direct Cargo consumers
- `arda-engine`, `arda-governance`, `arda-aule` (`full-cli`), `arda-mandos`
- `arda-economics`, `arda-orome`, `arda-vaire`, `arda-varda`
- `manwe` (`full`) and `apps/arda-launcher/src-tauri`

## Documentation
- `INDEX.md` — module map
- `BREAKDOWN.md` — module inventory and responsibilities
- `STATUS.md` — build/runtime evidence
- `OWNERSHIP.md` — authority and integration boundaries
- `docs/interop/landscape.md` — current interop evidence and open questions
