---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "historical_evidence"
  owner: "PROMETHEUS"
  status: "archived"
  reviewed: "2026-09-08"
---

> 🜏 Soterion: 📜 historical_evidence | owner: PROMETHEUS | status: archived | reviewed: 2026-09-08

# Objective runtime checkpoint history

Historical record, not an executable backlog or current-state authority.
Later checkpoints supersede earlier statements. Continue only from the
[active cutover plan](../plans/2026-09-01-arda-objective-runtime-cutover.md).

## Outcome and authority

The installed `arda` daemon owns durable objective control, scheduling, bounded
concurrent execution, restart recovery and receipt-backed closure. Engine
ObjectiveStore is transactional SQLite (`data/arda/objectives.sqlite3`); Engine
RunStore owns immutable execution evidence. Hermes/Oromë provide authenticated
ingress; Vairë supplies context/outcomes; Aulë supplies execution mechanisms and
observability, not another scheduler.

This is the implementation dependency of the [autonomous milestones](../plans/autonomous-task-completion/README.md), not a second acceptance program. Provider transport belongs to [provider convergence](../plans/PROVIDER_WORKER_CONVERGENCE.md); useful real-project overlap belongs to [Milestone 4](../archive/2026-09-15-plan-reconciliation/autonomous-task-completion/04-real-multi-project-execution.md); continuity and operator burden belong to [Milestone 5](../archive/2026-09-15-plan-reconciliation/autonomous-task-completion/05-vaire-operator-acceptance.md).

## Current evidence, not completion

- [September 7 revalidation](../audits/2026-09-07-objective-cutover-revalidation.md) records restored cycle checks, receipt/path validation, revision safety, equal-string workspace exclusion, persisted-stage reclaim, sibling receipt retention and transactional persisted-attempt limits. Earlier implementation-complete declarations were superseded.
- Source follow-up after the installed alias slice: `runtime.rs` now persists each completed leaf result through `FuturesUnordered` before waiting for pending siblings. A RED/GREEN interruption/reopen regression proves a finished sibling is not reclaimed; all four focused runtime tests pass. Final follow-up gates passed: 276 Engine tests, 4 ignored, five root-daemon tests, strict all-target Clippy and release build. Exact diff `9c0bb587b91bbc376d8e11297739dee153d745690a6ba27f538f4f2a13cbaee8` received independent review; its incorrect claims that `join_all` blocks or cannot be aborted were rejected. Installed at 20:58 PDT as PID 492164, executable SHA-256 `88a33157832c2a1281442c3acc95e3dc61419874f142733b71aabee8061cb50a`, with healthy unchanged-binary Manwë child PID 492180. Rollback: `~/.local/state/arda/rollbacks/20260907-205759-sibling-durability/`. HUD/gateway PIDs stayed unchanged, operator projection remained readable, and no leaf stage/attempt changed during cutover. This is qualified deployment plus fixture regression evidence, not a genuine new multi-project acceptance run. This does not persist intermediate stages inside an unfinished Workbench call. `workbench.rs` still derives `run_id` from the incremented lease attempt and assembles fresh context; stable identity alone would not preserve context/outcome receipt bindings. `src/main.rs` still sleep-polls and awaits objective shutdown without a drain bound. Schedule storage tests do not prove due schedules are consumed.
- September 7 installed source repair: a real symlink regression first returned two simultaneous claims for one physical directory. Canonical-path exclusion now covers stable aliases, relative roots, nested/uncreated descendants, concurrent claimers and store reopen. `cargo test -p arda-engine` and strict all-target Clippy passed. The first review missed candidate-window starvation; a new RED test reproduced it and bounded keyset pagination fixed it. The revised exact diff received independent review; incorrect review claims about monotonic IDs and cross-process SQLite locking were rejected against source. Final gates: 275 Engine tests passed, 4 ignored, strict all-target Clippy passed, five root-daemon tests passed, release build passed. Do not mark the broader physical-identity gate complete.
- September 7, 20:48 PDT installed verification: gracefully stopped old root PID 1829 and its Manwë child, verified both disappeared, installed SHA-256 `3f75ea90d7ed18c3d71c916726501b41a5e462ecfa12a6309df998d5ad0e3092`, started once as PID 480798 and verified executable hash plus healthy supervised Manwë PID 480814. Rollback binary/unit/database snapshot and deployment record: `~/.local/state/arda/rollbacks/20260907-204820-workspace-admission/`. HUD PID 414874 and Hermes gateway PID 425928 stayed unchanged; presentation status retained both accepted image-session records. This is deployment/readback evidence, not new visual or multi-project acceptance.
- Installed retry-bound reconciliation changed the one exhausted running objective to failed without changing any leaf stage or attempt. No provider action was replayed for this check. Alias/fairness behavior is regression-tested and now installed, but a genuine operator-authored concurrent-workspace scenario remains under Milestone 4.
- Legacy queue files were dirty before this effort. The autopilot source unit still names `core/projects/tasks/queue.jsonl`; its installed timer continues running. No absence-of-writer or freeze claim is justified yet. Do not erase history or blame optional powered-off devices.

## Remaining implementation order

Current handoff: the identity-persistence slice has passed review (operator
confirmation); do not repeat it. Scheduling, scheduler-local readiness and
substantial shutdown/recovery integration are implemented and source-tested in
the checkpoints below. They are not missing implementation wholesale. The
numbered descriptions below retain contract requirements, not a claim that all
their code remains unwritten.

Execute the remaining gates in this order:
1. Finish physical-root safety: bind aliases and parent/child mutation overlap,
   explicit missing/inaccessible-root behavior, admission-to-execution changes
   and protection of already-running work, then genuine installed admission.
2. Classify/freeze legacy producers; prove new work and controls use Engine state
   and never retired JSONL authority.
3. Build/review the exact deployment candidate, retain rollback, and verify
   bounded installed shutdown/startup.
4. Run installed same-run recovery, scheduling, sibling-isolation and
   terminal-close acceptance without replaying accepted consequential actions.
5. Reconcile evidence and retire this active plan only after these gates pass.

### Retained contract requirements — not a second execution checklist

1. **Physical workspace admission — bounded fix installed; wider acceptance open.** Do not repeat the completed alias/fairness implementation or its guarded deployment. Exercise a genuinely authorized installed admission scenario. Preserve original project/contract spelling while excluding the same physical root and parent/child mutation overlap. Keep bind-mount aliases, directory/symlink retarget during a live lease, missing/inaccessible roots and candidate fairness explicit until proven. Preserve read-only admission on dirty projects; do not bypass exact project approval or Workbench validation.
2. **Stable run recovery.** Persist execution identity separately from claim/recovery count. Reclaim interrupted execute/verify/review/close against the same RunStore run. Recover the original context assembly as well as the execution ID; never substitute a freshly timestamped assembly for an existing receipt binding. Legacy attempts without an unambiguous durable binding must remain explicitly unreconciled rather than guessed. Reconcile a durable RunStore receipt written before its SQLite projection without repeating provider execution or Vairë effects. Include interruption at the retry limit; preserve canonical dependency receipt payloads/digests. Do not change a failed attempt into fabricated success or reset live objectives to create evidence.
3. **Resident supervision.** Wake through existing ingress/control notification plus bounded due-time/lease recovery. Consume deferred/recurring schedules, suppress paused/cancelled/terminal wakes, track joined tasks and persist each stage as it completes. Reconcile RunStore/leases before new admission. Stop claims on shutdown, bound drain, preserve recoverable stage/lease state and reap children. Report readiness, active leaves, next wake and reconciliation error through Harness; `/health=ok` alone is insufficient.
4. **Canonical ingress/projections and producer freeze.** Confirm authenticated intake and inspect/pause/resume/reprioritize/revise/approve/reject/cancel all use Engine state; reject changed-payload replay. Notify runtime after accepted mutations. Fail closed on store loss, never fall back to JSONL. Classify installed autopilot/research/other legacy writers and test emissions separately; route eligible new objectives through canonical admission. No new task/control/schedule/continuation may enter retired JSONL authority. Preserve compatibility readers and historical ledgers until their consumers are explicitly retired.
5. **Qualified deployment and installed acceptance.** Build exact-source daemon and CLI, complete independent review, retain rollback and bind source/candidate/installed hashes. Inspect active work before restart; stop cleanly, verify target PID disappearance, start once and verify actual readiness. Exercise same-run recovery, due wakes, canonical projection/control behavior, sibling isolation and one terminal close. Do not re-execute accepted consequential actions merely to obtain new receipts.
6. **Closeout.** Reconcile milestone evidence, run final gates after the last semantic edit and independently review the exact candidate. No commit or push unless explicitly requested. Keep the wider multi-project/continuity/operator gates open until their own genuine scenarios pass.

