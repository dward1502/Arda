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

# 🪙
# CHARON Quick Reference

Status: in_progress (updated 2026-06-22)
Owner: charon
Human plan: `human/plans/CHARON.md`
Crate: `crates/arda-charon`
Core runtime: `core/state/charon_router.json`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

CHARON owns inference routing, provider health, execution-lane selection, and route-class policy across local and edge-capable model backends.

## Current Contract

- IPC-first routing daemon is live and running
- Provider state, cooldowns, and route selection are exported for operator visibility
- Execution lane and context targeting are part of routing decisions
- JSONL state writes are serialized and malformed records are surfaced
- **litellm_gateway** provider added to config (not yet running)
- **crawl4ai** service activated and integrated

## Primary Runtime Surfaces

- `data/charon/state.jsonl`
- `core/metrics/by_crate/charon/`
- `config/charon.providers.toml`
- `core/state/fleet_backbone.json`

## Readable Context

Use `human/plans/CHARON.md` for the operator-facing plan narrative and graph node.

## Open Tasks (0 total)

### Expose Prometheus `/metrics` endpoint (added 2026-04-27)

Charon binds `:5110` for the OpenAI-compatible API but has no `/metrics` endpoint. Fleet Prometheus on beelink (`:9090`) declares charon as a scrape target; the job is currently DOWN until this is implemented.

**Why:** Aligns with the architecture goal that every Annunimas crate exposes health metrics for the warden/orchestrator relay. Without charon metrics we have no per-provider request count, latency p50/p95/p99, failover frequency, or quota-burn rate visible to Grafana.

**Current Status:** Completed (verified 2026-06-22). Endpoint is live and returning Prometheus text exposition format.

**Defer until:** broader audit of crate observability conventions — do not implement in isolation; pick a metrics crate (`prometheus`, `metrics-exporter-prometheus`, or `axum-prometheus`) that all Annunimas crates can share, and codify the labeling scheme (`fleet`, `node`, `crate`, `route_class`, `provider_id`, `model`).

**Acceptance:**
- `curl http://100.78.138.113:5110/metrics` returns 200 with Prometheus text exposition format.
- Beelink Prometheus shows `charon` job health=up.
- Counters present: requests by provider+model+route_class, failover events by trigger reason, quota-burn by provider.
- Histograms present: end-to-end request latency, time-to-first-token for streaming.