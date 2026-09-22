---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "archived"
  reviewed: "2026-09-10"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: archived | reviewed: 2026-09-10

# Resident Objective Runtime — Remaining Cutover

## Outcome and current blocker

The installed `arda` daemon must own durable objective control, scheduling,
bounded concurrent execution, same-run recovery and receipt-backed closure.
Engine ObjectiveStore owns transactional SQLite state; RunStore owns execution
evidence. Hermes/Oromë own authenticated ingress, Vairë context/outcomes, and
Aulë execution mechanisms/observability. No second objective scheduler.

The cutover is NOT complete. The current dirty candidate passes the explicitly
enabled original adapter regression
`mount_change_after_command_construction_cannot_redirect_writes`; the earlier RED
result on merged baseline `a1c4cd368c0b79f38be898c22271ab5b71d328ec` is historical,
not the candidate's current result. Production mount capture and retained runtime
grants are installed. Bounded installed execution, interrupted-verifier recovery,
release-ACK loss and explicit post-reboot terminal reconciliation have evidence
below. The daemon and independent keeper match the qualified candidate; runtime
readiness is restored. Wider recovery/hardening review, producer classification,
genuine operator/multi-project/Vairë acceptance and combined background-workload
acceptance remain open. Component and bounded fixture results do not close those
whole-system requirements.

This plan previously accumulated checkpoint reports and superseded statements.
Those are preserved in [historical evidence](../../audits/2026-09-10-objective-cutover-checkpoint-history.md),
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
| Legacy replay/resource regression | Installed operator/next-action readers use SQLite summaries; retired CLI replay rejects immediately; autopilot timers disabled; daemon measured at about 18 MiB RSS ([evidence](../../audits/2026-09-13-legacy-queue-resource-repair.md)) | Combined game/YouTube/work observation; remaining producer audit |
| Physical admission | Retained-only v3 physical-tree digest binds store/envelope/worker while tolerating unrelated namespace mounts; strict v2 behavior preserved; focused independent review found no blocker ([identity tests](../../../crates/engine/tests/retained_tree_identity.rs)) | Original nested-mount race through Harness; exact installed namespace qualification |
| Snapshot store/keeper | Real bounded client, independent owner, preparation journal, release ACK persistence, restart-loss rejection exercised by [keeper fixture](../../../crates/engine/tests/fixtures/keeper_adapter.rs) | Ownership review, orphan reconciliation, daemon configuration and crash-boundary injection |
| Snapshot worker/adapter | Retained chat/export/artifact validation, fencing, bounded cancellation; reviewed receipt replay without live lease | Full Harness dispatch fixture, production runtime grants and installed recovery |

Source entry points:
[store](../../../crates/engine/src/objectives/store.rs),
[snapshot interface](../../../crates/engine/src/objectives/snapshots.rs),
[runtime](../../../crates/engine/src/objectives/runtime.rs),
[Workbench](../../../crates/engine/src/objectives/workbench.rs),
[adapter](../../../crates/engine/src/adapters/hermes.rs),
[worker](../../../crates/engine/src/bin/arda-snapshot-worker.rs),
[daemon](../../../src/main.rs).

## Remaining execution order

### 0. Remove legacy replay from ordinary background operation

This resource defect is part of the cutover, not a new queue-optimization project.
The workstation must remain usable alongside Total War: Warhammer III, YouTube
and development; spare RAM at an idle desktop is not acceptance.

- [x] Remove legacy queue/schedule reads from live operator and next-action
  projections. Query current ObjectiveStore summaries read-only, without schema
  migration, historical replay, execution prompts or recovery capsules; absent
  authority has no legacy fallback. [Regression coverage](../../../crates/engine/tests/retired_queue_projection.rs).
- [x] Reject `autopilot once/run/status` before world/queue hydration, disable
  installed replay timers, and prevent the automation installer from re-enabling
  them. [CLI regression](../../../crates/spine/observability/arda-aule/tests/retired_autopilot_cli.rs)
  and [installer regression](../../../scripts/test_install_arda_automation_units.sh).
- [x] Independently review and install the exact repair candidate with rollback;
  measure daemon RSS, service anonymous/cache accounting and read volume over
  repeated projection ticks; verify unchanged legacy file size/mtime.
  [Installed measurement](../../audits/2026-09-13-legacy-queue-resource-repair.md).
- [ ] Confirm background operation remains acceptable during the operator's
  combined game/YouTube/work workload. Idle-memory proof does not establish frame pacing.
- [x] Remove the oversized legacy blobs from the four unpublished commits with
  explicit operator authorization, preserving all other committed paths and local
  queue/schedule files. A local rollback branch retains the original history.

### 1. Complete the retained execution path

