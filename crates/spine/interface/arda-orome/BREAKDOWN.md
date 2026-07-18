---
soterion:
  sigil: "REPAIR"
  glyph: "⟁"
  role: "interface_layer"
  owner: "ARDA"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-orome
Interface layer for Arda agents: inbound/outbound comms, routing,
intent classification, MCP server, boardroom/council service events,
context enrichment, edge device registry, slash/CLI relay, and service
runtime integration.
Owner: arda | Sigil: ⟁ REPAIR | Status: active

## Summary
`arda-orome` is the widest interface crate in the spine. It owns:
- A2H message types and queueing
- A2A message/envelope/protocol
- agent registry + message router with retry/dead-letter/expiry
- intent classification with 3-tier routing
- MCP server/tools/protocol/governance gate
- rich service layer for boardroom, council, approvals, interruptions,
  approvals, subagent completions, comms events
- context enrichment from Mnemosyne with cached ranked memories
- edge device registry and status formatting
- slash command handling for Discord interactions
- Hermes agent + relay
- governance-backed broadcasting via `arda-governance`/`arda-vaire`/
  `arda-economics`

No direct imports were found in `arda-engine` or `apps`; this crate is
an interface contract and implementation target rather than a compiled-
in part of the runtime main today.

## Where it lives
- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/interface/arda-orome`
- Tests: none active; `cargo test -p arda-orome` shows 0/0
- Key config path: same workspace, no standalone config directory

## Verification status
- `cargo check -p arda-orome`: OK
- `cargo test -p arda-orome`: 0 unit tests, 0 doc tests
- Warnings from `arda-core` leak through but no crate-local errors
- No repo consumers found in `crates/engine` or `apps`

## Agentic-OS relevant abstractions
- **Message surfaces**
  - A2H: `Authorize`, `Notify`, `Response`, `Approval`, `Clarify`, `Status`
  - A2A: request/response/notification/handshake/heartbeat with TTL
  - `Envelope` + hop tracking + expiry + optional signature
  - `MessageQueue` async bounded send with 30s timeout
- **Routing**
  - `AgentRegistry`: by-id, realm, capability, availability, stale pruning
  - `MessageRouter`: per-agent queuing, dead-letter, max queue size,
    retry on failed delivery, expiry draining, broadcast result
- **Intent classification**
  - Tier-1 rule-bound with perfect confidence for `!`, `status`/`queue`,
    `schedule ...`, `@...`
  - Tier-2 heuristic richness for questions/help/review/meeting/thanks
  - Tier-3 fallback with conservative default confidence
- **MCP server**
  - JSON-RPC initialize, tools list/call, resources list/read, prompts
  - governance validation: approval token, network allow, destructive
    allow, triad metadata presence
  - runtime governance emits triad/bacon-lite/love/joule signal on each
    tool call and background-plutus work tracking
- **Service layer**
  - boardroom post/quorum/oracle/charon-route evidence
  - council discussion notes, promotions, approvals
  - task approval proposal/packet/projection
  - subagent completion packet/projection
  - operating-room events with visibility/risk/safety state
  - comms event model with promotion state and redaction awareness
- **Context enrichment**
  - Mnemosyne-backed enriched prompt context
  - weighted scoring: significance 0.40, recency 0.25, tier 0.15,
    tag match 0.10, query relevance 0.10
  - static cache with TTL; lazy global cache
- **Edge device registry**
  - role model: Scout/Marketer/Analyst/Worker/Standby
  - GPU/memory status, health checks, formatted emoji status lines
- **Slash/CLI**
  - Discord interaction model + slash command handler

## Crate layout
| Module | Role |
|--------|------|
| `lib.rs` | Compact public surface: comm + message re-exports |
| `comm.rs` | A2H protocol, channels, priorities, governance metadata |
| `message.rs` | A2A message, envelope, thread, delivery status |
| `types.rs` | Heavy schema surface: boardroom, council, operating-room,
               comms events, approvals, completions, routing hints |
| `agent.rs` | `HermesAgent` implementation of `arda-core::Agent` |
| `service.rs` | HermesService: the runtime glue for posting/routing |
| `service/*` | Domain events and state: boardroom, council, decision,
               inbound/outbound, interrupts, queue, runtime, support,
               task_approval, subagent_completion, semantic_channel,
               status, classification, comms_event |
| `intent.rs` | 3-tier inbound intent classifier |
| `router.rs` | `MessageRouter`: queued delivery with retry/dead-letter |
| `registry.rs` | `AgentRegistry`: discovery by id/realm/capability |
| `edge.rs` | Edge device registry and formatted status |
| `mcp/` | MCP server, channel, protocol, tools, browser/external |
| `slash.rs` | Discord interaction/slash command types |
| `protocol.rs` | A2A handshake/heartbeat/forward validation |
| `relay.rs` | CLI relay inbox writer with command detection |
| `context_enrichment.rs` | Mnemosyne-backed enriched context + cache |
| `context_cache.rs` | Generic context cache primitive |
| `mnemosyne_integration.rs` | Mnemosyne service integration |
| `formatter.rs` | Output formatting conveniences |
| `discord_health.rs` / `discord_safe_message.rs` | Discord-specific guard rails |

## Consumer wiring
- No observed imports in `arda-engine` or `apps`
- Indirect couplings through:
  - `arda-core::task::Task`, `Agent`, daemon envelopes
  - `arda-governance` triad/bacon-lite in MCP runtime governance
  - `arda-economics` PlutusService work tracking
  - `arda-vaire` MnemosyneService memory recall

## Ideas for improvement
1. Add tests: no unit/integration coverage currently; start with
   router retry/expiry, intent classification, and MCP governance
2. Unify duplicate message abstractions: `comm::InboundMessage`,
   `types::InboundMessage`, `message::A2AMessage`, and the service
   inbound/outbound types should collapse to one canonical set
3. Replace `'static str` labels everywhere with typed enums to avoid
   mismatched route/intent strings
4. Make registry/router state sharable via embedded runtime trait from
   `arda-core` rather than local Arc/RwLock implementations
5. Persist `MessageQueue`/agent registry state so router survives restarts
6. Split service layer into standalone protocol packages:
   boardroom, council, approvals, operating-room
7. Normalize governance hooks: instead of inline triad/bacon calls in MCP,
   reuse ` GovernanceGates` from `arda-core` centrally
8. Add typed approval/interruption envelopes backed by ledger writes
9. Replace the static Lazy context cache with bounded async cache under
   `once_cell` → `tokio::sync::RwLock`, or use core background gates
10. Wire one interface package into engine/CLI as a live smoke path
