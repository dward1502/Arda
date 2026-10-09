---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "program_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-10-06"
  tags: ["task-loop", "scheduler", "verification", "continuation"]
---

> 🜏 Soterion: 📜 program_plan | owner: PROMETHEUS | status: active | reviewed: 2026-10-06

# Autonomous Task Completion — Gate 1 Acceptance

## Outcome and authority

Gate 1 remains open. One genuine operator-authored objective must enter through
Hermes, span two real projects, schedule and execute bounded work, survive restart
and failure, revise after independent criticism, use Vairë context, and reach
verified closure with explicit operator acceptance. This file owns all five
milestones; their former separate files are consolidated here.

The [cutover](../../archive/2026-09-01-arda-objective-runtime-cutover.md) owns runtime repairs,
retry limits, physical isolation and loss/recovery qualification. Do not duplicate
those implementation tasks here. [Project fabric](../CONNECTED_PROJECT_FABRIC.md)
owns contract review/attachment; [provider convergence](../PROVIDER_WORKER_CONVERGENCE.md)
owns admitted routes. Only the necessary contracts/routes are prerequisites to
this scenario, not completion of every portfolio/provider expansion.

Engine ObjectiveStore owns objective/control/schedule authority; Engine RunStore
owns run/checkpoint receipts; `data/workbench/projects.json` owns project contracts;
Hermes owns conversation; Vairë owns scoped continuity. Operator projections are
read-only. Legacy queue/schedule JSONL is not an intake or recovery fallback.

## Session index

The consolidated acceptance requirements below remain authoritative. Supporting
session plans provide execution detail, not independent acceptance checkboxes.
Status reconciled on 2026-10-08; historical run success does not establish all
requirements in a milestone.