## Physical-root follow-up — not deployed

Claim-time canonical root and nearest existing ancestor identity are now retained
transactionally in `lease_workspace_identities`; Unix identity includes device and
inode. Admission fails closed when a live lease's root changes or its identity is
unknown. Expired recovery cannot silently replace that identity. The migration
adds the table without inventing historical identities; started legacy leaves
without one remain explicitly blocked. Original workspace contract spelling is
unchanged. Restoring the original directory/link permits recovery.

RED/GREEN [store regressions](../../crates/engine/tests/objective_store.rs) cover
symlink retarget, same-path directory replacement, reopen, expired recovery and
legacy migration. Full Engine/root-daemon tests, strict Engine Clippy and workspace
check pass (`/tmp/arda-workspace-identity-engine.log`,
`/tmp/arda-workspace-identity-daemon.log`, `/tmp/arda-workspace-identity-workspace.log`).
The operator confirms the bounded identity review passed; the former review-pending
note was stale. No installed services or live data changed for that slice.

This is detection at admission, not a filesystem lock or cancellation of already
running work. Bind-mount aliases, mutation between check and execution, precise
missing/inaccessible-root policy and installed acceptance remain open. Creation
of a previously missing root conservatively changes its captured ancestor identity
and blocks further admission/recovery rather than silently replacing the baseline.

The subsequent [bind-mount regression](../../crates/engine/tests/objective_store.rs)
runs real mounts inside an isolated user/mount namespace. It reproduced two leases
for the same physical directory. Admission now also compares device/inode identities
at ancestors and their relative subtrees: direct bind aliases and parent/child
overlap in both directions are excluded, while independent siblings remain eligible.
The namespace regression and store suite pass, as do full Engine tests and strict
all-target Engine Clippy (`/tmp/arda-bind-root-{red,green,engine,clippy}.log`).
The namespace test is opt-in and was explicitly run with `--include-ignored`;
ordinary suite success alone does not exercise mounts. This is a bounded source
slice, not complete physical-root safety: mounts nested inside otherwise disjoint
roots, filesystem races and running-worker containment remain unqualified.
Independent review found no admission defect within this bounded scope; reviewed
store SHA-256 `af4b228efeba58c186e74edb6791cf26d5f316361de962c75d811b8bdae4f446`
was verified unchanged afterward. The reviewer found a test panic-cleanup defect:
an unwind-safe mount guard now unmounts before temporary-directory destruction.
A forced-panic regression verifies the bind disappears; the explicitly enabled
store suite and strict all-target Clippy pass again
(`/tmp/arda-bind-root-reviewed-{store,clippy}.log`). The amended test file was
not part of the earlier review hash; this is not exact deployment-candidate
qualification. Nothing in this slice establishes installed admission acceptance.

Nested-mount follow-up: the [real namespace test](../../crates/engine/tests/objective_store.rs)
reproduced two claims for disjoint parents sharing a mounted subtree, including a
second bind whose source is a child of the first. [Admission](../../crates/engine/src/objectives/store.rs)
now includes nested mountpoints and filesystem-relative source roots from Linux
`/proc/self/mountinfo`. Comparing ancestry alone missed the second case because
a child bind hides its source parent. Escaped mount paths are decoded as bytes;
an unrelated opaque `nsfs` source root is tolerated, while an opaque source inside
the compared workspace scope fails closed. This remains a point-in-time overlap
check, not a topology lock or containment of already-running mutations.

The explicitly enabled mount/store suite, full Engine rerun, strict all-target
Engine Clippy and root-daemon tests pass. Initial full Engine verification hit
the existing adapter PID-file startup failures; the unchanged adapter target and
full rerun passed. Logs: `/tmp/arda-nested-mount-red.log`,
`/tmp/arda-nested-mount-green-opaque.log`,
`/tmp/arda-nested-mount-{engine,adapter-rerun,engine-rerun,clippy,daemon}.log`.
Bounded independent review found no concrete admission-safety defect. Parent
readback confirmed unchanged reviewed SHA-256 values: store
`58a12873cdb57f3ca770f3ceb0707e7c0f7b8b2e3fe86191814d899a5a961c52`, tests
`61d8ddc9b5cd6a5619084193ff01903288511aa0e25a9d282b6db3ed9e01203d`.
The review inspected logs rather than rerunning tests. Covered mount records can
conservatively over-exclude; relevant opaque roots and non-directory mounts can
fail closed. Dedicated escaped-source-root and opaque-source parser assertions
remain a coverage improvement, not a demonstrated safety defect.
Missing/inaccessible-root policy,
filesystem changes after admission, running-work safety and installed admission
remain open. No installed service or live objective was changed by this slice.

### Missing/inaccessible-root policy — source verified and bounded review passed

- [Admission](../../crates/engine/src/objectives/store.rs) retains the existing
  missing-root reservation policy: capture the nearest existing ancestor without
  creating the path. This is an ordinary claimed attempt, not a new waiting state
  or authorization to create a workspace. Later creation changes the identity
  baseline and cannot silently authorize recovery; the reopen regression checks it.
- Existing roots and missing-root anchors must be directories with read/search
  access under the daemon's effective credentials. Permission failures roll back
  admission/recovery without advancing attempts. No write permission is required
  merely for read-only admission, and no permission repair is attempted.
- [Workbench execution](../../crates/engine/src/objectives/workbench.rs) checks
  the exact root before opening ObjectiveStore or Vairë or calling the adapter.
  Missing, non-directory and inaccessible roots return an explicit execution
  error; the existing runtime error policy still applies. Receipt-only historical
  reconciliation skips this new execution preflight, not existing store identity
  or receipt-integrity checks. No claim of recovery through an inaccessible root.
- RED/GREEN regressions reproduced mode-000 admission and missing-root preflight
  ordering. Coverage includes read-without-search, search-without-read, inaccessible
  missing-root ancestors, permission restoration, read-only directory access,
  unchanged attempts, root creation/reopen and no adapter/memory/store effects from
  failed Workbench preflight. Permission fixtures require an unprivileged Unix user.
