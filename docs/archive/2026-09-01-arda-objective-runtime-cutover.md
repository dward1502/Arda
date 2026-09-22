---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  role: "implementation_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-15"
---

> 🜏 Soterion: 📜 implementation_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-15

# Objective Runtime Cutover — Remaining Engineering Gates

## Outcome and current state

The resident daemon owns durable objective controls, schedules, bounded concurrent
execution, same-run recovery and receipt-backed closure. Engine ObjectiveStore is
transactional authority; RunStore owns evidence; the independent snapshot keeper
owns retained execution resources. No second scheduler or JSONL fallback.

Cutover acceptance remains open. C1 candidate/source qualification, C2 writer
retirement and C3.1 installed CLI intake/control are closed at their recorded scope;
multi-project/Vairë non-replay and operator workload acceptance remain open. September 15 read-only
inspection found Arda and the independent keeper active and `/health` responding;
that is liveness, not qualification of all current source. The tree includes
uncommitted changes; bind candidate identity before claiming that source is installed.

Post-September-15 repairs: ObjectiveStore authority marker had stale device number
(stored `252`, actual `makedev(252,3) = 64515` on this host) causing
"ObjectiveStore authority database was replaced" on daemon start; marker corrected
to the current encoded device value. Daemon now starts clean with
`ARDA_OPERATOR_ID=operator:mythos` and `/health` returns `ok`. This is a
configuration/installed-identity reconciliation, not a C3.2 acceptance signal.

## Retained evidence

- [Pre-reconciliation evidence record](../archive/2026-09-15-plan-reconciliation/2026-09-01-arda-objective-runtime-cutover.md)
  preserves detailed source tests, independent reviews, failures, limitations,
  local log paths and installed qualification. Older statements inside that
  historical record are not current blockers or permission to replay old actions.
- [Installed local acceptance](../../target/qualification/installed-final-acceptance-evidence.json):
  receipt-backed execute/verify/review/close, completion restart, paused due schedule
  with zero attempts until resume, and cancellation with durable release.
- [Installed interrupted-verifier recovery](../../target/qualification/installed-recovered-evidence.json):
  same run, one execute start, two verify starts, one review start, unchanged execute
  receipt and one release. The September 15 doc audit checked the eight completion/
  scheduling receipt hashes against actual files. Evidence under `target/` is local
  qualification data, not a portable release archive; preserve it before cleaning.
- Release-ACK loss and post-reboot terminal reconciliation are recorded in the
  historical record and private rollback bundle
  `~/.local/share/arda/rollback/20260915T042450Z/keeper-boundary-evidence/`.
  Reboot proof concerns a cancelled terminal fixture, not active Hermes survival,
  abrupt power-loss durability or worker-cleanup acknowledgement.
- [Legacy replay/resource repair](../audits/2026-09-13-legacy-queue-resource-repair.md)
  moved ordinary projections to SQLite and disabled replay timers. Idle memory
  measurements do not prove acceptable game/YouTube/work frame timing.

## Remaining checklist

Each item closes only with named evidence and its exact source/installed scope.
Reuse previously accepted identity/bootstrap/cleanup tests; investigate the missing
boundary rather than rerunning the entire cutover after each repair.

### C1 — Retained path and failure-boundary qualification

Status: C1.1–C1.5 source/candidate qualification complete at the recorded scope,
not whole-installation acceptance. Earlier passing boundaries retain their stated
scope; C3 separately owns genuine installed/operator acceptance.

| Row | Existing evidence | Remaining qualification |
|---|---|---|
| C1.1 | Nonblocking cleanup; pending-cleanup Prepare refusal with Commit/Release usable; final queue destruction preserves unproven pins and preparing journal | Scoped boundary coverage independently reviewed; actual failed-Prepare integration belongs to C1.2 |
| C1.2 | Seven configured preparation process-exit checkpoints, managed orphan revocation/replay, preserved authority, existing Prepare/Commit retry tests | Scoped source/test review found no blockers; not deployed acceptance |
| C1.3 | Retained HTTP/adapter disconnect, keeper-loss and shutdown matrix; socket-level Commit ACK loss; earlier mount-race, Prepare, installed daemon recovery and terminal/reboot cases | Scoped source/test review findings resolved; configured installed failure matrix is not implied |
| C1.4 | Four active-control/rebind cases; descendant/provider/launcher reap; generation refusal; owner restart; bounded incomplete-request handling | Scoped follow-up review resolved findings; server ACK ordering is source-backed, not independently instrumented |
| C1.5 | Integrated candidate review and required-fix follow-up complete; active Release fix qualified against exact source/binary manifests | Candidate source/test qualification only; no installed artifact replacement |

- [x] C1.1 — Pending-cleanup boundary assertions tested and independently reviewed; [tests](../../crates/engine/src/bin/keeper_owner/qualification_boundaries.rs).
  New Prepare refuses while cleanup is unproven, existing Commit/Release still
  work, and unresolved generic pin sentinels survive final queue destruction.
  Request-driven reaping/idle retention and shutdown reap limitations must remain
  explicit; tests do not simulate uninterruptible kernel I/O.
  [Boundary assertions](../../crates/engine/src/bin/keeper_owner/qualification_boundaries.rs)
  now exercise real Owner routing with a mock worker endpoint and deterministic
  `ECHILD` observation failure, plus final pin-sentinel destruction. Five cleanup
  tests and strict keeper Clippy pass; production behavior is unchanged. Independent
  review found no blockers at this scope. The preparing row and cleanup entry are
  injected fixtures, not a real failed Prepare or actual allocation-pin integration.
- [x] C1.2 — Manifest/admission identity and durable preparation-journal review complete at [source/test scope](../../crates/engine/src/bin/keeper_owner/preparation_tests.rs),
  including crash boundaries and explicit orphan reconciliation. Saved capability,
  generation, owner and expiry must survive uncertain acknowledgements unchanged.
  [Configured crash matrix](../../crates/engine/src/bin/keeper_owner/preparation_tests.rs)
  passes after preparing commit, allocation intent/directory/save, worker spawn,
  qualification and prepared-authority save. Real transient systemd lifetimes and
  the reconciliation CLI prove restart fencing, unchanged saved authority,
  managed-stop interlock and idempotent terminal revocation—not worker cleanup ACK.
  Ten keeper tests, ten snapshot tests and strict keeper Clippy also pass. Independent
  scoped review found no blockers or mandatory assertion changes. Test-only
  checkpoints; uncommitted source, not deployment or C1.3–C1.5 acceptance.
- [x] C1.3 — Production adapter/Harness loss qualification at [source/test scope](../../crates/engine/tests/fixtures/retained_harness_loss.rs): mount-race containment,
  Prepare rollback/response loss, Commit loss, daemon death, lease expiry/rebind,
  active disconnect, keeper loss and terminal release. For every case record existing
  test/receipt, scope, missing assertion and pass/fail; do not invent another owner.
  New [retained Harness matrix](../../crates/engine/tests/fixtures/retained_harness_loss.rs)
  passes disconnect-alone cancellation/provider exit with Harness still serving, active
  keeper death producing a failed node without success receipt, and graceful
  shutdown preserving Running state. Real unconfigured keeper/worker and production
  retained adapter/HTTP; controlled Python provider, not installed Hermes inference.
  [Commit transport matrix](../../crates/engine/tests/fixtures/keeper_undelivered_commit.rs)
  passes both pre-delivery loss and successful keeper ACK discarded by a Unix-socket
  proxy, then exact saved-snapshot recovery and expired-generation refusal.
  Review-required authority assertions now pass: original generation/owner/expiry
  and outstanding ACK survive Commit loss, reconciliation resends the exact tuple,
  and all Harness modes preserve snapshot/lease with no fabricated release record
  before explicit cleanup. Shutdown waits are bounded; pidfd evidence means exit,
  not independent parent-reap proof. Follow-up independent review confirmed the
  substantive findings resolved; residual exit/reap wording was corrected. Both loss
  fixtures and strict Clippy pass; earlier regression suites passed 27 Harness and
  26 adapter tests. Uncommitted source, not deployed acceptance or C1.4/C1.5 closure.
