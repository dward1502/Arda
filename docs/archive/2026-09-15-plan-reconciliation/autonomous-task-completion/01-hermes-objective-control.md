---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "archived"
  reviewed: "2026-09-04"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: archived | reviewed: 2026-09-04

# Milestone 1 — Hermes Objective Control

## Status

Engineering and installed loopback bridge acceptance pass with authenticated event-shaped input. Objective intake, projection reads, pause/resume, reprioritization, revision, fresh approval, unclaimed cancellation, and post-terminal suppression were exercised against canonical records. A genuine message delivered by an external messaging platform remains the open platform gate.

## Human-visible result

In a normal Hermes conversation, the operator can ask what Arda is doing, see one truthful objective summary and its next action, then pause, resume, reprioritize, revise, approve, reject, or cancel it without handling IDs or editing ledgers.

## Existing foundation

- Engine publishes `core/state/operator_projection.json` from resident ObjectiveStore and RunStore; old queue-shaped evidence is historical.
- The loopback harness exposes read-only `GET /v1/operator-projection`.
- Engine ObjectiveStore owns canonical pause/resume, reprioritization, pre-execution revision/fresh approval, and cancellation. Aulë is an adapter/projection consumer, not a competing mutation owner.

## Work

1. Add one typed Hermes-facing objective summary sourced from `OperatorProjection`; do not re-derive queue or run state.
2. Resolve conversational references to one exact `objective_id` and current `task_id`; ambiguous references must ask one concrete question.
3. Route each control through authenticated Harness to Engine ObjectiveStore. Mid-execution revision currently rejects safely; a fresh approved objective is required rather than silently changing running work.
4. Require an explicit confirmation for consequential reject/cancel/revision actions while allowing read and bounded pause/resume under existing policy.
5. Return the updated canonical projection after each mutation, including source freshness and any blocker.
6. Preserve one command receipt linking Hermes session, objective, task, mutation, and resulting canonical record.
7. Verify `MAX_OBJECTIVE_ATTEMPTS=5` retry cap prevents runaway re-claiming in the resident ObjectiveRuntime.

## Acceptance scenario

1. Start one real objective through Hermes.
2. Ask “what are you working on?” and receive title, current task, status, next continuation, wake, route, budget, evidence, and blocker from the canonical projection.
3. Pause it conversationally and prove the installed scheduler does not claim it.
4. Resume and reprioritize it; prove the next canonical claim reflects the new priority.
5. Revise its outcome and prove it remains pending until fresh approval.
6. Approve it and observe execution resume without a parallel Hermes task record.
7. Cancel a disposable objective and prove no later wake or execution occurs.

## Evidence commands

- Focused Hermes/Arda contract tests for read, ambiguity, mutation routing, and read-after-write projection.
- Existing Aulë canonical-control package tests.
- One installed Hermes conversation receipt covering the acceptance scenario.

## Exit gate

The operator can inspect and control a real objective conversationally, and every displayed state and mutation is traceable to the same canonical Arda records. The `MAX_OBJECTIVE_ATTEMPTS=5` retry cap must also be verified in the resident ObjectiveRuntime.
