# Arda Objective Runtime Cutover Plan

Goal: Retire the legacy global JSONL task queue and make the installed `arda` daemon the durable owner of objective control, scheduling, concurrent execution, restart recovery, and receipt-backed closure.

Architecture: Add an indexed transactional objective store to `arda-engine`, start one persistent objective runtime inside the existing `arda` daemon, and keep Engine `RunStore` receipts as immutable execution evidence. Hermes ingress and operator projections use the same Engine API; `arda-aule` supplies bounded execution mechanisms but no longer owns scheduling authority.

Tech stack: Rust, Tokio, SQLite in WAL mode through `rusqlite`, existing Engine `RunStore`, existing authenticated Harness ingress, existing `arda.hermes_execution_receipt.v4` receipts.

## Implementation status — 2026-09-02

Implemented and installed. The resident daemon owns authenticated objective
creation, controls, indexed scheduling, leases, dependency release, Workbench
execution, canonical receipt persistence, terminal closure, and restart
recovery. The dependency contract is verified at each production boundary:
the store claims a join with every completed predecessor close receipt, the
adapter preserves those references unchanged, and Workbench loads and
digest-validates the durable receipt payloads before independent review.

This cutover is accepted by those general runtime invariants, not by teaching
one provider to satisfy a synthetic project prompt. Milestone 4's remaining
human-visible real-project acceptance is tracked separately below and does not
reopen queue authority.

## Scope decision

The old queue is not migrated. `core/projects/tasks/queue.jsonl` and `core/projects/tasks/schedules.jsonl` are frozen legacy artifacts and are not runtime authority. Queue-specific systemd units are deleted; historical ledgers, receipts, and compatibility readers are retained until a separate archive cleanup can remove them without discarding operator evidence. Git history remains the final historical archive.

Canonical `data/runs/**` execution receipts, cited audit evidence, and Vairë memories are retained. The interrupted legacy objective `operator-task-020fddb7c36065cf` is not promoted into the new store; Milestone 4 uses one fresh authenticated objective created after cutover.

## Target ownership

- `arda` daemon: sole local lifecycle owner of the objective runtime.
- `arda-engine`: canonical objective, leaf, dependency, lease, schedule, control, and terminal-root state.
- Engine `RunStore`: immutable run checkpoints and execute/verify/review receipts.
- `arda-aule`: execution adapter and observability only; no canonical queue or scheduler authority.
- Hermes/Oromë: authenticated conversational ingress and controls through the Harness.
- Vairë: context-use and terminal outcome continuity.
- `core/state/*.json`: read-only projections only.

## Canonical store

Create `data/arda/objectives.sqlite3` with WAL journaling, foreign keys, transactions, and schema migrations owned by `arda-engine`.

Required indexed tables:

1. `objectives`: identity, authenticated source, operator, text, lifecycle state, revision, priority, approval binding, timestamps, terminal receipt digest.
2. `objective_projects`: exact reviewed project authorities and project contract digests.
3. `leaves`: leaf identity, objective, project, workspace, authority class, lifecycle stage, attempt, lease owner/expiry, budget, current run ID, terminal receipt digest.
4. `leaf_dependencies`: explicit DAG edges.
5. `schedules`: next wake, recurrence, pause/cancel state, and idempotency key.
6. `stage_receipts`: stage, canonical `arda.hermes_execution_receipt.v4` digest, RunStore path, provider/model, start/end timestamps, and verdict.
7. `control_actions`: authenticated pause/resume/reprioritize/revise/approve/cancel decisions with single-use idempotency keys.

Every hot-path query is indexed by lifecycle state, wake time, objective ID, project ID, workspace root, and lease expiry. Runtime selection must never deserialize or scan historical receipts.

## Phase 1 — Freeze and specify the replacement

Files:
- Modify: `docs/plans/autonomous-task-completion/README.md`
- Modify: `docs/plans/autonomous-task-completion/04-real-multi-project-execution.md`
- Modify: `docs/plans/autonomous-task-completion/EVIDENCE_HISTORY.md`
- Modify: `docs/audits/2026-08-30-autonomous-loop-installed-acceptance.md`