- Full Engine tests, strict all-target Engine Clippy and root-daemon tests pass:
  `/tmp/arda-root-access-{red,green,workbench-red,workbench-green,engine,clippy,daemon}.log`.
  Bounded independent review found no blocking production defect; parent readback
  confirmed all reviewed SHA-256 values unchanged: store
  `22777a9343b9fb4ba59a2e8ba2d4afaf8b25483c71ae9cd2f624a8af9dae4914`, Workbench
  `1ade1e94f6ae13019758343fc502d9c983c033345b7cbeaa3fc797795f00aa1b`, store tests
  `e9ab83b8b64c6e6b7cc45ae6a29be3823def5fcda2980dafc82f3149f6350c24`.
  The reviewer inspected logs without rerunning tests. Permission regressions are
  not portable to privileged/root test runners as written; root can legitimately
  bypass mode-000 permissions. Fixtures restore permissions before assertions,
  but unexpected panics during calls lack an unwind restoration guard.
  This is point-in-time availability only;
  identity changes after admission, already-running work and installed acceptance
  remain open. No installed restart, live objective change, commit or push occurred.

### Pinned worker containment — source tested and bounded review passed, gate open

Operator-selected policy: keep writes bound to the originally approved directory;
deny replacement-target writes, then stop and reconcile when a change is observed.

- [Hermes adapter containment](../../crates/engine/src/adapters/hermes/workspace.rs)
  opens the workspace directory and passes its descriptor to Linux bubblewrap.
  The host filesystem is read-only except the pinned project and an explicit,
  lexically disjoint `HERMES_HOME` state directory; scratch storage is namespace-private.
  This state exception is trusted configuration, not physical/bind-alias-aware
  admission, and is reopened for each subprocess. Its qualification remains open.
  Existing stage/toolset authority semantics are retained. This is filesystem
  target containment, not new enforcement of read-only stage tool policy.
- [Harness admission binding](../../crates/engine/src/harness/runs.rs) checks the
  descriptor's identity against the resident execution's persisted workspace
  identity before starting the provider. Missing resident identity fails closed.
- Root identity/access or mountinfo changes trigger cancellation through the
  existing bounded process-group reaper. Harness retains interrupted running
  state for reconciliation, not a successful stage receipt. The namespace—not
  the 50 ms polling interval—is the write boundary.
- Real namespace regressions rename the root while the worker is live: writes
  remain on the pinned directory, replacement-target writes fail, and monitored
  changes stop/join the process before return. Full Engine tests, strict all-target
  Clippy and root-daemon tests pass. Evidence:
  `/tmp/arda-pinned-workspace-tests.log`,
  `/tmp/arda-pinned-engine-qualified.log`, `/tmp/arda-pinned-clippy.log`,
  `/tmp/arda-pinned-daemon.log`. An initial unrelated project-adapter PID-startup
  failure passed on rerun. The golden fixture needed explicit worker-state storage
  and private scratch space rather than writable access to arbitrary host paths.
- Initial independent review found three defects, not an approval: unrelated
  inheritable descriptors could bypass read-only mounts; namespace-local PIDs
  invalidated shutdown checks and could target unrelated host PIDs; command-time
  changes between chat/export were misclassified as ordinary failures.
  Source follow-up marks all non-stdio descriptors CLOEXEC before allowing only
  bind descriptors, preserves typed workspace-change interruption at command
  construction, and uses host Unix peer credentials plus pidfds for provider and
  descendant lifecycle assertions. The corrected timeout regression exposed a
  fourth defect: waiting only for the outer supervisor returned before descendants
  exited. The reaper now also waits for owned process-group disappearance within
  its existing grace bound. Follow-up review found no further scoped production
  blocker, but found another stale-PID cleanup in the inherited-output unit fixture.
  That fixture now acquires a host pidfd via a socket handshake before releasing
  the descendant; both timeout fixtures clean up using owned pidfds before failure
  assertions. The integration fixture preserves its pre-cleanup survivor verdict.
  Full Engine and strict Clippy pass after this test-only correction:
  `/tmp/arda-pidfd-cleanup-engine-final.log`, `/tmp/arda-pidfd-cleanup-clippy.log`.
  Final narrow test-safety review found no blocker. Parent hash readback matched:
  adapter `d8e4ac9458623a4517b1be01784fc81ce206f7c6a41ec77b0f4db8fe08ef2ff5`,
  adapter tests `813dbe1d492dd72e76867b0bcd67621d06cefae25006b9d5b4e103bfc3446514`.
  This closes the bounded review findings, not the physical-root gate.
  Full Engine, strict all-target Clippy, root-daemon and explicitly enabled real
  mount regressions pass after these changes:
  `/tmp/arda-containment-review-engine-final.log`,
  `/tmp/arda-containment-review-clippy-final.log`,
  `/tmp/arda-containment-review-daemon.log`,
  `/tmp/arda-containment-review-mounts.log`.
  Descendant RED/GREEN evidence:
  `/tmp/arda-containment-review-descendants.log` and
  `/tmp/arda-containment-review-descendants-fixed.log`.
- Admission topology binding is source-tested; bounded fingerprint review found
  no encoder, persistence or execution-comparison blocker. Parent SHA256 readback
  matched all four reviewed files. This is not race-free mount containment.
  The persisted identity uses a versioned tuple containing the SHA256 of the
  admission process's full mountinfo bytes. Execution compares the pinned
  snapshot using the same encoder; live admission and expired recovery compare
  against the saved baseline. Legacy identity tuples fail closed, without
  silently capturing a replacement baseline. This deliberately conservative
  namespace-wide policy also rejects unrelated mount changes and namespace/reboot
  drift; it does not establish transparent cross-reboot recovery.
  A real nested bind added after admission with the root inode unchanged went
  RED, then GREEN after reopening SQLite; rejection leaves the attempt unchanged.
  Evidence: `/tmp/arda-topology-red.log`, `/tmp/arda-topology-store.log`,
  `/tmp/arda-topology-engine-final.log`, `/tmp/arda-topology-clippy-final.log`,
  `/tmp/arda-topology-daemon.log`. Full Engine, strict Clippy, root-daemon tests
  and explicitly enabled mount tests pass. No installed acceptance is implied.
- Review identified two residual races requiring physical-root qualification:
  live leases and candidates capture topology independently during admission,
  so a mount change between reads can mix baselines within one transaction;
  execution validates before bubblewrap namespace setup, while an open directory
  descriptor does not freeze descendant mounts. SQLite does not lock mounts and
  polling cannot prevent the intervening writes. These are source-inspected
  interleavings, not reproduced race regressions. A consistent admission-round
  snapshot with revalidation narrows the first gap; it does not alone establish
  atomic filesystem containment. Keep both races open until addressed and tested.
- Admission follow-up now uses one topology snapshot for live leases, candidates
  and overlap checks, then revalidates before commit. An injected-drift regression
  verifies transaction rollback leaves attempt zero and no identity row after
  reopen. Engine, explicit mount tests and strict Clippy pass:
  `/tmp/arda-round-topology-engine-rerun.log`, `/tmp/arda-round-topology-mounts.log`,
  `/tmp/arda-round-topology-clippy.log`. The first Engine run hit the existing
  descendant-startup deadline fixture; the unchanged rerun passed. This prevents
  mixed baselines, not atomic coordination with kernel mount operations.