- [ ] Finish review/hardening of the implemented independent keeper and bounded
  client; wire daemon reconnection without owning the keeper's lifetime.
  The unbounded preparation-error `Child::wait` is replaced by an owned deferred
  cleanup queue with nonblocking reap. New Prepare requests refuse while cleanup
  remains unproven; existing Commit/Release handling is not gated. Pending pins
  remain owned until reap; final owner teardown retains unresolved pins for the
  remaining process lifetime, without claiming cleanup or deleting journal rows.
  [Cleanup regressions](../../../crates/engine/src/bin/keeper_owner/qualification.rs)
  pass in debug/release: the structural blocking-wait guard reproduced RED, an
  injected unresolved observation retains a pin sentinel until real child reap,
  and real qualification failure returns promptly with eventual PID disappearance.
  This does not induce uninterruptible kernel I/O. Independent source review
  found no must-fix ownership, pin-retention or serial-owner liveness defect and
  reproduced all three focused cleanup tests. Nonblocking limitations remain:
  reap is request-driven (idle owners can retain exited children/pins until the
  next request), and shutdown polling skips failed-qualification reap while
  ordinary children remain unreaped. Direct tests of pending-cleanup admission
  rejection alongside existing Commit/Release, and unresolved-pin retention on
  final queue destruction, are still absent. These are source/unit results;
  installed keeper is unchanged and deployment acceptance remains open.
- [ ] Review new manifest/admission identity agreement and durable preparation
  journal; prove crash-boundary behavior and expose explicit orphan reconciliation.
- [x] Wire daemon ObjectiveStore to the tested keeper client. Crash/ACK-loss recovery must reuse the
  saved capability and immutable generation/owner/expiry intent, never prepare
  from current paths for an admitted run.
  Installed startup attaches the configured keeper and persists fail-closed
  policy ([daemon tests](../../../tests/root_daemon.rs)). Installed same-keeper
  verification restart and release-ACK loss preserve the saved authority;
  keeper reincarnation requires explicit terminal reconciliation as qualified below.