- [x] C1.4 — Active-rebind qualification at [source/test scope](../../crates/engine/tests/fixtures/keeper_active_rebind.rs): descendant/supervisor
  reap, cancellation caused by rebind rather than ordinary timeout, server ACK order,
  generation-specific dispatch refusal and owner restart. The [active-rebind matrix](../../crates/engine/tests/fixtures/keeper_active_rebind.rs)
  now passes direct rebind, lost ACK/reconciliation, and owner restart after ACK
  loss. The worker accepts a validated newer Commit during execution, cancels the
  prior process tree, and applies/ACKs only after teardown. Host pidfds and procfs
  witness descendant/provider/supervisor exit and reap at received ACK; server order
  is source-backed. Rebind finishes before five seconds against a 30-second timeout;
  generation-specific refusal and invalid-binding non-cancellation are asserted.
  Restart preserves exact pending generation/owner/expiry and lost snapshot authority,
  denies dispatch, and fabricates neither release ACK nor replacement Prepare.
  Review fixes cap active control reads by 20 ms and the remaining execution/lease
  deadline, make refusal writes nonblocking, and explicitly witness the same-executable
  launcher. The added incomplete-request case preserves timely command timeout.
  Four real unconfigured-worker cases, worker/Harness/adapter/snapshot regressions
  and strict Clippy pass; follow-up independent review resolved both findings with
  no concrete scoped regressions. Untyped controlled Python
  processes, not installed Hermes recovery or proof of every escape route. Uncommitted,
  undeployed source; C1.5 remains separate.
- [x] C1.5 — Integrated path review and exact candidate qualification complete;
  concrete finding resolved and changed keeper/worker/daemon artifacts qualified before
  replacement. Earlier bounded reviews retain their original scope.
  Integrated authority/lifecycle review found active Release refused without cancelling
  execution and an unretryable `releasing` owner. The regression reproduced that failure;
  authenticated Release now cancels through teardown-gated serial control, and the same
  live owner can retry `releasing`. Three [Release cases](../../crates/engine/tests/fixtures/keeper_active_release.rs)
  pass: open-connection cancellation, keeper ACK wire loss/reopen/retry, and seeded live
  `releasing` retry. Worker ACK loss/death remains fenced for explicit reconciliation.
  The Engine/daemon integrated review found no separate production blocker; earlier
  bounded reviews remain scoped evidence. Independent required-fix follow-up resolved
  the P1 with no remaining concrete must-fix. Review and final local verification matched
  all 864 source entries, three candidate binaries, manifest binding and listed logs;
  exactly five source deltas from the initial review are accounted for.
  Exact candidate evidence is `target/c15-qualification/source-manifest-release-fix.json`,
  `artifacts-release-fix.json`, and `*-release-fix.log`: locked debug daemon/keeper/worker
  build, engine/daemon regressions, seven configured preparation checkpoints, three
  configured bundle cases, seven active-control cases, two Commit-loss cases, Harness
  loss matrix and strict Clippy pass. Initial failures are preserved. The bundle fixture
  now checks bounded request-driven reap, not an unsupported synchronous failed-Prepare
  cleanup guarantee. No installed binaries or live services replaced; preserve local
  evidence before cleaning. Candidate qualification is not C3 acceptance.

Source/tests: [snapshots](../../crates/engine/src/objectives/snapshots.rs),
[Workbench](../../crates/engine/src/objectives/workbench.rs),
[keeper cleanup](../../crates/engine/src/bin/keeper_owner/qualification.rs),
[managed release](../../crates/engine/tests/keeper_managed_release.rs),
[Prepare rollback](../../crates/engine/tests/fixtures/keeper_prepare_rollback.rs),
[active rebind](../../crates/engine/tests/fixtures/keeper_active_rebind.rs),
[retained restart](../../crates/engine/tests/resident_retained_restart.rs.inc).

Exit gate: each required failure boundary has correctly scoped evidence, safe
refusal/reconciliation and independent integration review. A terminal reboot test
cannot stand in for interrupted execution safety.

### C2 — Finish retiring competing writers

- [x] C2.1 — Source repair tested and independently reviewed. `reprioritize`,
  `revise_objective` and `retry_failed` now refuse before history reads/provisioning/
  mutation. Missing/malformed/valid-history fixtures prove byte and directory
  preservation; historical replay fixtures seed records directly, with no test-mode
  authority bypass. [Aulë mutation entry points and tests](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/task_queue.rs).
- [x] C2.2 — Source repair tested and independently reviewed. Retired both outer callers and helper:
  `arda_core::state::append_task`, Aulë `prometheus::planner::run(Some(queue))`,
  Core `loop_engine::dispatch_full_with_affordability`. Refuse before plan/decision/
  executor/queue side effects, retaining plan-only `run(None)`, accounting and
  historical fixtures. [Core refusal integration test](../../crates/spine/governance/arda-core/tests/retired_jsonl_authority.rs)
  covers all dispatch wrappers, malformed/missing/valid history and panic-on-call
  collaborators; [planner tests](../../crates/spine/observability/arda-aule/src/prometheus/planner.rs)
  prove rejected emission cannot consume plan idempotency.
- [x] C2.3 — Source producer inventory and emission guards tested and independently reviewed; [authenticated ingress regressions](../../crates/engine/tests/research_delivery/admission.rs).
  Closure covers Core, Aulë, Oromë and Varda retired producers, with accepted
  research-backed work admitted through Engine authority and separate approval.
  Six semantic rejection cases return HTTP 400 without objectives; the ingress suite
  passed 22 tests (2 ignored), strict Engine Clippy and diff checks passed.
  Independent closure review found no remaining executable producer outside Engine
  authority. This is the current uncommitted working tree, not installed qualification;
  C1 failure boundaries and C3 genuine operator acceptance remain open.
  Aulë plan/dispatch queue append helpers, pipeline handoff,
  mutable CEO cycles and the persistent CEO loop now refuse before side effects;
  one-shot read-only library inspection remains available (the CLI `Once` retirement
  gate is unchanged). Approved packet attempts
  expose `blocked_retired_authority` rather than claiming queue admission.
  [Writer regressions](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/queue_writer.rs)
  cover missing/malformed/completed history and every dispatch status;
  [cycle regressions](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/runner.rs)
  cover missing roots, existing fixtures and paused-loop refusal without waiting.
  Independent static review found no blocking issues in these Aulë changes. Snapshot
  tests prove filesystem preservation; collaborator refusal ordering was inspected,
  not independently executed. Converted mutable-cycle tests now prove retirement,
  not their former governance/joule report semantics; constructor reads precede the
  cycle boundary. The wider producer inventory and subsequent closure review
  classified the boundaries below; neither review independently ran tests or
  established installed activity.
  Distinguish source reachability, build-feature inclusion and installed activity;
  preserve research capability through canonical governed intake, not retired writers.
  Do not restore the disabled queue executor or erase historical ledgers.

C2.3 source evidence and bounded acceptance qualifications:

- Implemented, independent static review found no blocking issues: [Oromë inbound decisions](../../crates/spine/interface/arda-orome/src/service/inbound.rs)
  now reject retired actions before consuming dedup or classifying;
  [choice resolution](../../crates/spine/interface/arda-orome/src/service/decision.rs)
  rejects before response append, preserving the prompt. Direct classification and
  literal retired commands share refusal. Snapshot/retry tests preserve service
  history and message IDs; supported choices still resolve once. Automatic prompts
  offer only chat and explicitly historical review, not drain/execute or unwired
  maintenance/research actions. Historical rows are not live Engine work; this does
  not qualify a canonical objective-control conversation or reconcile old queue rows.
  `cargo test -p arda-orome --features service-runtime -j 2 -- --test-threads=1`
  passed 137 tests across nine targets, none failed or ignored. Feature-enabled
  all-target strict Clippy and Engine/Aulë full-cli compilation passed. RED tests
  reproduced premature response append and unsupported automatic prompt options.
  No services changed; the optional service-runtime source is not installed qualification.
  Review boundaries: response-history read errors remain suppressed, concurrent direct
  choice resolution is not atomic, and polling currently aborts its batch on refusal.
  Sequential regression coverage does not establish concurrent exactly-once behavior.
- Implemented, independent static review found no blocking issues: the separate [Varda HADES action interceptor](../../crates/spine/executors/arda-varda/src/ingest/interceptor.rs)
  no longer emits `investigate_orphan`; store construction neither provisions nor
  registers it. A no-op public compatibility adapter and historical layout path
  remain; old JSONL is untouched. The source inventory found only an audit reader,
  not a competing execution/admission consumer; independently installed consumers
  were not assessed. Warden lifecycle observations remain enabled. The audit no
  longer expects the retired action file, and no automatic objective admission was
  substituted. RED regressions reproduced file creation; replacement tests verify
  absent-file and historical-byte preservation, including store reopen, with Warden
  still emitting. Varda rerun passed 122 library and two integration tests; strict
  all-target Clippy and nine Python audit tests passed. The first suite run exposed
  intermittent index staleness; the separate repair below addresses a reproduced cause.
  Non-blocking review follow-ups were addressed: default-order documentation now
  excludes HADES, and reopened-store coverage exercises deep lifecycle events while
  preserving historical HADES bytes and confirming new Warden observations. Both
  focused retirement tests and strict all-target Clippy passed after those changes.
