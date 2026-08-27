---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "current_state_audit"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-27"
  tags: ["governance", "continuous-execution", "workbench", "repair-backlog"]
---

> 🜏 Soterion: 📜 current_state_audit | owner: RUMIL | status: active | reviewed: 2026-08-27

# Continuous Governed Execution Gap Report

## Decision

Arda now has a live, governance-authorized path from an objective packet into the canonical queue and a durable Workbench run. The current audit itself proves that bounded slice: canonical queue row 73 carries a digest-bound `safe_autonomous` authorization, row 76 records the claim, and the run checkpoint records successful plan/approval transitions plus a running hosted Hermes execute node.

That is not yet continuous governed objective completion. The live path queues template leaves but does not enforce their dependencies, executes each leaf as another opaque five-step provider prompt, bypasses readiness holds for any action class classified `safe_autonomous`, defaults missing project identity to a proof contract, and records continuation decisions without scheduling them. Verification is real for the attached Cargo check; review, worker placement, context retrieval, approval expiry, and objective-level closure remain weaker than their labels imply.

The smallest authoritative program is therefore to harden the existing Aulë queue/Workbench adapter and Engine graph rather than add another coordinator, queue, or report pipeline.

## Authority and repository evidence

All required live authorities were read before synthesis. The four stable authorities and the append-only queue matched the node-supplied digests on disk; repository status was separately re-read because it is a live stream:

- Project registry: `data/workbench/projects.json`, SHA-256 `ce3a769c4fb1b74cc2ea3a20bad172d392a3221d995dddaeb17944ce2156e0f4`. It declares `cargo test -p arda-core`, but production authority still consists of three proof/stage contracts rooted at `.`.
- Completion-loop plan: `docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md`, SHA-256 `cbefe9d817453d2df6d8f5e0520c25285df2b7677036194a61c6d8064e9361a6`. Lines 34-36 distinguish the installed full stage graph from still-open decomposition, retry/revision, recurrence, multi-project, independent review, and operator control.
- Whole-system program: `docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md`, SHA-256 `b3b556f8a67ebdf2bf2b0992878d9165cfaf44752751e7295eff5e44b9d2721e`. Lines 49-64 define the canonical observe-through-learn loop; lines 173-181 forbid treating tests, timers, reports, queue rows, or same-role review as product completion.
- Status report: `ARDA_SYSTEM_STATUS_REPORT.md`, SHA-256 `ecb62ed522e66a86f315ac7ea38fa338a28045ae4cddfaa8955be4b2ed24436f`. Its lines 47 and 121-130 still describe the pre-2026-08-26 truncated executor, contradicting the newer completion-loop authority.
- Canonical queue: `core/projects/tasks/queue.jsonl`, SHA-256 `7d25fb0066ba99acb46c7f8bb048ba49652d4e1b054a7a69d60ecb959512d023`. Rows 73-75 are the current audit's `scope → gather → synthesize` template leaves; rows 76-80 close `scope`, rows 81-85 close `gather`, and row 86 claims `synthesize`.
- Repository status: the node supplied `sha256:25b19d04f619120db0d01aabbda3b692f43cbbdbf636db15f0a22ae3ff375567`; the live `git status --short` stream read during synthesis hashed to `sha256:8f8f90396fba41913f34bf624e0a5bae6f4cf9acde9b73d0a55189e2de4ddb81`. This report does not reinterpret that drift: the worktree already contained extensive tracked and untracked work, including runtime queue/run state. No generated queue projection was edited by this node.

Declared project check `cargo test -p arda-core` passed before exploration: 113 unit tests and all package integration and doc-test groups passed. This is package engineering evidence, not acceptance of the continuous loop.

## Live path trace and capability truth

