---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "documentation"
  owner: "HADES"
  status: "archived"
  last_reviewed: "2026-07-27"
crate: arda-aule
owner: prometheus
status: archived
reviewed: "2026-07-27"
---

> Arda-AULE: 📜 observability / orchestration telemetry | owner: prometheus | status: archived | reviewed: 2026-07-27

# Aule (PROMETHEUS) Foundation Plan — Archived

`PROMETHEUS` was consolidated into the live `arda-aule` observability and
operator-control crate. This completed foundation plan was archived on
2026-07-27. Current maintenance truth lives in
`crates/spine/observability/arda-aule/BREAKDOWN.md`; the runtime snapshots below
are retained as historical evidence rather than current health claims.

Status at archive: implementation complete; historical runtime snapshot degraded
Owner: prometheus
Archived plan: `docs/archive/PROMETHEUS.md`
Primary runtime projection: `core/state/arda_snapshot.json`
Control-plane projection: `core/state/control_plane_lockdown.json`
Runtime settings projection: `core/state/runtime_settings.json`
Task ledger: `core/state/queue.jsonl`

## Purpose

PROMETHEUS owns Arda orchestration, executive state, control-plane projections, automation policy export, and the sovereign `/core` bridge consumed by operator surfaces and downstream agents.

## Historical Review Summary

At the 2026-06-21 review, the PROMETHEUS quick reference aligned with the then-live runtime surfaces. The primary entrypoint `core/state/arda_snapshot.json` projected 51 sections, including `control_plane_lockdown` and `runtime_settings`.

That review observed a degraded system posture: `core/state/control_plane_lockdown.json` reported `autonomy_runtime.mode = degraded` from `autonomy_error_budget_guard`, with `athena_lookup_stale` as the recorded violation. This runtime-wide recovery evidence did not represent unfinished Aule crate consolidation.

## Runtime Surface Evidence

| Surface | Review result |
| --- | --- |
| `core/state/arda_snapshot.json` | Exists; primary entrypoint; includes `control_plane_lockdown` and `runtime_settings`; 51 projected sections. |
| `core/state/control_plane_lockdown.json` | Exists; authority `control_plane_lockdown_projection`; generated on 2026-06-21; autonomy runtime degraded by stale ATHENA lookup. |
| `core/state/runtime_settings.json` | Exists; authority `runtime_settings_projection`; generated on 2026-06-21; runtime/env templates present. |
| `config/runtime.generated.env` | Exists through runtime settings; 36 generated runtime keys. |
| `config/runtime.env.example` | Exists through runtime settings; 114 runtime template keys. |
| `config/.env.example` | Exists through runtime settings; 42 shared template keys. |

## Contract at Closure

PROMETHEUS should continue to provide:

- executive pipeline and escalation flow projections;
- ARDA snapshot/source-map projections from `/core/state`;
- context-engineering policy exports for automation;
- control-plane lockdown and autonomy posture reporting;
- runtime settings projection for generated and template environment surfaces;
- queue and plan review continuity through append-only task evidence.

## Historical Runtime Posture

The 2026-06-21 runtime evidence said the control plane was available but degraded:

- `autonomy_runtime.auto_degraded = true`
- `autonomy_runtime.mode = degraded`
- `autonomy_runtime.source = autonomy_error_budget_guard`
- active violation: `athena_lookup_stale`

These were runtime-wide operating constraints at review time, not open foundation-plan acceptance items. Current runtime authority remains in `core/state/` and normal approval/triad gates.

## Historical Runtime Follow-up Recorded at Review

1. Clear or explain the `athena_lookup_stale` autonomy violation.
2. Verify Manwe/Aule service readiness before claiming healthy orchestration.
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

- Live crate authority: `crates/spine/observability/arda-aule/BREAKDOWN.md`
- Archived foundation records: `docs/archive/arda-aule/`
- Runtime projections: `core/state/arda_snapshot.json`, `core/state/control_plane_lockdown.json`, `core/state/runtime_settings.json`