- The execution race is now reproduced, not just source-inspected:
  `mount_change_after_command_construction_cannot_redirect_writes` mounts an
  outside fixture over a nested workspace directory after command construction.
  The provider exits zero and writes the outside marker. Explicit regression
  remains RED: `/tmp/arda-preexec-mount-red-confirmed.log`. The test is opt-in
  because it requires user/mount namespaces; an ordinary green suite cannot
  close this gate. No host project or profile is used by the fixture.
  Installed bubblewrap resolves bind FDs to paths. Namespace-local mount IDs
  prevent directly comparing the existing raw mountinfo digest inside a cloned
  namespace. Do not replace this boundary with additional polling.
- The operator approved independent supervised snapshot ownership across daemon
  restarts, with explicit reconciliation after keeper loss or reboot. A standalone
  [snapshot worker](../../crates/engine/src/bin/arda-snapshot-worker.rs) now retains
  a recursively private staged mount tree and exposes prepare/inspect, manifest-bound
  commit, bounded execution and release. It is **not integrated or installed**.
  Its first review found launcher-environment injection, broad host-file/socket
  exposure, unsafe state-path setup, a parent-death bootstrap gap and cleanup/deadline
  defects. An actual loader-constructor regression reproduced an outside write
  (`/tmp/arda-snapshot-loader-red.log`). The hardened worker installs provider
  variables inside bubblewrap, uses an empty filesystem with explicit `/usr` and
  workspace grants, unconditionally isolates its namespace, anchors scratch writes
  to directory descriptors, arms parent-death protection before exec, bounds cleanup,
  refuses reuse after uncertain cleanup, and enforces a total request-frame deadline.
  [The explicit namespace fixture](../../crates/engine/tests/snapshot_worker.rs)
  passes, including retained mounts/root rename, synthetic private-file exclusion,
  symlinked state-parent rejection, normal release cleanup and synchronized active
  keeper death checked with a host pidfd (`/tmp/arda-snapshot-hardening-test.log`).
  Full Engine tests and strict Clippy pass (`/tmp/arda-snapshot-hardening-engine.log`,
  `/tmp/arda-snapshot-hardening-clippy.log`). Follow-up review found a remaining
  bubblewrap-internal bootstrap death window and state exposure through `/usr`.
  A single-use execed launcher now makes bubblewrap PID 1 of a fresh outer PID
  namespace, with pidfd-checked parent-death protection on both launch edges.
  [The deterministic ptrace fixture](../../crates/engine/tests/fixtures/snapshot_bootstrap.py)
  stops real bubblewrap at its bootstrap fork before the child can arm protection.
  Removing the outer namespace reproduced a surviving child
  (`/tmp/arda-snapshot-bootstrap-negative-control.log`); restoring it passes.
  The expanded namespace suite also rejects a state-filesystem alias mounted
  exactly at `/usr` before any state write and asserts loader-test execution success
  (`/tmp/arda-snapshot-bootstrap-final.log`). Full Engine tests and strict Clippy
  pass (`/tmp/arda-snapshot-bootstrap-engine.log`,
  `/tmp/arda-snapshot-bootstrap-clippy.log`). Independent bounded follow-up review
  found no new blocker in the bootstrap or `/usr` fixes; reviewed hashes matched.
  Scratch state currently requires a private parent on a separate filesystem not
  mounted within the workspace or shared with `/usr`; nested `/usr` mounts are refused. These conservative
  limits and the `/usr`-only executable runtime are not authenticated Hermes/profile
  compatibility. Release now replies only after explicit teardown, reports cleanup
  errors, and exits fail-closed; an unexpected-file fixture verifies nonrecursive
  removal failure preserves that file instead of acknowledging success.
  A child ownership guard attempts termination/reaping on early error returns,
  without extending an existing cleanup deadline or clearing poisoned state.
  The focused guard test, explicit namespace suite, full Engine tests and strict
  Clippy pass (`/tmp/arda-snapshot-cleanup-unit.log`,
  `/tmp/arda-snapshot-cleanup-integration-final.log`,
  `/tmp/arda-snapshot-cleanup-engine.log`, `/tmp/arda-snapshot-cleanup-clippy.log`).
  Independent bounded cleanup review found no blocking finding; the four reviewed
  source/fixture hashes matched on readback. The ownership test exercises guard
  Drop, not fault injection at every production syscall, and best-effort cleanup
  is not proof of full descendant teardown. Abrupt-death artifact scavenging and
  durable manager-side cleanup reconciliation remain open.
  Mount retention does not freeze file contents, ordinary
  descendant renames or hard links. Production still needs transactional capability
  persistence, lease fencing/rebind, output/cancellation transport for chat and export,
  terminal-safe durable release and independent service ownership. The original
  adapter regression above remains RED until it executes through the retained tree.
- Still open: review of admission-round fix and namespace-setup mount containment;
  qualification and review of state-directory exceptions; authenticated installed
  Hermes compatibility, genuine installed admission and recovery acceptance.
  Local filesystem isolation does not constrain remote/network tools or host
  service APIs. No deployment, service restart, real profile edit, commit or push.

### Retained snapshot admission foundation — no production keeper wiring

[ObjectiveStore admission](../../crates/engine/src/objectives/snapshots.rs) now
has an explicit independent-keeper interface. Configured claims prepare before
SQLite admission commits, persist the opaque capability and immutable
generation/owner/expiry intent, then reconcile the keeper acknowledgement before
returning claims. Lost acknowledgements retry the saved capability; no recovery
path calls prepare for an already-started leaf. A durable policy marker prevents
an unconfigured reopened store from silently admitting without snapshots.

The initial bounded review found two defects, both reproduced RED: cancellation
after acknowledgement loss cleared the mutable lease needed for reconciliation,
and policy activation could race the separate precheck. Terminal reconciliation
now requests idempotent release instead of recommitting cancelled admission,
records release only after acknowledgement, and preserves immutable lease intents.
Policy is rechecked inside the actual claim transaction. [Store fixtures](../../crates/engine/tests/objective_snapshots.rs)
exercise admission/release acknowledgement loss, reopen, missing keeper/config,
preparation rollback and immutable commit payloads. The policy unit fixture forces
activation between precheck and admission. Full Engine tests, strict all-target
Engine Clippy and root-daemon tests pass:
`/tmp/arda-snapshot-admission-reviewed-{engine,clippy,daemon}.log`.
RED evidence: `/tmp/arda-snapshot-admission-{cancel,policy}-red.log`.
Corrective independent review found both prior blockers repaired and no new
concrete blocker within the storage/interface scope. It independently reran seven
compiled integration fixtures and the policy unit fixture, without rebuilding.
The reviewed file hashes match the current source. Review evidence:
`/var/home/mythos/.hermes/cache/delegation/live/deleg_4f64932b/task-0.log`.
This review does not establish production-manager release durability, actual
process-crash recovery, retained recovery routing or installed acceptance.

This is a tested storage/interface foundation using a recording keeper, not an
implemented production manager, real process-crash proof or installed acceptance.
The daemon does not configure this interface. Independent prepare ownership,
durable keeper release receipts, orphan reconciliation, store-to-worker lease fencing,
admission-manifest validation, retained-tree recovery and chat/export transport
remain open. Existing host-topology guards are unchanged. The original adapter
mount-race regression remains RED; no live service/profile/data mutation occurred.

### Worker fencing and output transport — integration still open

The [worker lease binding](../../crates/engine/src/bin/snapshot_lease/mod.rs)
now requires run identity, increasing generation, owner and absolute expiry.
Same-generation payload changes and older generations are rejected; identical
acknowledgement retries do not extend the original monotonic deadline. Execution
is bounded by both its timeout and lease expiry. Rebind is serialized after prior
execution teardown and refuses poisoned cleanup state. It never recaptures paths.

