---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-05-21"
crate: arda-athena
kind: agent
agent: athena
realm: knowledge
sigil: "𓁿"
capabilities:
  - ingest
  - research
  - code
  - decision
  - general
status: operational
search_tags: [agent, athena, llm, knowledge]
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: operational | reviewed: 2026-05-21

# arda-athena

Knowledge ingest, query, deep-analysis, and synthesis agent.

## Purpose
Provide local knowledge ingest, corpus query, deep-analysis queue execution,
and provider-routed research support for Athena workflows.

Operationally, the crate now includes:
- append-only digest and per-source Books JSONL ingest persistence
- local query and deep-analysis workflow support
- deterministic governance/accounting scaffolding
- Unix socket IPC and HTTP/SSE transport surfaces
- CLI-facing daemon/service integration

Non-ingest task types still route through the configured LLM provider.

## What's in this crate
- `lib.rs`: Athena agent implementation, ingest/query/deep routing, model route selection, and LLM execution flow.
- `ingest.rs`: top-level ingest orchestration plus test coverage for local storage, query, deep-analysis, and policy/event behavior.
- `ingest/`: extracted helper surfaces including scholarly metadata, deep recovery, policy, observability, query, source classification, routing, and views.
- `transport/`: daemon transports (`ipc.rs` and feature-gated `http.rs`) and daemon config/startup wiring.
