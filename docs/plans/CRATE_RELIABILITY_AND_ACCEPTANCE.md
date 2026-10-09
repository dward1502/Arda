---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "execution_plan"
  owner: "ARDA"
  status: "active"
  reviewed: "2026-10-08"
---

# Arda reliability tasks and next session handoff

Start with R01, a fresh diagnosis of installed readiness, and R02, the source
validation baseline. Then establish continuous checks before advancing the
existing connected acceptance scenario. This list was requested by the operator
after the [crate assessment](../audits/2026-10-05-crate-assessment.md).

This is the ordered remediation list for those findings. The
[whole-system program](ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md) retains product
Gates 1–6; the [M1–M5 owner](autonomous-task-completion/README.md) retains connected
acceptance. Links below reuse those gates rather than redefine them. Progress
on supporting code does not close installed or operator acceptance.

## Joined source review completed, October 6, 2026

The operator explicitly authorized discarding the failed join. Only its failed
run artifacts and empty trailing usage entry were cleared while the root writer
was stopped. The existing approved plan recreated the join; completed projects
and ObjectiveStore admission were retained. An additional prompt fix projects
excluded-memory IDs as a count/digest while retaining the full selected content,
original capsule and exact authority binding. Engine tests 214 passed / 9 ignored;
adapter contract tests 31 passed / 14 ignored; strict Clippy and root build passed.

Objective `operator-objective-c98d83591ddab744` is now durably **completed**.
Execute/Verify/Review/Close succeeded, Review returned `VERDICT: APPROVE`, and
both complete dependency reports were verified in the actual worker prompt.
The two completed project runs were not replayed; all original project receipts
and both workspace baselines match. Mesh-llm remains off. Exact evidence:
`target/qualification/m4-join-discard-20261006/manifest.json` and `after.json`.

M4-specific closeout checks now pass: installed restart non-replay, exact declared
operator package checks, dirty staged/unstaged/untracked preservation, concurrent
symlink isolation and the separately invoked bind-mount fixture. The shared Git
cleanliness policy is now enforced before fresh resident mutation claims; blocked
candidates do not stop eligible readers. Scheduler/store tests: 68 passed, 2 ignored
(with the bind-mount case separately passing); Aule: 391 passed; strict Clippy,
formatting, root build and diff checks pass. The root fix is installed.
Evidence: `target/qualification/m4-closeout-20261006/manifest.json`.

**M4 is now closed for the admitted bounded source-review scope.** Its resident
M3 prerequisite is implemented, installed and live-qualified: real semantic
rejection, authenticated immutable correction, fresh approval/execution,
verification, independent acceptance and terminal close. Exact-event replay and
root restart preserve both chains, context, provider sessions and one memory
outcome. Four attempts were consumed under the inherited five-attempt cap,
including two process-timeout resumptions. All original M4 results and real
workspaces remain unchanged. Evidence:
`target/qualification/m3-critic-20261006/manifest.json` and the M4 owner's
`restart-m3-final.json`. Mesh-llm remains off; provider selection remains in the
profile rather than model-specific source code.