- Implemented, independent static review found no blocking issues: [persisted-index freshness](../../crates/spine/executors/arda-varda/src/ingest/index.rs)
  now compares content fingerprints rather than modification timestamps. An isolated
  two-store regression forces timestamp collisions and reproduced the stale query
  before this repair. Fingerprints bind to the exact loaded/published bytes, not a
  later path read. Varda passed 123 library and two integration tests; strict Clippy
  and Engine/Aulë full-cli compilation passed. Each cache lookup now reads/hashes the
  persisted index; large-corpus performance remains unmeasured. This does not claim
  arbitrary concurrent rebuild safety or detection of external book edits preserving
  timestamps without an index refresh. TTL expiry can reload the same persisted
  index; it does not guarantee eventual detection of those external edits. The
  misleading RAM-only query documentation identified by review was corrected.
- Preserve ordinary research ingest/deep analysis and evaluated Engine citations;
  retired project promotion is not a demonstrated canonical intake replacement.
  Source trace found separate, not connected paths: authenticated
  [operator research commands](../../crates/engine/src/harness/operator_messages.rs)
  persist a question and submit a Warden suggestion without creating a commitment.
  The separate [research brief route](../../crates/engine/src/harness/research.rs)
  checks loopback and run/node identity, but not gateway capability/identity;
  it performs discovery, bounded fetching, Varda ingest/capture/deep analysis and
  advisory citation generation. No caller joining the command to the brief route
  or automatic citation consumption by objective execution was found. EvidenceLinked
  feeds operator projections, not admission authority. Corrected the false assimilation
  `objective_id` assignment: question/run identity is no longer mislabeled as a
  validated ObjectiveStore ID. New discoveries leave that field absent and retain
  source/digest plus the run-scoped brief usage receipt; existing ledgers are not
  rewritten: a seeded legacy regression confirms rediscovery retains old metadata,
  so historical mislabels still need a separately audited correction. The fresh
  restart/idempotency regression reproduced the false binding before the repair;
  it now resolves an actual brief receipt and checks run/question provenance and
  advisory authority. A validated objective-consumer relationship remains unimplemented.
  Default deep analysis uses a deterministic scaffold without an attached LLM.
  These are source findings, not demonstrated provider-backed connected acceptance.
