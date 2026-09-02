---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-01"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-01

# Milestone 4 — Real Multi-Project Execution

## Status

Partial. The resident ObjectiveRuntime cutover is implemented, installed, and owns objective execution without the legacy queue executor. Deterministic production-boundary tests prove two distinct project leaves execute concurrently, their dependent join remains blocked until both close, both canonical predecessor receipt payloads reach Workbench review with digest validation, terminal-root closure is receipt-backed, and restart recovery does not duplicate completed stages. These runtime invariants do not close Milestone 4 by themselves: the remaining gate is one useful human-visible outcome across two real registered projects with measured overlap, not a provider-specific synthetic prompt.

## Runtime cutover boundary — 2026-09-01

- `core/projects/tasks/queue.jsonl` and `core/projects/tasks/schedules.jsonl` are frozen legacy inputs and excluded from future acceptance authority.
- The queue-executor service/timer templates and installed copies are removed. `arda.service` is the sole objective execution owner.
- Existing canonical Engine RunStore receipts remain immutable evidence. The legacy queue history itself will not be migrated into the indexed ObjectiveStore.
- The active implementation and destructive-retirement sequence is [Arda Objective Runtime Cutover](../2026-09-01-arda-objective-runtime-cutover.md).

## Resident-runtime evidence — 2026-09-02

- `ObjectiveStore` returns a claimed join only with close receipts for every completed dependency.
- `WorkbenchExecutionAdapter` preserves those receipt references, and the explicit Workbench path loads and digest-validates their canonical payloads before review.
- `objective_runtime` integration coverage proves two active distinct-project leaves, dependency ordering, join completion, terminal-root closure, expired-lease recovery, and no duplicate stages after restart.
- The source-current release binary was installed; `arda.service` remained active and `/health` returned `ok` after restart.
- Both retired queue-executor units report `LoadState=not-found` and `ActiveState=inactive` after daemon reload.
- No synthetic provider response is treated as acceptance evidence.

## Installed evidence — 2026-09-01

- Project 1 leaf `operator-task-fb5a52e3a268ec2d__inspect-authorities-project-1--c0bd75351425b508` remained bound to `b22c0000-e29b-41d4-a716-446655440002`; execute, verify, and independent review receipts all report `succeeded`.
- Project 2 leaf `operator-task-fb5a52e3a268ec2d__inspect-authorities-project-2--a3d067604f2755dd` remained bound to `c33d0000-e29b-41d4-a716-446655440003`; execute, verify, and independent review receipts all report `succeeded`.
- All six terminal leaf receipt digests are carried in `closure_evidence_receipts`. The root acceptance artifact is the genuine final critic receipt at `data/runs/queue-operator-task-fb5a52e3a268ec2d__verify-acceptance--f2c11fab72358530/execution-receipts/review.json`, not operator-supplied acceptance metadata.
- The installed root reconciled to `arda.workbench.objective_terminal.v1`, `completed`, `close_complete`. Source regression coverage now prevents replay from appending additional terminal-root records; the accepted pre-fix run retained six identical terminal records, so exact-once replay is source/deployed verified but not retroactively rewritten.
- The project execute receipt timestamps were `1788259095613` and `1788259726188` milliseconds. They do not overlap; installed same-objective overlap remains open.

## Human-visible result

One objective coordinates useful work across two real registered projects. Independent work overlaps when safe, mutations remain isolated, and each project’s checks and evidence converge into one objective close.

## Work

1. Replace proof-only production project entries with two reviewed real project contracts.
2. Bind each leaf to its exact project, workspace root, authority class, checks, budget, and provider constraints.
3. Run independent read-only or distinct-workspace work concurrently through real admitted providers.
4. Serialize mutations targeting the same physical root, including aliases.
5. Preserve existing dirty operator work; fresh mutation on a dirty root must remain blocked while read-only inspection is allowed.
6. Join per-project evidence only through the shared objective lineage.

## Acceptance scenario

A single operator objective creates dependent leaves in two distinct real projects. At least two safe independent leaves overlap measurably. Each project produces a reversible human-visible result and passes its own declared checks. A forced same-root conflict is deferred rather than double-mutated. The objective closes only when both project acceptance contracts pass.

## Exit gate

One useful human-visible result must be observed across two real registered projects. Canonical receipts and timestamps must prove overlap, mutation isolation, project checks, and one evidence-backed objective close. Runtime contract tests establish the substrate but do not substitute for this product acceptance.
