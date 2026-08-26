---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "current_state_audit"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-26"
  tags: ["operator-vision", "repair-backlog", "autonomy", "composition", "acceptance"]
---

> 🜏 Soterion: 📜 current_state_audit | owner: RUMIL | status: active | reviewed: 2026-08-26

# Operator-Vision Review and Prioritized Repair Backlog

## Decision

Arda has a substantial, tested organism kernel and now has one real restart-safe Workbench graph, but it does not yet fulfill the operator vision. The highest-value repair is not another schema, monitor, named agent, or embodiment. It is to make the existing authorities own a general objective through truthful project context, decomposition, placement, execution, verification, challenge, continuation, and operator-visible closure.

The current implementation proves a bounded single-project adapter slice. It does not yet prove that Arda continuously chooses and completes the obvious next work across real projects, uses the available worker fleet according to policy, retrieves the operator's durable context, or improves projects daily without management overhead.

This review treats source, tests, generated state, installed-runtime claims, and operator acceptance as different evidence classes. The declared package check `cargo test -p arda-core` passed during this review: 113 unit tests and all package integration/doc-test groups passed. That establishes `arda-core` engineering health only; it is not whole-system acceptance.

## Operator outcome used for the review

The active completion contract in `docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md` requires the operator to state an outcome once, then have Arda retrieve durable context, frame and decompose bounded work, apply existing authority, schedule and place it, execute and verify it, challenge material claims, continue across restarts, and stop only at verified completion or a concrete operator decision. The product doctrine in `docs/architecture/ARDA_1_0_PERSONAL_AGENT_ECOSYSTEM.md` additionally requires one coherent phone/desktop relationship, Personal Operations, scoped Vairë memory, proactive communication policy, optional local/council capability, truthful projections, and removable external capabilities.

The repair order below optimizes for reduced operator management burden. Downstream Mirromere, RELIC expansion, sensors, payments, public accounts, and commercialization remain held.

## Capability truth snapshot

