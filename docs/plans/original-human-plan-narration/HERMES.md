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

# 🪙
# HERMES Quick Reference

Status: in_progress (updated 2026-05-05; system degradation in effect)
Owner: hermes
Human plan: `human/plans/HERMES.md`
Crate: `crates/spine/interface/arda-orome`
Core runtime: `core/state/hermes_command.json`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

HERMES owns communication routing, boardroom state, provider messaging, and external/edge-facing command flow.

## Current Contract

- provider-aware messaging runtime is live but system degraded
- boardroom, council, reroute, and interruption surfaces are exported
- edge-worker bridge patterns are proven against a real remote worker
- operator-facing provider and subcomponent status is available
- Soterion compact communication headers are now governed by `docs/contracts/soterion-communication-contract.md` and `core/state/soterion_communication_contract.json`
- **Discord gateway** is built-in via Serenity library (WebSocket connection)
- No separate `hermes-gateway.service` — legacy Python gateway is disabled

## Primary Runtime Surfaces

- `data/hermes/boardroom.jsonl`
- `data/hermes/reroute_metrics.jsonl`
- `data/hermes/interruptions.jsonl`
- `core/state/soterion_communication_contract.json`
- `docs/HERMES_AGENT_EDGE_BRIDGE.md`

## Readable Context

Use `human/plans/HERMES.md` for the operator-facing plan narrative and graph node.

## 2026-05-25 Live Discord Dispatch Decision

Operator decision: live Discord dispatch is approved for the bounded `general` and `tasks` channel lanes.

Current evidence:

- `core/state/hermes_discord_runtime.json` reports the Discord provider configured and online, with listener status `running`.
- The current delivery posture is `online_no_recent_delivery_receipt`; provider online status is not treated as delivery proof until a send receipt exists.
- `core/state/matrix_boardrooms.json` keeps Discord as a Hermes-moderated optional adapter, while Matrix/Element remains the doctrine anchor.
- `docs/contracts/soterion-communication-contract.md` requires Discord messages to include compact Soterion header, short plain-language fallback prose, and receipt/cause/next action for nontrivial or high-risk state claims.

Safe next live-dispatch steps:

1. Send one low-risk receipt-backed canary to `general` using `cargo run -p arda-cli -- hermes send --provider discord --channel general ...` after policy guard accepts the network scope.
2. Verify a concrete delivery receipt or message id before treating the lane as active.
3. Send one bounded task-status canary to `tasks` with the same Soterion-plus-plain-language format and no secrets/private payloads.
4. Refresh `core/state/hermes_discord_runtime.json` or the equivalent Hermes delivery receipt projection so ARDA/Hermes visibility reports recent outbound/dispatched totals.
5. Keep high-risk, external/private, queue-mutating, or approval-changing Discord dispatches gated through Review Gate packets; Discord channel/message IDs remain delivery metadata, not canonical task identity.

## Open Tasks (4 total)

### 1. Implement richer provider adapters and live streaming surfaces

Current provider adapters are basic. Need to support more providers and streaming responses.

**Status:** Pending

### 2. Strengthen fanout and routing orchestration

Improve message routing for multi-agent coordination and boardroom discussions.

**Status:** Pending

### 3. Expand edge-worker and fleet communication policy

Document and implement policies for edge worker communication and fleet coordination.

**Status:** Pending

### 4. Broaden ARDA HUD consumption of core and human plan surfaces

Enhance ARDA HUD to better display HERMES-related data and status.

**Status:** Pending