- [ ] Route same-run recovery through that capability without weakening fresh
  admission overlap checks. Rebind must fence older executions before ACK.
  Candidate recovery preserves saved identity/run/capability after root removal
  or replacement and serializes uncertain overlap
  ([snapshot tests](../../../crates/engine/tests/objective_snapshots.rs)). This is
  not daemon-to-installed-Hermes recovery acceptance.
  Review found that a durable Commit never delivered before expiry deadlocked
  later claims. The new [real-keeper regression](../../../crates/engine/tests/fixtures/keeper_undelivered_commit.rs)
  reproduced refusal, then passed after the worker began recording an expired
  intent only as a non-executable fence. Recovery reuses the exact snapshot/run,
  advances generation, rejects expired execution and old-generation Commit, and
  releases once without another Prepare. Unit tests retain monotonic expiry even
  after wall-clock rollback and reject changed same-generation payloads. Full
  serial Engine tests, strict all-feature/all-target Clippy and explicitly enabled
  snapshot-worker integration pass. Logs: `/tmp/arda-expired-commit-engine-tests.log`
  and `/tmp/arda-expired-commit-clippy.log`. Independent repair review found no
  concrete safety/correctness defect and reproduced the real-keeper test and all
  three lease tests. Integration asserts execution rejection generically; unit
  coverage establishes expiry specifically. Release-build qualification now
  passes six explicitly enabled keeper-adapter fixtures (expired undelivered
  Commit, both Prepare rollbacks, both active rebind cases, synthetic adapter),
  three lease unit tests, three preparation-cleanup tests and the namespace-worker
  integration. Full debug Engine tests and strict all-feature/all-target Clippy
  pass (`/tmp/arda-bounded-cleanup-engine-tests.log`); the full suite requires the
  actual Rust toolchain directory ahead of rustup shims inside its sandbox.
  Deployment is blocked: the genuine installed-Hermes adapter fixture failed twice
  before tool execution with Manwe HTTP 502/no eligible provider; the configured
  local provider is unhealthy and alternatives fail routing constraints. No
  installed binary or service was changed. Private failure evidence is retained
  at `/var/tmp/.tmpR2NEql`; log `/tmp/arda-keeper-release-provider.log`. Restore the
  approved provider route and rerun genuine release qualification before the
  reviewed atomic keeper/worker installation and idle-runtime restart checks.
  Historical next-stage diagnosis identified the transport blocker: host Tailscale was
  `NeedsLogin`, with no assigned mesh IP despite active `tailscaled`. The local
  inference `/health` on loopback port 9337 returns `status=ok`, while the configured
  core mesh address on that port times out. Manwe on port 7171 is healthy but marks
  `edge_core` unhealthy. Operator Tailscale reauthentication is required before
  rechecking the configured route and genuine provider fixture; do not rewrite
  provider URLs, enable alternate providers or weaken tool eligibility to bypass
  this gate. The resident runtime remains ready with zero pending recovery and
  no active leaves; no installed artifacts or service configuration were changed
  during that diagnosis. Operator reauthentication subsequently restored Tailscale
  with core address `100.105.24.100`; its inference health and model catalog respond.
  Updated core address in `config/fleet.toml` and `config/manwe.providers.toml`.
  Live reload returned 401; an idle-verified daemon restart loaded the new URL
  without restarting the keeper (PID unchanged). Runtime readiness remains healthy.
  The initial restart retained unhealthy provider state and HTTP 502 failures
  (`/var/tmp/.tmpWtffFY`, `/tmp/arda-keeper-release-provider-restored.log`). An
  operator-authorized authenticated `/probe` subsequently returned the expected
  marker with HTTP 200; read-back reports `edge_core` ready with zero consecutive
  failures. No intelligence ledger edits or eligibility weakening were used.
  Corrected the subsequent evidence diagnosis: empty model-authored `tool_evidence`
  and `test_evidence` arrays are explicitly required, because Arda derives them
  from the transcript. The failed session (`/var/tmp/.tmpa45BJ3`) combined setup
  and a rewritten check command, so successful tool exit alone could not attest
  the exact declared check. The shared adapter prompt now requires each declared
  command verbatim in a separate terminal call with the project working directory;
  strict transcript matching and receipt validation are unchanged. The prompt
  regression was observed failing before repair; all 26 regular adapter tests
  then passed. A genuine repeat caught a remaining relative-to-absolute script
  rewrite (`/var/tmp/.tmpONc4hN`); the qualification now declares the exact absolute
  script path under the retained project root, with instructions referring to
  that single check_commands value rather than repeating a relative command.
  Three consecutive genuine release qualifications then passed with nonempty
  derived evidence, unchanged check source, retained artifact verification,
  receipt validation after reopen, and durable release/restart assertions.
  Logs: `/tmp/arda-keeper-canonical-check-1.log` through
  `/tmp/arda-keeper-canonical-check-3.log`. Full Engine tests and strict Clippy
  passed after the final change (`/tmp/arda-exact-check-engine-full.log`,
  `/tmp/arda-exact-check-clippy.log`). This is isolated genuine-provider qualification,
  not installed candidate or whole-system acceptance. Keeper/worker candidate
  binaries remain uninstalled. Independent review found no concrete correctness
  or security defect in the shared prompt repair and independently ran all 26
  regular adapter tests successfully (8 ignored). Its scope did not independently
  qualify the absolute-path fixture refinement or rerun genuine-provider tests;
  the three genuine passes above are implementation verification. Existing
  whitespace trimming and explicit adapter-root `cd` wrappers remain accepted;
  the new guidance does not broaden matching.
- [x] Qualify explicit Hermes executable/runtime/profile grants. The current
  candidate implements explicit retained runtime grants and private generated
  configuration. The generator's private/idempotent/schema integration test and
  publication race/file-type regressions pass. Installed Hermes execution,
  verification and review through those grants passed the bounded local
  acceptance in section 3; private scratch remains separate from workspace and `/usr`.
- [x] Route chat/export and artifact checks through retained execution; bounded
  cancellation and expired/released receipt replay have corrective review and
  [regression coverage](../../../crates/engine/tests/fixtures/retained_replay.rs).