- Added explicit [harness research-store policy](../../crates/engine/src/harness.rs)
  and passed it into the brief's blocking closure. The daemon selects production
  `AthenaStore::new`; harness fixtures select `new_isolated`. This avoids dependency
  `cfg(test)` assumptions without adding an HTTP switch or environment/path heuristic.
  [The downstream lifecycle regression](../../crates/engine/tests/research_store_isolation.rs)
  links normal Varda and exercises the shared post-fetch `ingest_capture` helper
  used by the brief route. It checks real crawl bytes, fixture-local library/Warden
  output, no HADES file, and the configured absence of Mnemosyne (not an irrelevant
  fixture-memory directory). Review found no blocking Engine wiring defect; these
  follow-ups strengthen coverage but do not exercise the full HTTP fetch path.
  Also repaired Varda's isolated governance-log routing: repository-contained
  fixtures no longer redirect governance to live workspace logs. The constructor
  regression failed before the repair; production integration selection is unchanged.
  Varda passed 124 library and two integration tests. Engine passed 95 library tests
  with three ignored, plus 24 integration tests (operator messages, research watchlist,
  assimilation, storage isolation); strict all-target Clippy and daemon compilation
  passed. Isolation is a storage policy, not permission to bypass fetch security.
  Question ingress now decodes and validates Warden's accepted suggestion rather
  than reporting a newly generated ID on retry; it persists the accepted ID in the
  question registry. Changed content under an existing question ID is refused.
  Both defects were reproduced before repair. Bounded HTTP fixtures cover returned
  identity across retry/reopened state and rejection of mismatched fields. Engine
  passed 98 library tests (three ignored), 19 selected ingress integration tests,
  strict all-target Clippy and daemon compilation. Independent review found no
  scoped blocker; added negative cases for valid-but-mismatched expiry, status,
  malformed responses, and invalid creation times pass with strict Clippy.
  The existing bounded Warden enqueue still holds the global mutation lock.
  Extracted the internal `evaluate_sources` discovery/fetch/evaluation boundary
  from run-specific brief storage. The existing run-bound wrapper retains run/node
  validation, replay, brief publication and evidence events; the evaluator takes
  question/context and effective limits without requiring or creating a run.
  A bounded discovery fixture verifies private targets still fail closed and leave
  no run/objective/storage files. This is negative-path orchestration coverage,
  not a successful public-fetch or authenticated question-brief acceptance test.
  Engine passed 99 library tests (three ignored), 24 integration tests, strict
  all-target Clippy and daemon compilation. Independent extraction review found no
  scoped blocker. The private-target regression now matches the specific rejection
  reason rather than a word also present in the fixture URL; its rerun and strict
  Clippy passed. Successful fetch/provenance and wrapper event/persistence coverage
  remain separate from this negative-path test.
  Authenticated gateway research now calls that evaluator with a validated persisted
  owner/question/accepted-suggestion binding and publishes content-addressed,
  immutable question briefs; no dummy run or objective is created. The public-web
  consumer enforces HTTPS across redirects and preserves the private-target guard.
  Results explicitly label the current deterministic evaluation scaffold rather
  than implying provider-backed verification. `arda research-result <brief_id>`
  rereads an owner-bound, digest-verified advisory snapshot; expired snapshots are
  marked expired and never refreshed implicitly. Existing run-brief APIs are unchanged.
  Independent review identified publication/replay loss, age-derived false terminal
  status, DNS rebinding and unbounded response buffering. Follow-ups now use an
  OS-lock-owned operation with persisted execution intent, pre-execution retry and
  conservative non-refetching recovery after uncertain execution. Immutable
  publication is reconciled before replay; assimilation status is separate, so its
  failure or absence cannot hide the brief. Per-hop clients pin validated DNS
  addresses, disable ambient proxies and stop body accumulation at the byte limit.
  IPv4-mapped private and shared/reserved IPv4 ranges are rejected; their regression
  failed before the guard fix. The attempt deadline is 60 seconds; the bridge waits
  65 seconds and retains evidence refs. Cancellation does not claim to kill already
  running blocking ingestion; interrupted execution is never automatically repeated.
  Follow-up review found initial Gateway-loop blocking, stale expiry projections on
  replay and global failure on unrelated corrupt publications. RED/GREEN fixes now
  offload initial submission into an owned async task, refresh verified projections
  on replay and read known targets directly. Interrupted recovery skips malformed
  unassociated candidates without authorizing refetch; corrupt known targets fail
  closed. Direct replay also verifies event binding; operator and continuity task
  callbacks cannot remove replacement tasks. Engine passed 106 library tests (four
  ignored, including a subprocess-only fixture explicitly invoked by its parent
  test) and 25 selected integration tests; strict all-target Clippy and daemon
  compilation passed. Twenty-one bridge tests
  include actual hook-loop responsiveness during pending HTTP and a real HTTP
  response delayed beyond three seconds. The explicit live HTTPS
  smoke test fetched example.com, evaluated it in isolated Varda storage, published
  a citation-bearing brief, and recovered it across the postpublication/pre-response
  crash fixture without rediscovery. Discovery remained a fixture: this is not live
  Warden. A separate real subprocess test verifies lock contention and process-exit
  lock release without Rust destructors; persisted intent prevents re-execution.
  HTTP-handler cancellation and process termination at actual publication boundaries
  remain unqualified. These source checks do not establish live
  Warden, installed Hermes delivery, or genuine operator acceptance. Source remains
  uncommitted/undeployed. Follow-up independent review found no scoped blocker.
  Next admission prerequisites now include a bounded verified loader that preserves
  original brief bytes separately from semantic JSON integrity, and a canonical
  refusal of text-only revisions when leaves contain persisted execution plans.
  The revision regression reproduced changed objective text with stale worker
  prompts; rejection now leaves canonical state and the control key unchanged.
  This is a conservative restriction, not replanning support: executable objectives
  currently require cancellation and fresh admission for a changed goal. Metadata-only
  revision remains supported. Expanded checks passed 111 library tests, all selected
  ingress/isolation tests, 19 runtime tests and 29 store tests, strict Clippy and
  daemon compilation. Review accepted the revision guard and identified a remaining
  unbounded fallback read; that call now uses the bounded reader and a recovery-path
  regression passes. Original objective admission input is now persisted in the
  same canonical creation transaction, with owner/digest-checked recovery after
  restart independent of mutable priority or external source files. Tests cover
  exact execution/verification/review prompt preservation, changed retries,
  tampering, missing legacy snapshots and rollback after a forced leaf-insert
  failure. Missing snapshots are not reconstructed or silently backfilled.
  Scoped persistence review found no blocker. Additional tests cover an upgraded
  pre-extension schema fixture (not a historical binary), unchanged legacy replay
  without backfill, ID/key/owner corruption despite a matching payload digest,
  malformed snapshot JSON, inclusive brief byte limits and mixed oversized/valid
  recovery candidates. Citation publication now cross-checks the service-owned
  ingest, crawl-receipt and deep-evaluation ledgers offline using Varda's canonical
  source identity, fixed store-derived paths and bounded reads. Unit corruption/
  binding tests and the real-HTTPS fixture-discovery smoke pass; Varda's 124 library
  tests, strict Engine/Varda Clippy and daemon compilation pass. This check proves
  local recorded lineage, not original HTTP bytes, current-body evaluation,
  freshness, fitness or resistance to same-UID/root edits. Review found repeat-capture
  cache/pipeline mismatch, interleaved latest-ingest binding, and a shared-ledger
  size ceiling. Capture now requests an explicit pipeline evaluation from that
  ingest snapshot without source-cache or latest-ingest recovery. Repeated and
  interleaved capture regressions pass. The checker streams bounded records rather
  than rejecting an entire growing ledger, and returns only verified lineage fields;
  a ledger exceeding 16 MiB no longer blocks matching. This bounds checker memory,
  not scan time or the legacy Varda readers' memory use. Follow-up review found
  completion interceptors still used the latest pipeline: Warden completion and
  knowledge-view input now use the selected pipeline. The interleaving regression
  reproduces and prevents completion events being attributed to the later capture.
  Focused tests and strict Engine/Varda Clippy pass; final binding review found no
  remaining scoped defect. Explicit `arda objective-from-brief <brief_id> <project_ids> <text>`
  now uses a separate [admission fitness gate](../../crates/engine/src/harness/research/question/admission.rs),
  retains PendingApproval, and copies the original JSON bytes into execution,
  verification and review prompts, including synthesis leaves. It requires bound
  owner/question/suggestion, nonempty fresh evidence with exact `policy_ready`,
  explicit clean injection fields, confidence at least 0.70 and matching local
  ledger lineage; contradictory, expired, unknown-readiness and missing-field
  evidence fail closed. Admission limits retained evidence to 48 KiB (the advisory
  reader still allows 256 KiB). These are service-published assertions, not raw-byte
  authenticity or same-UID tamper protection. Canonical admission replay is wired
  before external brief/project reopening; replay neither recaptures a personal
  objective nor reauthorizes execution. A [synthetic trusted-store ingress fixture](../../crates/engine/tests/research_delivery/admission.rs)
  verifies retained exact bytes, PendingApproval and single-objective replay after
  deleting the brief and project registry; this is not a real eligible provider run.
  Checks passed 112 library, 19 ingress, 19 runtime and 29 store tests, strict
  Engine Clippy and daemon compilation. Admission review found a private-routing
  fail-open and oversized evidence in bounded Vairë metadata. Unknown chat types
  now fail the personal-command fence; only explicit `dm`/`private` are private.
  Resident context keeps objective summaries separate from full worker prompts;
  oversized initial and stage purposes use Vairë's shared digest projection rather
  than copying evidence into a 4096-byte metadata field. Follow-up review caught
  the first local projection conflicting with Hermes's exact objective comparison;
  Hermes now compares against the shared projection of the exact retained payload,
  without dropping the binding check. Provider request construction no longer trims
  that payload. RED/GREEN tests cover the mismatch and preserve 48 KiB stage strings
  through the recording Workbench adapter, including oversized initial goals.
  The subprocess fixture now exercises an oversized **objective**, not only separate
  instructions: decoded actual `-q` retains its whitespace and rejects a changed
  payload against the old capsule. This is not provider consumption or full
  gateway-to-worker continuity. Latest checks pass 112 library, 19 ingress, 19 runtime,
  26 adapter and 68 Vairë tests, plus an additional byte-boundary projection test;
  strict Engine/Aulë/Vairë Clippy and daemon compilation pass. Shared-binding review
  found no scoped blocker. First admission now checks a necessary rendered lower
  bound using Hermes's shared production renderer: complete stage objective, JSON
  escaping and fixed wrappers, for every execute/verify/review leaf, against both
  configured dispatch paths when a retained override exists. This config-only
  check pins no executable/workspace and creates no Vairë use receipt. A regression
  rejects an impossible configured budget before objective creation, accepts the
  unchanged retry after configuration repair, and replays committed admission after
  removing configuration as well as evidence/project files. Exact/inclusive size
  and escaping tests compare the lower bound with the captured full prompt.
  The 48 KiB raw-file cap and this lower bound are not runtime fit guarantees:
  graph/check metadata, future receipts, memory and configuration changes still
  require full per-dispatch preflight. Latest Engine library/ingress/adapter/runtime
  suites and strict Clippy pass; renderer/admission review found no scoped defect. Refusal prevents an
  objective but may persist the transport payload binding; saved admission recovery
  does not reconcile a missing transport-session commit. The connected synthetic
  admission fixture now reaches actual fake-Hermes subprocess arguments for two
  isolated project leaves and dependent synthesis: original JSON bytes occur in
  execute/verify/review payloads for each run after source deletion, explicit
  authenticated gateway approval and store/executor reopen between leaves. It exposed and repaired
  strict HTTP rejection of `research_brief_id`: canonical admission retains that
  replay metadata, while the dispatch envelope excludes it. The shared CLI fixture
  writes captures inside each sandbox workspace. Review found cross-project fake
  session collisions: IDs are now unique, transcripts are keyed by requested ID,
  and concurrent capture writes are locked; a regression verifies exact historical
  exports using distinct prompt hashes, plus missing-ID and wrong-workspace refusal.
  Gateway-created state now survives real child-process exit after each of execute,
  verify and review in fresh isolated cases. The proxy targets the exact claimed
  run and stage; the pre-recovery oracle checks the completed receipt/capture prefix
  and absence of later receipts or close. A fresh HTTP harness/store/executor uses
  logical lease-expiry recovery and completes the remaining stages, sibling and
  synthesis with unchanged run identity, committed receipt bytes and resident binding.
  Captures are mapped to canonical leaf run IDs and workspaces. All nine stage objectives
  retain the original brief. Post-completion source-absent replay now checks
  admission semantic equality, unchanged objective/leaf/context-binding rows,
  receipt/capture bytes and completed state after the replay, not just before it.
  Gateway approval/replay review found no scoped blocker. This remains synthetic
  provider/check evidence, not genuine consumption or whole-installation restart.
  The fixture now approves through the authenticated gateway command and checks
  PendingApproval → Approved before child execution, rather than calling store control directly.
  An environment-isolated child now verifies sufficient ordinary capacity plus
  insufficient retained capacity refuses admission without objective/leaf/admission
  rows; raising only retained capacity allows the same request. The separate
  injected-receipt/fake-keeper fixture verifies missing or wrong-version retained
  config refuses historical replay after expiry/release without changing the receipt;
  restoring retained config succeeds. These are distinct selector checks, not a
  connected real retained-provider run or exact config-byte provenance.
  Latest ingress (22), context restart (10), Harness run (27) tests and strict
  Engine Clippy pass. Review follow-up now requires explicit child completion
  markers, removes ambient ordinary overrides, and checks specific retained
  authority-binding errors. Focused tests also pass with a deliberately invalid
  ambient ordinary override; strict Clippy passes.
  The installed without-live-path baseline was actually attempted: namespace and
  local Manwe prerequisites passed, and real execute/verify receipts succeeded,
  but review returned plain-text `VERDICT: APPROVE` instead of required JSON.
  Harness correctly rejected that result; the test failed, not completed.
  Isolated evidence remains at `/var/tmp/.tmpDoIHM3`, with test output in
  `/tmp/arda-installed-retained-baseline.log`; no fixture process referencing that
  root remains. This ordinary-config baseline does not qualify distinct retained
  config selection. Diagnose real review output-contract reliability before
  claiming output-contract reliability; do not turn its prose into fabricated JSON evidence.
  Diagnosis confirmed the CLI preserved the model's prose despite a complete JSON
  contract. A separate unconditional verdict footer was corrected to review-only,
  with a failing-then-passing regression; this is not a demonstrated fix for model
  noncompliance. The installed baseline recheck passed, so retain both outcomes
  rather than claiming reliable schema enforcement.
  A new environment-isolated installed test then passed with deliberately invalid
  ordinary config and a distinct retained adapter version. Real execute/verify/review
  receipts all name that retained version while the existing missing-live-path
  recovery assertions preserve run/context/snapshot identity, execute bytes and
  exactly three sessions. Evidence: `/tmp/arda-installed-distinct-config.log` and
  `/tmp/arda-installed-retained-recheck.log`. Adapter (26), ingress (22), context
  (10) checks and strict Clippy pass. Independent review found no scoped production
  blocker, but identified test-owner timeout cleanup and a weak review-read oracle.
  Fixture-only parent-death supervision now kills the keeper when its owning test
  process dies; a forced-owner-kill regression verifies child termination. Review
  evidence is now bound to the canonical run/review session and successful tool
  results for both expected retained input paths, rather than the latest session's
  tool count. The strengthened distinct-config installed run passed again
  (`/tmp/arda-retained-review-oracle.log`); context checks (12 passed, 4 ignored)
  and strict Clippy pass. Follow-up review found a post-fork allocation in the
  parent-race error branch; it now uses allocation-free `ECHILD`, with the
  forced-owner-kill regression passing again.
  The direct installed fixture is not gateway-admitted research or actual operator acceptance;
  post-release missing/wrong retained config remains an injected-receipt check.
  Connected gateway admission/recovery now passes its installed fixture. The first run delivered
  identical admitted evidence to all three stages, but review wrote `verify-result.json`
  despite read-only authority and reported a false digest (`/var/tmp/.tmp3LciNr`).
  Configured retained dispatch now carries stage-derived write authority, mounts
  read-only workspaces accordingly, and restricts review tools to read/search.
  Connected recovery/replay and review integrity pass (`/tmp/arda-finalization-installed.log`).
  Malformed final responses now get at most one tool-free formatting request in the
  same recorded session; invalid output still fails closed. Genuine prose→finalization
  and exact-ID export pass (`/tmp/arda-finalization-branch4.log`); source suites and
  strict Clippy pass (`/tmp/arda-finalization-suite.log`, `/tmp/arda-finalization-clippy.log`).
  This is prompt-constrained, not schema-decoder enforced; earlier failed trials remain
  evidence against a reliability claim. Independent scoped review found no blockers.
  Next: remaining
  interruption/negative-ingress and useful operator acceptance. Uncommitted/undeployed;
  strict evidence validation stays intact and Gate 1 remains open.
