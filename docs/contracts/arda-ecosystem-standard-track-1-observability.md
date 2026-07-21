---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "WARDEN"
  status: "active"
  reviewed: "2026-07-13"
---

> 🜏 Soterion: 📜 contract | owner: WARDEN | status: active | reviewed: 2026-07-13

# Telemetry Contract

This contract defines the telemetry substrate for Arda observability.

## Scope

Telemetry is a transport layer, not a viewpoint.
Arda observability already has formatted views:
- `queue_observability_snapshot`
- `format_operations_briefing_text`
- `build_governance_observation`
Telemetry events are durable evidence behind those views.

## Event schema

All events use `arda.telemetry` or `arda.telemetry` namespace.
Canonical event groups:
- `agent.<crate>.command`
- `llm.call`
- `governance.triad`
- `router.route`
- `system.supervisor`
- `queue.event`

Required event fields:
- `event_id`
- `timestamp`
- `schema_version`
- `trace_id`
- `span_id`
- `agent_id`
- `crate`
- `action`
- `inputs`
- `outputs`
- `outcome`
- `latency_ms`
- `error`

## Evidence classes

| evidence class | meaning |
|---------------|---------|
| `documentation` | event schema documented |
| `local_heuristic` | in-process event emission exists for one path |
| `source_metadata` | schema version emitted in logs |
| `runtime_receipts` | events persisted to `data/telemetry/` per agent run |
| `policy_enforcement` | telemetry writes are gated so they cannot affect critical command latency |
| `independent_review_receipts` | dashboard/log/receipt cross-checks verify event parity |
| `service_init` | crate/service init emits typed `schema_version` runtime surface |

## Source of truth

Live track registry: `core/state/contract_registry.json`
Track id: `arda-ecosystem-standard-track-1-observability`

## Current default projection

Expected current level: `service_init` after registry-backed runtime checks; system-wide remains `runtime_receipted` until dry-run manifests are validated end-to-end.

## CLI behavior

- `arda telemetry schema` prints current schema document.
- `arda telemetry receipt <run_id>` prints persisted telemetry bundle metadata for a run.

## Stop condition

Logs and dashboards reflect the same event schema as in-process receipts and persisted bundles.