- [ ] Extend tested durable owner release/restart behavior with crash/ACK-loss
  injection and explicit reconciliation. Never infer release from socket absence. Reconcile orphans and
  uncertain cleanup without recursive deletion through workspace mounts.
  Candidate offline inspect/stop-proof/revoke commands preserve tombstones and
  distinguish terminal revocation from worker cleanup ACK. [Offline regressions](../../../crates/engine/tests/keeper_reconcile.rs)
  cover nonmutating inspection and owner exclusion; [isolated real-systemd proof](../../../scripts/test_keeper_managed_reconciliation.py)
  covers a pre-authority admission, managed stop, revocation and idempotent retry.
  [Real-authority managed test](../../../crates/engine/tests/keeper_managed_release.rs)
  now proves allocation-bearing admission, owner-lock refusal after socket loss while
  managed processes remain live (not the cgroup teardown predicate or anti-escape
  confinement), path replacement, policy-free keeper restart, lost Release
  response and one Engine release marker. Its five-point crash matrix now runs
  the real reconciliation CLI handler, evidence/receipt validation, offline locks
  and anchored VFS inside a test executable; the normal keeper executable retries
  each revocation idempotently. Precommit exits preserve the original row and
  allocation; postcommit exit preserves the receipt/tombstone and releases the
  allocation. All cases preserve authority bytes and recover terminal release.
  This is process-exit recovery, not power-loss or installed-binary qualification.
  A real manager restart between the final teardown observation and commit fails
  at owner exclusion (asserted from its journal), leaves no keeper PID, and cannot
  prevent the committed receipt from recovering. A separate real stop-timeout
  case uses a nonterminating `KillSignal=SIGCONT` fixture, observes `deactivating`,
  rejects inspection while locked, then verifies timeout escalation, PID
  disappearance and release recovery. It does not reproduce a production SIGTERM
  handler stall. [Helper tests](../../../crates/engine/src/bin/keeper_reconcile/tests.rs)
  additionally cover rollback on failed final recheck; [predicate tests](../../../crates/engine/src/bin/keeper_managed/tests.rs)
  cover historical cgroup/invocation mismatches. The complete managed integration
  target passes with ignored tests enabled; independent review reproduced the
  target, verified normal-binary exclusion of test hooks, and found no new recovery
  blocker. The configured-worker hostile descendant fixture now passes: double
  fork/setsid, ignored SIGTERM, denied cgroup migration/mount, nested user namespace,
  proc-root and manager-socket access, no extra inherited descriptors, observed
  managed-cgroup membership and disappearance after stop. This uses synthetic
  local code through the typed bootstrap, not real Hermes/provider acceptance.
  Independent review reproduced the scenario and found a response-oracle defect:
  the test read `exit_code` instead of the worker's `code`. A regression reproduced
  false acceptance of a successful response; the corrected oracle rejects success
  and malformed replies, and the complete managed target passes again. Probe skips
  profile validation; syscall failure probes do not establish every escape route.
- [ ] Run the original adapter mount-race regression through the production
  route and make it GREEN. Cover admission rollback, ACK loss, daemon death,
  lease expiry/rebind, active disconnect, keeper loss and terminal release.
  The original post-construction mount-race regression now passes explicitly on
  the namespace-capable host. The ordinary adapter captures private mount trees
  before returning its prepared command; a dedicated launch owner binds the
  spawning thread through cleanup ([capture tests](../../../crates/engine/src/adapters/hermes/workspace/capture_tests.rs),
  [owner tests](../../../crates/engine/src/adapters/hermes/launch/tests.rs)). Runtime
  grant capture, configured bundle validation/substitution, and installed Hermes
  import-root probe tests also pass explicitly. Real provider execution through
  the installed retained path passed section 3's bounded acceptance; the complete
  loss/recovery matrix remains open.
- [ ] Independent integration review of the resulting path; repair concrete
  blockers. Do not repeat accepted identity/bootstrap/cleanup reviews unchanged.
  The admission/rebind review identified externally successful Prepare followed by
  response loss/Engine rollback, and rebind during live execution as missing proofs.
  Two new [real-owner tests](../../../crates/engine/tests/fixtures/keeper_prepare_rollback.rs)
  now pass: a wrapper discards the real successful Prepare response before Engine
  records authority; a test-only SQLite trigger aborts snapshot insertion after
  that response. Both prove zero committed Engine claim/identity/lease intent,
  one owner preparation with no committed execution lease, changed-path refusal,
  same-authority/run/socket-inode retry after store reopen, and one durable Engine
  release marker (not an independently counted release RPC or cleanup observation).
  The abort trigger is removed before the identity-refusal assertion so it cannot
  mask false admission. Strict Engine Clippy passes. These are deterministic
  transport-boundary/transaction-abort fixtures, not abrupt process/power-loss,
  configured runtime allocation, or keeper-loss orphan reconciliation proof.
  Independent review reproduced both tests and strict Clippy, finding no blocking
  defect in the scoped rollback/retry oracles. New
  [active-rebind tests](../../../crates/engine/tests/fixtures/keeper_active_rebind.rs)
  pass in debug/release through ObjectiveStore, KeeperClient and the real owner.
  SO_PEERCRED/pidfd witnesses prove one Python process live at Commit delivery
  and exited/reaped when a successful generation-N+1 response is observed. Cases
  cover a discarded successful ACK and delayed owner refusal while N remains live, store
  reopen/immutable retry, premature N+1 and stale N dispatch refusal, subsequent
  N+1 writes, one Prepare/same snapshot, owner released state and endpoint removal.
  Engine's injected clock advances while the real worker lease remains live;
  these untyped-process tests are not daemon/real-Hermes crash recovery or wire
  ACK-loss acceptance. Independent review reproduced both tests in debug/release
  and identified an implicit umask dependency: runtime permissions are now set
  explicitly to 0700, and both tests pass under umask 0022 in debug/release with
  strict Clippy. The failure oracle now requires the exact owner-refusal message,
  excluding caller transport errors; the protocol hides the internal cause, so
  elapsed time plus refusal does not uniquely prove owner timeout. Prior execution
  ends at its configured execution timeout, not proven rebind-triggered cancellation.
  Descendant/supervisor reap and exact server ACK-send ordering remain unproven;
  premature dispatch processing order is not instrumented, and dispatch refusals
  assert generic failure rather than generation-specific reasons. Store reopen
  retains the same client/live keeper and does not exercise owner restart.

