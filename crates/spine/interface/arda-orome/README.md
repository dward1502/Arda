---
soterion:
  sigil: SCROLL
  glyph: 📜
  code_point: U+1F4DC
  role: documentation
  owner: ARDA
  status: active
  last_reviewed: 2026-07-25
---

> 🜏 Soterion: 📜 documentation | owner: ARDA | status: active | reviewed: 2026-07-25

# arda-orome

Interface layer for Arda agents: comms, routing, intent classification, MCP, service events, governance records, context enrichment, edge policy, and provider/runtime integration.

## Verified surface

- A2H/A2A messages and envelopes
- Agent registry and message router with retry/expiry/dead-letter handling
- Three-tier intent classifier
- MCP server/tools/protocol/governance gate
- Boardroom, council, approval, interruption, completion, comms, and status events
- Mnemosyne-backed context enrichment with bounded async cache
- Provider registry, adapters, runtime, and streaming sessions
- Bounded timeout/retry dispatch, typed fanout, metrics, and expiry rejection
- Explicit local/trusted/external fleet scope policy
- Ledger-backed task approval and interruption envelopes
- Discord health/safe-message helpers and slash/CLI relay
- Optional HTTP/SSE via the `http` feature
- Engine smoke integration at `arda_engine::orome::manual_smoke_dispatch`

## Provider integration

Implement `provider::ProviderTransport` for a real provider client, register its `ProviderConfig`, and dispatch through `ProviderRuntime`. Do not bypass runtime timeout, retry, fanout, scope, metrics, or receipt behavior. Provider selection and inference routing remain owned by Manwe.

## Verified evidence

- `cargo test -p arda-orome`: 21 passed, 0 failed.
- `cargo test -p arda-engine --test orome_smoke`: 1 passed, 0 failed.
- `cargo fmt -p arda-orome -p arda-engine -- --check`: passes.

## Documentation

- `CHECKLIST.md`: completed implementation checklist and evidence
- `CRATE_PLAN.md`: canonical contracts and residual boundaries
- `BREAKDOWN.md`: detailed module/invariant breakdown
- `STATUS.md`: current operational status
- `OWNERSHIP.md`: ownership and boundary constraints
- `INDEX.md`: crate artifact index
