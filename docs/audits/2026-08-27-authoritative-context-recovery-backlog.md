---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "current_state_audit"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-27"
  tags: ["context-recovery", "objective-plan", "continuation", "repair-backlog"]
---

> 🜏 Soterion: 📜 current_state_audit | owner: PROMETHEUS | status: active | reviewed: 2026-08-27

# Authoritative Context Recovery: Prioritized Repair Backlog

## Decision

The current run recovered the declared project contract, active plans, status report, canonical queue ledger, objective-plan receipt, repository state, and current executor source. The supplied file digests all match live bytes. The supplied repository-state digest does not match the later live `git status --short` output because runtime/run artifacts continued to change the dirty worktree; repository state must therefore be treated as a time-bound observation, not a stable source identity.

The smallest urgent repair is to execute an already-materialized objective leaf directly. The live executor now materializes durable leaves, but a claimed leaf is wrapped in another five-stage Workbench plan and provider prompt. This run is itself evidence of that recursion: the outer `recover-context` leaf became a generic `plan → execute → verify` task, whose execute node was again given `recover-context → inspect-authorities → synthesize-findings → produce-outcome → verify-acceptance`.

## Evidence and truth levels

| Surface | Implemented capability | Configured runtime | Live deployed proof |
|---|---|---|---|
| Project authority | `data/workbench/projects.json` declares Cargo checks and approval-required filesystem authority. | Three contracts are registered, all rooted at `.`; two are proof/fixture identities. | The declared `cargo test -p arda-core` check passed in this job. This proves the package check, not truthful portfolio attachment. |
| Grounded planning | `ObjectiveDecomposer::decompose_grounded` emits context sources, acceptance criteria, dependency tasks, leaf contracts, budgets, and checks (`decomposer.rs:220-240,421-453`). | `objective_plan_for_task` hashes five files plus `git status --short`, validates the plan, and binds the selected project (`workbench_executor.rs:906-975`). | `audit/workbench-queue/queue-tsk_20260827T235626Z_operator-objective-t2-t5-live-20260827-v2__recover-context__plan/objective_plan_receipt.json` contains the five validated leaves and exact declared source digests. |
| Durable leaves | The queue executor materializes objective leaves and returns `continue_next_task` (`workbench_executor.rs:88-103`). | Claimed leaves are recognized and recover their parent plan (`workbench_executor.rs:313-320`). | Canonical queue rows 126-131 materialize the v2 objective leaves; rows 133-136 show the recovered leaf subsequently decomposed into another generic task chain. |
| Leaf execution | Leaf-specific authority, evidence, budget, timeout, and checks are represented. | The run graph receives the leaf contract. | The execute provider still receives `objective_execution_prompt(&objective_plan, ...)` (`workbench_executor.rs:368-385`), so one leaf is not yet a single bounded execution unit. |
| Continuation | Failure classification returns retry, revise, replan, operator decision, or close (`workbench_executor.rs:760-804`). | Attempt limits are read from leaf budget metadata. | The queue contains `continue_next_task` and stage continuations, but this job provides no live proof that retry/revise/replan materializes a corrected successor and later closes the parent objective. |
| Context continuity | Vairë records context-use and outcome receipts. | Without a supplied receipt, the executor constructs a one-hour system-only capsule. | The fallback has no memory references or unresolved failures (`workbench_executor.rs:826-903`); this job recovered files manually, not through a live retrieved context capsule. |
| Status truth | Active plans describe the installed full graph and its bounded acceptance proof. | `ARDA_SYSTEM_STATUS_REPORT.md` remains active documentation. | The status report still says the executor omits verify/review/close, contradicting the newer active plan and source. No reconciliation check prevented the drift. |

## Prioritized repairs

### P0.1 — Execute materialized leaves directly

Evidence: `dispatch_claim` recovers the parent objective plan for a leaf, but the execute stage still serializes that whole plan into a provider objective (`workbench_executor.rs:313-373`). The queue ledger demonstrates recursive decomposition around the v2 `recover-context` leaf.

Human-visible behavior: an operator sees one objective with five durable leaves. Each timer tick claims exactly one eligible leaf, performs only that leaf's bounded work, and advances its dependent without creating a nested copy of the plan.

Smallest authoritative implementation surface: change only Aulë's Workbench executor prompt construction and graph provenance for objective leaves. Build the execute objective from the selected `PlannedTask` plus its `ExecutableLeafContract`, acceptance criteria, and bounded context references; retain the full plan receipt as provenance. Do not add another queue, graph type, or coordinator.

