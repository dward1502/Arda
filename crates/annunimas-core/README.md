---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
sigil: "⚡"
domain: "operations"
purpose: "Shared primitives - Task, Agent, Router, Ledger, Soterion"
status: "active"
references:
  - "core/realm/annunimas.toml"
  - "core/realm/agents.toml"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# annunimas-core

Shared primitives and contracts used by the whole workspace.

## Purpose
Provide the canonical data model and traits (`Task`, `Agent`, `Router`, `Ledger`, config, errors) so every crate speaks the same protocol.

## What's in this crate
- `agent.rs`: `Agent` trait and agent metadata contracts.
- `task.rs`: task lifecycle/state and timing/joule fields.
- `router.rs`: capability-based task routing.
- `ledger.rs` + `message.rs`: append-only event log model.
- `daemon.rs`: canonical daemon IPC command/response envelopes.
- `llm.rs`: provider abstraction and OpenAI-compatible client.
- `soterion.rs`: sigil metadata/index utilities.
- `config.rs`, `tool.rs`, `error.rs`: runtime config, tool registry, shared errors.
