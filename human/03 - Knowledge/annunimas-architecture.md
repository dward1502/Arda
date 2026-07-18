---
sigil: SCROLL
soterion:
  id: annunimas-architecture
  version: 1.0.0
  classification: architecture-overview
  author: Aulendil
  created: 2026-03-20
  last_edited: 2026-05-03
  status: active
  domain: architecture
  tags:
    - annunimas
    - architecture
    - overview
    - system-design
  mnemosyne:
    lineage: annunimas-architecture-overview
    memory_type: system-architecture
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# Annunimas Architecture Overview

## Overview

Annunimas is a distributed agentic control system designed to orchestrate complex workflows across edge and cloud environments. It provides a sovereign agentic control layer that integrates with existing host operating systems while maintaining operational stability and predictability.

## Core Principles
### Task
A unit of work with a state machine: Pending → Running → Complete/Failed/Retry.
Every task has a type (e.g., "ingest") that determines which agent handles it.

### Agent
Implements the `Agent` trait. Has a name, capabilities (task types it handles),
and an async `execute` method. Agents are registered with the Router at startup.

### PROMETHEUS (Pipeline)
The executive orchestrator. Receives tasks, applies confidence + council gating,
routes delegated tasks via the Router, logs lifecycle events, and writes machine
thought/order records.

### Tool Registry
External capabilities (crawl4ai, llmfit, etc.) are tracked in `registry.toml`.
Agents can query the registry to discover available tools at runtime.

### Ledger
Append-only JSONL log. Every task submission, assignment, completion, and failure
is recorded. This is the system's memory — never modified, only appended.

## Crate Map
- `annunimas-core` — Shared types, traits, error handling
- `annunimas-prometheus` — Canonical executive orchestrator (Pipeline + autonomy modules)
- `annunimas-ceo` — Compatibility shim re-exporting `annunimas-prometheus`
- `annunimas-charon` — Inference routing and provider-state daemon
- `annunimas-athena` — Knowledge ingestion agent
- `annunimas-hades` — Cleanup/lifecycle/order maintenance agent
- `annunimas-hermes` — Communications and boardroom messaging agent
- `annunimas-mnemosyne` — Memory/identity persistence service
- `annunimas-cli` — Binary entry point

Archived but retained crates live under `archive/crates/` for future reintegration.

## Adding a New Agent
1. Create `crates/annunimas-{name}/`
2. Implement the `Agent` trait
3. Register in `annunimas-cli/src/main.rs`
4. Add to workspace `Cargo.toml`
