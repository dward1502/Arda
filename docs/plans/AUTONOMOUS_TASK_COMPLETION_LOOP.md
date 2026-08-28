---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-27"
  tags: ["task-loop", "scheduler", "verification", "review", "continuation"]
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-27

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

The installed `WorkbenchQueueExecutor` resolves the selected task's project contract and runs `plan → approval → execute → verify → review → close` through the durable Engine graph. It also materializes a validated objective as independently durable canonical queue leaves. Each leaf carries project, authority, checks, evidence, dependency, budget, and digest-bound plan metadata; eligible successors and executable retry/revision/replan records are consumed on later invocations without chat context. Terminal-leaf reconciliation converges after a crash between terminal append and successor/continuation append, and retry claims receive attempt-qualified Workbench run IDs.

The T2/T5 source slice is test-verified, but its final installed-timer acceptance is still open. A live installed-timer run proved objective decomposition, five durable leaves, dependency blocking, and first-leaf dispatch; the user systemd bus then became unavailable before restart, forced-failure correction, unattended closure, and artifact-bound terminal acceptance could be observed. Durable recurrence, `wait_until`, pause/cancel scheduling, multi-project objectives, independent critic selection beyond the verifier role, and operator-facing continuation control also remain open.

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

### 2026-08-27 durable-leaf and executable-continuation slice

- Source behavior: validated objectives materialize as five independently durable append-only queue leaves. Leaf records retain stable objective lineage, full dependency IDs, project/authority/check/evidence contracts, derived budgets, acceptance metadata, an objective-plan run ID, and a receipt digest that is recomputed and checked against the persisted receipt before dispatch.
- Continuation behavior: `retry_same_task` and `revise_task` append same-lineage executable records with incremented sequences and fresh attempt-qualified Workbench run identity; `replan_objective` validates root identity and metadata before append. Persisted `max_attempts` bounds retries. Startup reconciliation repairs a crash after a terminal leaf append by activating successors or materializing the recorded continuation idempotently on the next invocation.
- Closure behavior: an objective closes only after every leaf has a digest-shaped execution receipt and the terminal acceptance leaf's declared artifact exists with every required marker.
- Verification: `cargo test -p arda-aule --features full-cli` passed 227 library tests, 8 CLI tests, 22 integration tests, and 2 doc tests. `cargo clippy -p arda-aule --features full-cli --all-targets -- -D warnings` passed. The installed release binary and `target/release/arda-cli` match at SHA-256 `3f66032b89fcceda8fba70937cb930ee57cb77dd762bef978193c6092ea60443`.
- Bounded live evidence: installed-timer objective `operator-objective-t2-t5-live-20260827-v2` reached root `waiting`, produced five durable leaves, kept four dependency-blocked, and claimed the eligible `recover-context` leaf without another instruction. It was append-only retired after the acceptance environment stopped before terminal proof.
- Blocker: `systemctl --user` returns `Failed to connect to user scope bus via local transport: Connection refused`. Therefore this slice does not yet claim real restart between leaves, installed-timer correction after forced verification failure, unattended terminal closure, or live artifact/evidence-bound acceptance. Those gates remain required before T2/T5 are marked workflow-proven.

### 2026-08-26 first non-trivial operator objective

- Operator objective `operator-objective-78370a00190fd9b8` asked Arda to review itself comprehensively against the operator vision. The operator-approved validated task `operator-objective-78370a00190fd9b8-validated-plan-v1` executed as deterministic run `queue-operator-objective-78370a00190fd9b8-validated-plan-v1`.
- The run persisted the complete `plan → approval → execute → verify → review → close` graph. Its continuation chain is `continue_verify` → `continue_review` → `continue_close` → `close_complete`; close receipt `sha256:33f3bd974006ad5b85c2ec39d430515e30de902b485ebf963e686570837aa15c` is retained in the canonical queue and run checkpoint.
- Declared verification `cargo test -p arda-core` passed. The required human-visible result is `docs/audits/2026-08-26-operator-vision-repair-backlog.md`, SHA-256 `25fdd8bec37c840c0fbde5ded7abc0fb2f6fd9153d036685b3f8368dc47e9bbd`; it separates capability truth levels and gives prioritized P0/P1/P2 repairs with evidence, smallest authoritative surfaces, human-visible behavior, and acceptance conditions.
- The run exposed two restart/authority defects rather than hiding them: stale generated active projections could suppress canonically approved work, and duplicate ID/source aliases could remain separately claimable. `TaskQueueAnalyzer::effective_records` and approved-task selection now derive latest effective work from canonical append-only evidence, with regressions for stale projection recovery and corrected source-key alias folding.
- The installed timer initially repeated the already completed task because `/var/home/mythos/.local/bin/arda-cli` still contained the pre-fix selector. The installed binary was replaced with the final verified build at SHA-256 `8e02451932230aebb62974e94f5616936e2d0f8d04700c2d8cb078371213ada5`; a full timer interval appended no new rows and installed `next-approved-task` returned `null`.
- Acceptance artifacts are now checked before terminal success. Missing artifacts or missing required markers convert an otherwise successful run into a durable failure/continuation decision instead of accepting provider narrative or a package check alone.
- Grounded objective-plan validation is present in the accepted run. Vairë context-use/outcome binding is implemented and covered by package tests, but the accepted run predates the final installed binary and does not carry a Vairë receipt; a later live objective must prove that path. The current graph still folds validated leaves into one worker job; durable independently schedulable leaves, executable retry/revision/replan materialization, recurrence, multi-project placement, genuinely independent review, and operator-burden acceptance remain open and are prioritized in the repair backlog.

## Done

The first-objective acceptance gate is complete: a real non-trivial task entered through normal intake and reached verified close without the operator manually issuing its intermediate tasks. The plan remains active until the open T5/T6 recurrence, independent-review, multi-project, operator-burden, and live Vairë-receipt gates are complete or explicitly retired.
