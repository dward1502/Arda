---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-08-02"
crate: federated-comms
owner: prometheus
status: active
reviewed: "2026-08-02"
---

> Arda Federated Comms: 📜 layered communications doctrine | owner: prometheus | status: active | reviewed: 2026-08-02

# Federated Comms Plan

`FEDERATED_COMMS` is the current Arda layered communications doctrine surface.

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

- `core/state/federated_comms.json` — current as of 2026-08-02
- `core/state/federated_comms_runtime.json` — current as of 2026-08-02
- `core/state/matrix_boardrooms.json` — source of the boardroom routing contract

## Corrections from review (2026-08-02)

- `core/state/queue.jsonl` was not canonical. Consumers read the compact
  `core/state/queue_summary.json` projection first; the append-only raw ledger
  remains implementation-level evidence rather than an active-plan dependency.
- `core/edge/targets.toml` does not exist in this checkout; edge device inventory lives in `core/state/embodied_interface.json` `hardware_targets`.

## References

- Doctrine: `docs/plans/FEDERATED_COMMS.md`
- Core runtime: `core/state/federated_comms.json`
- Runtime state: `core/state/federated_comms_runtime.json`
- Matrix routing: `core/state/matrix_boardrooms.json`