[Output collection](../../crates/engine/src/bin/snapshot_output/mod.rs) now uses
bounded nonblocking reads without detached reader threads, capped at 64 KiB per
stream (raw bytes, not serialized wire size; UTF-8 replacement and JSON escaping
can expand the response). Execute responses distinguish timeout, cancellation and output overflow
from transport success. Client disconnect cancels active execution and completes
bounded teardown before reuse, without releasing the retained capability.

The expanded [namespace fixture](../../crates/engine/tests/fixtures/snapshot_worker.py)
executes a newer lease against the original retained tree after root replacement,
rejects stale leases, bounds a running command by expiry, verifies stdout/stderr
and overflow, and observes disconnect cleanup using a host pidfd. Four worker
unit tests, the explicitly enabled namespace fixture, full Engine tests and strict
all-target Clippy pass: `/tmp/arda-snapshot-transport-{unit,namespace,engine,clippy}.log`.
Independent bounded review found no concrete introduced blocker and independently
reran the four worker unit tests and explicit namespace fixture. All five reviewed
source/fixture hashes matched on parent readback. Review evidence:
`/var/home/mythos/.hermes/cache/delegation/live/deleg_4fa2cba7/task-0.log`.
These worker-level tests do not prove
ObjectiveStore-to-worker transport, manager ownership, lease renewal, Hermes
chat/export routing or installed acceptance. The worker still denies `HERMES_HOME`
and only grants `/usr` plus the retained workspace. Production integration and
the original adapter escape regression remain open; no live services or profiles
were modified.

## Recovery implementation in progress — not deployed

- Added a nullable persisted `execution_run_id` on claims; new work keeps the same identity after lease expiry/reopen. Started legacy rows are not assigned a guessed identity.
- Workbench now uses that identity and binds the original `ContextAssembly` to an immutable request digest in the existing ObjectiveStore before adapter dispatch. Reclaimed fixture execution preserves the original stage/context-outcome receipts, with one outcome rather than a second memory effect. Changed execution payloads and unbound legacy claims fail closed.
- Added Vairë current-authority validation before fresh provider execution. A RED/GREEN cached-capsule regression verifies revocation is checked and the use-receipt ledger stays byte-identical; expiry rejection is also covered. Existing foreign-context rejection semantics are retained.
- September 8 source repair: the real Workbench/Harness boundary rejected verification because the execute capsule carried different predecessor authority. Stage contexts now derive deterministically from the original snapshot, preserving memory content and expiry while binding each stage's purpose and actual predecessors. Fresh execution retains current-authority validation. Concurrent context-use persistence now locks lookup/deduplication/append; a reproduced duplicate-receipt race passes 20 repeated runs after repair. [Vairë regression coverage](../../crates/spine/memory/arda-vaire/tests/context_capsule.rs).
- The [isolated subprocess crash fixture](../../crates/engine/tests/resident_restart_fixture.rs.inc) exits with code 73 after provider/context-outcome receipts but before ObjectiveStore projection. Reopen preserves the same run and receipt bytes, keeps the fixture-provider invocation count at three, and suppresses post-close execution. This uses deterministic fake Hermes, not live provider or installed acceptance. Independent review found dangling stage-use references could pass historical projection; a RED/GREEN regression now removes each stage-use receipt and requires explicit reconciliation failure without provider replay or receipt recreation. Historical validation does not renew expiry or require fresh authority. Focused independent re-review cleared that finding.
- Verification: `cargo test -p arda-engine` (279 passed, 4 ignored), `cargo test -p arda-vaire` (91 passed, 1 ignored), `cargo test -p arda-aule --features full-cli` (393 passed), strict all-target Clippy for these packages, and `cargo check --workspace` passed; ignored tests are not acceptance. Engine tests, Clippy and workspace check were rerun after the review fix. Local logs: `/tmp/arda-stage-context-*-tests.log` and `/tmp/arda-stage-context-reviewed-*.log`.
- Initial-binding recovery is now fixture-tested: new first claims persist an explicit unbound marker; snapshot binding atomically consumes it before dispatch. The subprocess fixture exits with code 74 before context assembly, then resumes the same run after lease expiry and completes with three fixture-provider invocations. Lost bound snapshots and unknown historical markers fail closed without memory recall/provider execution. Engine tests, strict all-target Engine Clippy and workspace check passed after this edit (`/tmp/arda-prebind-*.log`); independent source review found no defects in this bounded marker implementation. Transaction-internal process exit and reclaim-versus-binding races were not forced by this fixture. [Implementation](../../crates/engine/src/objectives/store.rs) and [crash regressions](../../crates/engine/tests/resident_restart_fixture.rs.inc).
- Retry-limit recovery has passed focused independent source re-review with no remaining findings in the two repaired boundaries: the [runtime](../../crates/engine/src/objectives/runtime.rs) separates receipt-only reconciliation from ordinary dispatch. [Subprocess regressions](../../crates/engine/tests/resident_restart_fixture.rs.inc) cover completion after the final lease expires, Harness outage followed by successful recovery, and missing canonical run without replanning/provider invocation. Independent review identified reused reconciliation generations and malformed successful GET responses becoming terminal failure; both were reproduced and repaired. [Fencing regressions](../../crates/engine/tests/objective_runtime.rs) cover delayed success/failure after same-owner reclaim and expiry without reclaim. Every claim advances the generation, receipt writes require it, and lease checks use completion time. The [adapter](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs) rejects malformed graphs as retryable errors. Engine tests, serial Aulë full-cli tests, strict Clippy and workspace check pass after these fixes (`/tmp/arda-fenced-recovery-*.log`); the reviewer did not independently rerun tests or assess live acceptance. The earlier parallel Aulë target-lock admission failure passed in isolation and serial execution but remains unresolved; broader interruption and installed acceptance gates remain open.
- September 8 lock/telemetry follow-up: [Aulë target guards](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs) now explicitly unlock target and read-slot descriptors on drop; the regression retains duplicate descriptors and freshly probes both locks. Independent review supported the Linux open-file-description mechanism, but did not identify the original forked child or establish partial-acquisition cleanup coverage. The separate [telemetry test](../../crates/spine/observability/arda-aule/tests/telemetry_surface.rs) reproduced 54 failures in 200 parallel test processes. Removing OTEL initialization did not eliminate the failure; removing the unsubscribed API smoke emitter did. Its scoped capture subscriber now prevents that test interaction with tracing-core 0.1.36's single-dispatch callsite cache and adds exact span/event count assertions. Existing attribute assertions remain intact. After repair: 300 repeated telemetry runs and four full parallel Aulë suites passed (394 tests each), plus strict Clippy. Independent source review found no blocking issue; this is a fixture repair, not a general tracing-core or collector-delivery fix. Logs: `/tmp/arda-telemetry-repro.log`, `/tmp/arda-telemetry-fixed-aule-{1,2,3,4}.log`, `/tmp/arda-telemetry-fixed-clippy.log`.
- September 8 intermediate-stage fixture: the [isolated subprocess test](../../crates/engine/tests/resident_restart_fixture.rs.inc) now exits with code 75 after the real Harness commits execute, verify or review and returns success, before Workbench receives that response. The test first failed with the old final-only exit code 73. Fresh Harness/Workbench recovery passes all three boundaries, retaining the run ID, original SQLite context binding and existing stage receipt bytes, with three total fake-Hermes invocations and byte-stable post-close event/use/outcome/close evidence. Independent source review found no blocking issue. This proves durable-stage response-loss recovery and post-close resident scheduling suppression, not interruption inside provider mutation, receipt/checkpoint write gaps or direct HTTP replay. The use-ledger prefix assertion proves preservation, not uniqueness of new stage-use entries. The existing timeout helper kills only its direct child; descendant cleanup on a hung-provider path remains unproven. Engine tests (286 passed, 4 ignored), Vairë tests (91 passed, 1 ignored), strict all-target Engine/Vairë/Aulë Clippy and workspace check passed after this test addition; the reviewer did not independently rerun those checks. Logs: `/tmp/arda-intermediate-{red,green,engine-tests,vaire-tests,clippy,workspace}.log`.
- September 9 operator decision: retain full recovery snapshots until explicit authenticated operator deletion, with **no automatic expiry**. [Vairë eligibility](../../crates/spine/memory/arda-vaire/src/service/retention.rs) is separate from memory-record decay and execution expiry/revocation. Unfinished recovery evidence remains protected, including cancelled/failed objectives without complete receipt-backed leaves.
- September 9 retention source slice, not deployed: [Engine control](../../crates/engine/src/objectives/store.rs) and [authenticated private Harness ingress](../../crates/engine/src/harness/operator_messages.rs) implement `arda delete-recovery-context <objective_id> <run_id>`. The immediate transaction requires the exact owner, a terminal objective, all leaves closed against stored receipt digests, no live lease and one exact bound run. It clears only that resident assembly body and retains the request digest, deletion tombstone and control idempotency record. This is logical deletion of the resident snapshot, **not** erasure of RunStore, Vairë records, backups or SQLite forensic remnants. Changed-payload replay fails closed; identical ObjectiveStore replay is idempotent while Oromë retains its existing HTTP 409 duplicate-transport response. The former uncalled marker helper is test-only, not an alternate deletion authority.
- [Schema and marker regressions](../../crates/engine/src/objectives/migrations.rs) reproduced missing-column migration and incorrect SQL parameter binding before repair. [Workbench regression](../../crates/engine/src/objectives/workbench.rs) rejects marked context before adapter dispatch; [real HTTP fixtures](../../crates/engine/tests/harness_operator_messages.rs) exercise private/capability/owner gates, unfinished and live-leased evidence, target binding, deletion, preserved sibling/receipt data and replay. These are isolated synthetic persisted-state fixtures, not installed or real-provider acceptance. Verification: Engine 289 passed/4 ignored; Vairë 92 passed/1 ignored; Aulë full-cli 394 passed; strict all-target Clippy for all three and workspace check passed (existing vendored GLib warnings remain). Logs: `/tmp/arda-retention-{gateway,engine,vaire,aule,clippy,aule-clippy,workspace}.log`. Focused independent source review found no blocking defect; its unsupported claims that Oromë can restore cleared assemblies and that the controls table emits duplicate-transport 409 were rejected against source/tests. The reviewer did not independently run the gates or assess installed acceptance. No restart, live deletion, commit or push occurred.
- September 8 read-only installed/topology check: the [monitoring configuration](../../config/monitoring-setup/centralized-monitoring-config.md) explicitly places Grafana/Prometheus on Beelink and forbids a duplicate workstation stack. SSH, model, Grafana health and Prometheus readiness probes to its configured `100.103.125.88` address timed out; remote deployment and OTLP collector placement remain unverified, not diagnosed as broken. User-systemd owns `arda.service`, PID 1827, with supervised Manwë PID 1911. Installed, running and existing release daemon SHA-256 all equal `88a33157832c2a1281442c3acc95e3dc61419874f142733b71aabee8061cb50a`: none contains this recovery work. Read-only ObjectiveStore counts showed 7 cancelled, 7 completed and 1 failed objective, no active objective. No cutover was attempted; candidate qualification, exact rollback validation and installed/live-provider recovery remain open.
- Installed snapshot-control validation, live provider mutation-count acceptance and qualified deployment remain open. Retry-limit, bounded intermediate-stage recovery and retention controls are source/fixture evidence, not installed acceptance. No service was restarted, no live objective reset, and no commit or push was made. Do not infer full recovery or installed behavior from these checks.

