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

# sigil: REPAIR
---
crate: arda-prometheus
kind: orchestrator
agent: prometheus
realm: command
capabilities:
  - pipeline-orchestration
  - budget-check
  - confidence-scoring
  - task-assignment
  - ledger-emission
status: active
search_tags: [prometheus, ceo, pipeline, orchestration, command]
---

# arda-prometheus

Canonical executive orchestrator crate aligned with `core/projects/Plans/PROMETHEUS.md`.

## Purpose
Run executive task flow with autonomy scaffolding: receive task, estimate joule cost, score confidence, route/delegate, and persist lifecycle decisions to ledger.

## What's in this crate
- `pipeline.rs`: main orchestration sequence with confidence + escalation gates.
- `core_link.rs`: bridge to `/core` artifacts (`core/realm/boot.toml`, `core/state/world.json`).
- `registry.rs`: order-of-battle snapshot loading from `core/state/world.json`.
- `heartbeat.rs`: startup heartbeat mode selection (`interval` vs `threshold`).
- `thought.rs`: machine thought ledger writer with Soterion-style header/body JSONL.
- `orders.rs`: append-only `orders.jsonl` + `escalations.jsonl` stores with active/pending counters and escalation resolution.
- `council.rs`: council-gate confidence adjustment scaffold for complex decisions.
- `service.rs`: status/roster/thought query API for daemon + CLI surfaces.
- `service.rs`: also exposes escalation queue listing and resolution APIs.
- `transport/`: IPC and feature-gated HTTP/SSE daemon interfaces.
- `router.rs`: router re-export for orchestrator API.