| Capability | Capability truth level | Evidence | Human-visible behavior now | Required next level |
|---|---|---|---|---|
| Durable Workbench execution graph | Workflow-proven, bounded single-project slice | `docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md`; `workbench_executor.rs` constructs plan/approval/execute/verify/review/close and records continuation stages | One approved task can survive executor restart and close after a project check | General multi-task continuation with real revision/retry and non-trivial acceptance |
| General objective decomposition | Implemented as a static planning shape; not operationally decomposed | `decomposer.rs:144-217` always emits the same five analysis stages; `workbench_executor.rs:780-811` folds them into one provider prompt | A broad objective still becomes one opaque worker job | Durable dependency tasks with individual scope, project, authority, checks, and successor eligibility |
| Verification | Workflow-proven for declared project checks | `workbench_executor.rs:320-373,515-529`; installed acceptance evidence in the active task-loop plan | A task is not closed without at least one passing attached-project check | Acceptance coverage beyond command success, artifact/runtime checks, and failure-driven revision |
| Independent review | Specified in graph, not independently workflow-proven | `workbench_executor.rs:912-925` assigns verifier to the same fixed hosted route; review is completed locally at `373-412` without a critic execution | “Review” can appear complete without an independently generated challenge | Distinct eligible failure profile, review findings, reject/revise path, and retained review receipt |
| Connected projects | Contract mechanism implemented; production portfolio not connected | `data/workbench/projects.json` contains three proof/stage records, all rooted at `.`; `CONNECTED_PROJECT_FABRIC.md` lists the unattached portfolio | Arda cannot safely plan or execute across the operator's actual repositories | Approved truthful Arda contract first, then classified portfolio waves and cross-project proof |
| Provider placement | Implemented/tested elsewhere; bypassed by canonical queue execution | `workbench_executor.rs:900-923` hard-codes `hosted:hermes-workbench`; `PROVIDER_WORKER_CONVERGENCE.md` records live Manwë capacity | Queue work uses the Hermes default rather than a receipted local/hosted policy choice | Manwë requirement request, selected/actual route receipt, explicit fallback, separate reviewer placement |
| Continuation and scheduling | Stage continuation implemented; objective continuation not root-composed | `workbench_executor.rs:588-610` classifies failure but no successor/retry task is scheduled; `AUTONOMOUS_TASK_COMPLETION_LOOP.md:T5-T6` remains open | A failed terminal row tells why it stopped but does not reliably create the next bounded action | Canonical continuation/schedule ledger consumed on later ticks with budgets and pause/cancel semantics |
| Vairë context continuity | Workflow-proven in bounded organism tests; weakly consumed here | `workbench_executor.rs:613-709` creates a system-only fallback context with no memory refs when no receipt is supplied | Worker context can omit relevant conversations, project memory, corrections, and unresolved failures | Retrieval plan bound to authorized personal/business/system scopes and a context-use receipt |
| Operator projection and controls | Root-composed read projection; incomplete and partly misleading | `core/state/operator_projection.json` uses IDs as titles, reports empty capabilities/communications/councils, fallback Joules, and explicitly lacks approval expiry | Operator can see runs but not a trustworthy outcome, next decision, route, budget, or full control state | One source-truth objective view with pause/reprioritize/revise/approve/reject/cancel controls |
| Daily research and improvement | Specified; not installed end to end | `DAILY_RESEARCH_IMPROVEMENT_LOOP.md` records absent timers, wrong survey root, and no governed research-to-change bridge | Research can create evidence or reports but does not reliably land verified improvements | Seven-day installed cycle with one improvement, one no-change, one rejected idea, and continuation |
| Proactive Personal Operations | Core policy/store implemented; production cycle incomplete | `crates/engine/src/personal_ops/proactive_cycle.rs`; active plans still hold scheduler/delivery composition open | Captures and prior reminder proof exist, but current work is not calmly resumed and communicated as one relationship | Event-driven policy cycle, delivery receipt, defer/dismiss, fatigue budget, and operator dogfood |
| External assimilation | Contracts and restart-safe lifecycle implemented/tested; production coordinator absent | `docs/audits/2026-08-25-autonomous-system-gap-audit.md:85-89`; Whole-System Program Phase 5 | External ideas can be evaluated in fixtures but do not traverse governed adoption or rejection in production | One real candidate through provenance, isolated trial, decision, landing/rejection, verification, and removal proof |
| Runtime/release truth | 0.9 baseline self-qualified; status estate is drifting | `ARDA_SYSTEM_STATUS_REPORT.md` still says the installed graph omits verify/review/close after the 2026-08-26 installed proof; phone support claims also differ across active documents | Operators and agents can act on stale maturity claims | Generated installed-vs-repository reconciliation and one current support matrix |
| Whole-system usefulness | Not operator-accepted | Whole-System Program Phase 6 and doctrine completion vocabulary | The operator still has to reconnect projects, providers, research, and next actions | Real two-project unattended objective with restart, fallback/reviewer, correction, closure, and burden assessment |

## Prioritized authoritative repair backlog

### P0.1 — Turn validated plans into durable executable task graphs

Evidence: `ObjectiveDecomposer::decompose_grounded` emits a fixed five-step shape for every objective (`decomposer.rs:144-217`), while `objective_execution_prompt` serializes those steps into one Execute-node prompt (`workbench_executor.rs:780-811`). The run graph therefore preserves a plan as provenance but does not execute its leaves as independently schedulable tasks. Task budgets are also fixed at 5,000 joules/$2 and two attempts (`workbench_executor.rs:874-885`) rather than derived from the objective.

Human-visible behavior: after stating a broad outcome once, the operator sees a bounded dependency plan whose leaves advance independently. Failed verification revises only the affected task; eligible successors start without a new instruction.

Smallest authoritative repair surface: extend `arda-core` queue/outcome contracts additively; make Aulë decomposition emit queue records or Engine graph nodes with project/check/authority/budget fields; let Engine own node transitions and successor eligibility. Do not create another queue or coordinator.

