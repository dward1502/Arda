---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-25"
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
- learning: routing bias, best-agent selection, loop economy snapshots
- messaging/audit: append-only JSONL `Ledger`, `Message`, Soterion metadata
- operations: `SystemdClient`, service registry state, contract/service
  record types
- runtime: `LlmProvider`, OpenAI-compatible HTTP client, capability routing

## Verified state
- `cargo check -p arda-core` -> OK
- `cargo test -p arda-core` -> 99/99 passing
  - 98 unit tests
  - 1 smoke test: `sovereign_baseline_contract_is_migrated`
  - 0 doc-tests
- README/BREAKDOWN/STATUS/PLAN aligned to current source
- Foundation stabilization plan complete as of 2026-07-25; the crate remains
  active for evidence-backed maintenance and additive evolution.

## Documentation
- `INDEX.md` — module map
- `BREAKDOWN.md` — module inventory and responsibilities
- `STATUS.md` — build/runtime evidence
- `PLAN.md` — combined plan and checklist for current and future work