## Resident supervision in progress — not deployed

- September 9 [supervisor source](../../crates/engine/src/supervisor.rs): RED regressions reproduced shutdown lost before waiter registration and worker handles detached when `select!` cancelled a draining join loop. Shutdown now retains its state; joins keep each pending handle until completion; the PID mirror task is joined and refreshed at shutdown. A pre-triggered shutdown suppresses child spawn. Verification: Engine 292 passed/4 ignored; root daemon 8 passed (including five process tests); strict all-target Engine/root Clippy passed. Logs: `/tmp/arda-supervision-{red,green,engine,daemon,clippy}.log`. Focused independent source review found no blocker for file SHA-256 `a103dd46d5035ff1128e454d4e18c526ce05cbd7c60ec1f719529252bc0a0da4`, verified unchanged after review; it did not independently run the tests. This follow-up is uncommitted after `b691a08b`, which captured all previously staged work; subsequent live runtime writes were not folded into that frozen snapshot.
- September 9 ownership follow-up in the [supervisor](../../crates/engine/src/supervisor.rs): RED/GREEN regressions reproduced sibling startup serialized behind a child wait and a pending health request surviving shutdown. Each worker now owns its child locally; health polling is a scoped future dropped before final status publication, not a detached task. Both children start concurrently and are reaped, and a held-open health connection closes during shutdown. The [daemon](../../src/main.rs) registers SIGTERM before background work and owns/joins its signal task. [Binary process tests](../../tests/root_daemon.rs) prove clean SIGINT and SIGTERM exits with the direct supervised child gone. These are temporary-root fixtures, not installed service acceptance.
- The [runtime](../../crates/engine/src/objectives/runtime.rs) now exposes `run_until_shutdown`: stop further rounds, drain the already-started round up to a caller-supplied deadline, then drop pending executor futures while preserving leases/stages for recovery. [Runtime regressions](../../crates/engine/tests/objective_runtime.rs) cover pre-stop/no admission, graceful in-flight completion/no dependent admission, deadline interruption, sibling receipt preservation and reopen/recovery without incrementing the completed sibling's attempt. The API is **not wired into the daemon**: aborting a resident HTTP request does not cancel its Harness provider handler, and [provider output readers](../../crates/engine/src/adapters/hermes.rs) have their own task lifetime. Wiring a deadline before owning that cancellation/reap path would be unsafe.
- Verification for this follow-up: Engine 296 passed/4 ignored; root daemon 9 passed; strict all-target Engine/root Clippy passed. Logs: `/tmp/arda-shutdown-engine.log`, `/tmp/arda-shutdown-daemon.log`, `/tmp/arda-shutdown-clippy.log`. Corrective independent source review found no blocker in this partial ownership/drain foundation; it rejected the initial review's stale shared-child-lock and detached-health-probe findings. Reviewed SHA-256 values were verified unchanged: supervisor `73dc1461f5a8a89ee9c040a702a590d60e683a9be30eaa876830a349bd3cd64c`, runtime `9eb0f6c21d7597fd8b8a85f0e1553c219912ae61adc9e043ce0bd5b75da21461`, daemon `f89583cfd8224ec7744cf1456c7000ea5ce72492aa70d944a043df81e0b5685f`. Review did not establish whole-daemon shutdown or installed acceptance. The remaining shutdown slice is Harness/provider cancellation and joining, preserve nonterminal interrupted evidence, keep execution dependencies alive while draining, and wire the bounded runtime API into daemon shutdown. Whole-daemon bounded stop, stage-by-stage persistence, ingress notification/due schedules, readiness and genuine installed interruption/restart acceptance remain open. No installed service was restarted.

