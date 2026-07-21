---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: arda-mandos
kind: agent
agent: oracle
realm: governance
sigil: "𓊝"
capabilities:
  - triad-reasoning
  - verdict-generation
  - pageindex-query
  - notification-formatting
status: active-prototype
search_tags: [agent, oracle, governance, triad, reasoning]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active-prototype | reviewed: 2026-05-21

# arda-mandos

Reasoning engine for governance decisions and query verdicts.

## Purpose
Evaluate requests through structured gate logic (Aurelius/Bacon/Sun Tzu style), return verdicts/resonance, and support evidence-aware query processing.

## What's in this crate
- `reasoning.rs`: `OracleEngine`, verdict model, gate analysis.
- `pageindex.rs`: document/page indexing support.
- `notify.rs`: formatting helpers for output channels.
- `context.rs`: reasoning context model.
