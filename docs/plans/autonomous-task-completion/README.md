---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "program_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-07"
  tags: ["task-loop", "scheduler", "verification", "continuation"]
---

> 🜏 Soterion: 📜 program_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-07

# Autonomous Task Completion Program

## Outcome

The operator states an outcome once. Arda retrieves context, decomposes bounded work, applies existing authority, schedules and executes it, verifies and independently reviews the result, revises failures, survives restarts, and continues until acceptance passes or one genuine operator decision is required.

## Revalidation — 2026-09-07

The cutover is **not complete**. [Current repair evidence and open gates](../../audits/2026-09-07-objective-cutover-revalidation.md) supersede earlier retry/runtime-complete claims. The daemon-lifetime polling cap was removed; persisted leaf attempts are capped transactionally after lease expiry, not when a reader opens the store. Source regressions also restore sibling receipt persistence, dependency-cycle and receipt validation, revision safety, equal-root exclusion, and persisted-stage reclaim. Installed same-run recovery, supervision/schedule integration, physical-root alias proof, and final deployment acceptance remain open.

## Current status

The source/package foundation and bounded installed receipts exist, but current cutover acceptance remains open. Earlier runtime-complete narratives are historical; use the [revalidation evidence](../../audits/2026-09-07-objective-cutover-revalidation.md) for the repaired source and remaining installed gates.

| Owning milestone | Retained evidence | Remaining acceptance |
|---|---|---|
| [1 — Hermes control](01-hermes-objective-control.md) | Installed bridge/control evidence | Genuine messaging ingress against canonical records |
| [2 — Scheduling/restart](02-installed-scheduling-restart.md) | Bounded restart/correction evidence | Resident same-run recovery and deferred/recurring wake |
| [3 — Critic revision](03-live-critic-revision.md) | Provider-backed contracts and bounded evidence | Preserve the milestone's real rejection/revision acceptance requirements |
| [4 — Multi-project](04-real-multi-project-execution.md) | Two-project serial joined close | Same-objective overlap and physical workspace alias isolation |
| [5 — Continuity/acceptance](05-vaire-operator-acceptance.md) | Outcome binding evidence | Live Vairë context-use binding and explicit operator acceptance |

The [cutover plan](../2026-09-01-arda-objective-runtime-cutover.md) is the implementation dependency for these gates, not another acceptance program. The [legacy loop index](../AUTONOMOUS_TASK_COMPLETION_LOOP.md) owns no separate tasks. Provider transport repairs belong to [provider convergence](../PROVIDER_WORKER_CONVERGENCE.md).

## Execution order

Complete these milestones in order. A milestone closes only through its human-visible acceptance scenario, not through schema or package tests alone.

1. [Hermes objective control](01-hermes-objective-control.md)
2. [Installed scheduling and restart acceptance](02-installed-scheduling-restart.md)
3. [Live critic rejection and revision](03-live-critic-revision.md)
4. [Real multi-project execution](04-real-multi-project-execution.md)
5. [Vairë continuity and operator acceptance](05-vaire-operator-acceptance.md)

Implementation and review history is indexed in [Evidence History](EVIDENCE_HISTORY.md). New detailed test output belongs in receipts or audit artifacts, not in this plan.

## Canonical authority

- Objective/control/schedule authority: the resident `arda-engine` ObjectiveStore defined by [the active runtime cutover plan](../2026-09-01-arda-objective-runtime-cutover.md). Until that store is installed, objective admission is frozen rather than falling back to legacy files.
- `core/projects/tasks/queue.jsonl` and `core/projects/tasks/schedules.jsonl` are frozen legacy inputs. They are not acceptance authority and must receive no new objectives, controls, continuations, or schedules.
- Run/checkpoint authority: Engine `RunStore` and Workbench run graphs
- Project authority: `data/workbench/projects.json`
- Read-only operator projection: `core/state/operator_projection.json`
- Conversational runtime: Hermes
- Context/outcome continuity: Vairë
- Provider placement: Manwë

Generated queue summaries and UI projections never become mutation authority.

## Program acceptance

The program is complete only when one operator-authored objective:

1. enters through Hermes;
2. spans at least two real registered projects;
3. is decomposed into dependency-aware bounded tasks;
4. uses canonical scheduling, including one deferred or recurring continuation;
5. survives one forced process restart and one forced verification or review failure;
6. executes through real admitted provider routes;
7. receives independent verification and a critic rejection that causes a corrected revision;
8. closes only after declared checks and human-visible artifacts pass;
9. records live context-use and outcome receipts in Vairë;
10. can be inspected, paused, revised, reprioritized, approved, rejected, and cancelled through Hermes against the same canonical records; and
11. is accepted by the operator as reducing management burden.

## Operating rules

- Work one milestone end to end before adding another subsystem slice.
- Start from the installed path; add focused tests only for defects or dangerous invariants exposed by that path.
- Keep package tests as supporting evidence, never as milestone completion.
- Preserve exact task/objective/run lineage across every surface and restart.
- Present unavailable or stale state honestly.
- Do not require the operator to inspect internal JSONL files or issue obvious next steps.
- Do not begin Ambient Agent embodiment work until this program is accepted.
