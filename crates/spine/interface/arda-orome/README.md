---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: ARDA
  status: active
  last_reviewed: 2026-07-22
---

> 🜏 Soterion: 📜 documentation | owner: ARDA | status: active | reviewed: 2026-07-22

# arda-orome

Interface layer for Arda agents: comms, routing, intent classification, MCP, service events, context enrichment, edge registry, and provider/runtime integration.

## Verified surface

- A2H/A2A messages and envelopes
- AgentRegistry + MessageRouter with retry/expiry/dead-letter
- 3-tier intent classifier
- MCP server/tools/protocol/governance gate
- Service events: boardroom, council, approval, interruption, subagent completion, comms, status
- Mnemosyne-backed context enrichment with bounded async cache
- Provider registry/runtime/adapter/streaming surface
- Discord health/safe-message helpers
- Slash/CLI relay
- Optional HTTP/SSE via `http` feature

## Verified evidence

Build/test proofpoint: cargo check -p arda-orome + cargo test -p arda-orome 14/14 passing.

## Live status

See STATUS.md for current health signals, open risks, and ownership.

## Work queue

See CHECKLIST.md for authorship and implementation tracking.