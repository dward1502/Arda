---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-26"
  tags: ["task-loop", "scheduler", "verification", "review", "continuation"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-26

# Autonomous Task Completion Loop

## Outcome

A task created from operator intent or system discovery is scheduled, executed, verified, reviewed, revised when necessary, and resumed across restarts until its acceptance criteria pass or Arda presents one concrete blocked decision. The operator does not need to issue every next step.

## Current reusable foundation

- canonical append-only task queue;
- one-minute systemd queue executor;
- Workbench project contracts and adapters;
- durable run graph/events/checkpoints;
- plan, approval, execute, verify, review, and close node kinds;
- retry, cancellation, failed-verification recovery, restart recovery, and receipts;
- Hermes adapter execution;
- adaptive placement endpoint and Manwë provider receipts.

## Current production state

The installed `WorkbenchQueueExecutor` now resolves the selected task's project contract and runs `plan → approval → execute → verify → review → close` through the durable Engine graph. It persists `continue_verify`, `continue_review`, `continue_close`, and `close_complete` decisions in the canonical queue, resumes by deterministic run identity, and refuses successful terminal projection unless close carries provider and passing project-check evidence.

Remaining work is broader than this executor slice: general objective decomposition, retry/revision/replan decisions, durable recurrence, multi-project objectives, independent critic selection beyond the verifier role, and operator-facing continuation control remain open.

## Task contract additions

Every executable objective requires:

- stable objective and task IDs;
- operator-authored intent and source lineage;
- one or more real project IDs;
- concrete acceptance criteria and required evidence classes;
- dependencies and ordering constraints;
- risk and authority class;
- filesystem, network, secret-name, deployment, and external-action boundaries;
- cost/token/time/attempt budgets;
- schedule or recurrence policy;
- allowed provider/access tiers;
- verification commands and human-visible checks;
- review independence requirements;
- retry, revise, defer, and escalation policy;
- completion, cancellation, and supersession semantics.

A broad objective is not directly executable. The decomposer must create bounded tasks whose combined acceptance criteria satisfy the objective.

## Implementation sequence

### T1 — Queue schema and migration

Extend canonical queue records without breaking prior JSONL replay. Add outcome, project, acceptance, schedule, budget, continuation, and evidence fields. Build deterministic migration/projection tests. Do not edit generated projections by hand.

### T2 — General objective decomposition

Replace hard-coded goal recipes as the only planning path. A decomposer reads the objective, project contracts, active plans, Soterion results, recent receipts, and current repository state. It emits a dependency graph and identifies questions that genuinely block safe execution.

The plan itself receives deterministic validation: every leaf is scoped, every dependency resolves, every consequential action has a gate, and objective acceptance is covered by leaf evidence.

### T3 — Full queue-run graph

Change `WorkbenchQueueExecutor` to construct:

`plan → approval(if required) → execute → verify → review(if required) → close`

The graph must use the selected attached project rather than a fixed UUID. Queue completion may occur only after close is backed by all required receipts.

### T4 — Verification and review

- Run project-declared deterministic checks after mutation.
- Capture changed paths, command results, artifact identities, runtime probes, and acceptance observations.
- Select a distinct critic/reviewer when risk, uncertainty, or failed attempts require one.
- Reject unsupported completion claims.
- Permit the reviewer to request revision with named defects and retained lineage.

### T5 — Continuation engine

After every run, compute exactly one durable decision:

- `close_complete` — all criteria pass;
- `continue_next_task` — dependency graph has an eligible successor;
- `retry_same_task` — transient failure and budget remains;
- `revise_task` — implementation or plan defect was identified;
- `replan_objective` — decomposition no longer satisfies the outcome;
- `wait_until` — external dependency or scheduled time;
- `request_operator_decision` — authority/intent/risk genuinely blocks progress;
- `stop_failed` — bounded attempts exhausted with evidence.

The next timer tick consumes this decision. It must not rely on chat context.

### T6 — Durable scheduling

Add a canonical schedule ledger keyed to objective/task lineage. Support immediate, one-shot, recurring, deferred, paused, and cancelled schedules. Systemd timers wake the local scheduler; they do not become separate task authority. Hermes cron may be used as a conversational/agent execution backend only when its output returns to the same Arda objective and receipt lineage.

### T7 — Restart and concurrency

Ensure at-most-one active mutation lease per project/worktree, recover abandoned leases, preserve checkpoints, and resume the last eligible node. Do not mix changes from unrelated dirty worktrees. Support bounded parallel read-only analysis and separate-project execution.

### T8 — Operator control

Expose objective state, current task, evidence, next continuation decision, schedule, provider, budget, and blockers through Hermes and the native HUD. Allow pause, reprioritize, revise, approve, reject, and cancel against the same canonical records.

## Verification ladder

1. Unit tests for schema replay, dependency resolution, decisions, budgets, and idempotency.
2. Integration tests for full graph execution, failed verification, revision, retry, review rejection, cancellation, and restart.
3. Real repository proof with a reversible defect and declared checks.
4. Overnight continuation proof across timer ticks and one forced restart.
5. Operator review of the completed result and management burden.

### 2026-08-26 installed completion-loop slice

- Installed binary: `/var/home/mythos/.local/bin/arda-cli`, matching the final release build at SHA-256 `a23556c78b2f09ca452f28213e5d970580314b5dab6013b0ca65ad745935d0f9`.
- Focused executor gate: `cargo test -p arda-aule --features full-cli prometheus::autopilot::workbench_executor -- --nocapture` passed 11 tests.
- Normal canonical intake task: `tsk_20260826T090051Z_completion_loop_acceptance_v2`; deterministic run: `queue-tsk_20260826T090051Z_completion_loop_acceptance_v2`; objective: the same stable task ID; source objective packet: `obj_completion_loop_acceptance_v2` in every queue continuation and terminal record.
- Forced restart: `ARDA_WORKBENCH_FORCE_RESTART_AFTER_STAGE=execute` exited the installed executor with code 86 after durable execute receipt `sha256:67db53e5b64e00a771f52b7256dbbcfe954d947bc0c298cd7af141faf2d9b6bb`. The installed one-minute executor resumed without another operator task.
- Persisted continuation chain: `continue_verify` → `continue_review` → `continue_close` → `close_complete`.
- Independent verification receipt: `sha256:4891c76e228867e9e235ae4d5c5c6e7ef272e009e68910caacc7f4e38e874301`; declared real project check `cargo test -p arda-core` passed.
- Evidence-backed close: `sha256:577fc4c8f6a92fc8d83667268edc5df17c8aa9e08974fe261e96bde3d3860161`, with the full graph terminal at succeeded and the canonical queue terminal carrying `close_complete`.
- Reversible human-visible result: ignored file `target/arda-completion-loop-acceptance-v2.txt` contains exactly `arda-completion-loop-verified-v2\n`, SHA-256 `0fe780361c2621c3340748d41c7fb882d3e8b65065d8c99daf9ef8943600f61c`.
- The append-only queue guard passed before and after acceptance. Generated `core/state/queue_active.json` and `core/state/queue_summary.json` remain runtime projections and are excluded from the implementation commit.
- Final code gates after formatting: the focused executor suite passed 11 tests; `cargo test -p arda-aule --features full-cli` passed 204 library, 8 CLI, 21 integration, and 2 doc tests; and `cargo check --workspace --all-targets` passed. Scoped `git diff --check` passed. Repository-wide `cargo fmt --all -- --check` remains blocked by pre-existing formatting drift in Engine tests, Oromë, and `autopilot/runner.rs`; strict package Clippy remains blocked by pre-existing warnings in `company_ops`, `execution_outcome.rs`, `queue_writer.rs`, and `learning_consumer.rs`. The task-owned Rust files pass direct `rustfmt --check`, and their Clippy findings were resolved before the final package test.

This closes the bounded installed-executor/restart slice, not the complete plan's general decomposition, recurrence, overnight, multi-project, or operator-burden gates.

## Done

This plan is complete only when a real non-trivial task enters through normal intake and reaches verified close—or a justified operator decision—without the operator manually issuing its intermediate tasks.