Keeper loss or reboot must fail closed and require explicit reconciliation.
Never silently rebuild an admitted snapshot from current paths. Retained mounts
are not immutable file contents: descendant renames/hard links remain distinct.
Filesystem containment does not govern remote tools or host-service APIs.

### 2. Qualify the deployment candidate

- [ ] Classify installed autopilot/research/other legacy writers and test emissions.
  Freeze retired JSONL admission/control/schedule/continuation authority while
  preserving historical ledgers and compatibility readers. Do not migrate history.
  Varda promotion HTTP/library boundaries refuse; Aulë knowledge-promotion,
  schedule mutation and continuation append helpers now refuse. Schedule completion
  and cancellation refuse before invoking queue callbacks, and legacy schedule
  reads no longer provision missing files/directories. The
  [library refusal matrix](../../../crates/spine/observability/arda-aule/tests/retired_queue_writers.rs),
  [CLI refusal tests](../../../crates/spine/observability/arda-aule/tests/retired_autopilot_cli.rs),
  knowledge-triage tests and continuation-history preservation regression pass;
  strict all-feature/all-target Aulë Clippy passes. The full Aulë library now
  passes all 365 tests, plus all five retirement integration tests. Historical
  fixtures seed bytes directly rather than weakening production guards. Higher-level
  execution preparation, cancellation and revised-objective approval now refuse
  before queue mutation; regressions compare historical ledgers byte-for-byte and
  verify no harness request is sent. Independent review found two further gaps:
  the public active-authority callback and crate-internal direct reconciliation
  could still write. Both now refuse at entry. Callback/no-provisioning and
  already-advanced due recurrence regressions pass with byte-preserved history;
  bounded independent source re-review found no remaining defect in either repaired
  path (it did not rerun tests or qualify installed execution).
  Installed CLI/Varda artifacts are bound and installed. Both autopilot timers
  and the legacy external-lane timer are disabled/inactive; original unit states
  and Varda binary are retained in rollback. This retires the legacy route, not
  retained research as a capability. Wider research emissions remain to reconcile.
  Post-reboot manager inventory still shows no active legacy admission timers;
  the Warden research unit is absent on this host. Further source tracing found
  Oromë decision completion/drain rewriting historical queue bytes and
  `scripts/task-pivot.sh` accepting fresh JSONL admission. Both now refuse before
  reading/provisioning/mutation. The two decision-handler regressions and isolated
  shell test first failed, then passed with byte-preserved history; missing and
  malformed history are covered. Full all-feature Oromë tests, strict all-target
  Clippy, scoped formatting, Engine check and all eight root-daemon tests pass.
  Independent review found no concrete defect and reproduced all three focused
  Oromë retirement tests plus the isolated shell test and syntax check. This repair
  hardens the optional `service-runtime` library surface. Feature-tree inspection
  of both the root daemon and full-feature Aulë CLI confirms they do not enable
  that feature; the changed handlers are not in those build graphs. No daemon
  replacement is warranted solely for this repair. Evidence:
  `target/qualification/orome-retirement-feature-scope.json`.
  The shell entrypoint itself is retired in place. Compatibility readers
  remain; wider producer inventory is not yet closed. Publication review also
  identified three pre-existing public Aulë mutation APIs
  outside the guarded paths above: `ActiveQueueExecutor::reprioritize`,
  `revise_objective`, and `retry_failed` in `task_queue.rs` still provision/append
  legacy history. Scoped review found test callers, not installed dispatch.
  These APIs are explicitly excluded from this batch's retirement claim and remain
  open in this writer-inventory gate: guard before reads/provisioning and replace
  successful-mutation tests with malformed/missing-history preservation coverage.
  Publishing this incomplete cutover does not certify all JSONL authority frozen.
  Another source gap is the older typed Phase-1 path: `arda_core::state::append_task` still
  provisions/appends JSONL; Aulë `prometheus::planner::run(Some(queue))` writes a
  plan before calling it, and Core `loop_engine::dispatch_full_with_affordability`
  reads historical tasks and records decisions before appending terminal state.
  No external caller of that legacy dispatcher was found in the scoped source
  search; do not confuse library reachability with installed activity. Retire
  mutation at the outer callers as well as the append helper, preserving pure
  planning/accounting APIs and historical test fixtures. Reject before any plan,
  decision, executor, queue read/provisioning or task-state side effect; a final
  append refusal alone is insufficient. These paths have not yet been changed.
