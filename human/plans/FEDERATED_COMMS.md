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
# Federated Comms Quick Reference

Status: active (reviewed 2026-04-30; architecture extraction, keep)
Owner: prometheus
Human plan: `human/plans/FEDERATED_COMMS.md`
Core runtime: `core/state/federated_comms.json`
Task ledger: `core/projects/tasks/queue.jsonl`

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
- `core/state/operations_flow.json`
- `core/edge/targets.toml`

## Readable Context

Use `human/plans/FEDERATED_COMMS.md` for the operator-facing plan narrative and graph node.