| Milestone | Execution detail | Current evidence boundary |
|-----------|------------------|---------------------------|
| M1 | [Consolidated M1 requirements](#m1--hermes-objective-control) | September 22 four-node run succeeded; M1.1–M1.5 acceptance remains unchecked. The formerly listed standalone M1 file is absent. |
| M2 | [Scheduling and restart](M2_SCHEDULING_AND_RESTART.md) | Implementation/fixture evidence exists; genuine scenario acceptance remains open. |
| M3 | [Critic revision](M3_LIVE_CRITIC_REVISION.md) | Current resident rejection → authenticated correction → fresh independent acceptance qualified October 6, including inherited retry cap and restart/replay preservation. |
| M4 | [Real multi-project execution](M4_REAL_MULTI_PROJECT.md) | Closed for the admitted bounded source review, with concurrency, checks, isolation, joined close, restart preservation and current resident M3 prerequisite qualified. |
| M5 | [Vaire continuity](M5_VAIRE_CONTINUITY.md) | CLI continuity and the October 7 operator verdict are retained; combined-lineage correction/privacy and intervention accounting remain open. |

## Evidence already available

- Bounded installed execute/verify/review/close, paused due scheduling, cancellation,
  same-run interrupted-verifier recovery and terminal reboot reconciliation are
  recorded in the [cutover evidence](../../archive/2026-09-01-arda-objective-runtime-cutover.md#retained-evidence).
  These are local CLI-authorized fixtures, not this combined product scenario.
- Historical loopback controls, live critic rejection/revision and a serial
  two-project joined close remain [bounded evidence](../../audits/autonomous-task-completion-history.md).
  They do not prove current genuine messaging ingress or concurrent real-project use.
- [Resident Workbench source](../../../crates/engine/src/objectives/workbench.rs)
  assembles context and binds outcome receipts. An empty capsule or metadata
  reference is not evidence of relevant memory use.

## Acceptance procedure

Work M1 through M5 on the same useful outcome, retaining exact objective/project/
leaf/run/context lineage. Reuse valid receipts instead of repeating accepted
consequential actions. Checkboxes below are acceptance obligations, not a claim
that their supporting implementation is absent. Store private message and receipt
IDs in canonical/private evidence, not public plan prose.

### M1 — Hermes objective control

**Retained run evidence (2026-09-22; not full M1 acceptance):** Run `m1-gate1-execute-20260922` completed successfully — all 4 nodes succeeded.
- Run `m1-gate1-acceptance-20260922`: plan node `m1-plan` succeeded (seq 4)
- Run `m1-gate1-execute-20260922`: full 4-node graph executed to completion:
  - m1-plan → succeeded (seq 4)
  - m1-approval → succeeded (seq 7, transitioned Pending→Ready→Running→Succeeded)
  - m1-execute → succeeded (seq 10)
  - m1-verify → succeeded (seq 13, with passing test evidence)
- Approval node `m1-approval` initially stuck pending: two POST /v1/runs/approve calls returned empty responses because the idempotency key was already applied by the loop (the function transitions Pending→Ready→Running→Succeeded in one call, and repeated calls with the same key return the current state). Resolved by calling approve endpoint directly — node transitioned to succeeded.
- Daemon port 7878 conflict resolved: old PID 926838 released after 5400s hold; daemon restarted cleanly as PID 995890, 37 projects loaded

- [ ] M1.1 — Receive the operator's genuine objective through the existing authenticated
  messaging channel. Resolve conversational references to canonical records without
  asking the operator to handle IDs; ask one concrete question for ambiguity.
- [ ] M1.2 — Return title, task/status, next action/wake, actual route, budget, evidence,
  blocker and freshness from the canonical projection. Do not rederive queue truth.
- [ ] M1.3 — Exercise pause/resume and reprioritization; show paused work is not claimed
  and the next eligible claim honors priority. Return canonical state after mutation.
- [ ] M1.4 — Exercise pre-execution revision and fresh approval; reject unsafe
  mid-execution revision without silently changing running authority. Confirm
  consequential revision/reject/cancel under policy; record command/session lineage.
- [ ] M1.5 — Cancel a disposable authorized objective and prove no later execution or
  wake. Include reject/approval controls and post-terminal suppression.

Exit gate: normal conversation truthfully inspects and controls one real objective
against the same Engine records. Media ingress and synthetic event-shaped input
alone do not close M1.

### M2 — Installed scheduling and restart

- [ ] M2.1 — Bind the installed binary/configuration to the tested candidate. Run
  immediate, deferred `wait_until` and recurring schedule states through resident
  authority; include pause/resume/cancel and terminal wake suppression.
- [ ] M2.2 — Carry a deferred or recurring continuation of the genuine objective across
  an authorized forced process restart after a durable checkpoint. Retain the same
  run/authority and prove completed execution is not repeated.
- [ ] M2.3 — Cause one declared verification failure; observe bounded retry, correction
  or a justified stop under persisted policy. For successful program acceptance,
  the corrected work passes checks and closes without a manual “continue”.

Exit gate: installed scheduling, interruption and correction succeed unattended
on the real outcome. Historical queue timers, direct CLI simulation and package
tests do not qualify the resident path. Arbitrary interrupted consequential
execution is not made replay-safe by a verification-stage restart proof.

### M3 — Live critic revision

- [x] M3.1 — Bind materially independent implementer/verifier/critic identities and
  admitted provider/model provenance to this resident scenario.
- [x] M3.2 — Demonstrate a real critic rejection with named defects, rejected artifact
  identity and parent receipt, followed by a durable corrected revision and fresh
  verification/independent acceptance. Preserve the historical accepted chain where
  applicable; revalidate the changed resident boundary, not unrelated accepted work.
- [x] M3.3 — Verify synthetic, workerless, stale, cross-run and provenance-free review
  receipts cannot close the objective. A critic that only approves is insufficient.

Exit gate: rejection → named defect → durable revision → corrected artifact →
independent acceptance is traceable under the current authority and attempt budget.

### M4 — Real multi-project execution

- [x] M4.1 — Use two operator-reviewed real project contracts from the fabric owner,
  not `target/arda-real-projects/...` proof roots. Bind exact roots, authority,
  acceptance/check commands, artifacts, budgets and provider constraints to leaves.
- [x] M4.2 — Run safe independent leaves with measured overlapping execution intervals
  (start/end evidence, not receipt timestamps merely occurring within one second).
  Each project produces a useful human-visible result and passes its declared checks.
- [x] M4.3 — Preserve dirty operator work and prove physical-root aliases cannot
  double-mutate. Use separately authorized disposable isolation fixtures; never
  force writes into read-only real projects to manufacture the test.
- [x] M4.4 — Join only validated predecessor receipts after both project contracts pass;
  prove one receipt-backed root close and no completed-sibling replay after restart.

Exit gate: one useful result spans two real projects with overlap, isolated authority,
per-project checks and one joined close. Single-host concurrency is acceptable;
remote-device execution has separate ownership in the provider/personal plans.
The removed M4 draft/payload is historical, not a valid approved intake request.

Current installed evidence (October 6): objective
`operator-objective-c98d83591ddab744` completed both bounded read-only project
reviews and the joined Execute/Verify/Review/Close chain; Review approved the
combined result. Project execution overlapped by 113,887 ms. Both complete report
chains were consumed by the join. Original project receipts and workspace hashes
remained unchanged. The failed join was discarded only under the operator's
explicit development-reset instruction; the admitted project work was not replayed.

Installed restart non-replay and disposable dirty/physical-alias isolation now
pass. The shared Git cleanliness guard is installed in resident admission;
blocked dirty writers do not starve eligible readers. All 24 retained run/outcome
files, three memory outcomes and both project workspaces survive restart unchanged.
The exact declared package checks pass as operator qualification; worker-native
check execution and staging acceptance are not claimed.

**M4 is closed for this admitted bounded scope.** Current resident M3 correction
is now implemented and live-qualified on an authorized disposable helper:
actual semantic rejection → immutable authenticated correction → fresh approval
and execution → compilation verification → independent acceptance → terminal
close. The original M4 project results were not replayed. M3 used four inherited
attempts out of five and survived root restart and exact-event replay without
new execution or duplicate memory. Evidence:
`target/qualification/m3-critic-20261006/manifest.json`.

The M3/M4 checkmarks record these current resident qualifications. Gate 1 still
requires the genuine combined messaging outcome and the remaining M1/M2/M5
obligations; a disposable fixture does not supply that scenario or the operator's
whole-system reduced-burden verdict. The October 7 bounded M5 CLI verdict is
retained below; remaining combined-lineage obligations do not reopen it.
The [M4 owner](M4_REAL_MULTI_PROJECT.md) records scope and validation;
`target/qualification/m4-closeout-20261006/manifest.json` indexes the new evidence.
Mesh-llm remains off; routing is profile-selected, not model-specific code.

The September retained-recovery incident is historical after the authorized
October development reset. Its expired grant is not the current execution blocker.
See the [cutover C3.2 record](../../archive/2026-09-01-arda-objective-runtime-cutover.md)
for that incident; it does not replace today's evidence or remaining acceptance gates.

### M5 — Vaire continuity and operator acceptance

- [ ] M5.1 — Retrieve relevant authorized Vairë context actually consumed by planning/
  execution; retain context-use references, scope and evidence of its influence.
- [ ] M5.2 — Bind results, failures, corrections and unresolved follow-up to the same
  lineage. Test correction/revocation/privacy and no duplicated Vairë outcomes on replay.
- [ ] M5.3 — Resume in a new authenticated Hermes session after daemon restart without
  making the operator reconstruct context. Inspect status and correct a decision.
- [ ] M5.4 — Record required policy decisions separately from avoidable status/continue/
  context-restatement prompts, with attempts, budget and elapsed-use evidence.
- [x] M5.5 — Present the verified result and obtain the operator's explicit reduced-burden
  verdict or named defects. Defects return to their owning milestone.

October 7: the operator explicitly accepted the demonstrated local CLI workflow
and requested forward progress. The [M5 verdict](M5_VAIRE_CONTINUITY.md#operator-verdict--october-7-2026)
records the bounded scope; this does not close the outstanding combined-scenario
M1/M2/G1 obligations. R05 execution-boundary work proceeds without replaying
completed projects or asking for the same verdict again.

Exit gate: the operator accepts that the complete loop materially reduces management
burden. An agent summary, successful build, health response or empty memory baseline
cannot supply that verdict.

## Program closure

- [ ] G1 — M1–M5 acceptance is linked to one genuine operator-authored outcome, the
  cutover's relevant safety requirements pass, declared checks/artifacts pass, and
  the operator accepts the result. Record evidence scope and remaining limitations.

Then update the [whole-system gate](../ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md#gate-1--complete-the-autonomous-loop)
and retire this plan from the active queue. Gate 1 does not close broader personal
usefulness or authorize deferred embodiment. Detailed receipts belong in the
[evidence index](../../audits/autonomous-task-completion-history.md), not new plans.