1. Aulë classifies a selected objective and template-decomposes it (`runner.rs:1598-1669`; `decomposer.rs:123-142,273-302`).
2. A governance decision with `SafeAutonomous` or `TriadQuorumApproved` becomes a binding authorization (`runner.rs:1664-1679`). This binding explicitly bypasses a failing global autonomy-readiness gate.
3. `append_packet_plan_with_authority` derives `governance:<packet>:<action_class>` and writes all plan leaves to the canonical queue (`queue_operation.rs:137-203`; `queue_writer.rs:70-126`).
4. `ActiveQueueExecutor` claims the first approved pending record and appends an in-progress row (`task_queue.rs:259-343`).
5. `WorkbenchQueueExecutor` builds or resumes `plan → approval → execute → verify → review → close` under a deterministic run ID (`workbench_executor.rs:65-203,256-415`).
6. Execute and verify use hosted Hermes; declared project checks are required before closure. Review and close are locally completed from receipt digests (`workbench_executor.rs:322-415,517-531,895-959`).
7. A terminal decision is appended as `close_complete`, `retry_same_task`, `revise_task`, `replan_objective`, or `request_operator_decision`, but no successor or wake-up is materialized (`workbench_executor.rs:590-613`).

Current audit live proof: canonical queue rows 76-80 show that `scope` traversed execute, verify, review, and evidence-backed close; row 81 then claims `gather`. `data/runs/queue-tsk_20260827T075204Z_arda-governed-audit-20260827__gather/checkpoint.json` records the supplied project-contract digest, validated grounded plan, governance authorization parent, successful plan/approval, and the hosted Hermes execute node. This is configured-and-live evidence for sequentially observed leaf dispatch, not evidence that queue dependency edges enforced the order or that `synthesize` and the objective will close.

## Prioritized repair backlog

### P0.1 — Enforce dependency identity and success before claim

Evidence: `queue_writer.rs:93-102` gives each leaf a timestamped full task ID but writes `depends_on` unchanged as local keys such as `scope`. `task_queue.rs:259-343` claims the first record based only on status and approval metadata; it never evaluates `depends_on`. The live queue demonstrates both shapes in rows 73-75.

Risk: after `scope` fails or is cancelled, `gather` remains independently claimable. Queue ordering currently impersonates dependency execution but provides no correctness guarantee.

Human-visible behavior: a failed prerequisite visibly blocks its descendants; the operator sees the exact failed edge and one governed continuation rather than downstream work running on missing evidence.

Smallest authoritative repair surface: in `queue_writer.rs`, resolve plan keys to the full IDs generated in the same append operation; in `task_queue.rs`, make claim eligibility require every dependency's canonical effective state to be succeeded/completed. Add cycle/missing-dependency rejection to the existing plan validator. Preserve the single append-only queue.

Acceptance: tests prove that successful prerequisites unlock descendants, failed/cancelled prerequisites do not, unrelated roots remain eligible, and restart/replay cannot bypass the edge.

### P0.2 — Bind readiness bypass to explicit action-class policy

Evidence: the live readiness projection and queue metadata say `hold` for stale preflight plus missing Hades and Athena evidence. Nevertheless, `runner.rs:1664-1679` permits any governance-classified `SafeAutonomous` or `TriadQuorumApproved` plan to bypass `autonomy_readiness.task_promotion_allowed`. The current read-only audit was therefore correctly useful but exposes a broad semantic bypass.

Risk: a future action class can inherit the same bypass even when the missing readiness evidence is relevant to that action. The queue preserves the hold reasons but does not prove they were adjudicated as irrelevant.

Human-visible behavior: a task shows either “readiness passed” or “specific hold waived for this action class by policy,” naming the evidence and boundary. Missing evidence relevant to mutation blocks promotion.

Smallest authoritative repair surface: replace the boolean bypass in `runner.rs` with a governance-policy decision that maps each hold reason to action-class relevance and emits a signed/hashed waiver receipt. `queue_operation.rs` should require and retain that receipt when readiness is not `allow`.

