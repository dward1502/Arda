---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: status
  owner: ARDA
  status: active
  last_reviewed: 2026-07-22
---

# arda-orome — status

Verified: cargo test -p arda-orome 14/14 passing on the local Hermes cutover branch

## health

Active interface-layer crate with verified compile path, provider runtime, and gRPC/build integration. Public surface is broad; runtime consumers outside tests are still limited.

## signals

- protocol path: A2H messages + A2A envelopes with TTL/hop tracking
- routing path: AgentRegistry + MessageRouter with retry/dead-letter/expiry draining
- intent path: 3-tier rule/heuristic/fallback inbound classification
- service path: boardroom/council/approval/interruption/completion/comms/status/classification
- context path: Mnemosyne-backed enrichment with bounded async cache
- mcp path: JSON-RPC tools/resources/prompts + governance gate
- provider path: registry + runtime + adapter/streaming surface
- transport path: IPC/message queues + optional HTTP/SSE if `http` feature enabled

## test evidence

Full suite: 14 unit tests across intent, router retry/expiry, provider runtime/adapter/registry/streaming. No doc tests.

## open risks

- no active compiled-in consumers in `arda-engine` or `apps`
- large service surface with limited test coverage outside provider/routing/intent
- duplicate message abstractions remain across comm/msg/service modules

## open tasks

See CHECKLIST.md for current ownership and next actions.