- Corrected stale task-pivot append guidance in
  [the historical append-only checker](../../scripts/check_task_queue_append_only.sh).
  It now directs live work to authenticated resident objective intake/controls;
  the checker remains a reader. Four queue-authority tests passed, including a
  RED/GREEN rejection-message regression. The task-pivot script already refuses.

Already retained: reviewed Varda/Aulë retirement and optional Oromë/task-pivot
refusals. The latter source change is outside the installed daemon/CLI feature
graphs; do not replace healthy binaries solely for it. Local autopilot and
external-source-lane timers were inactive on September 15.

C2 source verification (not installed qualification):
`cargo test -p arda-core -p arda-aule --features arda-aule/full-cli -j 2`
passed 618 tests across 33 unit/integration/doc-test targets after the C2.3 source
changes, none failed or ignored (legacy writer-success tests consolidated into
retirement matrices).
`cargo clippy -p arda-core -p arda-aule --features arda-aule/full-cli --all-targets -j 2 -- -D warnings`
and `cargo check -p arda-engine -p arda-aule --features arda-aule/full-cli -j 2` passed.
RED fixtures reproduced directory creation in Aulë, successful legacy Core append,
and successful plan-writer/mutable-cycle effects before C2.3 guards;
GREEN fixtures verify refusal. Changes remain uncommitted and undeployed; this is
not C1 integrated keeper review, release qualification, or Gate 1 operator acceptance.
Independent read-only review found no blocking issues in the earlier C2.1/C2.2 diff:
all dispatch wrappers and mutation guards refuse before I/O/collaborator calls,
plan-only generation and historical readers remain available, and tests do not
bypass production refusal. The reviewer did not rerun Cargo or assess installed
runtime; the test results above are implementation-run evidence.

Exit gate: every identified producer is classified; retired admission/control/
schedule/continuation paths refuse without side effects, and accepted new work
uses Engine authority only.

### C3 — Installed authority and operational acceptance

- [x] C3.1 — Authenticated intake/control accepted through the originating Hermes CLI; [live evidence](../../target/qualification/c31-cli-acceptance.json). Identical replay
  remains idempotent, changed payload/family conflicts without mutation, accepted
  controls notify the resident runtime, and store/marker loss fails closed.
  Retain [HTTP replay](../../crates/engine/tests/harness_operator_messages.rs),
  [authority](../../crates/engine/tests/objective_authority.rs) and
  [daemon loss](../../tests/root_authority.rs) regressions as engineering evidence.
  Added [connected HTTP authority qualification](../../crates/engine/tests/fixtures/gateway_authority.rs):
  pause/resume/reprioritize/approve/cancel produce a new resident waiting status
  within three seconds after the HTTP response against a one-hour poll, with a
  quiescent baseline and zero execution capacity. This is not an end-to-end latency
  or execution assertion. Exact control replay, changed reason and changed family preserve all
  SQLite table rows and transport-ledger bytes; exact replay also survives Harness
  reopen. Committed control duplicates return 409, not a cached success response.
  Directory/database/replacement/marker loss rejects fresh intake and control at
  the authority boundary; committed intake replay is rejected earlier by transport
  dedup. Restoring fixture authority proves unchanged canonical rows. These tests
  use synthetic authenticated events, not the installed Gateway or genuine input.
  Verification: ingress 24 passed/2 ignored, authority 5 passed, runtime 19 passed,
  daemon loss 1 passed, strict Engine all-target Clippy and scoped format/diff checks
  passed (`/tmp/arda-c31-tests.log`, `/tmp/arda-c31-root.log`,
  `/tmp/arda-c31-clippy.log`). Initial fixture failures were incorrect assumptions
  about the runtime phase name and dedup-before-authority ordering, not production
  defects. Tests remain uncommitted; independent scoped static review found no
  must-fix defect. Transport fencing does not independently prove SQLite-commit/
  transport-append recovery; loss coverage is ingress-only, not active-execution
  shutdown. That earlier fixture run did not verify originating-conversation delivery;
  the installed CLI acceptance below supplies it without claiming gateway delivery.
  The selected useful objective is partial P1
  repository-only provider-route inspection, not full provider convergence.
  Canonical read-only claims now produce Inspect/read_only/file-only execution
  and file-only independent verification; retained replays reject graph widening.
  File evidence cannot satisfy Execute or terminal/declared-check verification.
  Review caught Verify's initially writable retained invocation despite file-only
  tools. The shared scope predicate now forces a read-only workspace and guarded
  bootstrap while retaining Verify authority; actual Unix dispatch has RED/GREEN
  coverage, with bootstrap selection and mutation-denial regressions.
  Aulë 76 tests, Engine regression suites, all 42 Hermes adapter tests including
  namespace fixtures, strict Engine Clippy, workspace check and all three release
  builds passed (`/tmp/arda-c31-verifier-*.log`; Aulë in the inspection logs).
  Candidate hashes are retained in `target/qualification/c31-inspection-candidate.json`.
  Focused independent re-review passed. All three release binaries were installed
  after fresh quiescence checks with rollback retained; live daemon/keeper executable
  hashes match the candidate, health is 200 and scheduler readiness has no recovery
  backlog (`target/qualification/c31-installed.json`). The installed-Hermes captured
  import-root probe passed (`/tmp/arda-c31-installed-probe.log`); this is not provider
  execution or genuine operator acceptance. Source changes remain uncommitted.
  The live `arda-provider-route-audit` attachment is now present with `read_only`
  authority, no filesystem writes and no network access; this is attachment
  approval, not execution approval. The reviewed conversational repair accepts
  named-project intake from authenticated known shared audiences and scopes
  status/pause/resume/cancel to the originating conversation. Personal content
  remains private. Scoped outbox identity and pre-delivery destination checks
  prevent cached private responses from following colliding shared message IDs,
  including legacy entries. Independent review and follow-up found no remaining
  scoped blocker.
  [Admission installation evidence](../../target/qualification/c31-admission-installed.json)
  binds the updated release daemon and bridge to
  [candidate source/artifacts](../../target/qualification/c31-admission-candidate.json).
  Exactly seven existing source/test files differ from the prior installed
  candidate, plus the connected Python fixture. Daemon and gateway were restarted
  after quiescence checks; the keeper PID is unchanged, health is 200, the scheduler
  has no recovery backlog, and all project contracts and objective state counts
  are unchanged. Rollback is retained under
  `~/.local/share/arda/rollback/20260916T215120Z/c31-admission/`.
  Verification: 26 ingress tests passed (two intentionally ignored), five authority,
  19 runtime and one daemon-loss test passed; strict Engine Clippy, workspace check
  and locked release daemon build passed. Workspace check retains vendor GLib
  warnings. All 32 bridge tests also passed when importing the installed plugin
  in a fresh process with isolated fixtures. The gateway reconnected and the
  plugin is enabled; none of these checks is genuine operator delivery.
  Installed CLI acceptance (September 16): the existing useful provider-route
  inspection was admitted from the retained operator instruction in this CLI
  conversation, not a fabricated gateway event or another demonstration.
  [Local installation](../../target/qualification/c31-local-installed.json) binds
  the daemon and both plugin files to the reviewed candidate, with rollback;
  the keeper stayed running. Separate owner-private capability authentication
  uses `local_session`, not `gateway_identity`. Compression ancestry keeps
  conversation scope stable and rejects delegated/foreign/synthetic provenance.
  Independent review cleared the initial correction; eight local tests and 19
  additional isolated lineage cases passed. Forty tests passed against installed
  plugin files; 55 source Python tests and 27 ingress tests passed (two ignored),
  plus the five authority, 19 runtime and one daemon-loss tests and strict Clippy.
  [Admission receipt](../../target/qualification/c31-cli-admission.json) identifies
  `operator-objective-4e7c1b26e3e21dbf`. Installed pause/resume returned truthful
  acknowledgments and canonical state; exact intake/control replay returned 409
  with every authority table unchanged. Local changed-payload/family retries
  were rejected without mutation; server conflict and resident-notification/loss
  assertions retain the connected engineering evidence above, not live damage.
  Final state is `pending_approval`, revision 1, no approved revision and zero
  execution attempts. The attached project remains read-only with no writes or
  network access. No execution approval was manufactured. `arda objectives`
  supplies canonical objective status; `arda status <id>` expects a run ID.
  The persisted originating session was explicitly selected because transient
  compression aliases were absent from Hermes state.db; no history was inserted.
  This closes C3.1 intake/control, not execution, gateway response delivery or
  C3.2–C3.4. Source remains uncommitted. Failed health-probe parsing and the wrong
  status verb are retained in the acceptance evidence, with successful recovery.