Steps:
1. Record that the JSONL queue is legacy and must not receive new objectives.
2. Mark the current Milestone 4 legacy attempt abandoned by architecture cutover, not failed by a provider.
3. Preserve cited RunStore receipts and explicitly exclude queue records from future acceptance authority.
4. Keep `arda-workbench-queue-executor.timer` disabled during implementation.

Gate: No installed producer may append a new task, control action, continuation, or schedule to the legacy files.

## Phase 2 — Build the Engine objective store with TDD

Files:
- Create: `crates/engine/src/objectives/mod.rs`
- Create: `crates/engine/src/objectives/store.rs`
- Create: `crates/engine/src/objectives/model.rs`
- Create: `crates/engine/src/objectives/migrations.rs`
- Create: `crates/engine/tests/objective_store.rs`
- Modify: `crates/engine/src/lib.rs`
- Modify: `crates/engine/Cargo.toml`
- Modify: root `Cargo.toml`

TDD cases:
1. Atomic authenticated objective creation with two project authorities and dependency-bound leaves.
2. Duplicate ingress idempotency returns the exact existing objective.
3. Transactional claim selects only runnable leaves and records a lease without scanning terminal history.
4. Distinct safe project roots can be leased concurrently; the same physical root cannot.
5. Expired leases recover after daemon restart without duplicating a completed stage.
6. Stage transitions require the exact prior canonical receipt digest.
7. Revision invalidates prior approval and requires a new authenticated approval.
8. Terminal root can close only when every required leaf carries canonical close evidence.

Gate commands:
- `cargo test -p arda-engine --test objective_store`
- `cargo test -p arda-engine`

## Phase 3 — Separate explicit execution from legacy queue claiming