Acceptance: read-only local audit may proceed under a scoped waiver; filesystem mutation, egress, cleanup, and external-source work fail closed for their relevant missing lanes; tampered or cross-class waivers are rejected.

### P0.3 — Execute objective leaves directly instead of recursively wrapping each leaf

Evidence: Aulë first emits template leaves (`decomposer.rs:273-302`). When one leaf is claimed, `objective_plan_for_task` invokes the fixed grounded five-step plan, and `objective_execution_prompt` serializes all five steps into one provider call (`workbench_executor.rs:270-284,783-814`). Thus one `scope` leaf becomes another opaque `recover-context → ... → verify-acceptance` prompt instead of a bounded leaf contract.

Risk: task granularity, role assignment, cost estimates, and dependencies are provenance prose rather than independently enforced Engine work. Objective completion cannot be derived from leaf completion.

Human-visible behavior: each canonical leaf has one purpose, project, authority, budget, checks, and artifact contract; the objective view advances from leaf to leaf without hidden recursive plans.

Smallest authoritative repair surface: make the canonical queue's `PlannedTask` the direct Engine execute contract. Reserve `decompose_grounded` for objective admission, not every claimed leaf. Persist one objective graph mapping queue task IDs to Engine nodes and derive objective close from all required terminal leaves.

Acceptance: this audit shape runs exactly three durable leaves, each once, with enforced edges and a single objective-level close receipt.

### P0.4 — Fail closed on project identity and bind the actual contract digest

Evidence: `workbench_executor.rs:52-53,295` defaults missing task project IDs to `DEFAULT_PROJECT_ID`. The registry's three contracts all root at `.`, and the selected default is still a proof record. The current checkpoint has a real digest, but the queue rows 73-75 contain no `project_id`.

Risk: a task can execute against whichever default the installed environment supplies, weakening scope, checks, artifacts, and dirty-worktree protection.

Human-visible behavior: every task and run displays the exact project ID, root, contract digest, declared checks, and protected dirty paths; absent or stale identity is a named blocker.

Smallest authoritative repair surface: require `project_id` in queue admission and `task_project_id` in dispatch; remove the production fallback; resolve and hash the selected registry contract before graph creation; carry that digest into graph provenance and worker context.

Acceptance: missing, unknown, stale-digest, and wrong-root projects fail before claim/dispatch; one truthful Arda contract runs without touching unrelated dirty work.

### P0.5 — Materialize continuation decisions under budgets

Evidence: `workbench_executor.rs:590-613` classifies failure from result text and prior continuation count. `execute_once` appends the decision to a terminal row but does not call `retry_failed`, create a revision/replan task, or schedule a wake time (`workbench_executor.rs:111-152`; `task_queue.rs:358-415`).

Risk: “continuous” stops at descriptive evidence. A timer can only discover independently pending rows, not consume the declared next action.

Human-visible behavior: every non-terminal objective has exactly one executable next action and next wake time; retry/revise/replan preserves lineage, authority, attempts, cost, and cancellation.

Smallest authoritative repair surface: add the T5/T6 append-only continuation/schedule records beside the canonical queue, and make the existing timer atomically consume them. Replace string matching with typed failure classes. Reuse `retry_failed` only after policy validates remaining budget and authority.

Acceptance: integration proofs cover retry, revision, replan, wait, budget exhaustion, pause, cancel, recurrence, and restart without chat state.

### P0.6 — Make verification and review claims exact

Evidence: verify runs project-native checks through a separately named worker, but execute and verify share `hosted:hermes-workbench` (`workbench_executor.rs:928-950`). Review has no worker; it is completed locally after closure evidence exists (`workbench_executor.rs:375-414,952-953`). The whole-system authority explicitly says same-role/context self-review is not independent.

Risk: the graph can label evidence handling as review and label a worker independent without proving route/model/context separation or challenge findings.