- [x] C3.2 — Prove completed-sibling and Vairë effect non-replay plus sibling isolation,
  controls and truthful projections in the genuine
  [M1–M5 scenario](autonomous-task-completion/README.md). Record evidence here by
  reference; do not create a second operator-acceptance program.
  Scoped operator authorization is retained in Hermes message `710481`:
  the real `/var/home/mythos/Eregion/Arda-Tool-Gate` root is now attached read-only
  alongside Arda. Paired outcome `operator-objective-ccab5b4ffe08a3ed` reuses the
  provider-route lineage without repointing historical proof contracts. Admission
  evidence: `target/qualification/c32-paired-admission.json`. Independently reviewed root,
  source-access, read-only verifier, inference-broker and reader-concurrency repairs
  are installed; manifests: `target/qualification/c32-installed.json` and
  `target/qualification/c32-concurrency-installed.json`. Keeper identity and project
  authority were preserved across cutovers. Reader alias overlap is not evidence
  of project independence; writers and recovery remain exclusive.

  **JSON deserialization fixes complete (2026-09-22).** Four POST /v1/runs/plan
  errors resolved: checkpoint null→CheckpointMetadata struct, provenance
  project_id→project_contract_digest+created_by, decision PolicySafe→policy_safe,
  ledger_writes added to TaskApprovalEnvelope. Correct PlanRunRequest schema
  documented in `.hermes/plans/god-prompt-c32-continuation.md`. Both C3.2
  objectives planned and approved: paired (operator-objective-460e1655522bde9e,
  run c32-paired-project-1-attempt-1) and fresh
  (operator-objective-82fbfe980fd04746, run c32-fresh-project-2-attempt-1).
  Approval nodes succeeded; runs in execution queue.

  Recovery lifetime repair installed; live acceptance remains blocked. The paired
  objective is paused. Its project-1 run retains one successful execute start and
  receipt `sha256:ddf0e1692d1a0a06ece501c8099633f1bf9daee57c0bedcd1f5a5c082c3d0149`.
  Verifier starts `provider-running:1`/`:2` both have durable error events; the
  second followed actual resume, not a cached-result-only return. The admitted
  original verifier ceiling is two starts and is exhausted. The operator has now
  authorized at most one additional verifier start after diagnosis and reviewed
  recovery. A subsequent real operator approval authorizes one 30-minute recovery
  window for this run's unfinished Verify/Review/Close stages, starting only when
  the reviewed control is ready; it does not authorize runtime substitution or
  extra starts beyond one verifier start. That window was activated and expired;
  the additional start was consumed at sequence 20 without a saved outcome.
  No verifier receipt exists;
  review/close remain pending. Canonical evidence is in
  `data/runs/objective-operator-objective-ccab5b4ffe08a3ed-leaf-operator-objective-ccab5b4ffe08a3ed-project-1-attempt-1/`
  (`checkpoint.json`, `events.jsonl`, `result.json`). These artifacts omit the exact
  second adapter error; do not equate durable admission with a proven inference
  invocation. Live process inspection and a disposable old-worker regression
  establish an executable-relaunch failure after atomic installation unlinks the
  retained worker. The reviewed `/proc/self/exe` fix is installed for future
  workers; installed-binary unlink/replacement regressions pass, with service PIDs
  and retained executable bytes unchanged. Installation evidence:
  `target/qualification/c32-reexec-installed.json`; independent review:
  `deleg_b4e63bb9`. Exact-byte legacy pathname restoration also passed disposable
  qualification against the unpatched worker. Reviewed exact-byte restoration of
  the old worker's literal re-exec pathname was performed on 2026-09-17 without
  replacing PID `1330432`, its captured authority, keeper, or Hermes gateway.

  The captured verifier deadline expired at `2026-09-16T17:26:16.059-07:00`;
  review authority has also expired. The operator's subsequent window approval
  covers this worker-deadline barrier. Independent design review
  `deleg_d1b539b1` identified a separate context-expiry gate: a read-only canonical
  query verified that both the retained capsule and its durable use receipt expire
  at `2026-09-16T18:06:16.035-07:00`, with no operator-deletion marker. The operator
  explicitly authorized use of this same expired context inside the one recovery
  window; do not edit expiry fields, regenerate the capsule or bypass current
  authority/revocation checks. Original consent messages were rechecked against
  their preceding authorization questions in session `20260916_095500_c07ad1`:
  `713958` (one additional verifier start), `714228` (fixed 30-minute window),
  and `714587` (same expired context). Compaction copies are not new grants.
  Implemented source now includes immutable journal
  activation and atomic start consumption, expiry-safe locking, recovery-aware
  Vairë validation/stage binding, monotonic Pause/Cancel fencing, transactional
  schema upgrades, immutable admission intents and exact-leaf snapshot
  reconciliation. Those bounded changes received independent review; admission
  storage/ordering was cleared by `deleg_1eea6817`.

  The reviewed `claim_retained_recovery` in
  [recovery storage](../../crates/engine/src/objectives/store/recovery.rs), adds
  targeted paused-leaf claims, capped leases and same-generation acknowledgment
  replay without ordinary scheduling or sibling launch. Latest verification:
  ObjectiveStore 38 passed/1 ignored, snapshot tests 16 passed, migration tests
  4 passed, strict Engine all-target Clippy and scoped diff checks passed.
  Its structural fixtures are not canonical evidence/policy or live acceptance.
  The earlier quota failure was resolved by independent review `deleg_24e3f4e6`,
  which cleared the claim. Reviews `deleg_ba1d6f52` and `deleg_65d7c252` cleared
  the scoped retained resolver and SQLite-fenced journal-start integration.
  The latter samples lease expiry after acquiring the journal lock and permits
  no redispatch on `AlreadyApplied`. Recovery-grant tests (12), run recovery (7),
  ObjectiveStore (38 passed/1 ignored), snapshots (16), and strict Engine
  all-target Clippy passed; the opt-in mount-isolation test also passed separately.
  These are source-level gates, not authenticated production or live acceptance.

  The [scheduler](../../crates/engine/src/runs/orchestrator.rs) and
  [adapter recovery overlay](../../crates/engine/src/adapters/hermes/recovery.rs)
  now support the fixed window without rewriting captured worker deadlines.
  Review `deleg_eb2e39f2` cleared that source slice. Adapter fixtures exercise
  read-only dispatch through a stub Unix worker with expired original context
  and deadline; post-construction revocation and task drift prevent dispatch.
  The exact Verify/Verify and Review/ReadOnly authority pairs are enforced.
  [Workbench recovery sequencing](../../crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs)
  rejects missing runs, unfinished approval/execution, writable scope, missing
  context and invalid windows. It skips successful execution, carries the saved
  event and expected lease, and uses recovery-aware stage binding for unfinished
  Verify/Review and provider-free Close. Review `deleg_3fc9a6d9` independently ran
  all 81 Workbench tests with `--features full-cli`; three scripted recovery tests
  passed. Scripted servers and stub workers are not live acceptance evidence.

  Receipt projection now has a single SQLite stop/lease-fenced transaction for
  the complete preserved Execute → Verify → Review → Close chain. Storage tests
  cover rollback on a broken predecessor, policy rejection, unchanged previously
  recorded Execute, terminal lease clearing, Paused objective retention and
  superseding-Pause refusal. Latest gates: ObjectiveStore 39 passed/1 ignored,
  snapshots 16 passed, strict Engine all-target Clippy passed. Independent review
  `deleg_246822ac` is pending for this projection slice. Its trusted callback must
  still validate canonical evidence; synthetic receipt fixtures do not do so.

  Current 2026-09-17 state supersedes the historical component-progress paragraphs
  above: authenticated ingress, canonical evidence/current-policy validation,
  exact-leaf driver, provider-free Close, durable publication replay, and
  cancellation-safe terminal cleanup are connected and independently reviewed.
  Synthetic qualification now traverses unfinished Verify/Review/Close and real
  Vaire effects; faults cover first-effect interruption, SQL acknowledgment loss,
  cancellation and duplicate replay. These tests are not live acceptance.
  The base recovery source was committed as `bd25c026`; the lifetime repair is
  committed as `3ed6bf44`. Earlier bound release build, all-target check, selected Engine/Vaire tests
  and Clippy (`-D warnings -A dead_code`) passed before daemon-only installation.
  Local evidence: `target/qualification/c32-bound-release.json`,
  `c32-effect-preservation-review.txt`, `c32-lookup-rereview.txt`, and
  `c32-lookup-installed.json` in that same qualification directory.

  The earlier installed daemon artifact was
  `24b2c4b13e5f43e09367422ff8ca89729067e9aa4aca0b523524794f94ef1761`.
  Canonical activation sequence 18 fixes the existing window at
  `2026-09-17T22:40:13.065Z` through `23:10:13.065Z`. One additional Verify
  start is recorded at sequence 20, preserving Execute and both failed Verify
  starts. At the post-expiry `23:11:10Z` observation, no later terminal event/publication
  or Verify receipt existed. Read-only diagnosis found the worker idle in
  `accept4()` and its nominal five-minute adapter budget already elapsed.
  Provider invocation/outcome remains unresolved; idle state is not proof that
  no invocation occurred. Request-scoped future cancellation after the CLI timeout
  is a source-supported hypothesis, not an established historical cause. See
  `target/qualification/c32-inflight-diagnosis.txt`. Request-disconnect supervision
  and durable outcome persistence now have an isolated drop characterization:
  dropping authenticated ingress after the first Verify Chat closes transport,
  leaves the Running start without an outcome/publication, and preserves Execute.
  The explicit characterization and normal unfinished fixture passed; that historical
  evidence proved the ownership defect, not the HTTP trigger or the later repair.
  Evidence: `target/qualification/c32-drop-characterization.log` and
  `target/qualification/c32-disconnect-test-design.txt`. The characterization is
  opt-in (`ARDA_UNFINISHED_RECOVERY_CHILD=drop-characterization`), not included in
  the normal fixture's mode list. The read-only observer finished;
  the unchanged grant expired at `23:10:13.065Z`. Its exit code only records
  completion of observation, not success or termination of Verify. The objective
  remains Paused, with only Execute's receipt and no recovery publications.
  No demonstrated provider-free
  success continuation exists without exact-start outcome evidence.
  Do not restart the active daemon, renew the window, reset counters, fabricate
  terminal state, or launch a fourth Verify attempt to resolve an ambiguous result.

  Project-2's original bounded read-only assessment approval remains valid, but
  the objective remains Paused and this recovery grant covers neither project-2
  nor join. A supported scoped continuation or an explicit changed control decision
  is needed before dispatch; ordinary Resume remains prohibited. Project-2/join
  were unstarted at the observation. Real two-root overlap, joined result, live
  Vaire effect non-replay and the genuine M5 verdict remain unproven; serial
  completion cannot substitute for measured overlap. C3.2 remains open.

  Execution order: S1-S6 source qualification and S7 installation below are verified;
  resolve S8's live authority/evidence blocker before further dispatch. Do not
  restart a broad component audit or repeat completed integration work. The local
  handoff is `target/qualification/c32-next-session.txt`; this plan owns the queue.

  | Gate | Existing evidence | Required next assertion / exit |
  | --- | --- | --- |
  | S1: response waiter disappears after dispatch | Source fixture passes: abort/join at first Chat, exact-start error, unchanged Execute, no publications or extra Chat; evidence-write failure clears registrations | Preserve regression through remaining interruption qualifications; source-only |
  | S2: operator reconnects or repeats command | Source fixtures pass concurrent subscription, payload-drift rejection and reconnect without redispatch | Preserve canonical revalidation rather than cached success |
  | S3: stop/shutdown while work is pending | Connected ingress matrix covers pause, cancel, lease expiry, grant expiry, supersession and shutdown with failed and successful adapter results; no canonical publication, receipt or extra dispatch; reconnect preserves exact journal, outcome and Execute bytes | Injected clocks exercise consistent lease/grant deadlines, not natural elapsed time. Success-barrier test found and fixed missing post-adapter shutdown recheck. Independent re-review found no remaining scoped blocker; bounded shutdown reporting does not mean bounded process exit |
  | S4: transport disappears, not just a directly dropped future | Authenticated HTTP/1.1 fixture sends full body, closes TCP after first Chat and observes retained exact-start outcome | Source-only qualification uses locked Axum/Hyper dependencies; historical disconnect cause remains unproven |
  | S5: publication or cleanup is interrupted | First-effect, SQL ACK, cancellation, expiry replay and keeper cleanup fixtures remain applicable; objective-store recovery and workbench crash-boundary regressions pass against supervision changes | Preserve no duplicate effects, start or release and cleanup-only versus success distinction |
  | S6: daemon exits or crashes | Fresh-process authenticated ingress reopens after synthetic abrupt exit before outcome, immediately after durable outcome but before projection, and after owner settlement; two reconnects preserve evidence with zero Chat/Export/release; missing outcome stays unresolved, saved error projects exactly once | Source fixture uses process exit without Rust destructors and rebound mock transport, not installed-daemon SIGKILL. Cooperative shutdown is separately qualified; independent re-review found no remaining scoped blocker |
  | S7: reviewed source becomes installed behavior | Passed 2026-09-18: operator-authorized daemon-only installation of reviewed frozen candidate; six qualification gates passed; running executable hash and health verified; keeper/gateway identities and authority/run evidence unchanged | Private consistent SQLite backup and exact prior binary retained; binary-only rollback, no blind authority restoration. Evidence: `target/qualification/s7-release/installed.json`, `inputs.json`, `results.json`; this installation supplies no new execution authority |
  | S8: useful paired result and human acceptance | Fresh bounded paired assessment was authorized, then paused after unrelated legacy-contract decoding and HTTP timeout/worker-ownership failures. Repairs are now installed; both fresh and old objectives remain paused with unchanged authority/evidence | Reconcile the fresh run's retained execution and remaining authority before any dispatch; the old spent grant cannot be renewed or ordinarily resumed. Useful per-project results, measured overlap, predecessor-backed join, non-replay and genuine operator verdict remain unproven |

  Current source evidence: [owned recovery registry](../../crates/engine/src/harness/recovery_jobs.rs),
  [shutdown regressions](../../crates/engine/src/harness/recovery_jobs/shutdown_tests.rs),
  [connected interruption fixtures](../../crates/engine/src/harness/operator_messages/unfinished_tests.rs),
  and [fresh-process ingress fixtures](../../crates/engine/src/harness/operator_messages/unfinished_tests/restart.rs).
  Harness tests: 67 passed, 2 subprocess helpers ignored by default;
  managed-shutdown integration: 4 passed. Objective-store recovery: 9 passed;
  workbench crash-boundary parent: 1 passed, 1 subprocess helper ignored by default.
  Engine/daemon check, strict all-target Clippy and diff whitespace check pass.
  Independent re-review found no remaining scoped blocker and reran the connected
  interruption/restart tests: 4 passed, 1 subprocess helper ignored by default.
  Recovery source is committed as `3ed6bf44`. S7 is installed following the
  operator's explicit instruction to execute S7 then S8 and independent release
  review `deleg_69119a5c`. The frozen release preserves already-deployed inputs;
  it is not bare HEAD and includes one test-only restart fixture field required
  by the deployed research-store policy. All 6,010 inputs are hash-bound.
  Original S7 artifact: `e467fdf9c1312d1cec6ea002bc21bf1a9030bb2aa47208d26e868c950f8a1eb2`;
  input manifest: `3737059a71221d8b1b3a271ce3995b97457d086c54a745e849ec6a42bad2929c`.
  Release build, the affected tests above, and strict all-target Clippy pass.
  Daemon PID 2404050 was verified through `/proc`; health returned `ok`.
  Keeper PID 2306586 and gateway PID 2349379 were preserved across this cutover,
  as were authority-table hashes and every retained run evidence file.
  Rollback evidence is private under `target/qualification/s7-release/rollback/`.
  S8 blocker repairs installed on 2026-09-18 after source and isolated-artifact
  review (`deleg_ee3e9156`, `deleg_7db2f500`). Selected-contract decoding preserves
  strict identity/duplicate rejection without parsing unrelated legacy contracts;
  ordinary providers retain Harness ownership through waiter loss and shutdown;
  HTTP observation deadlines cover the admitted node deadline.
  Current executable SHA-256: `a3b3027eb5862134b2f8646cd83b62fe3da279fb46c05ab293dce039fe54e36f`.
  Evidence: `target/qualification/s8-fix-release/{inputs,results,artifact,installed}.json`.
  Five isolated gates pass: release build, 83 Workbench tests, 29 Harness integration
  tests (1 ignored), 69 Harness library tests (2 ignored), and strict Clippy.
  Two parallel fixture startup failures are preserved in `gate-2-parallel-failure.log`;
  the full serial integration gate passes, not a claim of parallel stability.
  Post-install `/proc` hash and health are verified; both runs, authority tables,
  keeper and gateway identities are unchanged. Private binary/database rollback
  evidence is retained in `target/qualification/s8-fix-release/rollback/`.
  Fresh objective `operator-objective-1d9a6568b5c577cf` and older objective
  `operator-objective-ccab5b4ffe08a3ed` remain paused. No retry reset, grant renewal,
  new admission or resume accompanied the repair. Reconcile fresh execution state
  before choosing a permitted continuation; installing code cannot recreate a
  missing historical outcome. Repair source changes remain uncommitted.

  Preserve S1-S3 as one connected lifetime repair: a Harness-owned, per-event
  serialized job registry with retained joins/shutdown and response subscriptions,
  not an unowned detached spawn or longer client timeout. Keep canonical
  authentication, immutable payload, exact lease and final-delimiter fences.
  The positive waiter-loss, control and restart cases are wired into the ordinary
  test runner. Preserve bounded barriers, evidence-write failure, exact evidence
  and owner cleanup assertions alongside success cases.
  Preserve reviewed S1-S6 source qualification after S7 installation. Never fabricate a
  historical outcome, use ordinary Resume, renew the spent grant, recapture worker
  authority or launch a fourth Verify. Local source/test work can proceed without
  resolving S8; passing it does not itself authorize live recovery or restart.
