---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-06-30"
---

> 🜏 Soterion: 📜 documentation | owner: HADES | status: active | reviewed: 2026-05-21

# 🪙
# PROMETHEUS Quick Reference

Status: in_progress (updated 2026-06-22)
Owner: prometheus
Human plan: `human/plans/PROMETHEUS.md`
Crate: `crates/arda-prometheus`
Core runtime: `core/state/arda_snapshot.json`
Task ledger: `core/projects/tasks/queue.jsonl`

## Purpose

PROMETHEUS owns orchestration, executive state, control-plane projections, automation policy export, and the sovereign `/core` bridge for the rest of the system.

## Current Contract

- executive pipeline and escalation flow are live
- ARDA snapshot and source map are projected from `/core/state`
- context-engineering policy export feeds automation
- control-plane lockdown projection verifies runtime contract alignment
- **crawl4ai** service activated and integrated
- **litellm_gateway** provider added to CHARON config


## Primary Runtime Surfaces

- `core/state/arda_snapshot.json`
- `core/state/control_plane_lockdown.json`
- `core/state/runtime_settings.json`
- `core/metrics/by_crate/prometheus/`

## Readable Context

Use `human/plans/PROMETHEUS.md` for the operator-facing plan narrative and graph node.

## Open Tasks (0 active)

Open tasks are reconciled from `core/state/queue_active.json`, which reports `active_task_count: 46` at `2026-06-30T19:40:21Z`. Prior plan-only entries below had no matching active queue task and are closed as stale under the reconcile-on-projection policy.

### Stale plan-only items closed as invalid

The following plan entries had no active queue record under `core/state/queue_active.json` and are closed as stale under the reconcile-on-projection policy.

#### 6. Improve ARDA HUD consumption of core and human plan surfaces

Better integrate PROMETHEUS projections into ARDA HUD for operator visibility.

**Status:** Closed as stale — no active queue record; closed 2026-06-22.

### Documentation & Integration

#### 7. Document system degradation and recovery procedures (added 2026-05-05)

Create documentation for current system issues and steps to recover from them.

**Status:** Done - recovery playbook added at `docs/operations/SYSTEM_DEGRADATION_AND_RECOVERY.md`

#### 8. Update system inventory to reflect current state (added 2026-05-05)

System_inventory.md is outdated (last updated 2026-03-14). Update with current status.

**Status:** Done - current inventory in `core/state/system_inventory.md`

#### 9. Reconcile plan files with current system state (added 2026-05-05)

Ensure all plan files reflect the current system configuration and known issues.

**Status:** Done - plan status updated; recovery/inventory docs reconciled into plan file references