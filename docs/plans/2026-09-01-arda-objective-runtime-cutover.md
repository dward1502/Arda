---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-10"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-10

# Resident Objective Runtime — Remaining Cutover

## Outcome and current blocker

The installed `arda` daemon must own durable objective control, scheduling,
bounded concurrent execution, same-run recovery and receipt-backed closure.
Engine ObjectiveStore owns transactional SQLite state; RunStore owns execution
evidence. Hermes/Oromë own authenticated ingress, Vairë context/outcomes, and
Aulë execution mechanisms/observability. No second objective scheduler.

The cutover is NOT complete. Its critical path is now retained-snapshot
integration: admission and execution must use the same approved mount tree.
The original real adapter regression
`mount_change_after_command_construction_cannot_redirect_writes` remains RED.
Standalone worker tests do not repair the production adapter.

This plan previously accumulated checkpoint reports and superseded statements.
Those are preserved in [historical evidence](../audits/2026-09-10-objective-cutover-checkpoint-history.md),
not repeated here. Update the status rows below in place. Do not append another
chronological gate report or treat each bounded review as a stopping point.

## Verified foundations to preserve

These are source/fixture results, not installed acceptance of the current tree.
Historical test counts, exact hashes and review limitations are in the evidence
record above; they must not be reused as qualification of a later candidate.

| Area | Current source state | Remaining boundary |
| --- | --- | --- |
| Objective control/recovery | Stable run identity, original context binding, receipt-only reconciliation, persisted attempts, stage response-loss recovery and completed-sibling durability | Installed same-run interruption/recovery; no provider or Vairë replay |
| Scheduling | Committed-store notifications, due wakes, recurring unfinished work, quarantine of malformed schedules | Installed due/pause/cancel behavior and quarantine diagnostics |
| Supervision | Retained stop, bounded resident drain, Harness/provider cancellation and child joining | Exact installed candidate stop/start and stalled-response bounds |
| Readiness | Scheduler-local phase/activity/wake/error/pending-recovery projection | Provider prerequisites, schedule errors and whole-startup qualification |
| Legacy replay/resource regression | Installed operator/next-action readers use SQLite summaries; retired CLI replay rejects immediately; autopilot timers disabled; daemon measured at about 18 MiB RSS ([evidence](../audits/2026-09-13-legacy-queue-resource-repair.md)) | Combined game/YouTube/work observation; remaining producer audit |
| Physical admission | Existing exclusion/policy preserved; new pre/post-clone admission identity and staged-root check | New normalization review; original nested-mount race through Harness |
| Snapshot store/keeper | Real bounded client, independent owner, preparation journal, release ACK persistence, restart-loss rejection exercised by [keeper fixture](../../crates/engine/tests/fixtures/keeper_adapter.rs) | Ownership review, orphan reconciliation, daemon configuration and crash-boundary injection |
| Snapshot worker/adapter | Retained chat/export/artifact validation, fencing, bounded cancellation; reviewed receipt replay without live lease | Full Harness dispatch fixture, production runtime grants and installed recovery |

Source entry points:
[store](../../crates/engine/src/objectives/store.rs),
[snapshot interface](../../crates/engine/src/objectives/snapshots.rs),
[runtime](../../crates/engine/src/objectives/runtime.rs),
[Workbench](../../crates/engine/src/objectives/workbench.rs),
[adapter](../../crates/engine/src/adapters/hermes.rs),
[worker](../../crates/engine/src/bin/arda-snapshot-worker.rs),
[daemon](../../src/main.rs).

## Remaining execution order

### 0. Remove legacy replay from ordinary background operation

This resource defect is part of the cutover, not a new queue-optimization project.
The workstation must remain usable alongside Total War: Warhammer III, YouTube
and development; spare RAM at an idle desktop is not acceptance.

- [x] Remove legacy queue/schedule reads from live operator and next-action
  projections. Query current ObjectiveStore summaries read-only, without schema
  migration, historical replay, execution prompts or recovery capsules; absent
  authority has no legacy fallback. [Regression coverage](../../crates/engine/tests/retired_queue_projection.rs).
- [x] Reject `autopilot once/run/status` before world/queue hydration, disable
  installed replay timers, and prevent the automation installer from re-enabling
  them. [CLI regression](../../crates/spine/observability/arda-aule/tests/retired_autopilot_cli.rs)
  and [installer regression](../../scripts/test_install_arda_automation_units.sh).
- [x] Independently review and install the exact repair candidate with rollback;
  measure daemon RSS, service anonymous/cache accounting and read volume over
  repeated projection ticks; verify unchanged legacy file size/mtime.
  [Installed measurement](../audits/2026-09-13-legacy-queue-resource-repair.md).
- [ ] Confirm background operation remains acceptable during the operator's
  combined game/YouTube/work workload. Idle-memory proof does not establish frame pacing.
- [x] Remove the oversized legacy blobs from the four unpublished commits with
  explicit operator authorization, preserving all other committed paths and local
  queue/schedule files. A local rollback branch retains the original history.

### 1. Complete the retained execution path

