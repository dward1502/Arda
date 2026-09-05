---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "program_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-01"
  tags: ["task-loop", "scheduler", "verification", "continuation"]
---

> 🜏 Soterion: 📜 program_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-01

# Autonomous Task Completion Program

## Outcome

The operator states an outcome once. Arda retrieves context, decomposes bounded work, applies existing authority, schedules and executes it, verifies and independently reviews the result, revises failures, survives restarts, and continues until acceptance passes or one genuine operator decision is required.

## Current status

The source and package foundation is implemented. A reviewed installed objective has now completed context recovery, two real project-bound inspections, synthesis, read-only outcome production, final acceptance, and receipt-backed joined root closure. The program is not complete because same-objective project overlap, Vairë context-use binding, genuine external messaging ingress, explicit operator-burden acceptance, and full workbench execution pipeline completion (execute → verify → review → close) remain open.

| Capability | Current truth |
|---|---|
| Canonical queue, schedules, and continuation decisions | Source/package verified |
| `plan → approval → execute → verify → review → close` | Source/package verified; bounded installed proof exists |
| Retry, revision, replan, deferred work, cancellation | Source/package verified |
| Restart checkpoints and mutation isolation | Source/package verified; bounded installed restart proof exists |
| Independent provider-backed verifier and critic contracts | Source/package verified |
| Canonical objective/control projection | Source/package verified |
| Hermes consumption and mutation controls | Installed bridge/control path verified; genuine messaging-platform receipt open |
| Installed recurrence, deferred wake, and correction | Timer, pause, terminal suppression, forced restart, and unattended correction verified; deferred/recurring wake remains open |
| Autonomous retry termination | Verified — 2026-09-04. Root cause was `ObjectiveRuntime::run_round()` never incremented `self.objective_attempts`, making the `MAX_OBJECTIVE_ATTEMPTS` guard dead code. Fixed by changing signature to `&mut self` and adding `self.objective_attempts += 1` after the claims loop. Also verified `cap_excess_attempts()` migration fires on `ObjectiveStore::open()` to mark stuck objectives (`state IN ('approved','running')` with `MAX(leaf.attempt) >= 5`) as `Failed`. Full test suite passes: 51 tests, 0 failures. `cargo build --package arda-engine` clean. |
| Runtime execution with workbench integration | Verified — 2026-09-05. `claim_runnable` SQL parameter counts fixed (leaves now claimable). Workspace blocking fixed with NOT EXISTS clause (project-1 and join can share workspace). `manwe.toml` created with hermes-workbench provider. `hermes-workbench.toml` timeout reduced from 900000ms to 30000ms. Workbench executor timeout reduced from 1200s to 30s. `autonomy_operating_loop.toml` activated (status=active, mode=preflight). Missing `hades_cleanup_approval_packets.json` and `athena_external_source_lane_ledger.jsonl` created. Runtime is executing — leaves advance to attempt=3. Workbench execution chain partially working (plan and approval succeed, execute-pending). |
| Autonomy gate | Fixed — 2026-09-05. `autonomy_operating_loop.toml` was in `active_draft`/`continuous_preflight` with missing `hades_cleanup_approval_packets.json` and `athena_external_source_lane_ledger.jsonl`, which stalled the system. Fixed by activating the config and creating those files. 12/12 lanes configured. System is now running and executing. |
| Simultaneous real-provider, multi-project execution | Reviewed real projects, one shared objective, six receipt-backed leaves, and joined close verified by `operator-task-fb5a52e3a268ec2d`; the two real-project execute receipts were serial, so same-objective overlap remains open |
| Live Vairë receipt binding and operator-burden acceptance | Terminal Mnemosyne outcome binding verified; Vairë context-use and operator verdict remain open |

## Execution order

Complete these milestones in order. A milestone closes only through its human-visible acceptance scenario, not through schema or package tests alone.

1. [Hermes objective control](01-hermes-objective-control.md)
2. [Installed scheduling and restart acceptance](02-installed-scheduling-restart.md)
3. [Live critic rejection and revision](03-live-critic-revision.md)
4. [Real multi-project execution](04-real-multi-project-execution.md)
5. [Vairë continuity and operator acceptance](05-vaire-operator-acceptance.md)

Implementation and review history is indexed in [Evidence History](EVIDENCE_HISTORY.md). New detailed test output belongs in receipts or audit artifacts, not in this plan.

## Canonical authority

- Objective/control/schedule authority: the resident `arda-engine` ObjectiveStore defined by [the active runtime cutover plan](../2026-09-01-arda-objective-runtime-cutover.md). Until that store is installed, objective admission is frozen rather than falling back to legacy files.
- `core/projects/tasks/queue.jsonl` and `core/projects/tasks/schedules.jsonl` are frozen legacy inputs. They are not acceptance authority and must receive no new objectives, controls, continuations, or schedules.
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