### Subsequent shutdown integration — source tests, not installed acceptance

The previous unwired-API statement describes the preceding checkpoint. The
[daemon](../../src/main.rs) now stops resident admission on SIGINT/SIGTERM,
drains the started round for five seconds, signals and joins Harness, and only
then stops its supervised dependencies. [Harness](../../crates/engine/src/harness.rs)
uses retained shutdown shared with provider handlers and its projection publisher.
Provider interruption awaits adapter cleanup and leaves the RunStore node
nonterminal rather than recording an operator stop as execution failure.

[Adapter regressions](../../crates/engine/src/adapters/hermes.rs) cover retained
pre-cancellation and inherited output pipes after leader exit. Both output readers
are now scoped under the process deadline, not spawned independently.
[Real HTTP fixtures](../../crates/engine/tests/harness_runs.rs) reproduce and verify
provider cancellation/reaping and shutdown with both run and presence SSE streams
open. Engine: 299 passed, 4 ignored; root binary tests: 6 passed; strict all-target
Engine/root Clippy passed. Logs: `/tmp/arda-provider-lifecycle-red.log`,
`/tmp/arda-harness-shutdown-{red,green}.log`, `/tmp/arda-stream-stop-{red,green}.log`,
and `/tmp/arda-drain-wiring-{engine,daemon,clippy}.log`.

Independent review found no concrete blocker in the integration it inspected.
Its final report duplicated the Harness hash for `main.rs`; the transcript's
actual hash matches the parent-verified daemon hash
`b2f88ac323f09ff8ca418d843278fc13c3ba95b9465df55536e04776aeb7b26c`.
This is bounded source review, not whole-plan acceptance.

Subsequent [HTTP regressions](../../crates/engine/tests/harness_runs.rs) verify
provider cleanup after client disconnect and reproduce an incomplete-request-body
shutdown hang. Request-ingestion cancellation now emits an error on stop instead
of waiting indefinitely, without dropping executing provider handlers. All 23
Harness run tests and strict all-target Engine Clippy pass. Focused independent
middleware review found no concrete defect; parent-verified unchanged hashes:
Harness `fedc7c331eee090047ec7c1f0d497b7eb845097e8819827426f37cf9e9e8a5ae`,
tests `8c8d25a924874ab23d8c9060ace45bb4b79b31867a0241ff9a7e4487196368e9`.
The review's trailer-preservation wording is incorrect: the data-only stream
discards HTTP trailers; these JSON routes do not consume them. Cancellation
emits a body error, not successful truncated JSON. Overall HTTP drain
under arbitrary stalled responses, active resident binary interruption/restart,
scheduling/readiness, physical-root qualification, producer freeze, and genuine
installed acceptance remain open. No installed service restart, deployment,
commit, push, or live objective reset occurred in this follow-up.

### Scheduling follow-up — local source verification, not installed acceptance

The [resident runtime](../../crates/engine/src/objectives/runtime.rs) now wakes
from committed store changes shared across independently opened in-process
handles, and selects the next eligible schedule/lease deadline ahead of fallback
polling. [Store regressions](../../crates/engine/tests/objective_store.rs) cover
future-schedule admission blocking, persisted one-shot consumption across reopen,
fixed-duration recurrence advancement, pause/cancel suppression and malformed
recurrence rejection. [Runtime regressions](../../crates/engine/tests/objective_runtime.rs)
exercise dependent completion and due-time waking with a 30-second fallback.
Recurrence wakes an unfinished approved objective; it never reopens a terminal
objective or grants fresh execution authority. Cross-process writes still rely
on fallback polling. Startup reconciliation, runtime readiness, canonical
schedule ingress and installed qualification remain open.

Focused tests, strict all-target Engine Clippy, root daemon tests and workspace
check pass. The first full Engine run failed two existing Python adapter tests
because their PID files were absent at the short startup deadline; the unchanged
adapter target and full Engine rerun passed. Preserve that intermittent result:
`/tmp/arda-scheduler-engine.log`, `/tmp/arda-scheduler-engine-rerun.log`,
`/tmp/arda-scheduler-workspace.log`. The first review's source hashes matched,
but its recurrence and notification objections contradicted the actual SQL and
early-return paths; it is not a blanket approval. A further RED/GREEN regression
exposed a genuine historical-data defect: one unsupported recurrence rolled back
all admission. [Schedule consumption](../../crates/engine/src/objectives/scheduling.rs)
now quarantines that schedule with a durable error, retains its payload, and
excludes it from timers without authorizing its leaves or blocking unrelated
objectives. Reopen/error persistence and a second recurring wake are covered;
22 store tests, 9 runtime tests and strict Engine Clippy pass. The subsequent full
Engine and root-daemon runs also pass (`/tmp/arda-scheduler-quarantine-engine.log`,
`/tmp/arda-scheduler-quarantine-daemon.log`). Bounded corrective review found no
defect in quarantine and recurring-wake eligibility. Its hash report and transcript
contain only abbreviated hashes, so it does not establish exact-source review
qualification; parent-computed full hashes do not repair that missing evidence.
Quarantine is per schedule, not a whole-objective failure transition. Readiness
must expose these schedule errors; that projection is still open. No deployment,
installed restart or commit is claimed.

### Recovery admission ordering — partial startup foundation

The [runtime regression](../../crates/engine/tests/objective_runtime.rs) exposed
fresh provider dispatch while a receipt-only recovery batch was still pending.
The [runtime](../../crates/engine/src/objectives/runtime.rs) now admits no normal
claims in a round containing receipt-only recovery; finished receipts reach
SQLite before subsequent fresh admission. This is not yet the full startup
barrier: below-budget recovery, error retries and
readiness projection still need qualification. The HTTP readiness surface is
not implemented by this ordering fix.

Verification: 10 runtime tests, 22 store tests, strict all-target Engine Clippy,
full Engine/root-daemon suites and workspace check pass. Logs:
`/tmp/arda-recovery-barrier-engine.log`, `/tmp/arda-recovery-barrier-daemon.log`,
`/tmp/arda-recovery-barrier-workspace.log`. Bounded review of the remaining
startup admission gaps returned unsupported race/reclaim claims, which are not
accepted as findings. A targeted regression did reproduce fresh admission while
an unexpired receipt-only recovery lease remained. The store now blocks normal
claims under the same admission transaction while an active objective has
unfinished receipt-only recovery. The regression verifies database reopening,
preserved lease ownership, recovery at expiry, and fresh admission after durable
completion. All 11 runtime and 22 store tests, full Engine/root-daemon suites,
strict Engine Clippy and workspace check pass; the expanded reopen/expiry test
also passes. Logs: `/tmp/arda-recovery-lease-engine.log`,
`/tmp/arda-recovery-lease-daemon.log`, `/tmp/arda-recovery-lease-workspace.log`.
Subsequent below-budget recovery: the expanded ordering regression failed on a
bound attempt below the retry limit. Normal claimed retries now probe completed
receipts before provider-capable continuation; lookup errors preserve recovery
without dispatch, while a successful lookup with no completed outcome permits
same-run continuation. Explicitly unbound retries retain initial-context recovery.
The transactional candidate query excludes fresh leaves while active bound work
remains unfinished. This is a conservative recovery-first policy, not proof that
every interrupted leaf has completed a global startup scan. Regressions cover
completed receipts below/at the retry limit, lookup failure without dispatch,
successful incomplete lookup and stable identity on subsequent reclaim.
All 12 runtime tests and strict Engine Clippy pass; full Engine/root-daemon suites
and workspace check passed after the production changes (logs:
`/tmp/arda-bound-retry-{engine,daemon,workspace}.log`). Bounded review returned
source-contradictory workspace-only and live-recovery-lease race claims: the
fresh exclusion is global, and the live receipt-only lease guard executes inside
the immediate admission transaction. Those claims are rejected, not recorded as
approval of the whole startup policy.