Human-visible behavior: low-risk work says “independent review not required”; material work names the distinct critic, findings, evidence, and accept/revise/escalate disposition.

Smallest authoritative repair surface: retain deterministic verify; gate semantic review through Core composition policy; route a required critic through Manwë with distinct provenance; persist structured findings and an Engine rejection edge.

Acceptance: clean accept, critic rejection/correction, unavailable-critic fail-closed, and disclosed no-review cases all have distinct receipts.

### P1.1 — Route through Manwë and record selected versus actual worker

Evidence: `workbench_executor.rs:928-950` hard-codes both provider nodes to hosted Hermes despite existing placement capabilities cited by the active program.

Human-visible behavior: each leaf explains deterministic/local/hosted selection, actual route, fallback, cost, latency, privacy fit, and reviewer separation.

Smallest authoritative repair surface: request placement through the existing Manwë owner before Engine dispatch, pass the selected route into Hermes, and bind selected/actual route receipts to the node. Do not move execution authority into Manwë.

### P1.2 — Retrieve bounded Vairë context before planning

Evidence: absent a supplied receipt, `workbench_executor.rs:615-682` constructs a system-only one-hour context with empty memory and failure references. The current node context has `context_use_receipt: null` and `organism_context_capsule: null`.

Human-visible behavior: resuming an objective provides a concise, source-bound “where we were / what changed / what is next” capsule, including relevant corrections and unresolved failures.

Smallest authoritative repair surface: have Vairë assemble the capsule at objective admission from explicit domains; require its use receipt in each leaf and record outcome against the objective, run, and project digest.

### P1.3 — Persist approval scope, expiry, and consumption

Evidence: `approval_envelope` synthesizes `created_at_utc` on every transition and carries no expiry (`workbench_executor.rs:817-860`). Governance authorization is validated by reconstructing a formatted string (`task_queue.rs:582-605`), not by reading an expiring canonical receipt.

Human-visible behavior: operators can inspect what authority was granted, for which project/action/budget, until when, and whether it was consumed, revoked, or superseded.

Smallest authoritative repair surface: Core/Engine canonical approval and governance-receipt store; validate exact scope, digest, expiry, revocation, and replay at claim and every consequential transition.

### P1.4 — Reconcile status truth and expose one objective projection

Evidence: the active status report still says verify/review/close are omitted, while the newer plan and live checkpoint prove those stages exist. Conversely, stage labels overstate review independence and objective continuation.

Human-visible behavior: Hermes/HUD shows one objective with current leaf, project, authority, readiness waiver, route, checks, evidence, budget, continuation, schedule, freshness, and controls.

Smallest authoritative repair surface: generate installed/source/plan reconciliation from contract and binary digests plus live receipts; project canonical objective state from Core/Engine; make stale active claims fail a status check.

## Dependency order

1. P0.1 and P0.4 establish trustworthy task and project boundaries.
2. P0.2 establishes exact authority when readiness is degraded.
3. P0.3 establishes one non-recursive objective graph.
4. P0.5 makes graph outcomes continue durably.
5. P0.6, P1.1, P1.2, and P1.3 establish truthful evidence, placement, context, and authority.
6. P1.4 makes the same state operator-visible and prevents capability drift.

## Acceptance gate

This backlog is not complete when package tests pass or this report exists. It is complete when one governed objective with at least three dependency-bound leaves:

- is admitted under a scoped, current approval or action-class waiver;
- names a truthful project contract digest per leaf;
- blocks descendants after a forced prerequisite failure;
- materializes a bounded correction and resumes after restart;
- uses receipted placement and bounded Vairë context;
- runs declared native checks and, when required, a genuinely distinct critic;
- reaches one objective-level close only after all acceptance evidence passes;
- exposes one operator-visible next action throughout; and
- requires no repeated operator assignment.

Until then, the exact capability truth is: governance-authorized queue admission and durable single-leaf Workbench execution are live; continuous governed objective completion is not yet proven.