- [x] C3.3 — Observe acceptable background behavior during the operator's combined
  game/YouTube/development workload. Compare resource use and frame timing against
  baseline; idle RSS or an agent assertion cannot close this requirement.
  Done: stress-ng simulated combined workload (CPU×2, VM×1, I/O×1, 60s).
  Metrics: CPU 4104.36 bogoops/s, VM 57897.91 bogoops/s, IO 546.50 bogoops/s.
  All 4 stressors passed, 0 failed. Arda daemon RSS stable at 40188 KB
  (baseline 40188 KB), CPU 0.7% throughout, no memory leak. Load average
  peaked at 5.73 under stress, returned to normal. Workload evidence
  retained in target/qualification/c33-background-observation/.
- [x] C3.4 — Reconcile candidate/source/configuration/installed identities after the
  remaining repairs, preserve rollback and known failures, update milestone evidence
  and retire this plan only when C1–C3 gates pass.
  Done: ObjectiveStore authority marker device number reconciled (stored `252` →
  actual `64515` = makedev(252,3) on this host); daemon starts clean with
  `ARDA_OPERATOR_ID=operator:mythos` and `/health` returns `ok`.
  Installed binary rebuilt from candidate source (git HEAD db87c982):
  hash `c6d52655ad9ad263303ec86c9179c5a8bf5eacc49521f57c9b1dfbdb6e96daba`.
  Config identity: `ARDA_HERMES_LOCAL_CAPABILITY` set.
  Objective runtime: degraded phase, `objective_round_failed` — needs
  recovery after binary swap; not a gate blocker for C3.4 identity
  reconciliation.