- [ ] Verify canonical authenticated Engine intake/control, changed-payload replay
  rejection, store-loss fail-closed behavior and notification after accepted changes.
  Objective/control event-key reuse is rejected inside the same immediate write
  transaction, including after reopening; identical-command retries remain valid
  ([store regression](../../../crates/engine/tests/objective_runtime.rs)). The HTTP
  regression verifies 409 without objective or operator-ledger mutation
  ([gateway regression](../../../crates/engine/tests/harness_operator_messages.rs)).
  Routine connections no longer create missing databases; a real file-removal
  regression verifies not-ready, no recreated database/sidecars, and recovery after
  restoring the original file ([store-loss regression](../../../crates/engine/tests/objective_runtime.rs)).
  Runtime callers (daemon, gateway and explicit execution) now use non-creating
  `open_existing`. A durable provisioning marker binds device/inode and a database
  nonce; missing/torn markers, missing stores and copied replacements fail closed.
  Constructor, resident-access and fresh CLI-process regressions pass
  ([authority regression](../../../crates/engine/tests/objective_authority.rs)); actual
  daemon restart refuses lost directories/databases/markers and replacement files
  before starting children ([daemon regression](../../../tests/root_authority.rs)).
  Offline `arda-objective-store init --database PATH` explicitly provisions fresh
  state; `adopt --database PATH` binds a pre-cutover store while retaining history.
  Stop writers before provisioning. Neither command resets an existing marker;
  interrupted provisioning stays refused. Restore the original database/marker
  rather than deleting bindings to manufacture a fresh runtime.
  All Engine tests pass with private-directory umask and direct toolchain PATH
  (`umask 077; PATH="$(dirname "$(rustup which cargo)"):$PATH" cargo test --locked
  -p arda-engine --tests -- --test-threads=4`); strict all-feature/all-target Engine
  Clippy and root-daemon/restart targets pass. Default-environment full runs exposed
  keeper-fixture permissions, rustup HOME isolation and a capture-test timing flake;
  these are not installed-runtime acceptance. Installation status is tracked below.
  Gateway ingress now validates/reserves the existing Oromë ledger before domain
  mutation, with one durable SQLite payload binding across all command families.
  Changed-family replay is rejected even after domain commit followed by failed
  transport recording and harness restart; identical retries recover without a
  second objective. Pre-cutover ledger-only entries, invalid adapters and concurrent
  changed-family requests are covered by the
  [HTTP replay regression](../../../crates/engine/tests/harness_operator_messages.rs).
  All Engine tests, Oromë bridge tests, daemon/restart tests and strict Engine Clippy
  passed after the gateway change. Root/projection and interrupted-verifier repairs
  have independent review and are installed. The material-evidence validator is
  unchanged. Post-reboot terminal reconciliation is now qualified below; genuine
  operator ingress acceptance remains distinct from these fixtures.
- [x] Expose schedule quarantine and actual provider/keeper prerequisites without
  relabeling `/health` or scheduler-local idle as whole-system readiness.
  Runtime status now exposes persisted `quarantined_schedules`, marks readiness
  false with `schedule_quarantined` while any remain, and reports unknown rather
  than zero when authority cannot be read. Restart and store-loss coverage is in
  the [runtime tests](../../../crates/engine/tests/objective_runtime.rs): all 18 pass,
  as does strict Engine Clippy. The separate
  [prerequisite endpoint](../../../crates/engine/src/harness/prerequisites.rs)
  (`GET /v1/execution-prerequisites`) now observes the authenticated Manwe catalog
  and same-user keeper socket transport with bounded time/body size, using the exact
  resident-configured socket including CLI precedence. It sends no keeper operation.
  Catalog availability and a connected socket are explicitly not Hermes execution,
  runtime-grant or admission qualification: `execution_ready` remains null and scope
  is `prerequisite_observation_only`. `/health` and scheduler-local readiness retain
  their distinct meaning. The HTTP route first failed with 404, then all 26 harness
  run tests passed, including catalog refusal/malformed/oversized responses and keeper
  loss without admission traffic. Strict Engine Clippy and root authority/daemon
  tests pass after the wiring change. The full serial Engine and root authority/
  daemon suites pass after the review repairs. An earlier full run failed the
  inherited-output deadline fixture because its descendant missed startup; the
  exact isolated regression and subsequent full rerun pass, but this intermittent
  fixture remains unresolved. Independent review found no concrete defect in the
  prerequisite observations; that does not qualify actual provider execution.
