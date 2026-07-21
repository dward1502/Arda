---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
crate: federated-comms
owner: prometheus
status: active
reviewed: "2026-07-21"
---

> Arda Federated Comms: 📜 layered communications doctrine | owner: prometheus | status: active | reviewed: 2026-07-21

# Federated Comms Plan Narrative

`FEDERATED_COMMS` is the current Arda layered communications doctrine surface.
Historic narration is preserved at
`docs/plans/original-human-plan-narration/FEDERATED_COMMS.md`. This document
retains the prior operator narrative; only references/metadata are updated for
the post-Annunimas Arda layout.

Status: active
Owner: prometheus
Human plan: `docs/plans/FEDERATED_COMMS.md`
Core runtime: `core/state/federated_comms.json`
Task ledger: `core/state/queue.jsonl`

## Purpose

Defines the layered communications doctrine above local sovereign control.

## Current Contract

- local control plane remains Unix sockets and loopback
- trusted device mesh remains Tailscale and internal HTTP/A2A/MCP
- Matrix is the leading federated-room candidate
- Element is the preferred federated client surface
- Discord remains an optional adapter, not the doctrine anchor
- Fetch.ai and economic discovery remain above the sovereign base layer
- future bitmesh transport stays out of the current hot path

## Primary Runtime Surfaces

- `core/state/federated_comms.json`
- `core/state/federated_comms_runtime.json`
- `core/edge/targets.toml`

## References

- Crate/surface: `docs/plans/FEDERATED_COMMS.md`
- Original narration archive: `docs/plans/original-human-plan-narration/FEDERATED_COMMS.md`
- Core runtime: `core/state/federated_comms.json`
