---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-21"
crate: arda-aule
owner: prometheus
status: in_progress
reviewed: "2026-06-21"
---

> Arda-AULE: 📜 observability / orchestration telemetry | owner: prometheus | status: in_progress | reviewed: 2026-06-21

# Prometheus Plan Narrative

`PROMETHEUS` is the current Arda observability and orchestration telemetry surface.
Historic narration is preserved at
`docs/plans/original-human-plan-narration/PROMETHEUS.md`. This document merges
the prior operator narrative with the current Arda surface names so old detail is
retained without stale crate assumptions.

Status: in_progress; runtime degraded by autonomy error-budget guard
Status: active
Owner: prometheus
Operator plan: `docs/plans/PROMETHEUS.md`
Primary runtime projection: `core/state/arda_snapshot.json`
Control-plane projection: `core/state/control_plane_lockdown.json`
Runtime settings projection: `core/state/runtime_settings.json`
Task ledger: `core/state/queue.jsonl`

## Purpose

PROMETHEUS owns Arda orchestration, executive state, control-plane projections, automation policy export, and the sovereign `/core` bridge consumed by operator surfaces and downstream agents.

## Current Review Summary

The PROMETHEUS quick reference remains aligned with the current runtime surfaces. The primary entrypoint `core/state/arda_snapshot.json` exists and currently projects 51 sections, including `control_plane_lockdown` and `runtime_settings`. Those sections point back to live generated projections with fresh timestamps from 2026-06-21.

The current system posture is degraded, not absent. `core/state/control_plane_lockdown.json` reports `autonomy_runtime.mode = degraded` from `autonomy_error_budget_guard`, with `athena_lookup_stale` as the active violation. This supports keeping PROMETHEUS in an in-progress recovery posture rather than marking the plan complete.

## Runtime Surface Evidence

| Surface | Review result |
| --- | --- |
| `core/state/arda_snapshot.json` | Exists; primary entrypoint; includes `control_plane_lockdown` and `runtime_settings`; 51 projected sections. |
| `core/state/control_plane_lockdown.json` | Exists; authority `control_plane_lockdown_projection`; generated on 2026-06-21; autonomy runtime degraded by stale ATHENA lookup. |
| `core/state/runtime_settings.json` | Exists; authority `runtime_settings_projection`; generated on 2026-06-21; runtime/env templates present. |
| `config/runtime.generated.env` | Exists through runtime settings; 36 generated runtime keys. |
| `config/runtime.env.example` | Exists through runtime settings; 114 runtime template keys. |
| `config/.env.example` | Exists through runtime settings; 42 shared template keys. |

## Current Contract

PROMETHEUS should continue to provide:

- executive pipeline and escalation flow projections;
- ARDA snapshot/source-map projections from `/core/state`;
- context-engineering policy exports for automation;
- control-plane lockdown and autonomy posture reporting;
- runtime settings projection for generated and template environment surfaces;
- queue and plan review continuity through append-only task evidence.

## Active Runtime Posture

Current runtime evidence says the control plane is available but degraded:

- `autonomy_runtime.auto_degraded = true`
- `autonomy_runtime.mode = degraded`
- `autonomy_runtime.source = autonomy_error_budget_guard`
- active violation: `athena_lookup_stale`

This means PROMETHEUS plan follow-up should prioritize recovery evidence and operator-facing clarity before expanding new autonomy. Safe local documentation and projection repair can proceed, but broader execution should remain bounded by the degraded posture and the normal approval/triad gates.

## Priority Follow-up Work

1. Clear or explain the `athena_lookup_stale` autonomy violation.
2. Verify CHARON/PROMETHEUS service readiness before claiming healthy orchestration.
3. Keep ARDA snapshot and control-plane lockdown exports current after queue changes.
4. Reconcile PROMETHEUS quick-reference open tasks against current queue projections.
5. Preserve governance gates for higher-risk automation: Triad, Bacon-lite, JouleWork, Love, and Soterion traceability.

## Gates

PROMETHEUS work remains subject to:

- Triad review for strategy or policy changes;
- Bacon-lite validation for runtime truth claims;
- JouleWork accounting for execution cost and budget posture;
- Love/resonance checks for operator-facing and user-impacting flows;
- Soterion traceability for queue, plan, and runtime evidence.

## References

- `docs/plans/PROMETHEUS.md`
- Original narration archive: `docs/plans/original-human-plan-narration/PROMETHEUS.md`
- Runtime projections: `core/state/arda_snapshot.json`, `core/state/control_plane_lockdown.json`, `core/state/runtime_settings.json`