The [resident runtime](../../crates/engine/src/objectives/runtime.rs) now owns a
retained in-process lifecycle observation channel, shared with the existing
[Harness](../../crates/engine/src/harness.rs) by [main](../../src/main.rs).
`GET /v1/objective-runtime` exposes phase, active in-process leaf IDs, next wake,
a bounded error classification, `ready` and `pending_recovery`. Readiness is
scheduler-local and conservative: false during rounds, stop or failed checks;
true after a successful round and a store scan showing no unfinished attempts
in approved/running objectives. Paused and terminal objectives are not runnable
recovery obligations. Unknown counts remain null, not zero. Live unexpired
leases, below-budget and exhausted retries cannot be hidden by empty claims.
A missing/closed owner returns unavailable
(503); cancellation clears active observations without deleting durable leases.
`/health` remains liveness only. `waiting` means no current round, NOT startup
reconciliation complete or automation ready; inspect the explicit verdict.
Provider readiness, schedule-quarantine diagnostics and whole-startup readiness
qualification remain open. Readiness additions pass full Engine/root-daemon
tests, strict Engine Clippy and workspace check (`/tmp/arda-readiness-engine.log`,
`/tmp/arda-readiness-daemon.log`, `/tmp/arda-readiness-workspace.log`); tests include
store loss/recovery, live-lease reopen, receipt-service failure and compiled-daemon
HTTP projection. Bounded review's two proposed counterexamples do not demonstrate
defects: the post-round scan covers live leases and subsequent rounds re-evaluate
readiness. The report omitted requested hashes and is not exact-source approval.
Follow-up source inspection found genuine premature-readiness paths: timer checks
ran after completion publication, and a draining round could briefly publish ready.
Both regressions failed before correction and pass afterward. Timer checks now
precede the single completion update; draining remains not-ready until stopped.
Full Engine/root-daemon tests, strict Engine Clippy and workspace check pass after
the correction (`/tmp/arda-readiness-atomic-engine.log`,
`/tmp/arda-readiness-atomic-daemon.log`, `/tmp/arda-readiness-atomic-workspace.log`).
No deployment occurred; whole-system and installed readiness remain open.

Verification: runtime idle/retained-stop and Harness shared-channel/owner-loss
regressions were RED then GREEN. Extended tests cover active sibling removal,
bounded interruption, lookup-error projection, and the compiled daemon's HTTP
projection before SIGINT/SIGTERM cleanup. Full Engine/root-daemon tests, strict
all-target Engine Clippy and workspace check pass after these changes. Logs:
`/tmp/arda-resident-status-engine.log`, `/tmp/arda-resident-status-daemon.log`,
`/tmp/arda-resident-status-workspace.log`. No deployment or installed restart occurred.
Bounded lifecycle review identified separate phase/error publications; round
completion now publishes them under one watch update, with error/reset coverage.
The alleged skipped wrapper finish and undetected closed channel contradict the
source and passing shutdown/owner-loss tests; they are not accepted findings.
Post-fix status unit, objective-runtime/Harness suites, root-daemon tests and
strict Engine Clippy pass (`/tmp/arda-status-atomic-tests.log`,
`/tmp/arda-status-atomic-daemon.log`). This does not close readiness qualification.

## Historical audit boundary — before the uninstalled changes above

- `crates/engine/src/objectives/workbench.rs` derives the run ID from `claim.attempt` and constructs fresh context on execution. The claim path increments attempts; that counter alone is not an original-run binding after recovery.
- `crates/spine/memory/arda-vaire/src/service/context_capsule.rs` returns the full `ContextAssembly` but persists its use receipt, not the memory projection/capsule body. `context_use_receipt` retrieves that receipt, not the original assembly. Do not reconstruct historical content by recalling current memory.
- The inspected provider path (`crates/engine/src/harness/runs.rs` and `crates/engine/src/adapters/hermes.rs`) passes the assembly in the request/prompt and binds execution receipts to its ID/digest/use receipt. This is not an identified durable full-assembly recovery API; a redacted worker export is not a canonical replacement.
- Reject two overbroad audit conclusions: `context_outcome.rs::record_context_outcome` explicitly persists and branches on `Used` versus other dispositions; `runs.rs::validate_durable_context_assembly` already validates expiry, digest, durable use receipt and run/project/consumer bindings. Preserve those checks. Reading completed evidence must not implicitly authorize a fresh provider execution with expired or revoked context.
- Next implementation must bind run identity and original assembly before dispatch, reuse existing Engine/Vairë authority, retain explicit legacy-data ambiguity, and cover crash-before-dispatch and receipt-before-SQLite-projection with isolated failpoints. Any persisted memory snapshot also needs retention/revocation handling; do not add an unmanaged second memory store. This dependency is unresolved, not a request to reset existing objectives or bypass context validation.

## Required invariants retained from the cutover contract

- Indexed objectives/projects/leaves/dependencies/schedules/control idempotency and stage receipts; transactional claims, WAL, foreign keys and compatible migrations.
- Exact reviewed project, workspace, authority, budget, approval revision and predecessor bindings; no cycles or post-start revision mutation.
- Persisted leaf-attempt limit (`MAX_OBJECTIVE_ATTEMPTS=5`) enforced during claim after lease expiry, not a daemon lifetime counter or mutation on projection-store open.
- Claim-returned dependency close receipts cross adapter/Workbench review without substitution; every terminal root binds all required leaf closure evidence.
- Preserve successful siblings when another leaf fails, terminate/recover within bounds and never detach untracked provider work.
- Existing retired queue-executor templates/installed units remain retired; no replacement one-shot worker or second daemon. `arda.service` is the sole objective lifecycle owner.
- No migration of historical JSONL queue records, remote/fleet expansion, unrelated credential/service changes or invented acceptance fixture. Keep canonical RunStore receipts and Vairë memories intact.

## Verification and retirement

Use focused RED/GREEN regressions, then `cargo test -p arda-engine`,
`cargo test -p arda-aule --features full-cli`, corresponding strict all-target
Clippy, release daemon/CLI builds, source/install hash verification, and genuine
installed control/restart/replay checks. Validate active-plan links and inspect
scoped Git diffs; exclude pre-existing Hermes adapter/HUD edits and runtime data
from any runtime-only commit. Source tests and successful installation are
checkpoints, not the complete cutover acceptance verdict.

Historical phase instructions remain in Git history and the [evidence index](../audits/autonomous-task-completion-history.md). Retire this active file only after its remaining cutover invariants and installed gates pass; product acceptance continues under the milestone owners.