Acceptance:
- one non-trivial objective produces at least three durable leaves with explicit dependency coverage;
- each leaf has a real project ID, acceptance evidence, authority class, and derived budget;
- a forced verification failure yields `revise_task` or `replan_objective`, lands a corrected successor, and later closes under the same objective ID;
- restart between leaves resumes the eligible node without chat context.

### P0.2 — Connect the real Arda project before expanding the portfolio

Evidence: `data/workbench/projects.json` has three historical/proof contracts, all with workspace root `.`, while `workbench_executor.rs:23,50-51,293` still permits a default fixture project ID. Run provenance writes an all-zero project-contract digest (`workbench_executor.rs:934-937`). This makes project identity and artifact provenance weaker than the passing check suggests.

Human-visible behavior: the operator can inspect one truthful Arda project card showing purpose, dirty-worktree policy, commands, protected paths, authority, rollback, and current backlog. Every run names the exact approved contract digest and root.

Smallest authoritative repair surface: retain `data/workbench/projects.json` and the existing harness project API; attach a canonical Arda contract with an actual digest; remove production fallback to the fixture ID; fail closed when task project identity is absent or stale. Then implement `CONNECTED_PROJECT_FABRIC.md` wave 1.

Acceptance:
- no production run can use the default fixture project implicitly;
- graph provenance matches the attached canonical contract digest;
- dirty unrelated files remain untouched;
- one correctly rooted Arda task passes its declared checks, followed by one two-project compatibility run.

### P0.3 — Route workers and reviewers through Manwë placement

Evidence: execute and verify workers are both hard-coded to `route_id = hosted:hermes-workbench` and `route_class = hosted` (`workbench_executor.rs:900-923`). This bypasses the existing adaptive-placement path and cannot prove privacy-local routing, capability fit, actual-route agreement, or reviewer independence.

Human-visible behavior: each task shows why deterministic/local/subscription/paid capacity was selected, what actually ran, cost/latency, and why any fallback occurred. Private work remains local; incompatible cheap models are rejected.

Smallest authoritative repair surface: add placement requirements to the canonical task; call the existing Manwë adaptive placement owner before provider execution; pass the selected route into the Hermes adapter; bind selected and actual route receipts to the run. Keep Hermes as worker runtime and Manwë as provider/model authority.

Acceptance: execute all seven proofs in `PROVIDER_WORKER_CONVERGENCE.md`, including no-model deterministic work, healthy local work, authorized hosted fallback, private-local enforcement, and a distinct reviewer route.

### P0.4 — Make continuation decisions executable and scheduled

Evidence: `continuation_decision` returns `retry_same_task`, `revise_task`, `replan_objective`, or `request_operator_decision` (`workbench_executor.rs:588-610`), but `execute_once` writes a failed terminal queue record and does not materialize or schedule the decision. The second failure is classified as replan solely from a continuation counter, without a typed failure policy. No canonical recurrence ledger is root-composed.

Human-visible behavior: every open objective shows exactly one next action and next wake time. Transient failures retry within budget; defects revise; changed assumptions replan; genuine gates ask one concrete question. Pause/cancel prevents later wakeups.

Smallest authoritative repair surface: implement T5/T6 in `AUTONOMOUS_TASK_COMPLETION_LOOP.md` as a canonical append-only continuation/schedule ledger keyed by objective/task/run; make the existing timer consume it. Use typed failure classes, attempt/time/cost budgets, idempotency, and cancellation tombstones.

Acceptance: integration proofs for retry, revision, replan, wait-until, budget exhaustion, pause, cancellation, recurrence, and restart; no decision relies on a previous Hermes session.

### P0.5 — Persist approval identity, scope, and expiry

Evidence: `core/state/operator_projection.json` explicitly reports `approval_expiry_store` as `not_configured`; Engine source states run graphs do not persist canonical approval identity, scope, and absolute expiry (`crates/engine/src/operator_projection.rs:149-154`). The queue executor synthesizes envelope timestamps on each call while carrying packet IDs from task metadata.

Human-visible behavior: the operator sees what was approved, for which project/action/run, until when, what budget it grants, and whether it was consumed, revoked, or expired. A resumed run cannot reuse stale or broader authority.