Acceptance: the v2 objective yields exactly five leaf identities; no leaf creates `__plan`, `__execute`, or `__verify` descendants; a restart resumes the same leaf; each leaf receipt names its own authority, budget, project, and evidence requirements.

### P0.2 — Materialize typed retry, revision, and replan successors

Evidence: continuation classification exists, but current source evidence only returns decision strings after terminal outcomes. The active plans still identify executable retry/revision/replan materialization as open.

Human-visible behavior: a failed check produces one corrected successor under the same objective lineage, with the defect and changed acceptance evidence visible; no new operator instruction is needed while authority and budget remain valid.

Smallest authoritative implementation surface: append one canonical successor/tombstone record from the existing queue executor when the decision is `retry_same_task`, `revise_task`, or `replan_objective`; use the existing leaf contracts, dependency identities, attempt budgets, and timer. Engine remains transition authority.

Acceptance: force one deterministic verification failure, observe `revise_task`, run one corrected successor after a timer/process restart, and derive parent closure only after all required leaves and acceptance evidence pass.

### P0.3 — Make repository-state evidence snapshot-stable

Evidence: all five file digests in the supplied node context matched the live files, while live `git status --short` hashed differently from the receipt's repository-state digest because runtime output continued changing the worktree.

Human-visible behavior: an operator can tell what repository state the planner actually consumed, when it was observed, and whether later drift affects the plan.

Smallest authoritative implementation surface: persist the canonicalized status bytes (or a bounded redacted snapshot artifact), observation timestamp, and digest in the objective-plan receipt. At execution, report drift separately instead of silently comparing a moving command output to an old digest. Exclude generated queue projections from authored change sets, not from truthful status observations.

Acceptance: receipt digest recomputation succeeds from retained snapshot bytes; later runtime churn is labeled drift; protected unrelated dirty paths remain unchanged.

### P0.4 — Replace fallback context with authorized retrieval

Evidence: the fallback context is system-only and contains empty `memory_refs` and `unresolved_failures`, despite the whole-system plan requiring conversations, corrections, receipts, repositories, and runtime state.

Human-visible behavior: returning to an objective shows a concise sourced packet of prior decisions, current project truth, unresolved failures, and the next bounded leaf; excluded or stale context is disclosed.

Smallest authoritative implementation surface: before provider dispatch, ask the existing Vairë service to assemble the leaf's permitted project/system context and bind its use receipt to objective, leaf, run, and source digests. Preserve the existing fallback only as an explicitly degraded mode.

Acceptance: one fresh worker resumes after restart from a bounded capsule, cites consumed references, honors correction/revocation, and records a context-outcome receipt.

### P1.1 — Attach a truthful canonical Arda project contract

Evidence: the registry has three records rooted at `.`, including a fixture, and the selected project ID remains the historical example identity.

Human-visible behavior: every leaf names a recognizable Arda project, exact contract digest, root, checks, protected dirty-worktree policy, authority, and rollback.

Smallest authoritative implementation surface: update the existing Workbench registry through its governed API; remove implicit production fixture fallback; bind the exact contract digest into run provenance. Do not hand-edit generated queue projections.

Acceptance: missing/stale project identity fails closed; the exact contract digest appears in plan and execution receipts; unrelated dirty work remains untouched.

### P1.2 — Reconcile active status claims automatically

Evidence: the active status report's completion-loop maturity and immediate-work text predate the active plan's installed full-graph proof.

Human-visible behavior: status surfaces distinguish implemented, configured, installed, live-proven, and operator-accepted states without contradictions.

Smallest authoritative implementation surface: add a read-only reconciliation check over active plans, project registry, installed artifact identities, and recent close receipts; report stale claims without making generated state an authority.

Acceptance: the known verify/review/close contradiction is detected, and future stale active claims fail the documentation/status gate.

## Dependency order and closure

1. P0.1 removes recursive execution and establishes truthful leaf identity.
2. P0.2 proves T5 continuation with a corrected successor across restart.
3. P0.3 and P0.4 make recovered repository and memory context reproducible.
4. P1.1 strengthens project authority and provenance.
5. P1.2 prevents the resulting proof from drifting into contradictory status prose.

Closure requires live evidence that one parent objective advances its original durable leaves, survives a timer/process restart, corrects one failed verification under the same objective lineage, and closes structurally from leaf receipts and acceptance evidence. A passing package check alone is necessary engineering evidence, not objective closure.