- [x] Establish service ownership, runtime grants, namespace support, active work,
  exact binaries/configuration and rollback before touching live services/profiles.
  Keep the snapshot keeper independent of `arda.service` restart/stop ownership.
  Candidate socket environment wiring passes the daemon regression; keeper
  readiness uses `Type=notify` after storage/policy/socket setup. Managed provenance
  requires effective cgroup confinement and runtime-directory preservation before
  admission. Isolated manager tests do not establish production unit qualification.
  Both units are now installed and active. Before cutover, the store contained no
  active objectives or schedules; three expired leases belonged to a failed
  historical objective and were preserved. The hash-checked rollback bundle is
  `~/.local/share/arda/rollback/20260915T042450Z/manifest.json`; the stopped store
  and authority marker were also preserved. Old daemon PID disappearance was
  verified; daemon restart preserved the independently supervised keeper PID.
- [x] Build and independently review the exact daemon/CLI/keeper candidate;
  bind source, candidate and installed identities. Preserve unrelated worktree
  edits and runtime data. No commits, pushes or coding delegation without request.
  Formatting and strict all-feature/all-target Engine/Aulë Clippy pass. Locked
  release daemon/Engine binaries and full-feature Aulë `arda-cli` build after the
  prerequisite changes. The first installed candidate and deployed retained
  configuration were hash-checked against `target/qualification/candidate.json`.
  The manifest was rebound and the reviewed root/projection repairs were installed;
  subsequent source changes require fresh qualification. The retained-config
  generator deployed `/usr/bin/python3.12` and actual installed Hermes import roots;
  keeper policy validation passes. Real installed Hermes chat, workspace write and exact-session export
  pass in the explicitly enabled release-profile `runtime_bundle_worker` test.
  All six explicitly enabled `keeper_managed_release` tests passed before the
  review repair. The absent-live-path failure was a tool-free review claiming file
  inspection; isolated failure evidence remains at `/var/tmp/.tmpBjBtPq`.
  The [owned bootstrap](../../../crates/engine/src/bin/snapshot_runtime/bootstrap.py)
  now sets `tool_choice=required` for tool-capable requests before the first tool
  result, then permits normal completion. Requests without tools remain unchanged;
  this requests actual work, never fabricates evidence or weakens its validator.
  The HTTP-wrapper regression failed before the change, then all three guard tests
  passed. Real release-profile recovery passes in an isolated absent-path run and
  two paired runs. The final [recovery regression](../../../crates/engine/tests/resident_retained_restart.rs.inc)
  checks the persisted review session directly: both scenarios produced two actual
  `read_file` results, preserved execution receipts and did not replay execution.
  Log: `/tmp/arda-review-required-observed.log` (two tests passed).
  Full serial Engine tests and strict all-feature/all-target Clippy pass using the
  established private-directory umask; the initial default-umask run failed the
  keeper-storage directory privacy tests, not review recovery.
  Candidate review found no blockers before the first installation. A new bounded
  local CLI-authorized acceptance objective reached the installed resident loop
  but failed before provider dispatch: daemon `repo_root()` returned `.` while
  retained dispatch requires an absolute root. The projection also confused a
  leaf-scoped RunGraph objective ID with the canonical objective ID. Both faults
  have failing-then-passing regressions in [daemon startup](../../../src/main.rs) and
  [projection publisher](../../../crates/engine/tests/operator_projection_publisher.rs).
  Full root tests, full serial Engine tests, strict root/Engine Clippy and release
  daemon/Engine builds pass. Independent review found no blockers and reran the
  absolute-root regression and all 14 projection publisher tests. The local objective
  `cutover-local-acceptance-20260915` was cancelled through ObjectiveStore control;
  its failed attempt remains preserved. Reviewed binaries are now installed.
  Varda was rebuilt/tested/replaced: `/status` returns 200 and the retired
  `/policy_promote` route returns 410. The legacy external-lane timer is disabled;
  Aulë autopilot units remain inactive. Rollback includes their original states and
  the previous Varda binary under `legacy-writers/` in the existing rollback bundle.

  Interrupted-recovery candidate repair now separates eligible retry inspection
  from receipt-only reconciliation, preserving exhausted-budget behavior and
  validating every required stage before treating Running as incomplete. It also
  moves Running-node scheduler admission after orphan recovery and counts durable
  provider starts per node, not the global checkpoint sequence. The real retained
  crash test exits while verification is durably Running; missing and replaced
  workspace variants pass with one execute start, two verify starts, unchanged
  execution receipt, two review file reads, stale-lease rejection and durable
  release. Serial Engine and full-cli Aulë tests, strict Clippy and affected release
  builds pass. Independent review found no blockers and independently passed
  18 objective-runtime tests, the provider-attempt regression and two retry
  inspection tests. Reviewed repairs are installed; acceptance follows below.