Smallest authoritative repair surface: add a canonical approval receipt/store under Core/Engine authority; validate scope and expiry at every mutating transition and continuation; project it read-only to Hermes/HUD.

Acceptance: expired, revoked, wrong-project, wrong-action, replayed, and over-budget approvals fail closed across restart; valid scoped approval resumes exactly once.

### P0.6 — Replace ceremonial review with independent challenge and revision

Evidence: verification is delegated, but the Review node has no worker and is completed by a locally generated digest after checking that verification evidence exists (`workbench_executor.rs:373-412,912-925`). Execute and verify also share the same route class. This proves evidence chaining, not independent review.

Human-visible behavior: material work displays the critic's named concerns, evidence, disposition, and required repairs. Low-risk deterministic work can disclose that independent review was not required rather than manufacture one.

Smallest authoritative repair surface: use Core composition policy to decide whether review is required; ask Manwë for a distinct critic profile; persist structured findings and `accept|revise|escalate`; let Engine route rejection back to the affected node. Keep deterministic verification separate from semantic review.

Acceptance: one clean acceptance, one critic rejection followed by correction, one unavailable-independent-review fail-closed case, and one disclosed low-risk no-review case.

## P1 — Compose the operator relationship

### P1.1 — Retrieve real bounded context, not a generated empty fallback

Evidence: when a queue task lacks a supplied receipt, `record_context_outcome` constructs a one-hour system-domain context with no memory references and generic acceptance (`workbench_executor.rs:613-709`). It does not retrieve relevant conversations, corrections, project decisions, personal constraints, or unresolved failures.

Human-visible behavior: returning to an objective yields a concise “where we were / what changed / next action” packet sourced from authorized memory and current project truth, with stale or unavailable data disclosed.

Authoritative repair: Vairë assembles context from explicit personal/business/system scopes; the planner records selected and excluded references; workers receive only the bounded capsule; outcome receipts record which memories influenced work. Acceptance requires correction/revocation and cross-model/restart tests plus one operator-used resume.

### P1.2 — Publish one truthful objective/control projection

Evidence: the live operator projection renders objective IDs as titles, has no projected capabilities, councils, or communications, reports default-fallback JouleWork as the budget source, and retains current runs whose statuses need reconciliation. Queue, run, Personal Operations, research, and approvals are still separate views.

Human-visible behavior: Hermes and HUD show outcome, current leaf, project, evidence, route, observed/estimated budget, next continuation, schedule, freshness, and blocker. The same record accepts pause, reprioritize, revise, approve, reject, and cancel.

Authoritative repair: Engine/Core remain truth owners; correct `operator_projection.rs` and harness mutations, then consume the same schema from Hermes and HUD. Do not let either interface mint completion. Acceptance requires source/freshness labels and restart-stable controls on one real objective.

### P1.3 — Activate the goal-driven daily improvement loop

Evidence: `DAILY_RESEARCH_IMPROVEMENT_LOOP.md` records that Warden timers are absent from the active unit set, the survey path is wrong, and triaged findings do not pivot into canonical work. Existing evidence records demonstrate research storage, not daily implementation.

Human-visible behavior: a quiet daily brief lists only verified improvements, continuing work, concrete decisions, justified no-change findings, and exhausted failures.

Authoritative repair: execute D1-D6 in the existing Warden/Varda → governance → completion-loop boundaries. Install only after canonical roots and useful-cycle receipts are correct. Acceptance is the plan's seven consecutive cycles, including improvement/no-change/rejection and restart continuation.

### P1.4 — Root-compose proactive Personal Operations

Evidence: `ProactiveCycleStore` implements durable evaluation, delivery permits, deduplication, and operator responses, but source search locates production behavior primarily in the library and tests; the active whole-system plan still identifies operator relationship composition as open.

Human-visible behavior: operator-authored captures, commitments, and transitions produce one calm, timely suggestion or reminder; dismiss/defer/acknowledge is durable; silence is normal when interruption is not justified.

Authoritative repair: feed fresh Personal Operations events and objective transitions into the existing proactive policy/store, deliver through Oromë/Hermes Gateway, and return delivery/response receipts. Acceptance requires fatigue/quiet-window, degraded transport, restart, and operator dogfood evidence.