M5 continuity qualification advanced October 6–7: a repeatable read-only audit
matches nonempty join memory to accepted worker prompts, checks correction
lineage and unique outcomes, and proves installed restart preservation plus
fresh authenticated local-session status retrieval. Vaire: 99 passed / 1 ignored;
Engine context/restart: final rerun 12 passed / 5 ignored, with one earlier
non-reproducing empty-claim failure retained. The [M5 owner](autonomous-task-completion/M5_VAIRE_CONTINUITY.md#qualification-progress--october-67-2026)
records exact evidence and limitations. Genuine messaging continuation, decision
correction and explicit operator acceptance remain open.

October 7 continuation: the operator chose the local CLI and delegated the work.
Two fresh interactive sessions used persisted CLI provenance, recovered all three
complete saved review outcomes across an installed restart, and narrowed the
follow-up to WGTT authorization regression-test design. Original M3/M4 state and
both projects stayed unchanged. This qualifies bounded conversational planning;
it does not execute fixes, mint resident authority or supply the operator verdict.
See the [interactive M5 record](autonomous-task-completion/M5_VAIRE_CONTINUITY.md#interactive-local-cli-continuation--october-7-2026).

October 7 operator verdict: the demonstrated CLI process is accepted; see the
[M5 verdict](autonomous-task-completion/M5_VAIRE_CONTINUITY.md#operator-verdict--october-7-2026).
R05 source separation is complete; continue at R06 provider/communication qualification. Gate 1's
combined outcome and remaining M1/M2/M5 technical obligations remain open. Source
findings still require remediation; real-project worker native-check execution
and staging acceptance are not established by this bounded review.

## Earlier join evidence repair, October 6, 2026 — superseded

R04/M4 dependency transport now supplies every admitted completed report to join
Execute, Verify and Review, validating the full canonical chain from each close
digest. Project-local memory filtering is unchanged; no unrelated report access
is granted. Aule tests 391 passed; Engine tests 213 passed / 9 ignored; strict
library Clippy and native root build passed. The root fix is installed.

The first authenticated resume stopped because its failed join receipt bound the
incomplete context. This was a historical intermediate result. The later explicit
operator-authorized discard and successful joined closure above supersede that
blocker and the proposed failed-Execute recovery task.
See the [M4 owner](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md) and
`target/qualification/m4-join-20261006/manifest.json` for exact results and evidence.

## Next session starting point

Repository: `/var/home/mythos/Eregion/Arda`; expected branch: `main`. The assessment
source baseline is `f5c8a361`; inspect the current HEAD and dirty work before changes.
Read `AGENTS.md`, this file, the assessment, and the exact owning task document.
Preserve operator workspace files, SQLite state, histories and project work.

R01 readiness repair and R02/R03 source qualification have recorded evidence below.
R03 remains on its qualified branch; merging and required-check enforcement are
separate actions. Bounded source-review and join evidence is retained above.
M4 and its current resident M3 prerequisite are closed for the admitted bounded
scope. The operator accepted the bounded M5 CLI workflow on October 7. R05 source
separation is qualified. [R07 measurement qualification](../operations/retrieval-quality-qualification-2026-10-08.md)
records retrieval, generated-answer and crawler results; R06 canonical budget/fallback and installed
route-attestation acceptance remain open, as do R04 combined-scenario obligations.
Keep mesh-llm off and preserve both completed project runs and operator workspaces.

Suggested next-session request:

> Execute R09 state lifecycle and installation qualification, then R10 and R11
> in dependency order. Start with ownership classification and consistent backup
> and restore on copies; preserve live identity/history and separate development
> composition from packaged startup. R08's bounded acceptance is recorded below.
> Preserve accepted M5 CLI and M3/M4 evidence, keep R06 and combined-scenario gaps
> explicit, leave mesh-llm off, and preserve operator state.

## Ordered task list

### R01 Diagnose installed readiness

- [x] **Priority P0 reproduced; owners: root, Engine, Manwe and Varda.**
  Recheck systemd state, restart counts, actual listeners and configured health
  endpoints. Inspect bounded recent logs and installed binary/config identities.
  Distinguish root failure, crawler restart, inference probe failure and optional
  exporter inactivity; do not infer one shared cause.
- [x] Select and verify the narrow repair after diagnosis. Re-probe readiness and
  observe for a documented interval longer than the configured restart delay.
  Exercise a harmless supported request without resuming consequential work.

Exit evidence: exact cause or bounded unresolved hypothesis, repair diff if any,
binary/config identities, successful readiness and request results, observation
interval and restart counts. Stop at a genuine authority boundary; this checklist
does not authorize historical database mutation or a new objective execution.

### R02 Establish the source validation baseline

- [x] **Priority P1; owners: Vaire and workspace maintainers.** Repair the
  `OrganismContext` initializer in `crates/spine/memory/arda-vaire/tests/context_capsule.rs`
  to reflect intended `excluded_refs` semantics. Inspect related initializers.
- [x] Run the focused test, then root workspace all-target validation. Triage any
  newly exposed failures separately. Record command, commit/tree identity,
  features, result and ignored tests; do not substitute old test totals.

Execution and triage recorded below; full workspace green status remains blocked
by native prerequisites and the recorded Clippy findings.

Suggested commands when machine resources permit:

```sh
cargo test -p arda-vaire --test context_capsule -j 2 -- --test-threads=1
cargo check --workspace --all-targets --all-features -j 2
```

Exit evidence: focused test and workspace target compilation pass, or explicit
owned blockers. Establish the broader test/Clippy baseline required by R03;
a successful check alone is not a passing test suite.

### R03 Add continuous validation

- [x] **Priority P1; owner: repository CI. Depends on R02.** Add bounded source
  checks for Rust compilation/tests and relevant formatting/Clippy, preserving
  documentation health and release provenance checks as separate guarantees.
- [x] Define jobs for root workspace, HUD native/frontend, Mirromere
  native/frontend and relevant shared UI. Document platform prerequisites and
  feature coverage; isolate tests requiring credentials, GPUs or managed services.
- [x] Verify the workflow on a real change. Inspect remote required checks and
  branch rules before proposing enforcement; local YAML alone does not prove it.

Exit evidence: reproducible commands, CI results and explicit coverage/skip
matrix. Root workspace success must not imply separate native-app success.

Implementation and coverage are recorded in
[source validation](https://github.com/dward1502/Arda/blob/ee6d0d1833824b3a163ba96c6711d0a76bc222c4/docs/operations/source-validation.md). The GTK prerequisite
was resolved on October 6 using a reusable Ubuntu 24.04 build image with GTK 3,
WebKitGTK 4.1 and protoc. The full root workspace, including launcher, passes
all-target/all-feature compilation; the earlier launcher exclusion is no longer
the current compilation baseline. The implementation branch is
`reliability/r03-native-ci`; installed services and operator databases remain
outside this source-validation work.

On October 6, the [final remote run](https://github.com/dward1502/Arda/actions/runs/37438628947)
passed all eight jobs at source revision `ee6d0d1833824b3a163ba96c6711d0a76bc222c4`:
root including launcher, both separate native workspaces, Rust formatting and
all four frontend/shared-UI jobs. Full local root tests passed **2,378 / 0 failed /
61 ignored**; full strict Clippy passed. Declared ignored tests remain opt-in.
The earlier third run was canceled by the final source push; it is not acceptance
evidence. Main protection was inspected: one approval, code-owner review and
conversation resolution, with no required status checks and no rulesets. No
protection settings changed. R03 implementation and branch qualification are
complete; merging and status-check enforcement remain separate actions.
Reproducible commands and coverage/skip distinctions are in the linked operations
record; raw logs and hashes are under `target/qualification/reliability-20261006/`.
That R03 qualification did not establish installed R01/M4 recovery. Subsequent
M4 closeout is recorded above; mesh-llm remains off.

### R04 Finish the connected acceptance scenario

- [x] **Priority P1; owner: Engine with Hermes/Orome and project adapters. Depends
  on relevant R01–R03 results.** Close the admitted bounded
  [M4 scope](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md), including installed
  restart/isolation and current resident M3 rejection/correction acceptance.
- [ ] Complete the remaining [M1–M5 requirements](autonomous-task-completion/README.md)
  using the genuine authorized outcome, real reviewed projects, durable recovery,
  critic revision, relevant consumed memory and explicit operator acceptance.
  Preserve existing approvals and completed effects; do not replay them to
  manufacture evidence or treat fixture success as installed qualification.

Exit evidence: the existing G1 owner records accepted lineage and checks, restart
non-replay, useful artifacts and the operator verdict. Unresolved provider
readiness timeouts return to R06 and its existing provider owner.

### R05 Separate execution from observability

- [x] **Priority P2; owners: Engine and Aule.** Map live callers, authority,
  persistence and feature requirements of the explicit Workbench adapter.
  Preserve behavior with meaningful boundary/recovery tests before changing it.
- [x] Establish a narrow execution contract and move implementation ownership out
  of observability where justified. Preserve Engine admission/workspace controls
  and receipts; retain metrics as observers. Do not simply rename a large module.

Exit evidence: Engine depends on a documented executor boundary, observer changes
do not require execution internals, and recovery/authorization regressions pass.
Follow the AGENTS requirement to refactor the boundary before changing behavior.

October 7: the [boundary map](../operations/workbench-execution-boundary.md)
records live callers, stores and feature ownership. The existing 85 executor
tests passed before extraction. A narrow `execution-contract` feature now exposes
the shared work/authority/executor interface without telemetry or provider-owner
dependencies; the concrete trait implementation moved beside its adapter.
Compatibility exports and admission behavior are preserved. Final qualification:
391 Aule and 216 Engine library tests passed (9 ignored), 12 restart-continuity
tests passed (5 ignored), and 3 contract-only tests passed; strict library Clippy
and formatting passed. That first step retained concrete execution in Aule.

October 7 executor extraction: `arda-workbench-executor` now owns explicit HTTP,
reconciliation, Close and admission contracts. Legacy queue scheduling stays in
Aule and consumes shared graph/receipt/project helpers. Engine imports the new
executor directly and no longer requires Aule's `full-cli`. Dependency inspection
shows the executor has no Aule, OpenTelemetry, Manwe or Varda dependency.
Working-tree regression results: 364 Aule, 216 Engine, 27 executor and 12 restart
tests passed (14 ignored); contract-only and compatibility tests each passed 3.
Clippy/formatting passed. The exact staged source compiled separately and passed
25 executor plus 3 contract tests. These results qualify the R05 source boundary;
installed deployment and the R04 combined acceptance scenario remain separate.

### R06 Qualify providers and communication

- [ ] **Priority P2; owners: Manwe and Orome.** Extend the existing
  [provider convergence](PROVIDER_WORKER_CONVERGENCE.md) matrix with actual tool
  use, streaming, timeout, quota/failure, fallback and budget observations.
- [ ] Test real-channel duplicates, cancellation, reconnect and acknowledged
  delivery. Separate manual/compatibility transport from proven live delivery.

Exit evidence: actual route/transport identity, success and latency, retry cost,
bounded failure and no unauthorized fallback. Use LiteLLM/A2A comparisons as
references, not assumptions that current mechanisms are absent or compliant.

October 7: the [R06 qualification](../operations/provider-qualification-2026-10-07.md)
now includes the installed health repair, three repaired Hermes auxiliary URLs,
pinned subscription tool use and emulated streamed-tool replay, native free-route
streaming, and an actual Hermes auxiliary-client completion. Final health/catalog
checks pass; the root remains running and mesh services remain off. Local inference
is unavailable. R06 stays open for canonical budget/fallback, upstream failure/retry
and real-channel evidence. The [storage audit](../operations/storage-retention-2026-10-07.md)
records larger cache/Git consumers, retention gaps and two malformed Manwe records;
no historical data was deleted.

October 7 follow-up: the [failure/privacy qualification](../operations/provider-failure-qualification-2026-10-07.md)
exposed and repaired dropped HTTP privacy selectors. The installed repair passes
306 library tests and all 13 isolated timeout/quota/fallback/privacy scenarios;
production health and local-only refusal checks pass. Canonical G2 remains open.
Next trace admitted placement requirements and actual adapter receipts (P2/P3),
then qualify canonical budget/privacy/fallback; preserve the current M3/M4/M5 evidence.

The subsequent [P2/P3 verification](../operations/provider-authority-verification-2026-10-07.md)
passed 39 executed tests across adapter, broker and retained admission. It found
that upstream route attestation is not persisted in typed canonical receipts and
unknown billing is not a proven spending cap. The subsequent
[durable route receipt implementation](../operations/provider-route-receipts-2026-10-07.md)
adds broker-owned evidence to versioned receipts while preserving historical
digests. Installed Engine/worker rollout, live attested acceptance and the
canonical budget/fallback gate remain open; keep canonical G2 open.

### R07 Measure memory and research quality

- [x] **Priority P2; owners: Vaire and Varda.** Extend existing retrieval evaluation
  with representative authorized questions and known relevant sources. Compare
  lexical/BM25 and only justified alternative retrieval methods on the same data.
- [x] Record selected/excluded context, scope, budget and actual consumption;
  test correction/revocation and cross-session continuation through existing M5.
  Evaluate citations, source faithfulness and degraded crawler behavior.

Exit evidence: versioned dataset and reproducible retrieval/answer scores,
consumed-context evidence and correction/privacy regressions. A vector database
is not a prerequisite; choose changes from measured deficiencies.

[October 8 R07 measurement qualification](../operations/retrieval-quality-qualification-2026-10-08.md):
12 frozen repository excerpts / 24 agent-labeled questions compare lexical and
production BM25 retrieval. Evaluator cutoff/duplicate-recall defects and plain-text
source coverage were repaired. BM25 Hit@1 rose from 0.375 to 0.708333; Recall@3
from 0.354167 to 0.958333. Lexical Hit@1 remains better at 0.916667.

The retained real-model answer experiment scores 23/24 corpus answers correct;
both unsupported controls abstain (25/26 overall). All 26 answers were judged
source-faithful by the parent agent and their exact citation quotes were checked
programmatically. The remaining corpus miss lacks the required boundary source
in top-three retrieval; its honest abstention is retained. These are agent labels
and judgments, not independent human ground truth or general quality claims.

242 source tests pass under default parallel execution, 2 live tests are ignored;
strict Clippy and the retained M5 audit pass. Independent code review found no
blocking defects after the benchmark repairs; its test-only server timeout nit
was addressed and checks rerun. Four local HTTP degradation cases confirm visible
failure activity and no partial-text indexing at the fetch boundary. Existing
M5 consumption evidence and current scope/budget/correction/revocation regressions
supply the continuity boundary; no new context-use receipt is claimed.

The isolated commit candidate passes 237 tests (1 ignored) and strict Clippy;
its only prerequisite is the missing empty `excluded_refs` test initialization.
Unrelated working-tree changes are excluded. No installation occurred.
Do not introduce vector storage from
this small corpus, migrate historical Books, or promote this offline measurement
to installed crawler/answer-service acceptance. Versioned corpus, model input,
answers, rubric and judgments are linked in the qualification report. Local raw
logs and current source hashes: `target/qualification/r07-20261008/manifest.json`.
R08's bounded acceptance is recorded below; R06's separately recorded live gates remain open.

### R08 Verify governance and reasoning influence

- [x] **Priority P2; owners: governance, Mandos and effect-owning callers.** Trace
  allow/deny/advisory results through actual execution boundaries. Test expired,
  missing or invalid authority and unavailable scorers without bypass effects.
- [x] Retain understandable policy/version/result evidence with appropriate
  redaction. Compare advisory judgments against human-labeled outcomes and
  quantify false approvals/rejections under existing G6.5.

Exit evidence: enforcement at each reviewed effect boundary, explainable decisions
and measured judgment quality. Advisory scores must not mint execution authority.

Current evidence: [R08 qualification](../operations/r08-governance-qualification.md)
contains the boundary matrix, reproduction commands, source hashes and frozen
judgment experiment. MCP protected-dispatch repair `5ccdea23` and Varda retained-
evidence repair `a9153029` are committed. The latest continuation is uncommitted. Earlier boundary reviews found no
remaining blockers within their scopes; the resident integration and its
additional installed changed-proposal evidence are described below.
The first row is closed for the reviewed source boundaries, not installed-runtime
acceptance. The bounded resident integration is now installed; global blocking
remains unchanged.

| Requirement | Current evidence | Remaining |
|---|---|---|
| Reviewed effect boundaries | Orome protected-dispatch refusal; 45 layered Engine authority/lease/replay checks; Varda evidence-unavailable refusal and quarantine/landing control | Source qualification reviewed; no single end-to-end canonical negative matrix claimed |
| Unavailable scorers | Manwe loopback HTTP and streaming dispatch exercise timeout/unavailable/backend errors; advisory status retained and provider exclusion sends no request | Production asynchronous receipt durability is outside this source-test claim |
| Explainable redacted decisions | Varda safe-description adapter preserves full scoring and correlated policy/rationale; source-text leak reproduced then repaired | Adapter and regression reviewed; historical logs not rewritten |
| Human judgment quality | Eight operator-labeled scenarios: four accept/four reject; false approvals 0/4, false rejections 0/4; all eight outputs conditional, decisive coverage 0/8 | Measurement complete; useful reasoning influence remains at G6.5, not established by zero error rates |

Current checks: 651 Rust tests pass (1 ignored), 2 example tests, 8 Python metric
tests, strict all-target Clippy for Manwe/Varda/governance/Mandos. The working tree
contains concurrent changes; these counts are not an isolated release-tree claim.

Operator labels and verbatim explanations are retained and the comparison ran
successfully. The operator rejected measurement-only closure, requiring the
operational caller to consume judgments and change a proposal/action. The
empty-context, fresh-engine baseline remains diagnostic only.
The resident provider route now consumes shared, persisted Mandos advice and
passes it to Hermes without granting authority or adding recurring approvals.
A real-provider source assessment consumed conditional advice, inspected source,
and distinguished proceeding within authority from revising private publication.
Independent review found neutral-evidence inflation, premature advice retention,
and legacy Verify replay drift. Further review found typed transport limits
checked after retention and indirect neutral-evidence confidence inflation.
These were also reproduced and repaired; scoped Engine/Mandos tests and strict
Clippy pass. The correction review confirmed both semantic repairs and found a
Linux-only field guard, now applied. The release root is installed and a real
Hermes source-assessment run consumed the resident service's conditional advice;
state, advice and its single verdict survive a root restart. The keeper remained
running. [Installed evidence](../operations/r08-resident-runtime-evidence.json)
records artifact identity and the bounded control. A completed-node POST retry
is refused by scheduler admission; no successful POST replay is claimed.
Independent review accepted the integration but found the prescribed source
question insufficient to prove a changed proposal. The follow-on installed run
`r08-installed-proposal-revision-20261008` supplies that missing assertion: after
conditional advice and actual source inspection, Hermes replaced the concrete
candidate to label all authorized memories Supporting/Independent with selective
claim- and provenance-backed classification while preserving Neutral/Unknown
defaults. The request prescribed no filename or verdict. The retained receipt
correlates original proposal, judgment, evidence and revision; authoritative
readback matches, one verdict exists, and no project source was modified.
R08's bounded operational continuation is complete; [acceptance and limits](../operations/r08-governance-qualification.md).
No more label input is needed. General G6.5 usefulness/calibration, R06 live gates,
successful terminal POST replay and interrupted-worker recovery are not claimed
by this evidence. No new implementation, publication or human approval was
authorized by the judgment. R09 remains a separate numbered assignment.

### R09 Clarify state lifecycle and installation

- [x] **Priority P2; owners: persistence modules, root and launcher.** Inventory
  paths as shipped inputs, mutable private state, retained evidence, cache or
  test fixtures. Define backup/export/redaction/retention and migration ownership.
- [ ] Qualify consistent backup and restore on copies, then clean installation,
  upgrade/rollback and truthful native readiness under existing
  [launcher acceptance](../operations/launcher-local-runtime-acceptance.md).
  Separate development composition from packaged startup.

Exit evidence: reviewed ownership table, proven restore/rollback and preserved
identity/history. Do not remove tracked state or migrate live databases merely
because they look generated.

October 8 R09 in progress: [state lifecycle ownership](../operations/r09-state-lifecycle.md)
classifies shipped inputs, private state, evidence, caches and fixtures and assigns
backup/export/retention/migration boundaries to existing owners. The first copy-only
regressions reproduced a hash/archive race and failure to reinstate the last
root displaced during restore. Both are repaired in the existing beta-operations
path; 22 Python tests pass, including SQLite integrity/history and opaque marker
byte preservation, source-change refusal, and install/upgrade/rollback fixtures.
Read-only readiness accepts the normal private `/run/user/<uid>/arda` path only
for the runtime role, current HOME and a current-user-owned mode-0700 parent.
Independent review caught reset cross-device and aliased-root regressions; the
exception is now readiness-only, with reset refusal before mutation and config/data/
cache/operation-state alias regressions. Mutation commands retain the original
external-root refusal. Isolated HOME and other external roots remain refused. Live readiness correctly reports missing endpoint configuration;
with the explicitly supplied, HTTP-200/`ok=true` loopback Manwe endpoint it reports
12 passing checks, including installed native libraries. This is installation
preflight, not native-window, current-artifact or runnable authority-restore
acceptance. No live state migration, authority rewrite, service restart or mesh
activation occurred. Raw bounded results: `target/qualification/r09-20261008/`.
R09 remains open; R10/R11 have not advanced past this dependency.

The bounded repair and ownership slice is committed as `6b66ee99`, independently
reviewed against tree `4d92367cd177cb768247f19806af695cf8c726d9`; all 22 tests were
repeated from exact exported blobs. The operator authorized a dedicated clean
Bluefin account. Native PolicyKit authentication succeeded after noninteractive
sudo refused: `arda-qualification` is UID/GID 1001, with private mode-0700 home
`/var/home/arda-qualification` and no supplementary groups. At that stage no
graphical login or runtime installation was established. Sanitized preinstall
readiness correctly fails for missing release, launcher, development tools and
provider configuration (`clean-account-preinstall-readiness.json` in the evidence
directory above); the earlier sudo blocker is superseded. The operator session
remains active and mesh remains off.

An isolated build-source export from `6b66ee99` passes launcher Cargo check,
20 frontend tests and the production frontend build. The export initially omitted
the source `scenes/state/OnboardingText.tsx`; restoring that exact committed source
fixed the export mistake without copying private state. The native release build
with embedded frontend (`tauri/custom-protocol`) succeeded. Its exact artifact
SHA-256 is `e4bf57dcf08321f92a3fd42f6c7fc8d18b82f269f8fc0bd36eef03d3816bc3b1`;
installation in the dedicated account and installed-hash readback succeeded.
Sanitized readiness now passes the native-library and installed-artifact checks
but truthfully reports missing provider configuration and development tools. The
installed process was observed `active/running` in the account's isolated Wayland
session under a bounded transient smoke unit. A retained repeat observation and
clean stop are in `native-process-observation-retry.log`; its first attempt found
the expired qualification compositor and did not start the launcher. This is native process startup,
not visual, complete first-run or operator useful-outcome acceptance. Launcher
native tests pass 40/40 after adding three exact committed workspace test inputs
(`services.toml`, provider configuration and contract registry) only to the isolated
test export, never to account state. Initial 36/40 evidence is retained; omitted
test inputs were an export error, not four repaired product regressions.
Build identities/logs are in `build-source.json`, `native-test-inputs.json`,
`clean-source-native-build.log`, `native-tests-with-declared-inputs.log` and
`clean-account-candidate-install.log` in the evidence directory. Scratch exports
and the account's minimal declared inputs are not full release distributions.

A subsequent regression exposed binary rollback replacing current stores from an
archive: it could discard post-backup history, replace database inodes and install
the older artifact before noticing an invalid archive. The committed repair validates
archive payloads before installation and preserves all current config/data in place;
state/schema restore remains with the persistence owner. Both new tests failed on
the old code. Independent review accepted the bounded repair and exposed a prior
manifest-validation ordering defect; malformed saved manifests now fail before
installation. State-preservation fields are persisted, and a valid archive with
corrupted payload has explicit non-mutation coverage. All 25 Python tests passed
from exact exported source; independent review approved tree
`4fad48041ea368eff5e48ab16041dfc5e898c8c7`, committed as `417205f1`.

Real account install/upgrade/rollback exposed a second-granularity rollback-directory
collision on immediate re-upgrade. A frozen-clock regression reproduced it; the
pending repair uses atomic unique directory allocation, preserving prior receipts.
All 26 Python tests pass. The repaired account lifecycle now completes prior install,
candidate upgrade, prior rollback and final candidate upgrade with exact installed
hash readbacks (`clean-account-lifecycle-fixed.log`). The account has zero durable
Arda state files: this proves artifact lifecycle, not terminal-history acceptance.
The directory-allocation repair is independently reviewed and committed as
`4ffe33e5`; all 26 tests passed from its exact exported source.

The next source audit found that native command root discovery ignored `ARDA_ROOT`.
The launcher correction respects explicit command root, then the documented
environment root, without silently falling back from a selected incomplete release.
Its subprocess regression failed before the repair; 41 launcher native tests passed
in the isolated build-source export.
The environment-root repair is committed as `1b83ea38`; its native release build
succeeded. Desktop persistence is committed as `4ea15541`: the
existing install manifest records the absolute release root, and native discovery
reads it after explicit/environment overrides. A relocated native test executable
selects the separate release with unset and empty `ARDA_ROOT`, rejects malformed
records without selecting a decoy ancestor registry, and preserves legacy discovery
when the manifest is absent. Both Python producer and native consumer regressions
failed before repair; 26 Python and 42 native tests passed at that checkpoint.
The source repair is retained; complete native lifecycle and runnable
authority-preserving restore qualification remain open. Gateway watchdog
integration and backup-failure refusal are committed as `f580a4bb`; incomplete
rollback now preserves persistent recovery backups in `6c20c367` (five isolated
installer tests and independent exact-tree review). These do not close R09.
A separate qualification account shares host loopback; the original runtime owns
port 7171 and Harness defaults to 7878. Do not count that runtime's health as
clean-account acceptance or start competing fixed-port services. Repeated copy
restores preserve distinct displaced histories (`747009a0`, 27 tests). Account-local
root composition, root/HUD/Mirromere identity, mandatory root verification and
root-unit rollback are committed as `2c01e9f8`; installed qualification remains open.
Launcher endpoint selection is committed as `83d1180c`: the selected release's
`services.toml` supplies its exact loopback health endpoint; invalid configuration
cannot fall back, and HTTP proxies/redirects are disabled. Independent review
approved this bounded slice. Exact-tree export `2816d148eead31d14caeaf136310be6e4ae451e2`
passes 44 launcher and 117 core tests, including healthy-old-endpoint rejection and
real selected-port/redirect HTTP probes. No live services or state were changed.
The previously observed sudo/cross-user systemd access limitation is not resolved
by these source tests. Independently reviewed commit `94aed0c7` captures the
endpoint from the registry's loaded bytes, wires it to the root Harness client,
and disables proxy/redirect routing for root and supervisor HTTP. Exact exported
source passes 11 registry, 9 supervisor, 8 root daemon and 74 Harness tests (two
Harness tests ignored). The retained root
shutdown fixture correction is included so this evidence does not depend on dirty
local tests. Root process fixtures verify selected-port
models routing despite proxy environment variables; the supervisor redirect
regression reproduced false healthy status before repair. Retained snapshot-worker
inference now carries the release-selected endpoint through an explicitly admitted
readonly NetworkInput manifest descriptor into the broker and Rust/Python guards
(independently reviewed commit `05623ba3`; not installed). Capture caches the
endpoint once under existing object-identity authority; the runtime bundle digest
does not attest manifest bytes. Chat/export without that authority refuse, while
probe/artifact verification remain compatible. The generator publishes matching
profile/manifest bytes and refuses conflicting existing output rather than migrating
it. Worker tests: 23 passed / 1 ignored; endpoint/parser parity: 4 passed; provisioning:
5 passed; Python bootstrap/transport: 8 / 3 passed, including HTTPX default-port
normalization (the initial independent review finding). The synthetic production-broker
fixture exercises nondefault HTTP forwarding and post-capture mutation isolation;
it is not installed keeper or live-provider acceptance. Exact exported tree
`0f06f99135293b3d3df92388f143b7e65ec1beda` passed the worker and Python guard
reruns; independent correction review confirmed the default-port repair.
Installed qualification has resumed in the dedicated account, without changing
original-account authority. The operator explicitly authorized persistent pkexec
for mythos; a noninteractive PolicyKit check now succeeds. Earlier successful
pkexec calls could still trigger desktop authentication and were not evidence of
unattended access.

Reviewed tree `0f06f99135293b3d3df92388f143b7e65ec1beda` produced release root,
worker, keeper and objective-store binaries. Installed hashes and prior backups
are retained at `/var/home/arda-qualification/evidence/reviewed-runtime-05623ba3/`.
The account's existing objective authority bytes/inode were preserved. Its new
private retained policy/profile capture the account release manifest's port 17171;
no original-account credentials or retained authority were copied. Fresh keeper
initialization and hardened managed startup succeeded. The reviewed root uses
Harness port 17878 and the private keeper socket. In the account manager's existing
isolated network namespace, selected Manwe/Harness health succeeds and old port
7171 refuses connection (`startup.json`). This is installed startup and endpoint
isolation, not retained admission, inference, native lifecycle or restore acceptance.
Independent installed-checkpoint review confirmed the bounded startup claim.
The root was stopped before the installed admission probe. A qualification helper
using the reviewed Engine library admitted one real read-only objective through
the installed managed keeper/worker and generated runtime policy, reopened the
same snapshot binding, then cancelled and durably released it without provider
execution. Readback found one objective/admission/snapshot/release, two controls,
and one keeper snapshot in `released` state. This is cancellation/reopen evidence,
not successful work, provider inference or backup restore. The helper's initial
invalid project metadata was refused; the corrected read-only authority passed.
Evidence and helper source are retained alongside `admission.json`.

The quiescent archive passed all 13 file hashes. An extracted copy plus copied
marker was refused at the canonical path; reinstating the displaced original
preserved its device/inode, marker hash and entire SQL history. Full managed
root/keeper restart passed health and retained the terminal release
(`original-reinstatement.json`). This is original-identity recovery, not runnable
backup-copy authorization. The reviewed transfer design and adversarial acceptance
are retained in [R09 state lifecycle](../operations/r09-state-lifecycle.md#runnable-restore-bounded-owner-design);
no transfer implementation or marker rewrite is claimed.

The same reviewed tree built and installed native launcher SHA-256
`48c93691f252a43a22df1ce0bbf791118785a188c41eb50b23320e40f730eba2`.
Its native WebKit document loaded from `tauri://localhost` with `ARDA_ROOT` unset
and observed the isolated runtime and gateway as healthy. Required-gateway outage
exposed a real stale-status defect: `LifecyclePanel` fetches only on mount/action.
The pending source repair adds sequential observation, failure/timeout invalidation,
late-result rejection and unmount cleanup; six observer regressions plus the
existing frontend suite pass (26 total), lint and production build pass. Independent
precommit review and corrected installed outage/recovery acceptance are next.
No visible-monitor or complete HUD/session lifecycle acceptance is claimed.

First-run still reports four warnings: stale root-protocol, environment-template
and portability-audit prerequisites, plus missing endpoint environment gates despite
selected-release lifecycle health. Trace their current readiness owner next; do not
add dummy files or bypass gates. The account Manwe uses governed defaults, not the
profile's edge_core provider. Temporary launcher/compositor/root/keeper units are
stopped with `MainPID=0`; the previously running gateway was restored. Public release
inputs, runnable copy restore and complete native acceptance remain open; R10/R11
remain dependency-gated.

### R10 Connect evaluation to daily usefulness

- [ ] **Priority P3; owners: Aule, economics, Rumil and existing daily-loop owner.**
  Connect objective → provider calls → artifacts → checks → feedback. Turn
  meaningful failures into regression cases. Measure accepted completion,
  intervention count, elapsed time, duplicate effects and cost/energy per accepted
  result, distinguishing hardware measurements from estimates.
- [ ] Complete the existing [daily improvement](DAILY_RESEARCH_IMPROVEMENT_LOOP.md)
  acceptance window and retain useful no-change/rejection outcomes as well as
  improvements. Link findings to exact revisions and confirmed resolutions.

Exit evidence: repeatable evaluation dataset, comparable outcomes and accepted
reduction in operator burden; telemetry volume alone is not success.

### R11 Qualify external adapters and device surfaces

- [ ] **Priority P3; owners: SDK, outpost crates and native UI owners.** Provide a
  working adapter example and conformance checks for framing, capabilities,
  compatibility, timeout/cancellation and error reporting.
- [ ] Qualify actual outpost disconnect/reconnect, stale observations and honest
  offline displays. Use existing project/device/native owners; broader embodiment
  remains deferred and is not a prerequisite for G1.

Exit evidence: a second real consumer where extension is justified, compatibility
results and native/device observations with no authority expansion.

## R01–R02 continuation evidence, October 5–6, 2026

The dated entries below retain intermediate findings. Their open-M4 handoffs are
superseded by the current closeout and next-session sections above.

Source: `main`, HEAD `f5c8a361f87d95cf04a932a90a15fd52f15eb6d1`, with the
pre-existing documentation/operator changes retained. This session changes two
test fixtures: empty `excluded_refs` in Vaire's context helper (no references
were excluded by that fixture), and empty `approval_receipts` moved from the
invalid CapabilityDeclaration initializer to RunGraph in arda-core's
`deterministic_composition` helper. Production initializers already supply
`excluded_refs`. No production behavior or installed binary was changed.
The exact two-fixture patch is retained as `source-fixes.patch`, SHA-256
`d72d18f72ee927a08bcd13ba7afe47025a14645bfde69cc70ee73bb5305517e1`.

Local raw evidence: `target/qualification/reliability-20261005/`, including
`identities.sha256`, `runtime-sample.txt`, `runtime-journal.log`,
`varda-status.json`, and command-specific Cargo logs. These are local evidence,
not committed release artifacts. Toolchain: rustc 1.98.0 / cargo 1.98.0;
build concurrency limited to two jobs. Mesh-llm was unexpectedly active at the
initial check; stopped successfully at October 5 22:47:49 PDT and still inactive
at October 6 00:07:16 PDT. No inference request or new objective was submitted
by this session. Existing automatic probe timers were not changed.

### R01 diagnosis and authority boundary

| Component | Fresh evidence | Disposition / owner |
| --- | --- | --- |
| Root Arda | Repeated `ObjectiveStore authority database was replaced; restore original authority`; marker device/inode `64515/279107384`, actual regular database `64513/278972929` | Confirmed identity mismatch; cause of the underlying replacement remains unresolved. Engine/M4 owns historical qualification before repair. Do not rewrite the marker, reprovision, or mutate historical SQL. |
| Crawl4AI | Exit 203/EXEC: `/usr/bin/docker` does not exist; installed unit invokes `docker start --attach crawl4ai`. Read-only Podman inventory contains no `crawl4ai` container. | Varda/deployment owner must restore a qualified runtime/container; changing the executable to Podman alone is insufficient. |
| Manwe probe | Connection refused on 7171; first failed attempt exits 0 because the configured threshold is two failures. Script can restart root after threshold. | Probe service exit 0 does not prove inference success. Root fails before its supervised Manwe is available; this is distinct from intentionally stopped mesh inference. |
| Metrics exporter | Exit 2, `unrecognized subcommand 'metrics'`; installed CLI SHA-256 `394aba48c8c26a9b5fb40944db99e6218bf3a3c1ead304e3a290ed7ed48c6385` | Aule owner: installed CLI lacks the command. Current source gates it on `http`; qualify an `http`-enabled replacement before deployment. Optional exporter failure does not explain root refusal. |
| Varda | Listener 127.0.0.1:5111; supported GET `/status` and `/policy_readiness` return HTTP 200, `ok=true`; NRestarts=0 | Harmless supported requests pass. Policy records remain `reference_only`; HTTP availability does not imply crawler/provider or policy acceptance. `/health` is not a supported route. |

Root SHA-256: `ec23935e067584526b9ad8035ea01b94ec8d10a14ee25a5a94b0e36112df4735`;
configured supervised Manwe: `target/release/manwe`, SHA-256
`d703212c271da91a135d53659423e4f5da639c07c635d69d126fef7ddb8fa78f`;
installed Varda SHA-256:
`78249f135d16951b61620135696508694dee78d55d359274528abd467d5a6a8b`.
Unit/drop-in and config file hashes are in `identities.sha256`; config contents
and credentials were not copied into this report.

Observation interval October 6 00:06:39–00:07:16 PDT (37 seconds) exceeds root's
3-second, crawler's 10-second and Varda's 5-second restart delays. Root restart
count increased 72 → 77; crawler 486 → 490; Varda remained 0. Root's fleeting
`active/running` sample occurs during repeated failing starts and is not readiness.
7171 `/health` and 7878 `/v1/status` refused connections. R01 diagnosis is done;
repair/readiness acceptance remains open at the existing M4 authority boundary,
with separate crawler and exporter deployment gaps. No installed recovery,
historical abandonment, keeper restart or authority edit was performed.

### R02 source results and remaining validation

- `cargo test -p arda-vaire --test context_capsule -j 2 -- --test-threads=1`:
  PASS, 9 passed / 0 ignored; default features.
- `cargo check --workspace --all-targets --all-features -j 2`: exit 101;
  launcher dependency `gdk-sys` cannot find `gdk-3.0 >= 3.22` through pkg-config.
  Owner: launcher/build environment. Full root-workspace coverage remains blocked
  until native prerequisites are available; this is not a Rust source failure.
- First check with `--exclude arda-launcher`: exit 101; separately exposed
  arda-core deterministic-composition fixture E0063/E0560, repaired as above.
- `cargo test -p arda-core --test deterministic_composition -j 2 -- --test-threads=1`:
  PASS, 4 passed / 0 ignored; default features.
- Repaired `cargo check --workspace --exclude arda-launcher --all-targets --all-features -j 2`:
  PASS, with existing warnings; 18 workspace packages, launcher excluded.
- `cargo test -p arda-vaire --all-features -j 2 -- --test-threads=1`:
  host rerun PASS, 97 passed / 1 ignored (operator-scale append/consolidation
  soak). Initial sandbox run failed only because the IPC test could not bind
  a local socket (69 passed / 1 failed in the library suite before stopping).
  Full workspace tests remain unestablished, with native compilation blocked as
  above; the focused and Vaire totals are not a whole-workspace test result.
- `cargo clippy --workspace --exclude arda-launcher --all-targets --all-features -j 2 -- -D warnings`:
  exit 101. Aule owns `cloned_ref_to_slice_refs` in
  `autopilot/task_queue.rs:8107`; Engine owns unused
  `independent_critic_review` in `workbench.rs:811` and `too_many_arguments`
  on `abandonment/application.rs:32`. These pre-existing production/test lint
  findings are recorded for R03, not suppressed or conflated with the fixture fix.
- `git diff --check`: PASS. Plan Markdown verification: 96 local links,
  zero broken links; completion-language check enabled.

Native HUD/Mirromere workspaces, remote CI, GPU/provider tests and installed
acceptance have not been validated by these commands. R03 must retain the native
prerequisite and coverage distinction.

## Documentation completion and evidence discipline

- [x] Reconcile Engine/Aule/Manwe READMEs, root topology and CODEMAP with inspected
  source; correct moved cutover links and milestone status/navigation.
- [x] Preserve the assessment, all 19 package roles, comparison sources and
  time-bounded runtime observations in the linked audit.
- [x] Add this ordered continuation list and link it from existing navigation.

For each completed task append date, source revision, commands/results, evidence
location and remaining limitations to its owning workstream. Update this list
with a concise completion link. Keep runtime observations dated; do not copy old
PIDs or test counts into claims about current health. Retire this list when its
remediation tasks are resolved or explicitly transferred to their existing owners.

## Documentation verification on October 5

The documentation-only pass checked 95 active-plan local links with zero broken
links and zero completion-language issues using
`scripts/rumil_markdown_link_check.py --root docs/plans --check-completion-language`.
A separate changed-document scan covered 110 local links with zero missing targets
and checked whitespace in new files. `git diff --check` passed. Rust builds/tests,
remote source CI, runtime repair and native acceptance were not performed.

## R01 recovery continuation, October 6, 2026

Metrics exporter and Crawl4AI deployment failures are repaired and live-checked.
Varda now supports endpoint-bound crawler authentication; 131 ordinary tests,
one explicitly invoked live test and strict Clippy pass. Root startup remains
blocked by the historical database identity mismatch. The retained baseline hashes
and five historical bindings were verified without rewriting authority. Full
results, deployment limits and the next identity-recovery boundary are recorded
in the [M4 recovery progress](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md#r01-installed-recovery-progress-2026-10-06-morning-pdt).
Mesh-llm remains off. R01 installed root readiness and R04/M4 acceptance stay open.

## R01 fresh development recovery, October 6, 2026

The operator chose to archive historical authority and start fresh. Objective and
keeper stores were explicitly initialized together, with rollback state and prior
binaries preserved. All five local service health endpoints pass; root and keeper
remain running with zero automatic restarts beyond their restart intervals.
Mesh-llm remains off behind a runtime startup guard; the inference probe is paused.
[R01 recovery evidence and the superseding operator decision](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md#fresh-development-authority--operator-decision-october-6-2026)
close the installed startup blocker. This does not claim historical retirement,
model inference, remote telemetry availability or M4 multi-project acceptance.

## R04 GPT readiness attempt, October 6, 2026

The worker now selects GPT through its captured profile; broker admission accepts
explicit local or cloud providers without requiring an `edge_` name. Manwe's
subscription credential selection honors refreshed pool credentials. Focused
broker/driver tests (5/10), strict Clippy and builds pass. Both projects pass lint
and their existing work is preserved. The authenticated objective remains pending:
GPT text inference passes, but tool calls fail upstream with
`native turn auth context mismatch: scopes`. Mesh-llm remains off. No worker
execution, overlap or joined acceptance is claimed. See the
[M4 GPT attempt and continuation](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md#gpt-readiness-attempt-october-6-2026).

The later [LLM restoration pass](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md#llm-tool-loop-restoration-october-6-2026)
supersedes that auth blocker: the existing credential passed both probes, and the
installed gateway passed a complete tool-result round trip. Fixed lost Responses
tool history and a remaining edge-only Python launcher restriction. Manwe's 298
library tests and the worker's 6 inference tests pass, with strict Clippy and builds.
The failed pre-inference objective was cancelled with evidence retained; a fresh
read-only replacement was approved. M4 completion remains unestablished.

The installed worker also now records validated session configuration provenance.
Final worker suite: 16 passed / 1 ignored; strict Clippy/build pass. Both project
leaves of `operator-objective-c98d83591ddab744` completed with independent reviews
and close receipts, with 113,887 ms of overlapping execute intervals. Native
package checks passed separately as operator evidence; worker verification was
source-only. The objective is paused at a distinct join-context failure: the
promoted outcome record exceeds Vaire's 4,096-byte memory-record bound. Next repair
bounded outcome packaging, then resume the same join without replaying completed
leaves. LLM execution is restored; M4 joined acceptance remains open.

## R04 lossless memory admission repair, October 6, 2026

The [memory-budget repair](autonomous-task-completion/M4_REAL_MULTI_PROJECT.md#lossless-memory-budget-repair-october-6-2026)
removes the redundant 4 KiB per-record restriction while retaining the existing
32 KiB aggregate budget. Full content and digests are preserved. Vaire: 99 passed /
1 ignored; Engine context/restart: 12 passed / 5 ignored; strict Vaire Clippy and
native all-features root build passed. The installed join consumed the previously
rejected 6,891-byte record. Completed project receipt hashes remain unchanged.

The next blocker is dependency-context completeness: the join receives only its
own project's report, not the second completed dependency's report. It returned a
failed synthesis result and is paused with evidence preserved. Next supply both
exact authorized dependency reports, preserving independent-leaf isolation, then
qualify joined closure and restart non-replay. Mesh-llm remains off.