Files:
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs`
- Modify: `crates/spine/observability/arda-aule/src/prometheus/autopilot/mod.rs`
- Add focused executor tests beside the implementation.

Steps:
1. Extract an explicit `ExecutionWorkItem` input containing objective, leaf, project, workspace, authority, run, attempt, and expected predecessor receipt.
2. Make the executor run exactly that supplied work item through execute, verify, independent review, and close.
3. Return canonical stage receipts to the caller; do not append queue records or select another task internally.
4. Remove temporary `m4-trace` diagnostics.
5. Retain structured `join_all`/joined concurrency; no detached provider tasks.

Gate: A supplied work item completes and returns exact receipt lineage without reading `queue.jsonl`.

## Phase 4 — Run objective supervision inside `arda`

Files:
- Create: `crates/engine/src/objectives/runtime.rs`
- Create: `crates/engine/tests/objective_runtime.rs`
- Modify: `src/main.rs`
- Modify: `crates/engine/src/harness.rs`

Runtime behavior:
1. Start one `ObjectiveRuntime` alongside the Harness and process supervisor.
2. Wake through `tokio::sync::Notify` on ingress/control changes and through a bounded timer for scheduled/lease recovery.
3. Transactionally claim up to configured capacity.
4. Spawn joined, tracked leaf tasks in a `JoinSet`; persist every stage receipt before advancing.
5. Apply exact project/workspace mutation exclusion while allowing safe distinct-project reads to overlap.
6. On shutdown, stop claiming, allow a bounded drain, release or expire leases, and preserve resumable stage state.
7. On startup, reconcile leases and RunStore receipts before accepting new execution.
8. Report readiness, active leaves, wake time, and last reconciliation error through Harness health/status.

Gate: Killing and restarting `arda.service` during a leaf resumes from the last canonical stage without duplicate provider execution or root closure.

## Phase 5 — Rewire authenticated ingress and projections

Files:
- Modify: `crates/engine/src/harness/operator_messages.rs`
- Modify: `crates/engine/src/operator_projection.rs`
- Modify: `crates/engine/src/next_action.rs`
- Modify: corresponding Engine harness/projection tests.

Steps:
1. Replace `ActiveQueueExecutor`, `TaskQueueAnalyzer`, and `ScheduleLedger` calls with `ObjectiveStore` transactions.
2. Submit, inspect, pause, resume, reprioritize, revise, approve, reject, and cancel the same canonical objective rows.
3. Notify the resident runtime after mutations.
4. Build operator and next-action projections from ObjectiveStore plus RunStore.
5. Fail closed if the store is unavailable; never fall back to JSONL.

Gate: Repo-wide Engine source has no operational dependency on `core/projects/tasks/queue.jsonl` or `schedules.jsonl`.

## Phase 6 — Installed runtime acceptance

Steps:
1. Build and install source-current `arda` and `arda-cli`; verify source/install SHA-256 equality.
2. Restart `arda.service`; confirm the resident objective runtime is ready.
3. Prove distinct project leaves can execute concurrently while a dependent join remains blocked.
4. Prove completed predecessor close receipts cross store claim, adapter, and Workbench review boundaries without payload or digest substitution.
5. Prove exact execute/verify/review/close lineage under `arda.hermes_execution_receipt.v4` and receipt-backed terminal-root closure.
6. Restart the daemon and prove expired work is reclaimed while completed stages and terminal roots are not duplicated.
7. Verify the installed service remains healthy and no legacy queue executor unit is loaded.

These are production invariants covered by deterministic store/runtime/adapter
tests plus installed daemon health and ownership probes. A provider response to
one hand-shaped fixture is not an architectural gate. Milestone 4 still owns
the separate human-visible real-project outcome and measured overlap gate.

## Phase 7 — Delete the legacy subsystem

Files/directories retired in this cutover:
- `config/systemd/arda-workbench-queue-executor.service`
- `config/systemd/arda-workbench-queue-executor.timer`
- installer activation of the queue executor; successful installation now
  disables and removes stale installed copies transactionally.

Deferred archive cleanup:
- frozen queue/schedule ledgers and their historical receipts;
- compatibility analyzers/CLI readers still covered by supported tests;
- generated queue projections and audit artifacts whose provenance must be
  retained until consumers are removed explicitly.

System actions:
1. Stop and disable the queue timer/service.
2. Remove installed unit files and run `systemctl --user daemon-reload`.
3. Preserve historical runtime files until their remaining consumers are retired and evidence retention is decided.
4. Verify no running process has a legacy queue file open.
5. Verify repo-wide operational references are zero; historical audit prose may remain explicitly labeled legacy.

Gate: `arda.service` alone owns objective execution. No queue timer or one-shot executor is installed or loaded, and JSONL state has no objective authority.

## Phase 8 — Final verification, review, and closeout

Commands:
- `cargo test -p arda-aule --features full-cli`
- `cargo test -p arda-engine`
- `cargo clippy -p arda-aule --features full-cli --all-targets -- -D warnings`
- `cargo clippy -p arda-engine --all-targets -- -D warnings`
- `cargo build --release -p arda --bin arda`
- `cargo build --release -p arda-aule --features full-cli --bin arda-cli`
- installed health, restart, replay, projection, and operator-control checks
- documentation-link and plan-state validation

Closeout:
1. Update the four Milestone 4 authority/evidence documents with installed evidence.
2. Produce the exact intended diff and SHA-256.
3. Obtain independent approval of those exact bytes/hash.
4. Stage only approved source/docs/deletions through an isolated index.
5. Exclude live SQLite data, RunStore output, projections, installed binaries, rollback artifacts, and unrelated dirty files.
6. Create one focused local commit; do not push.

## Tie to Milestone 5

Milestone 5 begins only after the new runtime closes Milestone 4. It does not build another scheduler. It exercises the same resident objective store and runtime across:

- a new Hermes session;
- an `arda.service` restart;
- Vairë context-use before execution;
- Vairë terminal outcome binding;
- correction without reconstructing prior context;
- explicit operator burden acceptance.

The automation build-out closes when Milestone 5 proves continuity and the operator accepts the reduction in management burden. Any defect in scheduling, replay, concurrency, or receipts reopens Milestone 4; any defect limited to memory/session continuity remains in Milestone 5.

## Non-goals

- No migration of the 82k legacy queue records.
- No second daemon or replacement one-shot worker.
- No synthetic acceptance fixture.
- No remote/fleet scheduler expansion.
- No redesign of provider routing, project contracts, or Vairë beyond what Milestones 4 and 5 require.