### 3. Installed acceptance and retirement

Final installed local CLI-authorized evidence:
`target/qualification/installed-final-acceptance-evidence.json` and
`target/qualification/installed-recovered-evidence.json`. These bounded fixtures
do not substitute for genuine messaging-gateway or real-project acceptance.

- [x] Backed-up bounded shutdown and replacement with no active objectives or
  unreleased snapshots; old processes stopped and installed binaries hash-checked.
- [x] Bounded installed original-workspace execution, verification, review and
  receipt-backed close; cancellation after retained dispatch durably releases.
- [x] Restart during durably Running verification completes on the installed
  daemon: one execute start, two verify starts, one review start, unchanged
  execution receipt, same run and keeper PID, one durable release. This is not
  a guarantee of replay safety for arbitrary interrupted consequential execution.
- [x] Final-candidate restart after completion preserves all four receipts;
  a due schedule stays paused across startup with zero attempts, then completes
  and releases on resume. Runtime ends ready without active leaves or recovery.
- [ ] Genuine operator-authored installed scenario, completed-sibling/Vairë
  effect non-replay, and sibling isolation/control/projection acceptance.
- [x] Installed release-ACK interruption: a private proxy dropped the real keeper's
  successful Release response; the installed daemon persisted zero release markers.
  Restart against the real socket persisted exactly one marker without changing
  authority/identity bytes. No provider dispatch occurred in this terminal fixture.
- [x] Installed keeper-loss/reboot fail-closed recovery. Managed stop followed by
  explicit inspect/stop-proof/revoke before restart passed, preserving authority
  and distinguishing revocation from worker-cleanup ACK. A separate test restarted
  the keeper before stop-proof: Release correctly refused, but subsequent offline
  inspection rejected `fresh or unknown invocation cannot prove historical teardown`.
  Independent source review confirms no supported same-boot CLI recovery for a
  nonempty replacement invocation. The supported ordering is stop/mask → proof →
  revoke before restarting; a proof alone would not survive the current teardown
  recheck. A real reboot provides the alternative historical fence, but still
  requires explicit stopped/masked inspection, proof and revocation before Release
  retry. The operator returned after a real host reboot and authorized the saved
  recovery handoff. The immutable snapshot boot differs from the running kernel;
  stopped/effectively runtime-masked inspect → stop-proof → terminal revoke passed
  before restart. Engine then persisted exactly one release marker with unchanged
  authority, identity, managed binding and authority-marker hashes. No run artifacts
  were created and no objective execution was dispatched. The receipt explicitly
  does not claim worker-cleanup ACK or verified cleanup. Evidence is preserved in
  `~/.local/share/arda/rollback/20260915T042450Z/keeper-boundary-evidence/post-reboot-recovery/`.
  This qualifies recovery of the existing cancelled terminal fixture across reboot,
  not survival of active Hermes execution or abrupt power-loss durability.
- [ ] Reconcile wider milestone evidence and remaining candidate/hardening
  checkboxes; archive only when every owning acceptance requirement passes.

Failed interruption evidence remains in
`target/qualification/installed-interrupted-evidence.json`; that test objective
was cancelled and released without checkpoint edits or execution replay.

Do not reset live objectives or repeat accepted consequential actions to manufacture
acceptance. Genuine operator-authored scenarios remain distinct from fixtures.

Current installed boundary: both services are restored, active and candidate-hash
matched. Objective runtime reports `ready=true`, `phase=waiting`, no last error,
zero pending recovery, zero quarantined schedules, no active leaves/objectives
and no unreleased snapshots. The original keeper unit and its disabled enablement
state are preserved; Arda starts it through its existing dependency. Temporary
runtime masks are removed. Successful ACK-loss, historical failed-restart and
post-reboot recovery evidence remain in
`~/.local/share/arda/rollback/20260915T042450Z/keeper-boundary-evidence/`.
The fulfilled restart handoff has been removed from the active plans folder.

The narrow Oromë/task-pivot retirement has independent review; its optional
service handlers are outside the current daemon/CLI feature graphs. Keep the
healthy installed binaries unchanged. The source tree has changed, so do not
reuse the candidate's source digest for future deployments. Resume the unchecked
retained-path integration/loss-matrix review and wider
legacy research-writer classification above. Genuine operator-authored two-project
work, completed-sibling/Vairë non-replay and operator-observed game/YouTube/work
acceptance remain separate gates; do not create synthetic replacements or repeat
already accepted consequential execution.

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
