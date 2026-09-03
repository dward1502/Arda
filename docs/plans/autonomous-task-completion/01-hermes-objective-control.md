---
soterion:
  sigil: "SCROLL"
  role: "implementation_plan"
  owner: "HERMES"
  status: "active"
  reviewed: "2026-08-30"
---

> 🜏 Soterion: 📜 implementation_plan | owner: HERMES | status: active | reviewed: 2026-08-30

# Milestone 1 — Hermes Objective Control

## Status

Engineering and installed loopback bridge acceptance pass with authenticated event-shaped input. Objective intake, projection reads, pause/resume, reprioritization, revision, fresh approval, unclaimed cancellation, and post-terminal suppression were exercised against canonical records. A genuine message delivered by an external messaging platform remains the open platform gate.

## Human-visible result

In a normal Hermes conversation, the operator can ask what Arda is doing, see one truthful objective summary and its next action, then pause, resume, reprioritize, revise, approve, reject, or cancel it without handling IDs or editing ledgers.

## Existing foundation

- Engine publishes `core/state/operator_projection.json` from canonical queue, schedule, and run state.
- The loopback harness exposes read-only `GET /v1/operator-projection`.
- Aulë owns canonical pause/resume, reprioritization, objective revision/fresh approval, and cancellation mutations.

## Work

1. Add one typed Hermes-facing objective summary sourced from `OperatorProjection`; do not re-derive queue or run state.
2. Resolve conversational references to one exact `objective_id` and current `task_id`; ambiguous references must ask one concrete question.
3. Route each control to the existing Aulë mutation owner.
4. Require an explicit confirmation for consequential reject/cancel/revision actions while allowing read and bounded pause/resume under existing policy.
5. Return the updated canonical projection after each mutation, including source freshness and any blocker.
6. Preserve one command receipt linking Hermes session, objective, task, mutation, and resulting canonical record.

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

The operator can inspect and control a real objective conversationally, and every displayed state and mutation is traceable to the same canonical Arda records.
