---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "program_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-30"
  tags: ["task-loop", "scheduler", "verification", "continuation"]
---

> 🜏 Soterion: 📜 program_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-30

# Autonomous Task Completion Program

## Outcome

The operator states an outcome once. Arda retrieves context, decomposes bounded work, applies existing authority, schedules and executes it, verifies and independently reviews the result, revises failures, survives restarts, and continues until acceptance passes or one genuine operator decision is required.

## Current status

The source and package foundation is implemented. One bounded installed objective has completed without repeated operator task assignment. The program is not complete because the remaining cross-system and installed-runtime acceptance gates have not all run successfully.

| Capability | Current truth |
|---|---|
| Canonical queue, schedules, and continuation decisions | Source/package verified |
| `plan → approval → execute → verify → review → close` | Source/package verified; bounded installed proof exists |
| Retry, revision, replan, deferred work, cancellation | Source/package verified |
| Restart checkpoints and mutation isolation | Source/package verified; bounded installed restart proof exists |
| Independent provider-backed verifier and critic contracts | Source/package verified |
| Canonical objective/control projection | Source/package verified |
| Hermes consumption and mutation controls | Open |
| Installed recurrence, deferred wake, and correction | Open |
| Live critic rejection followed by revision | Open |
| Simultaneous real-provider, multi-project execution | Open |
| Live Vairë receipt binding and operator-burden acceptance | Open |

## Execution order

Complete these milestones in order. A milestone closes only through its human-visible acceptance scenario, not through schema or package tests alone.

1. [Hermes objective control](01-hermes-objective-control.md)
2. [Installed scheduling and restart acceptance](02-installed-scheduling-restart.md)
3. [Live critic rejection and revision](03-live-critic-revision.md)
4. [Real multi-project execution](04-real-multi-project-execution.md)
5. [Vairë continuity and operator acceptance](05-vaire-operator-acceptance.md)

Implementation and review history is indexed in [Evidence History](EVIDENCE_HISTORY.md). New detailed test output belongs in receipts or audit artifacts, not in this plan.

## Canonical authority

- Queue authority: `core/projects/tasks/queue.jsonl`
- Schedule authority: `core/projects/tasks/schedules.jsonl`
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
