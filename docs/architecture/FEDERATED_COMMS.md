---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "prometheus"
  status: "active"
  last_reviewed: "2026-08-21"
crate: federated-comms
owner: prometheus
status: active
reviewed: "2026-08-21"
---

> Arda Federated Comms: 📜 layered communications doctrine | owner: prometheus | status: active | reviewed: 2026-08-21

# Federated Communications Architecture

`FEDERATED_COMMS` is the current Arda layered communications doctrine surface.

## Purpose

Defines the layered communications doctrine above local sovereign control.

## Current Contract

- local control plane remains Unix sockets and loopback
- trusted network posture remains Tailscale with bounded local/remote probes
- Hermes/Linux Foundation A2A is the cross-process/machine/framework agent wire; it carries Arda work-envelope references but does not mint Arda objectives, approvals, memory, placement, or completion
- MCP is tool/resource invocation, not organism task, peer-identity, or node-health authority
- Oromë owns normalized semantic work/handoff envelopes and transport receipts, not a proprietary peer network
- engine/outpost contracts own enrolled compute-node identity and expiring observations; Hermes profiles/A2A Agent Cards own conversational peer identity
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

- Digital organism authority map: `docs/architecture/DIGITAL_ORGANISM_AUTHORITY_TRANSPORT_MAP.md`
- Doctrine: `docs/architecture/FEDERATED_COMMS.md`
- Core runtime: `core/state/federated_comms.json`
- Runtime state: `core/state/federated_comms_runtime.json`
- Matrix routing: `core/state/matrix_boardrooms.json`
