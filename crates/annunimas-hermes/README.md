---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: annunimas-hermes
kind: agent
agent: hermes
realm: communications
sigil: "𓅃"
capabilities:
  - a2a-messaging
  - discord-commands
  - channel-adapters
  - relay
  - intent-classification
  - daemon-transport
  - boardroom-logging
  - delivery-retry
status: active-mvp
search_tags: [agent, hermes, discord, mcp, messaging]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active-mvp | reviewed: 2026-05-21

# annunimas-hermes

Communication and message-routing layer for agent/human interaction.

## Purpose
Handle communications routing, tiered intent classification, boardroom posting, and provider-facing transport for the CITADEL stack.

## What's in this crate
- `message.rs`, `protocol.rs`, `router.rs`, `registry.rs`: A2A protocol, queues, and routing.
- `mcp.rs`: channel adapters and transport abstraction.
- `serenity_bot.rs`: Serenity-based Discord bot and slash commands.
- `slash.rs`: typed slash interaction models.
- `poller.rs`, `relay.rs`, `formatter.rs`, `edge.rs`: status polling, relay, presentation, edge metadata.
- `intent.rs`: tiered message intent classification (rule -> heuristic -> fallback).
- `service.rs`: storage-backed HERMES service (`status`, `providers`, `classify`, `send`, `retry_outbound_queue`, `boardroom`, `calendar_sync`).
- `provider.rs`: config-driven provider runtime manager (health, retry/backoff dispatch, polling over MCP adapters).
- `transport/`: IPC + optional HTTP/SSE daemon interface.
- `agent.rs`: `HermesAgent` integration with the core router.

## Phase 3 additions
- Webhook ingestion path for external events (`ingest_external`, `/webhook/:provider`).
- Council boardroom flow (`council_open`, `council_report`, `council_close`).
- Provider poll deduplication to prevent repeated inbound classification on daemon polling.

## Owns
- outbound and inbound message orchestration
- channel/provider dispatch and retry behavior
- Discord and slash-command interaction surfaces
- queue-backed delivery state
- intent classification and metadata enrichment
- CHARON-assisted route metadata for operator flows

## Reads
- Hermes provider/runtime config from the repo config surface and environment
- queue and world-state context from [`core/state/`](/var/home/mythos/Annunimas/core/state)
- comm/runtime artifacts from [`data/hermes/`](/var/home/mythos/Annunimas/data/hermes)

## Writes
- outbound/inbound queue state under [`data/hermes/`](/var/home/mythos/Annunimas/data/hermes)
- communication receipts and boardroom-related runtime state under [`data/`](/var/home/mythos/Annunimas/data)

## Main Interfaces
- CLI: [`annunimas-cli hermes ...`](/var/home/mythos/Annunimas/crates/annunimas-cli/src/commands/hermes.rs)
- HTTP: [`src/transport/http.rs`](/var/home/mythos/Annunimas/crates/annunimas-hermes/src/transport/http.rs)
- IPC: [`src/transport/ipc.rs`](/var/home/mythos/Annunimas/crates/annunimas-hermes/src/transport/ipc.rs)

## Common Commands
```bash
cargo run -p annunimas-cli -- hermes status
cargo run -p annunimas-cli -- hermes providers
cargo run -p annunimas-cli -- hermes classify "status check"
```

## Debug Path
- delivery bug:
  start with `src/service.rs`, then `src/provider.rs`, `src/router.rs`, and `src/relay.rs`
- Discord/slash bug:
  start with `src/serenity_bot.rs` and `src/slash.rs`
- queue/state bug:
  start with `src/service.rs`, then inspect `data/hermes/`

## Refactor Note
`src/service.rs` is the current Hermes hotspot. Splitting queue orchestration, provider dispatch, and boardroom flows would reduce context cost materially.
