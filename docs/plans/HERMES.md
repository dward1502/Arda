---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-06-30"
crate: arda-orome
agent: hermes
realm: interface
reviewed: "2026-06-22"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# 🪙
# HERMES Plan Narrative

`HERMES` is now implemented in `crates/spine/interface/arda-orome`. Historic
narration is preserved at `docs/plans/original-human-plan-narration/HERMES.md`.
This document preserves the detailed HERMES operator narrative while updating
Arda crate and surface names. The prior Hermes crate surface now maps to
`arda-orome`; the plan below retains both current runtime claims and older
state evidence useful for operator review.

Status: in_progress
Owner: hermes
Human plan: `docs/plans/HERMES.md`
Crate: `crates/spine/interface/arda-orome`
Core runtime: `core/state/hermes_command.json`
Task ledger: `core/state/queue.jsonl`

## Purpose

HERMES owns communication routing, boardroom state, provider messaging, and external/edge-facing command flow.

## Current Contract

- provider-aware messaging runtime is live
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

Use `docs/plans/HERMES.md` for the operator-facing plan narrative and graph node. Historic narration is preserved at `docs/plans/original-human-plan-narration/HERMES.md`.

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

## Degraded / Blocked Work

The older narration also notes an earlier degraded-system posture; that remains useful operator context. Current state indicates live Discord provider status, but prior degradation signals should not be assumed cleared until delivery receipts appear in `core/state/hermes_discord_runtime.json` or equivalent projection.

## Hardening Contract

- Commands remain gated by Soterion headers, plain-language fallback, and receipt/cause/next-action requirements for nontrivial claims.
- External messaging lanes require explicit approval, receipt proofs, and human/WARDEN review before high-risk or queue-changing dispatch.

## Verification

- `cargo check -p arda-orome` / corresponding interface-path workspace checks
- `python -m json.tool core/state/hermes_discord_runtime.json >/dev/null`
- `python -m json.tool core/state/soterion_communication_contract.json >/dev/null`
- `scripts/check_task_queue_append_only.sh`

## Alignment with Arda Principles

- Evidence-first dispatch
- Receipt-backed messaging truth
- Operator-visible comms state
- Safety-gated external interaction

## Open Questions

1. Which additional provider adapters beyond Discord/Matrix are viable without broadening blast radius?
2. When should `online_no_recent_delivery_receipt` be considered stale enough to trigger Hermes self-recovery versus operator review?
3. What richer HUD surfaces should consume boardroom/reroute/interruption streams without becoming a mutation path?

## References

- Crate: `crates/spine/interface/arda-orome`
- Original narration archive: `docs/plans/original-human-plan-narration/HERMES.md`
- Core runtime: `core/state/hermes_command.json`
- Operator contract: `docs/contracts/soterion-communication-contract.md`
