---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: arda-hades
kind: agent
agent: hades
realm: operations
sigil: "𓁷"
capabilities:
  - sweep
  - lifecycle-cleanup
  - orphan-detection
  - removal-queue
  - audit-log
status: active-mvp
search_tags: [agent, hades, cleanup, lifecycle, sweep, soterion]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active-mvp | reviewed: 2026-05-21

# arda-hades

Cleanup, lifecycle, and order-maintenance service for CITADEL artifacts.

## Purpose
Traverse managed paths, interpret Soterion sigils, queue and execute safe lifecycle actions, and keep an append-only HADES audit ledger.

## What's in this crate
- `service.rs`: sweep engine, queue/log/status, orphan/coin handling, archive/remove flow.
- `transport/`: IPC + optional HTTP/SSE daemon.
- `agent.rs`: `HadesAgent` integration for core router.
- `types.rs`: action/sigil/sweep models.
