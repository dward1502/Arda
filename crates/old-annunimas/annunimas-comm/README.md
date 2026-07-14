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
crate: annunimas-comm
kind: protocol
agent: human-interface
realm: communications
capabilities:
  - a2h-messaging
  - approvals
  - status-updates
  - channel-modeling
status: active-prototype
search_tags: [comm, a2h, approvals, discord]
---

# annunimas-comm

Agent-to-human communication protocol definitions.

## Purpose
Define message schemas and queue utilities for human approvals, notifications, clarifications, and task status.

## What's in this crate
- `lib.rs`: message enums/structs, channel types, queue wrapper, helper constructors.