### P1.5 — Operationalize governed external assimilation

Evidence: external-capability, AIPKG, and assimilation lifecycles are implemented and tested, but the 2026-08-25 audit found no production coordinator spanning candidate acquisition through contract choice, isolated trial, governance, landing/rejection, verification, and removal evidence.

Human-visible behavior: when a measured need exists, Arda can say why an external system was evaluated, what was actually tried, what authority it requested, and why it was adopted, adapted, clean-room reimplemented, or rejected.

Authoritative repair: compose the existing Engine assimilation store, Core external-capability/AIPKG validation, governance, Workbench checks, and continuation scheduler. Acceptance is Whole-System Program Phase 5 with one real open-source candidate and rollback/removal proof.

### P1.6 — Reconcile installed, source, and documentation truth automatically

Evidence: `ARDA_SYSTEM_STATUS_REPORT.md:47,119-130` still describes the pre-2026-08-26 truncated executor even though `AUTONOMOUS_TASK_COMPLETION_LOOP.md:122-135` records the installed full-graph proof. Active documents also disagree about whether phone continuity is accepted for the declared personal profile or unsupported in 0.9.

Human-visible behavior: status surfaces name artifact identity and distinguish specified, implemented, root-composed, workflow-proven, operator-accepted, and release-supported without contradictory labels.

Authoritative repair: generate an installed-vs-repository reconciliation report from artifact digests, service commands, endpoints, project registry, active plans, and evidence receipts; make stale active claims fail a documentation/status check. Resolve phone support terminology against one declared profile/matrix.

## P2 — Prove usefulness before embodiment or release expansion

### P2.1 — Run whole-system operator acceptance

After P0 and P1 gates, run one operator-authored objective spanning at least two connected projects, a deterministic tool, local worker, hosted fallback or independent reviewer, restart recovery, failed-check correction, Vairë context return, proactive but bounded update, and verified closure. Continue unattended long enough to prove scheduling and correction. Measure operator prompts, repeated explanations, false alerts, elapsed time, observed cost, and management interventions. The operator—not a test count—decides whether burden was materially reduced.

### P2.2 — Qualify truthful Personal Operations and phone/desktop continuity

Use the same session/objective lineage for capture, resume, approval, defer/dismiss, and completion from phone and native desktop. Exercise loss of phone connectivity without stopping local execution. Verify scoped personal-memory handling, export/deletion boundaries, calm-language policy, and unavailable states. This closes required 1.0 base behavior, not public multi-user support.

### P2.3 — Keep optional expansion gated

Only after P2.1 operator acceptance may the active authority reconsider Ambient Agent Phase 3. Mirromere, RELIC expansion, sensing, device ingestion, payments, public agent accounts, and commercialization each retain their own privacy, authority, deployment, and acceptance gates. None is a workaround for incomplete core composition.

## Dependency order and stop conditions

1. P0.1 and P0.2 establish truthful work and project identity.
2. P0.3, P0.5, and P0.6 establish truthful capability, authority, and review.
3. P0.4 makes the work continue without repeated assignment.
4. P1.1 and P1.2 make the same loop understandable and controllable.
5. P1.3-P1.5 feed real recurring and external work into that loop.
6. P1.6 prevents proof/status drift.
7. P2 validates human usefulness and only then permits downstream expansion decisions.

Stop and request a concrete operator decision when project ownership or mutation scope is unclear, approval is absent/expired, private data would cross an undeclared boundary, cost exceeds the objective budget, review independence cannot be met for material risk, attempts are exhausted, or objective acceptance is genuinely ambiguous. Dirty repositories remain read-only until their contract explicitly protects existing work.

## Backlog completion gate

This backlog is not complete when its code compiles. It is complete when the P2.1 objective closes with native project evidence and the operator confirms that Arda reduced the burden of remembering, assigning, checking, and recovering the work. Until then, the honest whole-system capability truth is: substantial kernel, bounded completion-loop proof, incomplete autonomous personal-agent composition.
