---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# sigil: REPAIR
---
crate: annunimas-mnemosyne
kind: memory
agent: mnemosyne
realm: knowledge
capabilities:
  - significance-gate
  - episodic-memory
  - recall-recent
  - identity-state
  - daemon-transport
  - consolidation
  - obsidian-sync
status: active-mvp
search_tags: [mnemosyne, memory, identity, episodic, significance]
---

# annunimas-mnemosyne

Continuous memory and identity persistence service (MVP implementation).

## Purpose
Encode significance-weighted memory events, retain episodic records with hash chaining, and provide recall + identity-state summaries for PROMETHEUS and Illuvatar.

## What's in this crate
- `service.rs`: storage, encoding, recall, identity-state synthesis.
- `significance.rs`: Joulework + Love Equation + Triad significance scoring.
- `transport/`: IPC plus optional HTTP/SSE daemon interface.

## Current surface
- Encode events: `encode(InformantEvent)`
- Recall: `recall_recent(hours, crate_filter)`, `identity_state()`
- Maintenance: `consolidate(hours)`, `stats()`, `status()`
- Human bridge: `sync_obsidian(vault_path, max_files)` (indexes Obsidian notes + encodes memory events)