- [ ] Finish review/hardening of the implemented independent keeper and bounded
  client; wire daemon reconnection without owning the keeper's lifetime.
- [ ] Review new manifest/admission identity agreement and durable preparation
  journal; prove crash-boundary behavior and expose explicit orphan reconciliation.
- [ ] Wire daemon ObjectiveStore to the tested keeper client. Crash/ACK-loss recovery must reuse the
  saved capability and immutable generation/owner/expiry intent, never prepare
  from current paths for an admitted run.
- [ ] Route same-run recovery through that capability without weakening fresh
  admission overlap checks. Rebind must fence older executions before ACK.
- [ ] Qualify explicit Hermes executable/runtime/profile grants. The current
  worker grants only `/usr` and workspace, rejects `HERMES_HOME`, and requires
  private scratch on a filesystem separate from workspace and `/usr`. It is
  not yet compatible with the installed Hermes runtime.
- [x] Route chat/export and artifact checks through retained execution; bounded
  cancellation and expired/released receipt replay have corrective review and
  [regression coverage](../../crates/engine/tests/fixtures/retained_replay.rs).
- [ ] Extend tested durable owner release/restart behavior with crash/ACK-loss
  injection and explicit reconciliation. Never infer release from socket absence. Reconcile orphans and
  uncertain cleanup without recursive deletion through workspace mounts.
- [ ] Run the original adapter mount-race regression through the production
  route and make it GREEN. Cover admission rollback, ACK loss, daemon death,
  lease expiry/rebind, active disconnect, keeper loss and terminal release.
- [ ] Independent integration review of the resulting path; repair concrete
  blockers. Do not repeat accepted identity/bootstrap/cleanup reviews unchanged.

Keeper loss or reboot must fail closed and require explicit reconciliation.
Never silently rebuild an admitted snapshot from current paths. Retained mounts
are not immutable file contents: descendant renames/hard links remain distinct.
Filesystem containment does not govern remote tools or host-service APIs.

### 2. Qualify the deployment candidate

- [ ] Classify installed autopilot/research/other legacy writers and test emissions.
  Freeze retired JSONL admission/control/schedule/continuation authority while
  preserving historical ledgers and compatibility readers. Do not migrate history.
- [ ] Verify canonical authenticated Engine intake/control, changed-payload replay
  rejection, store-loss fail-closed behavior and notification after accepted changes.
- [ ] Expose schedule quarantine and actual provider/keeper prerequisites without
  relabeling `/health` or scheduler-local idle as whole-system readiness.
- [ ] Establish service ownership, runtime grants, namespace support, active work,
  exact binaries/configuration and rollback before touching live services/profiles.
  Keep the snapshot keeper independent of `arda.service` restart/stop ownership.
- [ ] Build and independently review the exact daemon/CLI/keeper candidate;
  bind source, candidate and installed identities. Preserve unrelated worktree
  edits and runtime data. No commits, pushes or coding delegation without request.

### 3. Installed acceptance and retirement

- [ ] Guarded bounded shutdown: stop admission, drain/cancel owned work, verify
  exact old process disappearance, then start once and verify actual readiness.
- [ ] Authorized installed physical admission and original-tree execution.
- [ ] Daemon interruption/restart retains the same run, context and capability;
  completed sibling receipts and provider/Vairë effects are not replayed.
- [ ] Due schedules wake; pause/cancel/terminal state suppresses dispatch;
  sibling isolation and canonical control/projection behavior are observed.
- [ ] One receipt-backed terminal close, durable release ACK recovery and no
  post-close execution. Keeper loss/reboot requires explicit reconciliation.
- [ ] Reconcile milestone evidence, rerun final gates after the last semantic
  edit, and remove this file from the active plan queue only when acceptance passes.

Do not reset live objectives or repeat accepted consequential actions to manufacture
acceptance. Genuine operator-authored scenarios remain distinct from fixtures.

## Invariants and verification

Preserve exact project/workspace/authority/budget/approval/predecessor bindings,
cycle/revision guards, WAL/foreign keys/compatible migrations, and the persisted
`MAX_OBJECTIVE_ATTEMPTS=5` limit. Retain complete recovery context until explicit
authenticated operator deletion, without automatic expiry. Logical deletion must
not be described as erasure of Vairë, RunStore, backups or SQLite remnants.

Use focused RED/GREEN tests and explicitly enable namespace regressions. Then run
Engine/root-daemon tests, affected Vairë/Aulë suites, strict all-target Clippy,
workspace check and exact release daemon/CLI/keeper builds. Preserve intermittent
failures rather than silently reporting only reruns. Run the existing Rúmil
Markdown link/completion-language checker after plan edits.

This is a dependency of the [autonomous milestones](autonomous-task-completion/README.md),
not another acceptance program. [Provider convergence](PROVIDER_WORKER_CONVERGENCE.md)
owns provider transport; [Milestone 4](autonomous-task-completion/04-real-multi-project-execution.md)
owns useful real-project overlap; [Milestone 5](autonomous-task-completion/05-vaire-operator-acceptance.md)
owns continuity/operator burden. Keep their wider acceptance gates intact.