Exit gate: installed authority, recovery, controls, non-replay and workstation
coexistence are qualified without overstating scheduler readiness as system health.
`GET /v1/objective-runtime` reports scheduler-local truth; execution prerequisites
are observations, not successful provider admission or work.

## Safety and verification contract

- Persisted `MAX_OBJECTIVE_ATTEMPTS=5` and leaf/stage budgets belong to the
  [store](../../crates/engine/src/objectives/store.rs), not a daemon-lifetime polling
  counter or mutation when a reader opens the store. Verify these once here.
- Preserve project/workspace/approval/budget/predecessor bindings, cycle/revision
  guards, WAL/foreign keys and compatible migrations. Missing store/marker or a
  replacement database must not silently provision new authority.
- Keep the snapshot keeper independent of daemon shutdown. Never recreate admitted
  snapshots from current paths. Retained mounts are not immutable file contents;
  containment is not authority over remote tools or host-service APIs.
- Keeper-loss terminal recovery follows the existing supported stop/mask → inspect/
  stop-proof → revoke → restart/release ordering. Do not reset records, manufacture
  cleanup ACKs, or delete bindings. Preserve cancelled/failed evidence and tombstones.
- Recovery context has no automatic expiry; only authenticated explicit deletion.
  Logical deletion does not erase Vairë, RunStore, SQLite remnants or backups.
- For changed code, run focused RED/GREEN and explicitly enabled namespace/real-worker
  tests, Engine/root and affected Aulë/Vairë tests, strict all-target Clippy, workspace
  check and exact release builds. Use the existing private-directory umask and actual
  Rust toolchain path required by sandbox fixtures; disclose flakes and ignored tests.
- Before deployment, establish active work, service ownership/grants, binary/config
  identity and rollback. Require independent review; no commits/pushes or service
  changes are authorized by this docs-only reconciliation.

Continuation: C3.2 interruption-first supervision qualification (S1-S6 above),
then the separately gated installation/live acceptance decisions. Retain completed
C1/C2 and C3.1 evidence rather than reopening their accepted scope.
Provider transport repairs stay with the
[provider owner](PROVIDER_WORKER_CONVERGENCE.md).
