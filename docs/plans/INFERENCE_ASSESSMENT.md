---
soterion:
  sigil: "SCROLL"
  role: "assessment_plan"
  owner: "HERMES"
  status: "draft"
  reviewed: "2026-10-09"
---

# INFERENCE_ASSESSMENT

## Current specification: mixed inference and home-server capacity

This section supersedes the earlier general-agent replacement framing and blanket
no-promotion disposition. Assess and implement useful roles across local inference,
valid free providers and frontier providers already available through Manwe.
Core need not own multi-step continuation. A model may qualify for a bounded role
while failing autonomous coding or research; retain failures at their actual scope.
Hermes is the worker/conversation harness and Arda retains policy and authority.

Required outcomes:
- Support concurrent interactive Arandur-oriented conversation, scheduled research,
  maintenance, tool development, project assessment and essential-service monitoring.
- Match each job to demonstrated capability, privacy, context, latency, availability
  and cost; escalate difficult work rather than demand frontier behavior locally.
- Distinguish catalog presence, authenticated inference access, quota and measured
  task success. Free-provider labels are not guarantees of continuing availability.
- Expand Beelink toward a home-server role, not merely another resident model.
  Inventory storage, services, backups, resource contention and recovery first;
  preserve reliable home services ahead of optional inference residency.
- Explore economically useful workflows toward self-sustaining operation and
  hardware reinvestment. Track costs and realized value; revenue is not assumed.
  Spending, customer commitments, financial transactions and expanded external
  authority require explicit operator approval.

Execution order under the operator's plan-adjustment/implementation request:
1. Refresh Manwe's live provider/capability/access matrix and Beelink's actual
   resource/service/storage inventory without exposing credentials.
2. Refresh Hugging Face discovery beyond the previously tested candidates. Pin
   promising artifacts/licenses/runtime prerequisites per role before downloading.
3. Qualify bounded local specialist jobs and valid remote escalation routes using
   existing Manwe/Hermes paths; do not invent a parallel scheduler or authority path.
4. Exercise mixed concurrent work with bounded queues, priorities, cancellation,
   latency/cost accounting and fail-closed unavailable-provider behavior.
5. Promote independently reviewed role-specific improvements incrementally with
   rollback. Existing model age motivates comparison, not presumed inferiority.

Acceptance: one useful interactive stream can coexist with a scheduled background
job without unacceptable contention; attribution, outcomes and failures are retained;
local specialists and remote escalation satisfy their separate contracts. Beelink
service additions require explicit service selection after inventory, secure network
exposure and recovery checks. Prior disruption limits remain; this request does not
permit wiping storage, buying hardware or replacing the entire fleet.

Earlier tests remain evidence, not a requirement that every node become a general
agent. Full-profile continuation is mandatory only for roles that actually own it.

### Mixed-role execution checkpoint

Current operator direction: retain the routing repairs, candidate evidence and
useful implementation below; the progress check is not a reset or cancellation.
Converge on one operating arrangement: desktop GPU reserved for operator use,
bounded background work on Server, and a demonstrated capable escalation route.
The operator stopped desktop `mesh-llm`; readback confirms inactive/MainPID=0.
Do not restart it for qualification. Keep backups and HUD/voice parked.
Current installed checkpoint: the reviewed Responses body-read timeout repair
is deployed in supervised child PID 86591 under unchanged Arda PID 2711031;
[deployment readback](../audits/inference-assessment-2026-10-09/escalation-timeout-deployment.json)
records hash `464348830f1a9614230624d736abc77fede8ed750b105d65e62ab38a27a49dcb`,
healthy subscription routing and preserved inactive desktop inference. All 320
release-mode library tests passed in the exact baseline export plus reviewed
single-file patch. Body-read timeouts now remain transport errors rather than
becoming empty malformed payloads; the configurable default allows 120 seconds.
The [installed Hermes task](../audits/inference-assessment-2026-10-09/subscription-hermes-acceptance.json)
completed missing-file recovery, evidence reads and report writing in 69.366 seconds.
The [failed pre-repair task](../audits/inference-assessment-2026-10-09/subscription-hermes-acceptance-pre-timeout-repair.json)
is retained. A timer-fired capped Server check and subscription interpretation
completed during the successful task, with 5.844 seconds of client-interval overlap.
This does not establish simultaneous upstream compute or sustained desktop latency.
Independent factual review `deleg_5ba72afa` accepted the retained report and actual
read/recovery/write behavior. Its reviewed two-cycle snapshot is preserved; the
[final combined evidence](../audits/inference-assessment-2026-10-09/installed-operating-acceptance-final.json)
adds the third successful cycle and correlates every result with a distinct built-in
scheduler execution, scheduled instant and saved cron output. Job `fc6b7719f468`
readback is 3/3, completed, disabled, with no next fire. First cycle predates this
repair; this is bounded recurring acceptance, not a contiguous post-repair soak.
The strict offline event validator passes the actual run and rejects five corrupted
controls (missing recovery, failed read/write, changed artifact, failed final).
The report's phrase "No inference requests" means no additional inference probes
by the assessment agent; Hermes itself used inference. Collector invocation text
describes its entrypoint, not absence of the enclosing scheduler. No filesystem
sandbox or future-response semantic guarantee is claimed.

Bounded allocation disposition: keep desktop inference off; use the capped Server
collector for endpoint facts and the explicitly selected subscription route for
reviewed evidence interpretation and file-only assessment. Do not promote failed
local structured-output or Cohere monitoring roles. This tested arrangement is
usable within those limits; the assessment has not qualified unattended automatic
role selection, sustained desktop/gaming latency, full-profile coding/research, or
voice. Backups and HUD/voice remain parked per operator direction. That acceptance
timer ended without installing permanent monitoring; the authorized operating
closeout below supersedes its scheduling status. Source changes and assessment
artifacts remain uncommitted.

### Authorized operating closeout (current execution owner)

The operator authorized finishing sustained coexistence, normal-profile coding
and research, automatic routing/failure behavior, a durable monitoring job, and
reviewed scoped closeout. Historical candidate checklists below are evidence,
not instructions to repeat failed candidates or restart desktop inference.

- Normal-profile per-invocation `arda-manwe` / `auto` research and coding tasks
  completed with real tools; independent factual reviews accepted bounded local
  evidence research and test-only coding (15 offline tests). This does not change
  the configured main model. Route history is time-correlated, not session-bound.
  Evidence: [research](../audits/inference-assessment-2026-10-09/normal-profile-auto-research.jsonl),
  [coding](../audits/inference-assessment-2026-10-09/normal-profile-auto-coding.jsonl).
- Reviewed deterministic monitor is installed as Hermes job `d0e3913bbf03`, every
  five minutes, script-only, local saved output, no inference or remediation.
  Source: `scripts/inference_endpoint_monitor.py`; state:
  `~/.local/state/arda/inference-monitor/`. `--status` evaluates receipt age now;
  its exit code is not endpoint-health acceptance. Pause with
  `hermes cron pause d0e3913bbf03`. No external alert delivery is claimed.
- The predeclared 30-minute resource/desktop-state observation completed:
  [raw window](../audits/inference-assessment-2026-10-09/operating-soak.jsonl),
  [final independent review](../audits/inference-assessment-2026-10-09/operating-closeout-review.json).
  All 60 sampled desktop states were inactive/MainPID=0; six monitor receipts
  correlate with six distinct completed scheduler executions. The terminal
  elapsed-window record is not an extra resource sample. This establishes bounded
  observation alongside separately retained interactive tasks, not continuous
  interactive load, gaming latency or long-term availability.
- Generic non-stream failure repair is independently reviewed and deployed:
  [review](../audits/inference-assessment-2026-10-09/nonstream-envelope-review.json),
  [deployment/readback](../audits/inference-assessment-2026-10-09/nonstream-envelope-deployment.json).
  Running SHA-256 is `4bedd9f7f86f04cfe090f37c7bcae05cdf0af3f261a808a445a406032e13f4f2`;
  only the supervised Manwe child changed, not its Arda parent or desktop inference.
  [Release log](../audits/inference-assessment-2026-10-09/nonstream-release-tests.log)
  retains 320 passing tests and the build. The reviewed 30-case matrix passed;
  [31-case followup](../audits/inference-assessment-2026-10-09/operating-envelope-matrix-followup-qualified.json)
  additionally verifies delayed-body latency, post-header truncated chunks and
  exact positive payloads. Automatic fallback, forced/local-only failure,
  timeout health policy and non-success HTTP status preservation are exercised.
  The post-install governed probe and ordinary forced completion passed.
  Scope does not establish native streaming or every provider dialect.
  The independently re-reviewed [hermetic replay](../audits/inference-assessment-2026-10-09/operating-envelope-matrix-hermetic-green.json)
  passes the same 31 cases without depending on live provider configuration.
  The runner requires an explicit `MANWE_TEST_BINARY`; build prerequisites are
  documented in its header. Older probes/transcripts retain external environment,
  dirty benchmark/voice-adapter and scratch dependencies: historical evidence,
  not turnkey reproduction. Retained artifacts contain private operational metadata.
- Technical review is complete for this bounded closeout; source/evidence are
  included in the scoped closeout commit. Backups, voice/HUD, purchases and desktop
  model reactivation remain excluded. Gaming responsiveness needs an operator
  verdict; measured latency alone cannot supply one.

#### Historical execution trail (superseded status and next actions)

At the earlier checkpoint, the next dependency was routed escalation: installed
Manwe reported `openai_sub` unhealthy with
`HTTP 502 provider_server_error: Unrecognized Responses payload`,
while the current Hermes conversation uses the subscription directly. Diagnose
that existing adapter boundary rather than assume subscription access is absent
or start a new model search. Both Server lanes remain catalog-ready, but their
retained role-specific quality failures still apply. Independent reconciliation
confirms payload preservation, context-reserve and negation repairs are proven
in source/isolated candidates, not installed acceptance. The existing Codex
Responses dirty changes require exact-artifact review separately; historical
release gates do not qualify the whole current tree. A
[fresh escalation probe](../audits/inference-assessment-2026-10-09/escalation-route-probe.json)
is rejected before upstream for `provider_unhealthy`; it does not reproduce the
historical payload failure. Next: review the existing adapter changes, qualify an
isolated artifact and recover through the governed provider/child lifecycle.
Do not clear health ledgers by hand, restart all of Arda, or restart desktop
inference. Preserve source repairs and reuse prior gates rather than resetting
the assessment. Adapter review reproduced terminal-snapshot text loss, colliding
normalized call IDs and incomplete SSE success. Focused RED regressions failed
for each; fixes now preserve terminal text, reserve valid IDs and reject incomplete
SSE. The source-only routing repairs are committed as `dc3f3a67`, not deployed.
All 318 Manwe library tests pass from the exact isolated commit tree; independent
review accepted it after optional-schema eligibility was separated from required
execution/context floors. Normal-SSE text-only qualification remains the scope.
The [isolated artifact probe](../audits/inference-assessment-2026-10-09/escalation-isolated.json)
then failed before upstream: the sole stored Codex pool entry is marked exhausted
with a future reset; its access token is not expired. This is credential eligibility,
not a reproduced payload failure or proof the direct conversation lacks access.
No cooldown or health state was cleared, credential override supplied, or service
restarted. Candidate shutdown/listener removal and desktop inactive/MainPID=0
were verified. The [free-route probe](../audits/inference-assessment-2026-10-09/escalation-free-probe.json)
subsequently reached Cohere through OpenRouter with reported cost zero. The
[operating pair](../audits/inference-assessment-2026-10-09/operating-pair.json)
completed sequential and overlapping concurrent Server/Nanbeige extraction plus
remote/Cohere interpretation, forced identities and fallback disabled. Background
latency was 1.32s sequential / 1.39s concurrent; remote was 10.61s / 8.02s.
Desktop inference remained inactive before/during/after. This is one text-only
pair, not sustained gaming/voice or difficult-work qualification. Extraction
returned fenced rather than bare JSON; remote interpretation misread since-boot
vmstat swap activity as current activity. Independent review confirms exact names
in both extractions but neither is bare JSON; neither remote interpretation is
fully correct. All four responses respected observation-only authority. Header
identity and completion passed for all four; client overlap does not prove
simultaneous upstream inference. The source snapshot predates inference and
cannot establish Server pressure during the pair. Provisional allocation: keep
desktop inference off, use Server only for validated bounded extraction, and
keep OpenRouter interpretation advisory. Do not promote Cohere as the capable
monitoring escalation route. Remaining acceptance is enforced background bounds,
measured during-job resources and a semantically qualified escalation route;
sustained gaming coexistence and automatic escalation remain untested.
The [explicit-format follow-up](../audits/inference-assessment-2026-10-09/operating-pair-structured.json)
used JSON-object mode and labeled vmstat intervals, not an unchanged-prompt retest.
Nanbeige still emitted fenced JSON and omitted container names; its subsequent
structured request was rejected before upstream. Readback shows the lane ready
for other work but structured-output capability false. Preserve that restriction,
not another retry or capability override. Cohere still conflated historical CPU
idle with present pressure in the concurrent response. Twenty-four GPU sample
rows were retained, but the rejected background request means they cannot prove
coexisting inference load. The incumbent Server unit has one slot but infinite
CPU/memory/runtime limits; client token/time bounds are not service resource caps.
No role promotion or service-limit changes were made.
The next [bounded deterministic Server check](../audits/inference-assessment-2026-10-09/bounded-server-check.json)
ran without inference: five endpoint self-reports returned HTTP 200. Its transient
unit read back CPUQuota=25% of one CPU, MemoryMax=128MiB, no swap, TasksMax=16
and a 25-second oneshot startup timeout with a 5-second stop timeout; failure-path
teardown timing has not been tested. The corrected rerun reported 397ms runtime,
103ms CPU time and 13M peak memory. Desktop inference was off before and after;
the temporary unit was inactive/PID=0 afterward. This is one manually invoked
job, not a scheduled deployment, end-to-end application health, or model escalation
qualification. The initial harness used RemainAfterExit with pipe/wait and hung
after child completion; cleanup ran, and the corrected lifecycle returns normally.
Independent review accepted the narrow observed-success claim and identified
aggregate-verdict and evidence-retention defects. These were corrected: all five
exact endpoints must pass, cleanup reads must succeed, and command results are
retained. Five regression tests pass (four reproduced defects before correction),
and the corrected live run passes. No failure-limit qualification is claimed.
The [remote semantic screening](../audits/inference-assessment-2026-10-09/escalation-semantic.json)
found GPT-OSS-120B and Hermes-405B admission-blocked by stored unhealthy/cooldown
state; GPT-OSS also conflicts with the request's reasoning-display policy. No
state was reset. The [matched alternatives](../audits/inference-assessment-2026-10-09/escalation-semantic-alternatives.json)
reached Nemotron Super through OpenRouter: HTTP 200, identity matched, reported
cost zero, 9.30s elapsed. All four synthetic monitoring values and the sustained-health
limitation are correct; backup uncertainty has an empty citation array rather
than expected S4. The strict rubric therefore fails; independent review will
assess the prompt's ambiguity before interpreting this as model failure. Separate
reasoning fields are present. Laguna XS reached shared-pool HTTP 429. Desktop
inference remained inactive at before/after snapshots. Next: reviewed Nemotron
contract paired with the bounded Server check, not another local-model retry.
These probes do not qualify difficult-work escalation or production scheduling.
Independent review accepted Nemotron's synthetic semantics, preserving the strict
mechanical failure as a citation-contract ambiguity. The new
[matched operating pair v2](../audits/inference-assessment-2026-10-09/operating-pair-v2.json)
explicitly requires citations for unknowns and requests reasoning exclusion.
Both returned responses contain final JSON without populated reasoning fields;
all four values and citations pass and neither answer recommends actions.
Sequential/concurrent remote times were 5.212/8.509s (reported cost zero);
Server-check wrapper times were 0.994/0.946s, with 443/367ms actual unit runtime.
Both Server runs returned all five HTTP 200 observations and verified configured
CPU/memory/swap/start/stop limits plus inactive cleanup. Client intervals overlap
for 0.946s, not proof of simultaneous upstream compute. Desktop snapshots remain
inactive before/after. The remote answers interpret the frozen synthetic packet,
not the concurrent check's eventual observations. One pair cannot attribute remote
latency variation to contention or establish sustained load. Independent pair
review accepted this narrow manual observation. A subsequent
[real endpoint interpretation](../audits/inference-assessment-2026-10-09/real-endpoint-interpretation.json)
collected fresh capped Server checks, sent only service names/timestamps/statuses
with source IDs, and returned all five observations correctly, with workload health
and backup recovery unknown. No URLs or credentials were included in the remote
packet. Hermes cron job `d431d0bafdf8` was created local-only for one acceptance
cycle; manual dispatch `deleg_efc8d53b` completed successfully. Readback confirms
`last_status=ok`, repeat 1/1, disabled/completed, and no next run. The retained
Hermes response at `/var/home/mythos/.hermes/cron/output/d431d0bafdf8/2026-10-11_02-35-08.md`
contains exact S1–S5 endpoint citations, the real packet timestamp range, unknown
workload/backup outcomes, and appropriate limitations. This verifies actual agent
report generation and local saved output, not merely script success. The manual
fire uses the cron execution path; timer-triggered execution is not established.
The job was paused after dispatch to prevent a later timed fire. Installed cron
refuses paused-job execution despite the tool description, so a brief resume was
required. Allocation for the tested role: preserve desktop availability, collect
facts using a capped deterministic Server job, and use Nemotron only for validated
observation-only interpretation of sanitized packets. Keep the existing Hermes
conversation for difficult work; automatic Manwe subscription escalation remains
blocked, not replaced or qualified by this endpoint task. Representative difficult
work, recurring operation and sustained scheduled coexistence remain open. No
production routing policy or model-service changes were made.
Subscription discrepancy diagnosis: live usage checks with both stored tokens
(singleton and pool) report allowed=true, limit_reached=false and 37% used.
Tokens differ but account/subject match. Hermes main resolution reads singleton;
Manwe honors the pool. Normal Hermes pool selection returns `ok` after its live
quota-restored check, but readback remains `exhausted`: persistence merges the
old timestamped cooldown over the recovery's cleared timestamp. An isolated
regression reproduces this exact divergence. A Hermes source repair stamps the
recovery observation before the probe, allowing later concurrent failures to win.
Independent review caught same-token terminal-state resurrection; the correction
preserves peer DEAD verdicts and adopts the locked merge before selecting recovered
credentials. Re-review `deleg_288b1c5e` accepted; 201 credential-pool/host-binding
tests pass. Normal live pool selection now returns `ok`, and disk readback is `ok`.
The [post-repair isolated probe](../audits/inference-assessment-2026-10-09/escalation-recovered-after-persistence.json)
returned HTTP 200, exact `ROUTE_READY`, provider `openai_sub`, model `gpt-6-astra`
in 1.58 seconds. Candidate process/listener cleanup passed; desktop inference
remained inactive. The earlier [blocked attempt](../audits/inference-assessment-2026-10-09/escalation-recovered.json)
is retained. No manual cooldown reset or credential override was used.
This establishes credential recovery and isolated text transport, not installed
routing deployment, representative difficult-work qualification or tool continuation.
The [guided text workload gate](../audits/inference-assessment-2026-10-09/escalation-workload.json)
now passes three mechanical cases: temporal monitoring evidence (9.01s), conflicting
credential evidence (13.61s), and concurrent credential code review (32.15s).
These are synthetic, explicitly guided fixtures, not autonomous difficult-work
qualification. Review `deleg_4c8562b2` accepts monitoring and incident reasoning,
but qualifies the code-review answer: the fixture rebinds `entry` without updating
`entries`, so its claimed disk overwrite assumes an omitted step. The stale returned
candidate concern remains valid. Mechanical checks are not semantic qualification;
the prompts provide substantial hints and do not establish independent bug discovery.
Review binds JSON SHA256 `b1f3bdd84989f6d49225e955beedcd031e4074db34f76a276f82804291367d0b`.
All replies
have exact subscription route attribution and terminal completion; candidate
cleanup passed and desktop inference stayed inactive at boundary snapshots.
Live discovery confirms the installed listener is port 7171 (an initial guessed
8091 probe was refused), owned by Manwe PID 2711047 under Arda PID 2711031.
Its running executable hash is `288c6c95fb11139ad9409e7ba761203e8319a7079fd93b3562af6e7d801bb641`,
which differs from both the reviewed debug candidate and current release-path bytes
(`4a1e575839a30ee1dc00340fbd5ca11c71ed09b933e67534099615a7fca57ce1`).
The running inode is deleted; a blind child restart would activate different,
not-yet-qualified bytes. Installed `openai_sub` remains unhealthy with the historical
payload error. Next: qualify/freeze the exact deployment artifact, preserve rollback
bytes, replace only the supervised Manwe child, and use authenticated provider probe
before ordinary-route acceptance. The exact-commit release was subsequently built
from a source export verified against 1,051 tracked Cargo/crate/vendor files;
all 318 release-mode library tests passed. Its isolated subscription marker passed
([release probe](../audits/inference-assessment-2026-10-09/escalation-release.json)).
The [deployment attempt](../audits/inference-assessment-2026-10-09/escalation-deployment.json)
installed that hash and restarted only the supervised child under unchanged Arda
PID 2711031. The authenticated probe with `ignore_throttle=false` was refused before
upstream: `provider_failure_memory` from the historical payload error. Source shows
this branch has no elapsed-time check; its reported 900 seconds is not a countdown.
The script automatically restored the previous running bytes, verified healthy
router PID 4169297 under the same parent and candidate absence; desktop inference
remains inactive. Rollback bytes and the candidate are retained under
`/var/home/mythos/.local/state/arda/manwe-releases/dc3f3a67/`.
No health ledger was manually edited. The operator then authorized reinstall and
stale-failure recovery. The [authorized deployment](../audits/inference-assessment-2026-10-09/escalation-deployment-authorized.json)
passed: running child PID 71886 has reviewed release hash
`46acd5c1a510042cbf2fa22c949d9408b676fe929a9ba09d8d920d8c0953e276`
under unchanged Arda PID 2711031. One authenticated provider-scoped
`ignore_throttle=true` probe succeeded, followed by an ordinary forced subscription
completion without an override. A subsequent authenticated probe with
`ignore_throttle=false` returned exact `RECOVERY_VERIFIED`; readback shows
`healthy=true`, `last_error=null`, `consecutive_failures=0`, `in_cooldown=false`.
The stale parser failure no longer refuses the subscription route. Historical
failure receipts remain; throttle policy is unchanged. Desktop inference remains
off and rollback artifacts are retained. This closes installed text-transport
recovery, not representative difficult-work, automatic selection or tool-loop qualification.
Orphan/duplicate tool-history validation, malformed incomplete/raw-JSON envelopes
and explicit credential-environment override semantics remain separate limitations,
not resolved by these tests.

Fresh [Manwe catalog snapshot](../audits/inference-assessment-2026-10-09/mixed-role-providers.json)
and [Beelink inventory](../audits/inference-assessment-2026-10-09/beelink-home-inventory.txt)
are retained. Beelink has substantial currently available memory/storage but still
needs backup, exposure and contention qualification before new home services.
The first free-provider monitoring probes demonstrate access limitations, not
model quality: Qwen's free route is excluded by model health/cooldown, while
[Gemma's free route](../audits/inference-assessment-2026-10-09/mixed-role-free-gemma-monitoring-probe.json)
reaches upstream and returns HTTP 429 from its shared pool. Do not bypass cooldown
or treat a healthy catalog row as usable quota.
[Hugging Face discovery](../audits/inference-assessment-2026-10-09/mixed-role-hf-discovery.json)
pins eight families and identifies Granite 4.2-3B extraction/classification,
Devstral Small 2 bounded code review and Qwen3-ASR-0.6B dictation as first role
candidates; runtime/license and genuine-input gates remain per candidate.
The [first Granite CPU smoke](../audits/inference-assessment-2026-10-09/granite-cpu-role-result.json)
used hash-verified publisher Q4 weights on Server without stopping either GPU
service: extraction, classification and unknown-value abstention pass; the
source-summary response gives the right port but omits the required citation.
This is 3/4 mechanical checks, not representative or routed qualification.
The bounded candidate is stopped and both incumbent endpoints read back healthy.
Independent review confirmed the bounded smoke and cleanup, not qualification.
The [isolated routed comparison](../audits/inference-assessment-2026-10-09/granite-routed-role-result.json)
then admitted truthful 4096-context Granite using explicit background target,
with tools and fallback disabled. Both Granite CPU and incumbent Nanbeige GPU
passed held-out missing-field extraction and distractor classification. Recorded
latencies on the hardened rerun were 4.22/1.37 seconds versus 0.51/0.14 seconds respectively; this is
not a same-hardware speed comparison or evidence of candidate superiority.
Upstream captures and route headers agree; the candidate is stopped and both
incumbents remain healthy. Independent review accepted the narrow smoke;
its harness findings were corrected and the rerun passed admission/capture
consistency, incumbent health and exception-contained cleanup checks. Built-in
fallback providers remain in the isolated router catalog, so do not claim a
two-provider-only catalog or tested failure-path fallback enforcement.
No production config was changed. Beelink discovery found existing rootless
Podman database/storage/monitoring services; backup recovery remains unverified,
not proven absent. The [real home-service workload](../audits/inference-assessment-2026-10-09/home-mixed-workload-result.json)
ran after a semantically equivalent wording change: "Do not make changes"
was falsely inferred as affirmative tool use by the installed router. This was
not a background scheduling failure. The [original Bonsai rejection](../audits/inference-assessment-2026-10-09/home-mixed-workload-bonsai-admission.json)
is retained. With "Do not change anything", Nanbeige admitted both roles and
returned all four sequential/concurrent responses locally with fallback disabled.
Sequential completion times were 6.63s background / 3.62s interactive; concurrent
were 8.65s / 3.34s on the same provider. This one pair does not qualify sustained
contention or voice responsiveness. Quality remains insufficient: responses
mischaracterize swap occupancy, overstate health/uptime and omit important failed
units. Transport success is not monitoring competence.
A source regression reproduced the negation bug. Independent review found
newline/comma and mixed-affirmative regressions in the first candidate; native
RED/GREEN cases now cover these and all 316 Manwe library tests pass again.
The [rebuilt isolated router](../audits/inference-assessment-2026-10-09/home-negation-fixed-router-result.json)
admits and executes the original prohibition wording locally with the background
role; its temporary listener is verified absent afterward. Follow-up review
accepted isolated qualification, with a nonblocking cross-message modifier
heuristic limitation retained. The [release artifact trial](../audits/inference-assessment-2026-10-09/home-negation-release-router-result.json)
also passes original-wording admission and actual inference with cleanup. This
is a scoped text heuristic, not general natural-language authority parsing.
The fix remains uncommitted and undeployed: live Manwe is owned by the root
Arda supervisor, so replacement requires its established child lifecycle and
retained-work preservation, not a blind root-service restart.
The [Bonsai/Nanbeige mixed-role trial](../audits/inference-assessment-2026-10-09/home-mixed-bonsai-result.json)
returned all four local responses, but quality still fails: Bonsai identifies
both failed units yet recommends restarts despite read-only instructions;
Nanbeige invents CPU pressure from idle vmstat rows. No proposed action was
executed. The [Aule home-server measurement slice](../audits/inference-assessment-2026-10-09/HOME_SERVER_MEASUREMENTS.md)
now consumes existing Prometheus/node_exporter samples and read-only system/user
failed-unit observations through `arda-cli metrics home-server`. It retains raw
sources, sample timestamps, measured/stale/unavailable status, and evidence-linked
deterministic summaries; model interpretation and actions remain absent. The
[live qualification](../audits/inference-assessment-2026-10-09/home-server-qualification.json)
checks 57 observations, all 24 CPU series and failed-unit names against direct
Beelink reads. This is not backup, application-health, contention or model-quality
acceptance. Independent review findings were corrected and follow-up found no
remaining concrete blocker. The subsequent [inventory qualification](../audits/inference-assessment-2026-10-09/home-inventory-qualification.json)
checks all 11 observed rootless containers and three selected system/user daemons
against independent reads. Caller-selected expectations distinguish running,
not-running, not-observed, stale and unavailable without creating service policy;
`minio-init` and the inactive duplicate user exporter are negative controls, not
new required daemons. Three inventory tests, six CLI tests, eleven projection
tests and 367 library tests pass. Independent read-only review found no blocking
findings in this increment.
Source-built only, uncommitted and not installed/scheduled.
[Read-only loopback application probes](../audits/inference-assessment-2026-10-09/home-application-probes.json)
returned HTTP 200 from Grafana, Prometheus readiness, MinIO readiness, OpenWebUI,
ClickHouse ping and Langfuse health. These are endpoint self-reports, not workload
or restore acceptance. The [integrated application report](../audits/inference-assessment-2026-10-09/home-application-measurements.json)
now also verifies all six self-reports through caller-selected `--application-probe`
profiles in the same Aule command. Fixed GET paths, four-second/16-KiB bounds,
no credentials or redirects, raw sources and stale/unavailable distinctions are
implemented. Four application tests plus seven CLI and eleven projection tests
pass; all 371 library tests pass. Independent read-only review found no concrete
blockers and validated the six retained application observations.
[Backup discovery](../audits/inference-assessment-2026-10-09/home-backup-discovery.json)
found restic/borg/rclone installed. The operator confirmed backup setup is still
outstanding; tool presence is not configured protection. Backup coverage and
restore recovery are not established. Next backup gate: select an independent
destination and protected datasets, then approve application-consistent backup,
retention and an isolated restore test before deployment. Do not search for an
assumed existing repository or treat same-disk copies as disaster recovery.
Backup setup is outside this read-only measurement slice; no installation,
scheduling, remote upload or credentials change is authorized by this status
clarification. The operator has explicitly deferred backup setup because no
destination is available; do not block inference work on it or repeat destination
questions. Sustained mixed-workload and model-summary quality remain open.

The [structured monitoring trial](../audits/inference-assessment-2026-10-09/structured-monitoring-trial.json)
retains six locally routed Nanbeige/Bonsai responses across sequential, synthetic
stale-input and concurrent cases. All six transports matched their selected local
provider with forced fallback disabled. Independent semantic review rejects both
models for this contract: Nanbeige invents failed user units and conflates container
liveness with endpoint self-reports; Bonsai omits required swap/scope distinctions.
All six responses violate bare-JSON output requirements; five also use the wrong
root key. Synthetic stale cases test supplied statuses, not live freshness detection.
Concurrent packets differ from sequential packets; this is not a controlled
identical-input concurrency comparison.
This one concurrent pair is not sustained-contention qualification.
An exploratory [ID-only selection attempt](../audits/inference-assessment-2026-10-09/structured-selection-trial.json)
was rejected before inference. The [prompt-only diagnostic](../audits/inference-assessment-2026-10-09/structured-selection-prompt-only.json)
retains the exact 502 local-provider-policy rejection with JSON response formatting
removed; do not attribute the failure solely to structured-output capability.
No policy bypass or remote fallback was applied. Source inspection finds the
substring `code` in "code fences" classifies the tool-free request as `code`,
which requires a tool-capable route. The existing `x-manwe-task-type: chat` header
truthfully declares this bounded selection workload without changing policy.
The [explicit-chat rerun](../audits/inference-assessment-2026-10-09/structured-selection-explicit-chat.json)
returns valid exact-schema IDs from both requested local providers, includes the
actual failed system-unit scope, and copies facts from source rows rather than
model prose. This is one live case per model, not role qualification or evidence
of useful prioritization: both omit pressure/activity/application evidence.
Deterministic Aule summaries remain the measurement surface; free-form model
summaries are not promoted. Independent review accepted the original bounded
selection artifacts (prompt-only JSON, not enforced JSON mode), then identified
harness fail-open gaps. The harness now rejects duplicate JSON/source IDs,
missing mandatory failed scopes, wrong provider/model, truncated or multiple
choices, and tool-call envelopes, and carries packet-wide scope into accepted
renderings. Two offline RED/GREEN regression tests replay both retained responses
and reject synthetic mutations; this is validator coverage, not model quality.
The [held-out selection probes](../audits/inference-assessment-2026-10-09/selection-heldout.json)
then reject Nanbeige on multiple failed scopes and both models on the adversarial
evidence case for missing a required failed scope. Both preserve schema on the
unknown/stale case; Bonsai alone covers both failed scopes. These explicitly
synthetic probes disqualify promotion of either selector under the current
contract. No model-generated selection becomes authoritative; deterministic
full reports remain available. Further local prompt tuning is not acceptance.
Next bounded role comparison should use an independently capable escalation
route or a different demonstrated specialist, preserving these failure fixtures.
Sustained contention remains a separate unqualified gate.
The generic substring classifier remains unchanged; explicit task typing avoids
that heuristic only for this correctly declared tool-free benchmark.
No remediation, service restart, routing deployment or configuration change is
part of this slice; keep HUD stopped and voice work parked.
Historical voice baseline discovery
found installed Parakeet and retained local Whisper turbo, while active Hermes
STT still uses OpenAI; neither local control is integrated voice acceptance.
[Fresh Parakeet control](../audits/inference-assessment-2026-10-09/parakeet-offline-control-result.json)
transcribed the retained synthetic speech in 0.25s process wall time, omitting
"Please" and leaking `<EOU>`; silence produced only that protocol marker.
[Fresh Whisper CPU/VAD control](../audits/inference-assessment-2026-10-09/whisper-offline-control-result.json)
retained every normalized speech word and returned empty silence, with 2.77s
model load plus 5.93s speech transcription. These are different timing scopes,
not a representative speed/quality ranking. Both bounded processes exited with
no microphone capture or configuration changes. The
[Qwen3-ASR CPU control](../audits/inference-assessment-2026-10-09/qwen-asr-offline-control-result.json)
now preserves the same speech sentence and returns empty silence: 14.59s speech
transcription and 0.83s silence, with 5.7G peak unit memory. Its exact revision's
weight hash was verified, and the isolated Python 3.12 environment passes
`uv pip check` with the [hashed dependency lock](../audits/inference-assessment-2026-10-09/qwen-asr-runtime.lock).
The upstream Sox sdist was built during installation; this is not an offline,
fully wheel-reproducible build. Runtime inference used CPU-only Torch, four Torch
intra-op threads and the same retained synthetic WAV hashes; the launch requested
network denial plus memory/time/CPU bounds.
Independent review accepted this bounded synthetic smoke and verified all eight
model hashes, the retained lock and installed dependency versions. Existing controls
have lower observed latency in these synthetic tests, not proven general superiority:
Whisper uses INT8/English/VAD versus Qwen FP32/language autodetection. Qwen load
was measured separately; Parakeet timing includes process orchestration. Keep Qwen
staged, not selected as the CPU default. Whisper CPU/VAD is the provisional local
adapter candidate, subject to genuine operator dictation/review before selection.
The launch requested network denial and resource limits; the JSON does not retain
launcher/enforcement evidence or an explicit completion marker. Successful process
exit was observed separately, not established by JSON presence. This is not a
comprehensive filesystem/device sandbox. Staging remains in expiring Hermes scratch,
not a deployed service; replay needs the staged runtime and retained WAV fixtures.
The [Whisper command integration](../audits/inference-assessment-2026-10-09/whisper-hermes-command-result.json)
now passes the installed Hermes public `transcribe_audio` dispatcher using isolated
configuration: correct synthetic speech, silence rejected without transcript, and
missing input rejected. Speech process/dispatch latency was 8.72s, not the earlier
loaded-model-only timing. The runtime and hash-verified model are staged durably at
`~/.local/share/arda/voice-whisper/`; the new command is in the existing
`adapters/voice-capture/` owner. Four new RED/GREEN tests and eight existing adapter
tests pass. Active Hermes provider selection is unchanged; no microphone was opened.
The first isolated-home launcher attempt failed dependency provisioning; the passing
probe instead uses the installed managed interpreter/dependency paths directly,
not a new Hermes installation or complete CLI voice session. Independent review
accepted inactive experimental registration, not production activation. The rerun
uses Python `-I`, a hash-addressed read-only source copy and a provider-owned
systemd launch requesting a 75s hard runtime, 4G memory, no swap, CPU quota and
network denial; speech/silence/missing-input checks pass again (8.78s speech).
The inactive `stt.providers.arda_whisper_trial` entry is registered and read back;
all other STT settings, including active `openai`, are unchanged. The installed
config validator required `--force` for this source-verified dynamic provider key.
Actual CLI voice source queues successful transcripts directly as agent input
(`hermes_cli/cli_voice_mixin.py`), so do not activate it as a review-before-send
surface. Next: use the existing Arda `transcript_pending_review` capture path for
genuine dictation, or implement an explicit editable confirmation boundary before
CLI activation. No microphone has been opened. This remains a trusted-local-WAV
interface, not an untrusted-upload service.
The [Arda review-path probe](../audits/inference-assessment-2026-10-09/whisper-arda-review-result.json)
now exercises the real adapter CLI with the experimental
[Whisper configuration](../../config/adapters/voice-capture-whisper-trial.toml):
correct speech returns `transcript_pending_review`, `editable=true`, and both
send/action authorization flags false. Silence and malformed WAVs return
`recoverable_inbox` without transcript. Literal quotes and shell metacharacters
in filenames do not execute; recordings survive adapter calls, with only the
synthetic test harness removing its own fixtures afterward. A truncated-WAV RED
regression is fixed and all 15 adapter tests pass, including oversize/malformed
checks. This is a review-envelope result, not an observed editable UI. The newer
hash-addressed command is used only by this trial configuration; the inactive
Hermes provider still references its earlier reviewed artifact. Independent review
found no blocker in the bounded validation or synthetic envelope path and verified
the source/release/config hashes. Retention values are metadata: the adapter does
not implement an inbox cleanup/recovery lifecycle.

The existing HUD `GlobalRapidCapture` now consumes the local voice envelope:
local WAV -> local Whisper transcription -> unsaved editable draft -> explicit
Save, with audio provenance passed through `createCapture` only on Save.
Transcript arrival neither persists a capture nor authorizes an action. Strict
validation, draft-revision invalidation, cancellation/late-permission cleanup and
provenance clearing are implemented. Native synthetic WAV import visibly produced
editable text and the audio path; native Save/readback and physical microphone or
headset permission acceptance remain unverified. The retained verification is
19 focused frontend/audio tests, TypeScript checking and frontend production build;
earlier Python and Rust voice suites passed 18 and 2 tests respectively. Independent
review found no remaining concrete blocker in those lifecycle fixes. Browser
permission prompts cannot themselves be aborted, and requesting-origin enforcement
was not established. Active Hermes STT remains unchanged. Voice work is parked:
the operator requested HUD stop; service readback is inactive/MainPID=0 with no
`arda_hud` process. Do not reopen it or activate a microphone until requested.
Backup recovery remains a separate home-service requirement.
Citation-bearing research remains excluded for Granite pending separate tests.
No autonomous-continuation requirement applies to this specialist role. No
production service placement or model replacement has been applied.

## New-session handoff and authority

Goal: determine and then qualify the most useful inference deployment for the
existing home fleet, across tool use, coding, research, retrieval, voice, vision
and creative image/video prompting. Do not reduce this to a coding benchmark or
a newest-model shopping list. Hermes remains the conversation/worker harness;
Arda retains policy, authority, routing governance and verified continuity.

The operator subsequently authorized bounded experiments: priorities are coding,
research, then voice to avoid typing; no explicit resource or latency limits.
One server model may be stopped temporarily while Core and Warden stay running.
No coding delegation or commit/push is implied. Independent review precedes any
commit or promotion; no host reboot or fleet-wide replacement is authorized.

Current state: [P0 review](../audits/inference-assessment-2026-10-09/P0_REVIEW.md)
is followed by [partial P1/P2 evidence](../audits/inference-assessment-2026-10-09/P1_PROGRESS.md).
[CUDA/Saluki/voice continuation](../audits/inference-assessment-2026-10-09/P2_P3_PROGRESS.md)
corrects the first runtime test: missing CUDA library discovery caused CPU
fallback. Corrected CUDA controls and Saluki trials now retain native tool and
streaming responses. Saluki passed the small direct-endpoint suite on both
runtimes; this is not representative or routed acceptance. Bonsai was restored;
Core/Warden and both server endpoints read back healthy. Resource sampler tests
and independent review passed. PortAudio/PipeWire preflight and synthetic STT
now work through `hermes-voice`; genuine spoken-input acceptance remains open.
[Governed routing recovery](../audits/inference-assessment-2026-10-09/P4_ROUTING.md)
applied the existing Core address correction through authenticated Manwe reload
and recovered Core/server health through real probes. Forced/automatic local
research synthesis passes; the routed inert tool-schema test fails while its
direct control produces a native call. Source diagnosis confirmed optional-tool
stripping; a repair and real HTTP regression now pass 313 library tests serially.
The credential-test race is fixed; repeated parallel suites pass, and an isolated
rebuilt candidate completes a real native-tool/read/result loop through Core.
Follow-up review passed. The actual isolated Hermes CLI also completes its native
read-file/result loop through the candidate. Full-profile acceptance, release
qualification and production deployment remain open. The frozen release passed
build/test/lint, but [repeated Hermes smoke failed](../audits/inference-assessment-2026-10-09/P4_RELEASE_GATES.md)
with refusals and an invented answer. A second transport defect (slimming when
`tool_choice` is absent) is repaired and covered by 314 library plus three binary
tests; preserving the full payload did not eliminate model refusals. Deployment
is held. Independent review found no blocker in the narrow payload repair, but
identified stale slimmed-context admission for lossless tool continuations;
that defect is now repaired and independently reviewed. Boundary regressions
cover explicit targets, large output limits and estimates above 256k. A further
Core default-output reserve discrepancy is repaired (RED/GREEN, full package
tests passing); independent follow-up review found no blockers and reran the
expanded regression successfully. A [fresh frozen release](../audits/inference-assessment-2026-10-09/P4_FRESH_RELEASE.md)
passes release build/tests/Clippy. Its non-thinking Hermes smoke still fails;
explicit thinking-enabled mode passes three fresh real read-file/result sessions.
This is bounded synthetic acceptance, not full-profile/general reliability.
The [older-runtime multi-step suite](../audits/inference-assessment-2026-10-09/P4_REPRESENTATIVE.md)
passed only one of six cases, with tool-shaped reasoning instead of native calls.
The [same-weight b11430 comparison](../audits/inference-assessment-2026-10-09/P4_RUNTIME_COMPARISON.md)
now passes all six unchanged cases: research 2/2, recovery 2/2, coding 2/2.
Review found weak recovery ordering. The [stricter follow-up](../audits/inference-assessment-2026-10-09/P4_STRICT_RECOVERY.md)
proves conditional tool ordering with serial calls and temperature=0, but one
answer invents a numeric ticket ID. Factual acceptance remains open despite
mechanical 6/6. Full-profile and installed-pair acceptance also remain open.
The earlier isolated server run was restored. A [grounding/Saluki continuation](../audits/inference-assessment-2026-10-09/P4_GROUNDING_AND_SALUKI.md)
rejects Qwen grounding as insufficient and records Saluki timeouts at the original
budget. The longer-budget Saluki batch finished with research 2/2 and recovery
2/2 mechanically passing, coding 1 timed out and coding 2 unrun. Both server
lanes and Core read back healthy after restoration. Pre-cleanup evidence
checkpointing and batch failure exit status were repaired and locally exercised.
The authorized dual-GPU comparison finished: all six mechanical cases pass at
the original budget, but coding explanations contain false claims about Python
truthiness. Independent review confirms mechanical 6/6, factual 4/6; no
unqualified 6/6 claim.
Both server incumbents are restored and healthy, as is untouched Core; the
candidate is stopped. The one-run authorization is consumed; no promotion.
No production runtime or Hermes defaults were replaced.
Exact artifact review passed within scope, with provenance limits recorded in
the fresh-release report; no production changes were made. The
[candidate provenance follow-up](../audits/inference-assessment-2026-10-09/CANDIDATE_FOLLOWUP.md)
corrects Clef template wording and pins the current Bonsai 2 runtime; untried
candidates have not acquired local qualification from research alone.
Runtime/template qualification, representative suites and routing remain open. No winning
allocation or replacement rollout is qualified. P0 corrections supersede
conflicting initial-research claims below; recommendations remain hypotheses.

The repository is substantially dirty from concurrent/prior work. Preserve all
unrelated changes. Earlier connection repairs changed `config/fleet.toml` and
`config/manwe.providers.toml`; these are not research-deployment results. The
research report and this plan are uncommitted at handoff. Do not reset the tree,
stage everything, or infer deployed configuration from a working-tree edit.

### Current candidate queue (supersedes Saluki-only continuation)

Operator direction: return to the whole candidate matrix and advance coding and
research alternatives. Retain Saluki's measured limitation; do not retry it until
it passes. This table is prerequisite triage, not deployment qualification.

| Candidate | Current boundary | Next prerequisite/action |
|---|---|---|
| Qwen3-Coder-30B-A3B-Instruct | Hash-verified; native routed continuation passes at 73728 context; first Hermes coding case times out | Retain eight-layer offload latency limitation; no automatic retry or quality rejection |
| Bonsai 2 27B | Direct smoke passes; full-offload OOM; 32K routed continuation admission-blocked | Parked; coding/research quality untested, no automatic retry |
| MiMo 9B | Hash-verified; 73728-context suite mechanical 6/6, factual 5/6 | Retain coding explanation defect; parser edge cases and license metadata caveat remain |
| Kolibri-1 | Pinned patched runtime; large RAM/offload candidate, not rejected | Validate patch/runtime numerical and tool support before weight staging |
| Clef | Decision specialist, not chat replacement | Verify systemone adapter, calibration/abstention contract and cheaper comparison |
| Mitsuba / HiMitsuba | Public provenance/access unresolved | Resolve real artifact, license and projector; do not invent a download |
| Saluki | Mechanical 6/6, factual 4/6 in dual-GPU fixture | Retain limitation; no further retry scheduled |
| Qwen3.5-9B/4B, Bonsai, Nanbeige | Existing controls, not new winners | Preserve baseline; compare identical task contracts |
| Qwen2.5-Coder-7B, Qwen3-8B | Optional older controls | Use only for a specific comparison, not automatic replacement |
| Qwen3-Embedding-0.6B / Reranker-0.6B | CPU development and question-heldout ranking/availability independently recomputed | Generated-answer abstention, representative corpus and live integration remain; no promotion |
| Whisper-large-v3-turbo / Kokoro | Local CPU ASR/VAD and TTS synthetic execution retained | Genuine speech, listening quality and Hermes integration remain |
| Gemma3-4B-it | Independent-family vision control | Resolve license acceptance and projector pair before image fixtures |

Primary-source recheck now pins Qwen3-Coder official revision
`b2cff646eb4bb1d68355c01b18ae02e7cf42d120` and Unsloth GGUF revision
`b17cb02dd882d5b6ab62fc777ad2995f19668350`; both API cards report Apache-2.0.
`Qwen3-Coder-30B-A3B-Instruct-Q4_K_M.gguf` is 18556689568 bytes, advertised LFS
SHA256 `fadc3e5f8d42bf7e894a785b05082e47daee4df26680389817e2093056f088ad`.
The completed Server download matches this SHA256. This does not establish GPU fit.
Sources: [official pinned model](https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct/tree/b2cff646eb4bb1d68355c01b18ae02e7cf42d120),
[pinned GGUF](https://huggingface.co/unsloth/Qwen3-Coder-30B-A3B-Instruct-GGUF/tree/b17cb02dd882d5b6ab62fc777ad2995f19668350).

Read-only server preflight confirms both incumbent services active, no swap use,
and substantial available RAM/disk; both GPUs still contain incumbent workloads.
No service was paused for this prerequisite batch. Prerequisite research now
selects Bonsai 2 PTQ1_0 as the next coding/research experiment: Apache-2.0 weights,
pinned Prism CUDA runtime with a Turing compilation path, and publisher-native
tool protocol. This is readiness to test, not local correctness or fit proof.
Side-by-side staging completed on Server under
`~/.local/share/inference-assessment/bonsai2-b072e1d3`; both actual hashes match.
The [direct smoke](../audits/inference-assessment-2026-10-09/bonsai2-direct-smoke.json)
returned a native lookup call and exactly grounded continuation. At 40 GPU layers,
8K context and F16 cache, the candidate used 4020 MiB; this is not full-model
residency or full-profile acceptance. Both incumbents were restored and independently
read back active/healthy. Runtime library discovery required explicit existing
CUDA12.8 library paths; no production library installation was changed.

The full-offload six-case Hermes/Manwe batch failed before any task ran:
[retained startup log](../audits/inference-assessment-2026-10-09/bonsai2-full-offload-startup.log)
shows CUDA out-of-memory during graph instantiation at 32768 context and F16
cache. Both incumbents were restored and read back active; batch health checks
returned ok. This is a configuration capacity failure, not six failed tasks.

The partial-offload comparison finished: model load and first native tool call
passed, but the preliminary routed continuation returned HTTP502 before any
Hermes coding task ran. Only one upstream capture exists; no second request
reached the recorder. Independent frozen-source review identifies pre-upstream
Manwe admission: tool-result history changes the route from chat to code, whose
64000-token floor exceeds this candidate's actual 32768 context. Parent retained
[route rejection evidence](../audits/inference-assessment-2026-10-09/bonsai2-admission-failure.json).
The generic filtered reason and missing HTTP error body limit exact diagnostics;
this is not coding quality rejection or six failed tasks. Both server endpoints
read back healthy. Bonsai is parked with no further GPU retry. Do not inflate
advertised context or weaken production admission to get a passing result.
Retained evidence: `p4-bonsai2-partial-coding-1.json` and batch JSON in the audit.

Qwen3-Coder Q4_K_M is hash-verified on Server under
`~/.local/share/inference-assessment/qwen3-coder-b17cb02d`. The bounded b11430
trial used eight GPU layers, F16 cache and actual per-slot context 73728 with
matching provider metadata. The required continuation floor is 64000 plus
8192 local headroom; no policy override was used. Runtime and explicit pinned
Qwen3-Coder template hashes were verified before launch.
[Retained batch](../audits/inference-assessment-2026-10-09/p4-qwen3-coder-batch.json)
confirms load/context and restored endpoint health.
[First coding attempt](../audits/inference-assessment-2026-10-09/p4-qwen3-coder-coding-1.json)
passed the preliminary routed native-call/result loop, then the Hermes child
hit its 80-second outer timeout with the original 60-second task budget.
The server spent 93.39 seconds processing the 3919-token Hermes prompt, so no
coding tool execution or final answer was observed by the child. Research and
remaining cases were not run. This is a measured serving-configuration latency
limitation, not demonstrated coding-quality failure. No automatic retry.
Next prerequisite work advances MiMo license/template resolution and Kolibri/Clef
runtime contracts in parallel; both Qwen3-Coder and Bonsai stay visible in the
matrix without promotion.

Bonsai model SHA256: `53107f530aa52eb00912263ab1ee29bd199261c87cd7b4ad4ca1318c1fe33ee3`.
Prism `prism-b10770-6684606` CUDA12.8 archive SHA256:
`98410e52ac687c11defc27e6c97a3713bf83bb702951ebf48dba1b67291d5651`.
Parent rechecked both advertised digests from the pinned HF API and GitHub release.
Bonsai prerequisites above are historical trial evidence, not a pending retry.
Any future routed trial must satisfy actual continuation capacity and retain
factual scoring independently; weight size alone does not establish fit.

Other candidates remain in scope:
- MiMo: official pinned README declares MIT; quant README says `other` without
  alternative terms, and neither inventory contains standalone license text.
  This remains a provenance follow-up, not demonstrated restrictive terms.
  Independent metadata inspection reports embedded template identical to the
  official pin (SHA256 `59a64ebb4df6d1489d09a91267cf3ceb106162d4a893c4f84833cfb8c897ff63`).
  b11430 uses its differential autoparser, not the specialized Qwen3-Coder parser;
  required-argument ordering is a specific qualification risk. Test reordered,
  nested and streamed arguments plus linked results without replacing the template.
  Server staging completed (`proc_f3c9c87f2e29`), directory
  `~/.local/share/inference-assessment/mimo9b-2ed8dfba`, pinned GGUF revision
  `2ed8dfba210e13c96c959580c4368fab1d8f0ba0`, Q4_K_M advertised SHA256
  `6d5019649ae3d5f094d4e6ec798d6725c0f2524c737e5cea893a6ffd7ac25c1e`.
  The completed download matches that digest. Bounded trial `proc_1bd8ab514029`
  requested 73728 context, one slot, 24 GPU layers, Q8 KV, embedded template,
  thinking enabled and a 256-token reasoning budget. Runtime context and template
  digest are asserted before the preliminary native-call/result check and original
  60-second coding/research suite. Only one incumbent lane was temporarily paused,
  with timed/finally restoration; Core and the other server lane stay untouched.
  Initial setup stopped before cases because the served template omits exactly
  one terminal newline; `mimo-template-provenance.json` retains that comparison.
  The exact served hash was corrected without changing template or task budget.
  [Verified batch](../audits/inference-assessment-2026-10-09/p4-mimo9b-verified-batch.json)
  mechanically passes all six cases; both incumbents and Core read back healthy.
  [SHA-bound independent factual review](../audits/inference-assessment-2026-10-09/mimo9b-factual-report.json)
  gives mechanical 6/6, factual 5/6, qualified false. Parent verified all six
  artifact hashes and reproduced the gate result. Coding repetition 2 calls
  negative numbers falsy; code passes but its explanation is incorrect.
  Both research and both recovery cases pass factual/causal review. Coding 1
  passes only the retained numeric/None/bool contract, not arbitrary-type filtering.
  [Native offline argument-order test](../audits/inference-assessment-2026-10-09/mimo-parser-order-result.json)
  now confirms a concrete parser limitation: schema-order required arguments
  produce one correct native call, reversed order fails strict parsing with no
  native call. This uses the exact served template and pinned autoparser/mapper,
  synthetic output rather than model generation or HTTP streaming. Follow-up
  [lenient wrapper execution](../audits/inference-assessment-2026-10-09/mimo-parser-wrapper-result.json)
  preserves ordered arguments but throws on reversed arguments in both final and
  partial modes for the complete synthetic string, with an empty generation prefix.
  A subsequent [incremental native probe](../audits/inference-assessment-2026-10-09/mimo-parser-incremental-result.json)
  derives the real generation prefix and checks all 242 prefixes across both orders,
  plus one-byte accumulation and native diff reconstruction. Ordered arguments
  reconstruct exactly without content leakage. Reversed order leaves an incomplete
  `{` argument in retained stream state and throws at finalization; partial wrapper
  calls themselves do not throw with this prefix. Thus the earlier empty-prefix
  partial exception must not be generalized to actual prefixed streaming behavior.
  The probe uses explicit reasoning NONE, not the prior served AUTO configuration.
  Independent review reproduced stdout/stderr exactly and verified prefix coverage.
  A [retained verifier](../audits/inference-assessment-2026-10-09/mimo-incremental-verification.json)
  now asserts the recorded positive/negative behavior, per-step argument integrity,
  empty reasoning/content and complete consumption; five corrupted-evidence controls
  are rejected. Current binary/library hashes are retained, not retroactive build
  attestation. The retained brace is accumulator state, not proven client output.
  A [follow-up AUTO probe](../audits/inference-assessment-2026-10-09/mimo-parser-incremental-auto-wired-result.json)
  explicitly sets both generator and parser reasoning mode; all prefix and replay
  outcomes equal the reviewed NONE fixture. The initial generator-only AUTO run is
  retained separately and superseded. Further [reasoning/nested fixtures](../audits/inference-assessment-2026-10-09/mimo-reasoning-nested-verification.json)
  cover 326 reasoning-bearing and 390 nested-object/boolean-array prefixes in AUTO.
  Ordered replay reconstructs reasoning and arguments exactly at every step, with
  no content leakage; reversed parameter order still fails at finalization.
  Extension review initially hit the usage limit; after operator reset, independent
  review verified all 326/390 prefixes, per-step properties and recorded hashes. A new weight-free
  [native response-serialization gate](../audits/inference-assessment-2026-10-09/mimo-serialization-verification.json)
  builds and invokes actual server task state/update/to_json code. Ordered nested
  calls reconstruct correctly at 1-byte, 7-byte and whole-input chunk sizes and
  in nonstream JSON. Reversed calls serialize an incomplete `{` for 1/7-byte
  chunks before final rejection; whole-input chunks emit no tool call. Independent
  [review and hardened verification](../audits/inference-assessment-2026-10-09/mimo-serialization-reviewed-verification.json)
  pass semantic native replay and all hashes. Seven corrupted-evidence controls
  are rejected, including model metadata and reversed reasoning corruption found
  by review. This proves native JSON production, not network transmission, SSE
  delivery or model generation; no general nested-schema acceptance is claimed.
  A subsequent [live native-stream probe](../audits/inference-assessment-2026-10-09/mimo-live-stream-result.json)
  completed MiMo → isolated retained Manwe candidate → installed Hermes → real
  read_file → grounded answer. The isolated router disabled default tool-stream
  emulation (`ARDA_MANWE_EMULATE_TOOL_STREAMING=false`); captured upstream requests
  use `stream=true` and return SSE. Production routing was not changed. A synthetic
  incomplete-tool stream control caused no observed tool execution but exceeded
  the 90-second harness bound: bounded failure/recovery remains OPEN, not passed.
  The prior reversed-argument fixture remains a separate offline negative control.
  Nanbeige was restored and both Server health endpoints passed; Bonsai/Core were
  not stopped. Independent review verified native SSE argument reconstruction,
  real tool execution and grounding; the negative-control timeout remains OPEN.
  Follow-up read-back confirmed both incumbent model identities and services,
  plus candidate service and restoration timer inactivity. A local-only
  [P4 disposition and failure comparison](../audits/inference-assessment-2026-10-09/mimo-p4-disposition.json)
  isolates the timeout to Hermes post-exhaustion auto-recovery waits, not a model
  inference stall. Default recovery exceeds a 15-second task budget and the
  45-second harness bound. Setting only isolated `agent.auto_recovery_cycles=0`
  returns an explicit provider error with exit 1 and no tool execution. This is
  restricted-config evidence, not a repaired default budget guarantee. Proposed
  disposition: assessment-only; no production promotion. Independent review
  accepts this restricted comparison; default bounded recovery remains OPEN.
- Kolibri: patched offload is untested, not rejected. Parent resolved `hob-b11434`
  to `8798b654ee06d27eae8d01e43db5a0055cb678a3`; the prebuilt source differs from
  the documented base-plus-patch pair. Independent comparison of all 15 affected
  paths found identical Kolibri architecture/converter and MoE function, but a
  newer backend with material CUDA/batching/KV changes; do not equate runtimes.
  [Comparison manifest](../audits/inference-assessment-2026-10-09/kolibri-source-comparison.json)
  retains file hashes. Prefer the full pinned fork for publisher-release reproduction.
  Renormalization follows `norm_topk_prob`; disabled is the default, not invariant.
  Top-6-of-384 selection and unbiased sigmoid weights need CPU/reference versus
  Turing hybrid numerical checks; the two-expert fixture is insufficient.
  The pinned full fork now builds locally CPU-only. Its baseline Kolibri architecture
  test passes. A retained [384-expert/top-6 synthetic fixture](../audits/inference-assessment-2026-10-09/kolibri-native-cpu.json)
  also passes CPU execution, save/load roundtrip and mixed-batch consistency at
  seed 42. Only scratch test dimensions changed; runtime source is unchanged.
  CPU-versus-itself is not independent reference numerical validation. A subsequent
  [native router oracle](../audits/inference-assessment-2026-10-09/kolibri-router-oracle-result.json)
  passes nine injected-input CPU cases: normal/promoted/very-negative logits across
  absent/false/true normalization metadata. A scalar oracle checks biased raw-logit
  selection and unbiased sigmoid weights at 1e-18 absolute + 2e-5 relative tolerance.
  Instrumentation changes scratch tests only, not runtime source; it does not test
  upstream matmul numerics or uninstrumented CUDA scheduling. The oracle mirrors
  native normalization's denominator clamp, so publisher normalization parity is
  now confirmed to diverge for tiny logits with normalization enabled; see the
  [publisher differential](../audits/inference-assessment-2026-10-09/kolibri-publisher-differential-note.md).
  Unset/false tiny cases pass; true reaches all required stages and fails the
  publisher formula. Optional true-path normalization is excluded from qualification.
  Independent review found missing per-stage assertions in the original oracle;
  the revised harness enforces them. Independent re-review accepts only the
  optional-true exclusion: absent/false receipts precede layout-only fixes;
  serialized metadata, per-execution coverage and executed publisher-model parity
  remain unqualified. No further known-failing true retry is needed. Turing
  hybrid checks, streamed tools and real-weight latency remain unverified.
  No candidate weights staged or model services changed for these Kolibri tests.
- Clef: dedicated systemone adapter, named template/joint-head metadata and
  whole-prompt physical ubatch admission remain open. Preserve choice/score/noul
  semantics; normalized choice confidence is not probability of correctness.
  Reference parity audit found differing float serialization, null/default handling,
  marker sanitization, state truncation, tie-breaking and confidence semantics.
  [Parent-executed reference oracle](../audits/inference-assessment-2026-10-09/clef-reference-oracle.json)
  passes its byte-tokenizer edge probes, not real-token or C++ parity.
  Pinned b11430 CPU-only build finished; native `test-jinja` and
  `test-chat-auto-parser` pass. The parent-executed
  [native numeric differential](../audits/inference-assessment-2026-10-09/clef-native-number-parity.json)
  fails reference parity: 1.23456789 renders as 1.23457, and 1.0 as 1.
  Review corrected the probe to compare parsed JSON rather than raw whitespace;
  the numeric mismatch still reproduces after rebuilding.
  This is actual pinned C++ Jinja execution, not Python Jinja emulation, but does
  not establish full server/tokenizer parity. No upstream code was changed.
  Clef remains adapter/semantic-contract blocked; choose explicit restricted input
  semantics or a reviewed upstream repair before numeric-state qualification.
  Labeled held-out calibration with a cheaper comparator remains required before
  any abstention threshold or authority grant.
- Retrieval: [CPU comparison](../audits/inference-assessment-2026-10-09/retrieval-cpu-result.json)
  now completes all 24 frozen development queries over 12 documents with both
  hash-verified official 0.6B models. Historical BM25 Hit@1 is 0.7083; reranked and
  embedding Hit@1 are both 0.9583. Recall@3 is 0.9583 for BM25/reranking and 0.9792
  for embedding. Reranking preserves the candidate pool (one query has only two
  candidates); the initial erroneous exactly-three assertion and partial run are
  retained separately. Models run sequentially CPU float32, four threads, no input
  truncation, last-token normalized embeddings and native yes/no reranker logits.
  [Execution receipt](../audits/inference-assessment-2026-10-09/retrieval-cpu-complete-run.json)
  records 3G peak cgroup memory and zero swap. These are agent-authored labels,
  not independent held-out evidence. Independent review verified all 24 cases,
  71 pair scores and no truncation; the SHA-bound review is retained in
  retrieval-independent-review.json. Frozen excerpts are historical, not current
  repository truth; eight source locations drifted and two excerpts changed.
  Historical indexed Books are absent, so original index state is not reconstructable.
  No abstention/unanswerable qualification, contemporaneous BM25 timing or retained
  embedding-vector replay is established for that historical run. A subsequent
  [question-heldout CPU comparison](../audits/inference-assessment-2026-10-09/retrieval-heldout-result.json)
  uses 12 fresh separate-agent-authored questions (six calibration/six heldout),
  contemporaneous explicitly specified BM25, retained vectors and raw scores.
  Threshold selection uses calibration only; heldout evidence-availability balanced
  accuracy is 0.875 BM25, 0.75 reranker and 0.625 embedding. The reranker incorrectly
  accepts one unsupported question; ranking improvements do not establish safe
  abstention. Heldout supported-only Hit@1 is 1.0/1.0/0.75 respectively (four cases).
  These are shared-corpus agent labels, not human gold or generated-answer tests;
  conflicting-premise cases are answerable corrections, not source conflicts.
  The bounded run completed in 58.567 seconds with 3.1G peak memory and no swap.
  Independent [scoring/fixture review](../audits/inference-assessment-2026-10-09/retrieval-heldout-independent-review.json)
  verifies labels, score replay and calibration-only thresholds. Threshold freeze
  is procedural, not independently sealed preregistration. The probe first-line
  development-only wording is stale; its executed bytes remain hash-bound.
  No generated-answer abstention or production promotion is justified.
  No vector store, live retrieval integration or model service changed.
- Voice: earlier synthetic success was gateway STT, not local Whisper. A new
  [local CPU Whisper probe](../audits/inference-assessment-2026-10-09/local-whisper-result.json)
  executes the hash-verified pinned CT2 turbo model in isolated Python 3.12.
  Requested int8 resolves to int8_float32. Synthetic speech transcribes correctly
  apart from punctuation; silence hallucinates "Thank you." without VAD and is
  empty with VAD. Speech took 5.08 seconds for 3.66 seconds of audio, not real-time
  acceptance. The initial JSON retains process RSS, not its cgroup receipt.
  [Reviewed follow-up](../audits/inference-assessment-2026-10-09/local-whisper-reviewed-result.json)
  verifies model bytes at execution, hashes probe/audio, isolates Python and uses
  fresh fixtures. Both speech variants have zero normalized word errors; VAD
  suppresses silence while preserving speech. A retained
  [bounded-run receipt](../audits/inference-assessment-2026-10-09/local-whisper-reviewed-run.json)
  records AF_UNIX-only socket restriction, 1.5G peak cgroup memory and zero swap.
  This is a synthetic VAD regression, not a representative speech corpus.
  PyAV 19.0.1 breaks faster-whisper 1.2.1 decoding (`metadata_errors` removed);
  av 16.0.1 fixes it. Executed versions are in local-whisper-requirements.txt.
  No microphone or model services were touched. Kokoro af_heart now also
  [synthesizes locally](../audits/inference-assessment-2026-10-09/local-kokoro-result.json)
  with hash-verified weights/voice and pre-staged English frontend in isolated
  Python 3.12 CPU execution. It produces a verified 24kHz mono WAV (4.05 seconds)
  in 1.24 seconds synthesis time, without clipped/nonfinite samples. eSpeak fallback
  is enabled, not yet specifically forced through an unknown-word test. The retained
  AF_UNIX-only bounded-run receipt reports 1.6G memory peak, zero swap.
  No playback or intelligibility verdict is claimed. af_heart carries the publisher's
  Apache-2.0 declaration without a separate restrictive voice annotation; frontend
  GPL/eSpeak bundled-data redistribution obligations remain separate.
  Genuine spoken input, listening review and Hermes integration remain open.
- Gemma3: known gated model/projector; operator license/access unverified.
- Mitsuba: both APIs still report 401; artifact provenance remains blocked.

Existing one-lane bounds remain; no permanent rollout or new dual-lane
interruption is implied. Saluki remains parked.

### Evidence and reading order

1. Read this entire plan, including the incorporated findings and detailed
   supplement. Challenge the recommendations rather than treating them as a
   preselected solution.
2. Read [fleet identity and recovery](../operations/fleet-access-and-recovery.md)
   and [the original research snapshot](../operations/inference-fleet-assessment-2026-10-09.md).
3. Inspect [fleet inventory](../../config/fleet.toml) and
   [provider configuration](../../config/manwe.providers.toml), then actual
   systemd ExecStart, running processes and model APIs.
4. Retained raw collection: [live discovery JSON](../audits/inference-assessment-2026-10-09/fleet-live.json)
   and [read-only collection script](../audits/inference-assessment-2026-10-09/collect.py).
   These preserve the five-node hardware, process, service, checkout and model
   API output. They contain no performance benchmark results. The collector's
   combined return code is not proof every subcommand passed; inspect stderr.
5. Supplementary topology observations are recorded below. Research retrieval
   results are cited at their primary URLs; re-fetch cards and pin revisions
   before installation. Scratch/delegation transcripts are not durable authority.

### Connection and installed repair state

| Node | Mesh address | SSH user | October 9 observation |
|---|---|---|---|
| Core / annunimas-core | 100.78.138.113 | mythos | Online, model health and actual chat tested |
| Server / annunimas-server | 100.102.250.115 | annunimas-server | Existing-key SSH and model lists work |
| Beelink / beelink | 100.103.125.88 | citadel | Existing-key SSH and model list work |
| Laptop / laptop | 100.97.15.88 | mythos | SSH restored, no scheduled key expiry reported |
| Warden / warden | 100.110.85.37 | numenor | SSH, scout health and model list work |
| CITADEL / citadel | 100.119.130.127 | citadel | Offline, operator-reported boot I/O error |

Correct tailnet: `tail91c3c1.ts.net`, identity `dward1502@github`.
The separate Gmail/`tail622209.ts.net` identity is not the fleet. CITADEL was
renamed from `raspberrypi-1`; it is not another machine. Beelink's SSH user
`citadel` does not identify the CITADEL Pi. Server and Beelink OS hostnames both
reported `bluefin`; use mesh identity and trusted host keys, not hostname alone.

Core model activation was explicitly requested BEFORE this research. Its CUDA
links pointed into removed container overlays. Pinned packages
`nvidia-cuda-runtime-cu12==12.6.37` and `nvidia-cublas-cu12==12.6.0.22` were
installed under `~/.local/lib/mesh-llm-cuda12`; persistent override
`~/.config/systemd/user/mesh-llm.service.d/20-cuda-runtime.conf` sets library paths.
The temporary development gate in
`/run/user/1000/systemd/user/mesh-llm.service.d/90-development-off.conf` requires
`/run/user/1000/arda-mesh-llm-explicit-enable`, which was created on explicit
operator request. Both /run paths are ephemeral; inspect after reboot rather
than treating the marker as permanent configuration. The underlying service is
enabled. Mesh-address health returned ok and a real Qwen chat response returned
`OK`; readback active/running with zero restarts after repair. Reboot acceptance
was not performed. Do not undo this working repair during an assessment.

## Scope, invariants and non-goals

- Optimize useful task completion, correctness, latency, energy and maintenance,
  not total parameter count or tokens/second alone.
- Keep functional baselines and exact rollback artifacts. Never replace all
  nodes or both server lanes at once.
- Preserve SSH host-key verification, Tailscale identity, SELinux and existing
  authentication. Do not broaden listeners/firewall rules for benchmarking.
- Preserve Hermes tool/schema semantics and Arda authorization; model confidence
  never grants mutation authority. Clef is advisory, not an approval bypass.
- No inference capacity is assumed on offline CITADEL. Its storage/thermal
  incident has a separate recovery scope; suspected overheating is not diagnosis.
- Do not stop desktop, image-generation or other services by name alone. Discover
  loaded workloads, then reserve an authorized benchmark window.
- Do not benchmark on production documents containing secrets without a scoped
  local-data agreement. Use retained sanitized fixtures where possible.
- Do not prescribe hardware purchases before measuring the existing fleet.

## Work packages and exit gates

### P0 - Fresh-eyes review (first continuation)

- [ ] Recheck sources for Saluki, Kolibri, Clef and Mitsuba; record exact model,
      quant, license, template, projector and required runtime revisions.
- [ ] Validate hardware identity and source/binary provenance from retained
      output plus targeted current probes; distinguish checked binary from
      actual /proc/PID/exe and loaded model.
- [x] Resolve core llmster/legacy wrapper ownership and Warden context drift.
      [P0 discovery evidence](../audits/inference-assessment-2026-10-09/P0_REVIEW.md):
      LM Studio has no loaded models; legacy wrapper is a container/log follower;
      Warden runs 4096 while both configs advertise 262144. No changes applied.
- [x] Challenge placement proposals, evaluate newer alternatives omitted by the
      initial search, and explicitly distinguish old controls from upgrades.
      [Candidate review](../audits/inference-assessment-2026-10-09/P0_REVIEW.md)
      adds Bonsai 2 and MiMo 9B, preserves current controls and rejects assumed
      full-GPU residency for Clef Q4. No candidate has local qualification.
- [x] Set task priorities, latency budgets, energy constraints, experiment disk
      budget and maintenance windows with the operator if not already specified.
      [Operator scope](../audits/inference-assessment-2026-10-09/P1_PROGRESS.md):
      coding, research, voice; no explicit limits; one server lane may pause.
      No invented numeric promotion thresholds.

Exit: corrected candidate/compatibility matrix, precise unknowns and bounded
benchmark scope. Newest release labels must not substitute for exact commits.
No model download is needed to finish this review.

### P1 - Retain baseline and define test contract

Proposed output location: `docs/audits/inference-assessment/` for compact manifests
and scorecards; large run data belongs under `data/benchmarks/inference-assessment/`
with explicit retention rules. Do not commit weights or private prompts.

- [ ] Capture service files/drop-ins, effective environment, actual executable
      paths and hashes, model/projector hashes, driver/backend versions and
      resource baselines for every participating node.
- [ ] Record actual model API IDs, template parameters, tool parsers, context,
      concurrency, bindings/access controls, restart policy and startup state.
- [ ] Define a machine-readable result contract: run_id, UTC time, node,
      hardware, model/revision/hash, runtime/commit/hash, quant, template hash,
      prompt-suite revision, context, batch/concurrency, offload layout, cache,
      reasoning setting, correctness, failures, timing and peak resource metrics.
- [ ] Inventory existing benchmark scripts before creating new ones. If new
      tooling is necessary, use failing schema/scoring fixtures before code,
      then validate both valid runs and missing/failed observations. Never encode
      a missing measurement as zero or a passed task.
- [ ] Run bounded baseline task suites on current workers, including actual
      Hermes/Arda client tool-call parsing; preserve raw responses and final state.

Exit: reproducible baseline plus agreed promotion thresholds. Numeric latency
thresholds remain operator/workload decisions, not invented acceptance results.

### P2 - Runtime refresh in isolation

- [ ] Select pinned upstream llama.cpp and Prism revisions with required model
      architectures and GPU/kernel compatibility. Review dependency provenance.
- [ ] Build/install side-by-side, not over current service binaries. Check CUDA
      libraries are independent of disposable container overlays.
- [ ] Evaluate old model + old runtime versus SAME model + new runtime before
      evaluating a new model. Confirm templates, streaming and tool calls.
- [ ] Benchmark server AVX2/CUDA ik_llama.cpp where architecture support exists;
      compare local dual-GPU split with independent lanes.
- [ ] Compare Beelink CPU/Vulkan and qualified HIP path; preserve shared-memory
      headroom and record exact AMD driver/distro support.

Exit: correctness-preserving runtime candidates and measured regression/gain
matrix. A successful load or /health response alone does not pass this gate.

### P3 - Candidate experiments

- [ ] Saluki: server first; compare against current models on identical tools,
      budgets and prompts; text-only before vision; thinking modes separately.
- [ ] Kolibri: verify required architecture patch, then server RAM/CPU expert
      offload with modest context. Record quant quality and prompt/decode costs.
- [ ] Clef: explicit /v1/systemone adapter, decision-head verification,
      calibration/abstention tests and comparison with cheaper classifiers.
- [ ] Mitsuba: first resolve actual downloadable artifacts, license, projector
      and Prism revision; then evaluate image grounding and generated-prompt
      usefulness. A 404 cannot be bypassed with an invented artifact.
- [ ] Retrieval, ASR/TTS and compact vision candidates: establish whether they
      remove expensive LLM work and improve end-to-end outcomes.

Exit: per-task pass/fail/blocked scorecards; rejected candidates remain legitimate
results. No obligation to deploy every candidate or maximize model count.

### P4 - Select allocation and qualify integration

The [per-job allocation proposal](../audits/inference-assessment-2026-10-09/p4-allocation-proposal.json)
compares retained outcomes and recommends preserving incumbent production lanes,
not promoting a candidate. It specifies context/concurrency constraints, eligible
fallback versus explicit unavailability, headroom uncertainties and per-node
rollback requirements. Runtime identities not freshly read back are labeled
historical or pending; no power/cost winner or simultaneous-load acceptance is
claimed. MiMo remains assessment-only. Independent review accepts only the
conservative no-change disposition; Server role restrictions, unverified fallback
enforcement and incomplete rollback provenance are explicit. The P4 checklist
and P5 rollout remain open. No configuration was applied. Next: read-only
incumbent identity/admission matrix, then isolated unavailable/undersized-provider
fixtures against existing Manwe before representative routed incumbent tasks.
A [fresh endpoint/catalog matrix](../audits/inference-assessment-2026-10-09/p4-incumbent-api-matrix.json)
records healthy model endpoints and runtime context/slots: Core 73728/1,
Bonsai 100096/1, Nanbeige 32768/1, Beelink 32768/4 and Warden 4096/4.
Warden serves on 1234; an initial wrong-port probe is retained and corrected.
Live Manwe still advertises Warden 262144 and marks Warden/Beelink unhealthy
while their model health endpoints return ok. This establishes catalog/runtime
drift, not that either route is usable or that health alone clears cooldown.
[Runtime identities](../audits/inference-assessment-2026-10-09/p4-runtime-identities.json)
now retain executable/model hashes and selected arguments for Core, both Server
lanes, Beelink and Warden. Core was read locally after confirming host identity;
failed loopback SSH authentication did not block local inspection. Complete
unit/drop-in/environment rollback capture remains open.
Seven [isolated installed-binary admission cases](../audits/inference-assessment-2026-10-09/p4-admission-result.json)
pass: eligible forced/automatic routes, two undersized contexts, tools-disabled,
unavailable forced provider and automatic selection with no eligible route.
Linked messages/schemas are preserved with native call-ID canonicalization;
rejected context/tool cases make no fixture upstream request. This is synthetic
HTTP admission, not live-model or post-failure dynamic failover acceptance.
Independent review accepted the recorded scope; strengthened route/header/error
assertions pass a fresh rerun. A separate
[post-failure fallback probe](../audits/inference-assessment-2026-10-09/p4-fallback-result.json)
returns explicit failure after primary 503: the forced-provider error path stops
without honoring fallback permission. Its backup ID also failed the router's
ID-based locality classifier despite local access-tier metadata. Do not use
forced routes as a qualified automatic-fallback contract. The corrected
[automatic failover pair](../audits/inference-assessment-2026-10-09/p4-fallback-auto-result.json)
passes with `model: auto`: primary 503 then eligible `edge_backup_local` 200,
with preserved linked tool history; the non-local-classified backup is not
called and the request fails explicitly. This qualifies only synthetic
non-streaming automatic fallback and ID-based local-only selection, not actual
network locality or model behavior. Independent review accepts this bounded pair.
The earlier forced-route fixture does not independently isolate the forced-error
branch because its backup was also ineligible; forced fallback remains unqualified.
Five focused source regression tests also passed:
under-context fallback, local headroom, lossless emergency context and two
runtime-state merge tests. Source tests do not qualify installed configuration.

Current next gate: repair/qualify initial optional-tool transport in installed
Manwe before incumbent representative workloads. The
[Core thinking preflight](../audits/inference-assessment-2026-10-09/p4-incumbent-core-research-1.json)
and retained
[nonthinking preflight](../audits/inference-assessment-2026-10-09/p4-incumbent-core-research-1-nonthinking-preflight.json)
both stop before behavioral cases: captured upstream requests omit supplied
`tools` and `tool_choice` on the initial request. Linked-history synthetic
preservation does not cover this boundary. Neither run qualifies model quality;
these stale-binary attempts did not execute representative cases. Reconciliation
confirmed that the running binary predates existing source repairs. A fresh
release build and four focused optional-tool/context regressions pass. The
[fresh-candidate six-case batch](../audits/inference-assessment-2026-10-09/p4-fresh-core-batch.json)
now passes preliminary native tool/result continuation but fails all six actual
Hermes cases (two each research, coding, recovery): only the first file read
executes, followed by prose announcing the next read.
[Hash-bound independent factual review](../audits/inference-assessment-2026-10-09/p4-fresh-core-independent-factual-report.json)
confirms 0/6 mechanical and factual passes: upstream stops without native tool
calls, with second-read XML only in reasoning. Retain Core as restricted baseline,
not a general-agent endorsement; do not retry unchanged. A fresh
[service preservation inventory](../audits/inference-assessment-2026-10-09/p4-service-preservation.json)
retains five process identities on four nodes with unit source locations/hashes
and environment names; secret values are deliberately excluded, so this is not
a complete portable rollback package.
The fresh artifact is assessment-only; no production process was restarted or
configuration applied. The running process still uses the old executable inode.

- [ ] Compare independent workers against server-local splitting; cross-node
      model sharding only if a measured workload needs it and network tests
      justify it. Avoid creating a new scheduler or bypassing existing routing.
- [ ] Select model/runtime/context/concurrency per job with explicit fallback,
      unavailable-node behavior, memory headroom and cost/energy reasoning.
- [x] Trace `config/fleet.toml` and `config/manwe.providers.toml` to their live
      Manwe/Arda consumers: [consumer and live reload evidence](../audits/inference-assessment-2026-10-09/P4_ROUTING.md).
      Provider edits require explicit authenticated reload of the selected file;
      reload preserves matching runtime health state and does not replace code.
      Arda reads the Warden scout URL at startup; observability rereads fleet
      inventory without activating routing or model-service changes.
- [ ] Exercise actual routed research, tool, coding, voice and vision paths for
      selected roles. A direct endpoint test is not routed acceptance.
- [ ] Independent review of proposed configuration and evidence before commit
      or promotion; resolve findings without disturbing unrelated changes.

Exit: evidence-backed deployment recommendation and a per-node rollback plan.

### P5 - Authorized staged rollout and closure

Historical general-agent disposition (superseded by the current mixed-role
specification above; not a blanket prohibition on specialist promotion): preserve
incumbents with the restrictions in the
[reviewed allocation proposal](../audits/inference-assessment-2026-10-09/p4-allocation-proposal.json);
no candidate promotion is justified. This is a negative deployment recommendation,
not successful integration acceptance. The full plan remains active: no-change
preservation does not satisfy its rollback, sustained-load or routed-role gates.
Further unchanged model retries are not authorized by this disposition. Closing
only the assessment decision requires operator agreement to that bounded scope;
production activation requires a separately specified change and secure rollback
package. Genuine speech/listening and routed vision acceptance require operator
inputs. No deployment, restart, or concurrency expansion follows automatically.

- [ ] Obtain/confirm rollout authority, preserve the working baseline, promote
      one lane, and verify exact post-change process/config/model identity.
- [ ] Verify task-level correctness, service restart, availability/fallback and
      a bounded sustained thermal/resource run. Reboot tests only in an approved
      window; distinguish untested reboot persistence from actual acceptance.
- [ ] Restore baseline immediately on correctness, stability or resource failure.
- [ ] Update fleet/provider config, service ownership and operational docs to
      match observed deployment; remove contradictory current-facing claims.
- [ ] Record remaining blockers, measured winners/losers and rollback locations.
      Archive this plan only after the agreed requirements are actually met.

## Practical verification entrypoints

Read-only probes (capture output with timestamps; do not infer health from exit
status of a multi-command shell alone):

```sh
git status --short
tailscale status --json
systemctl --user show mesh-llm.service -p ActiveState -p ExecStart -p Environment -p DropInPaths
nvidia-smi --query-gpu=name,memory.total,memory.used,driver_version --format=csv
ssh -o BatchMode=yes -o StrictHostKeyChecking=yes annunimas-server@100.102.250.115 nvidia-smi topo -m
curl --fail --max-time 10 http://100.78.138.113:9337/v1/models
curl --fail --max-time 10 http://100.102.250.115:8097/v1/models
curl --fail --max-time 10 http://100.102.250.115:8098/v1/models
curl --fail --max-time 10 http://100.103.125.88:9337/v1/models
curl --fail --max-time 10 http://100.110.85.37:1234/v1/models
```

Expect matching identity/model payloads, not predetermined performance. Laptop
had no active model endpoint at discovery; failed inference there is not failed
SSH. Changes since the snapshot are evidence to reconcile, not automatically a
regression. Do not repeat large recursive model scans if retained data answers
the question.

## Detailed research supplement and uncertainty register

Research workers reported the following published release snapshot from live
GitHub queries on October 9. These were not installed or independently exercised;
revalidate tags and exact source support during P0. llama.cpp named releases and
b-number builds are different identifiers; do not order them lexically.

| Project | Reported snapshot | Qualification |
|---|---|---|
| llama.cpp | v0.6.0, October 5 | Clef quant card separately requires b11430+ |
| ik_llama.cpp | October 9 commit 5194a9e; no latest release | AVX2/ARM CPU, Turing+ CUDA focus |
| vLLM | v0.31.0, October 5 | Framework supports CC7.5+, specific kernels may not |
| SGLang | v0.5.21, October 2 | Research found CUDA13 docs; 0.5.19 last CUDA12 lane |
| ExLlamaV3 | v1.6.0, October 7 | Turing experimental since1.5.2; preliminary ROCm10/gfx1100+ claim needs verification |
| ExLlamaV2 | v0.3.2, July13 2025 | Older EXL2 path, not interchangeable with EXL3/GGUF |
| KTransformers | v0.7.1, September15 | AVX2 guide recommends24GB+ GPU, outside individual fleet cards |

AMD research found a Ryzen ROCm7.2.1 matrix with HX370-class gfx1150/gfx1151,
Ubuntu24.04.4 and FP16 validation. Exact PRO SKU/Fedora/quant support is not
established. ExLlama's reported ROCm10 requirement is a separate stack, not proof
of compatibility with this production matrix. ik row repacking can impair hybrid
prefill, and some graph-splitting/partial-offload settings have produced incorrect
output: test numerical behavior before chasing throughput. exo's retrieved Linux
CPU-only language conflicted with NVIDIA extras; no fleet support claim accepted.

Additional primary software discovery:
https://github.com/ggml-org/llama.cpp/releases
https://github.com/turboderp-org/exllamav3/releases
https://github.com/vllm-project/vllm/releases
https://github.com/sgl-project/sglang/releases
https://github.com/b4rtaz/distributed-llama
https://github.com/exo-explore/exo

Model details retained from initial research (publisher/quantizer claims, not
local measurements; file sizes decimal GB, exclude runtime overhead):

- Saluki release October7; architecture identifier qwen35. Reported competition
  math retention82-85%; parallel calls42/100 strict versus55/100 forgiving parse.
  Thinking can be verbose. Optional vision contradicts older text-only launch
  wording; use current pinned card rather than announcement alone.
- Kolibri release October3; exact total78103074560 / active3457573120 parameters;
  50 layers,384 routed experts, top6 plus shared expert,4:1 sliding/full attention.
  Official FP8 weights about78GB; official plugin described vLLM0.29. Research
  reported AIME2025 96.9, SWE-bench66.4, TerminalBench2.1 27.7 and BFCL multi-turn
  47.5: different harnesses/tasks, not comparable to Saluki's subsets. Community
  Q3_K_M37.5GB, Q6_K64.2GB, Q8_0 83.1GB supplement Q2/Q4/Q5 below. Patch base
  836d571 and prebuilt hob-b11434 were documented. Limited reference-logit and
  long-context tests do not establish full quantized benchmark equivalence;
  CUDA graph instability was reported.
- Clef release October1; 27B backbone plus vision and joint schema head. Supports
  images/video frames, not Clef-omni audio. Hosted64K versus example encoder16K
  are different limits. Official custom PyTorch loader tested H200; SGLang
  dev-clef support and planned0.5.22 were reported, not tested locally. Decision
  Q3_K_M14.48GB and Q2_K11.71GB exist, but quantizer recommends Q4+; decision
  quality is not established by generic quant-format descriptions.
- Mitsuba indexed card named v1.18 PQ2_0 7.32GB and PTQ1_0 6.00GB. An optional
  adult-content-oriented LoRA was mentioned in indexed material, not requested
  for deployment. Do not let refusal behavior substitute for vision/prompt
  correctness. Direct404 leaves availability/license unqualified.

Supporting-model artifact detail:

| Model | Selected artifact | License / qualification |
|---|---|---|
| Qwen3.5-9B | Q4_K_M5.68GB + optional F16 projector0.92GB | Apache2; already core baseline |
| Qwen3.5-4B | Q4_K_M2.74GB + projector0.67GB | Apache2; laptop/Warden baseline |
| Qwen3-8B | Publisher Q4_K_M5.03GB | Apache2; older control |
| Qwen2.5-Coder7B-Instruct | Publisher Q4_K_M4.68GB | Apache2; older coding control |
| Qwen3-Coder30B-A3B | Q4_K_M about18.6GB | Apache2; server offload challenger |
| Qwen3-Embedding0.6B | About596M parameters, up to1024 dimensions | Apache2; use correct query instructions/pooling |
| Qwen3-Reranker0.6B | About596M parameters | Apache2; yes/no scoring, bounded candidate set |
| Whisper-large-v3-turbo | 809M parameters | MIT; CTranslate2 FP16/int8_float16 or CPUINT8 |
| Kokoro82M | 82M parameters | Apache2 weights; frontend/voices separately reviewed |
| Gemma3-4b-it | Google QAT Q4_0 3.16GB + projector0.85GB | Custom Gemma license acceptance required |

Primary model cards follow the exact IDs under https://huggingface.co/ ; quant
sources for Qwen3.5 and Qwen3-Coder were Unsloth repositories, Qwen3-8B and
Qwen2.5-Coder publisher GGUF repositories, and Gemma Google's QAT repository.
Re-resolve exact file hashes rather than relying on these rounded sizes.

## Incorporated research findings

The following retains the original assessment inside this plan so the next
session does not need chat history. It is a dated snapshot, not an implementation
completion declaration.

# Inference fleet assessment - 2026-10-09

Status: read-only hardware audit and sourced research complete; replacement deployment and comparative benchmarks NOT performed. Recommendations are candidates, not measured winners. No service or model changes were made for this assessment.

## Verified hardware and current deployment

Live SSH/system/GPU/API inspection covered core, server, Beelink, laptop and Warden. CITADEL remains outside usable capacity until its boot issue is resolved.

| Host | Hardware observed | Present inference | Recommended role to test |
|---|---|---|---|
| Core | Ryzen 5 5600X, 32GB class RAM, RTX3080 10GB | Qwen3.5-9B Q4_K_M on 9337; configured 73728 context; 8117 MiB total GPU use at audit | Interactive general agent and short research synthesis; one resident generation model |
| Server | Threadripper2950X 16 cores, 128GB class RAM, two RTX2080Super 8GB | Bonsai27B Q1 on 8097 and Nanbeige4.2-3B Q4 on 8098; model APIs respond | Independent worker lanes normally; scheduled large MoE/offload experiments |
| Beelink | Ryzen AI9 HX PRO370, Radeon890M, 29000458240 bytes OS-visible RAM | Prism Ternary-Bonsai8B Q2 on 9337 | Background extraction/summarization; CPU retrieval/TTS; test Vulkan vs HIP |
| Laptop | i5-13420H, 7917580288 bytes system RAM, RTX3050 6GB | No responding model endpoint on checked ports; Qwen3.5-4B file and inactive service exist | Optional voice/vision and compact offline assistant, not required fleet capacity |
| Warden | Pi5 Cortex-A76 4 cores, 8453947392 bytes RAM | Qwen3.5-4B CPU, actual service context 4096; scout and SearXNG active | Retrieval/scout/control, bounded short local inference |

Server GPU topology is PHB, not NVLink. Two 8GB devices are not a transparent 16GB GPU. Beelink reports a 4GiB VRAM allocation and 30GiB GTT limit; GTT is an addressable shared-memory limit, not additional physical RAM. Do not sum these with OS RAM.

Important drift: Warden inventory advertises a 262144 model/context capacity but the actual service is configured at 4096. Core also has an active legacy llama-server service wrapper and llmster processes; this does not establish duplicate loaded models. Inspect loaded processes/GPU allocations before any benchmark, rather than killing services by name.

### Runtime age and provenance

- Core checkout: March 15 source; running service binary previously identified itself as b8668/b2f6a68ae. Checkout date is not proof of running executable provenance.
- Server stock checkout: July 18; checked build-cuda2 binary reports 10068 (571d0d540). Prism binary reports 1 (9fcaed7).
- Beelink stock checkout: March 15, but active Prism checkout/binary is July 18 / 9fcaed7.
- Laptop checkout: March 19; currently no loaded inference service on checked ports.
- Warden checkout: March 13; active CPU service.

These justify a runtime refresh experiment. They do not establish that every installed model is inferior merely because it is older. Preserve exact binaries/templates as baselines and test replacement candidates separately.

## Priority candidate models

### 1. Saluki: first new general-agent trial

ConwayResearch/Underdog-Saluki-27B-1.0 is a Qwen3.8-27B low-bit derivative, Apache2. Its IQ2-mix GGUF is 7.89GB decimal. Current card documents stock llama.cpp with Jinja template, thinking control, and optional 629MB Q8 or 928MB F16 vision projector. This corrects launch text that called it text-only.

Test first on server, with an explicit GPU split/offload plan, then on core during an isolated window. Core already uses about 8GB VRAM with the current service/desktop; do not load Saluki alongside it. Full GPU fit at useful context is unverified. Begin text-only, modest context, and thinking off for direct tool calls.

Publisher reports 88/120 selected BFCL tasks and 30/50 SWE-bench issues. These are publisher subsets, not a complete independent benchmark. Strict parallel tool-call result of 42/100 cautions against treating the model as a reliable autonomous executor without local validation.

Sources: https://underdog.ai/saluki
https://huggingface.co/ConwayResearch/Underdog-Saluki-27B-1.0

### 2. Kolibri: server-only large-model experiment

Aleph-Alpha/Kolibri-1: 78B total, 3.46B active MoE, Apache2. Native context is 262144, with a validated extension to 1M; neither is a practical fleet promise. Active parameters reduce per-token computation, not weight residency requirements.

Community GGUF Q4_K_M is 47.5GB; Q5_K_M 55.6GB; Q2_K 28.6GB. Server RAM makes Q4/Q5 plausible. Core, Beelink and laptop are not recommended hosts. Use CPU/RAM expert offload, measure prompt processing as well as generation, and start with much shorter context. The 2950X memory subsystem may dominate latency.

Crucially: community conversion requires its specific Kolibri llama.cpp patch/fork. Stock llama.cpp or ik_llama.cpp support must not be assumed. Official vLLM plugin support does not imply the official FP8 kernels work on Turing GPUs. Treat the patched runtime as an isolated experimental service and verify numerical/template behavior.

Sources: https://huggingface.co/Aleph-Alpha/Kolibri-1
https://huggingface.co/Hob-forge/Kolibri-1-GGUF
https://github.com/Aleph-Alpha/aleph-alpha-inference

### 3. Clef: typed decision/classification worker

Cloudflare/clef is not a chat generator. It reads state plus typed questions and returns option probabilities. Potential jobs: triage, classification, relevance, routing and bounded workflow decisions. Do not use its output as independent authority for consequential execution.

The current bartowski GGUF documents architecture clef, Q8 decision-head tensors, llama.cpp b11430 or later and /v1/systemone. This is not a drop-in /v1/chat/completions replacement. Q4_K_M is 17.65GB before context/buffers/projector; server hybrid offload is the first placement. Beelink may be memory-feasible but its current available RAM and latency leave less margin. Do not assume backbone-only GGUF files preserve the decision model.

Benchmark calibration, abstention and task accuracy against a compact classifier/general model before dedicating this much memory. No generation tokens/second metric is appropriate for its decision endpoint.

Sources: https://huggingface.co/Cloudflare/clef
https://huggingface.co/bartowski/Cloudflare_clef-GGUF
https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

### 4. Mitsuba: image-to-prompt specialist, availability unresolved

User-supplied release and indexed card describe a Qwen3.8-27B derivative specialized for ComfyUI image/video prompts, recommended 7.3GB PQ2_0 weights, optional smaller PTQ1_0 with vision trade-offs, PrismML runtime and thinking off. Indexed card recommends a 16GB GPU. Direct retrieval of both original and renamed repositories returned 404 during this assessment; artifact availability, projector requirements and current license remain unverified.

Treat as a specialist candidate, not coding/general-agent replacement. Server is the first fit experiment if multi-GPU vision support is confirmed; Beelink is a potential shared-memory alternative. Core is tight and laptop not a first target. Generate prompts remotely rather than keeping this VLM and an image generator resident together on core. Evaluate image grounding, retained composition/details, hallucinated objects, prompt controllability and final generation usefulness. Claims of superiority over Bonsai/Qwen remain publisher claims.

Discovery URLs: https://huggingface.co/isichan-ai/Mitsuba-ComfyUI-27B-GGUF
https://huggingface.co/isichan-ai/Mitsuba_and_HiMitsuba-27B-GGUF

## Supporting specialists and controls

These are established comparison candidates, NOT a list of the newest releases or a justification to replace current models with older ones.

- Keep existing Qwen3.5-9B/4B, Bonsai and Nanbeige as measured controls.
- Qwen3-Coder-30B-A3B-Instruct: Apache2, 30.5B total/3.3B active, Q4 around18.6GB. Server offload coding challenger; full two-GPU residency is not expected at that quant.
- Qwen2.5-Coder7B and Qwen3-8B: optional older control models only, not recommended upgrades based on novelty.
- Qwen3-Embedding-0.6B and Qwen3-Reranker-0.6B: Apache2 retrieval pipeline candidates. Preserve prescribed pooling/query instructions and yes/no rerank scoring. CPU first if latency permits; avoid consuming a whole GPU for light retrieval.
- Whisper-large-v3-turbo with faster-whisper/CTranslate2: ASR comparator to existing voice arrangements, not a claim it is newer or better. Test local microphone recordings and silence hallucinations.
- Kokoro82M: CPU TTS candidate, Apache2 weights; audit voice/frontend licensing separately.
- Gemma3-4B-it: independent-family vision control under custom Gemma terms, not first tool-agent choice.

Sources:
https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct
https://huggingface.co/Qwen/Qwen3-Embedding-0.6B
https://huggingface.co/Qwen/Qwen3-Reranker-0.6B
https://huggingface.co/openai/whisper-large-v3-turbo
https://github.com/SYSTRAN/faster-whisper
https://huggingface.co/hexgrad/Kokoro-82M
https://huggingface.co/google/gemma-3-4b-it

## Serving software strategy

1. Upstream llama.cpp is the common baseline: CPU, CUDA, Vulkan/HIP, hybrid offload, familiar API and broad GGUF coverage. Pin the exact tested commit/build; do not automatically overwrite the working binaries.
2. Keep PrismML separate for Bonsai/Mitsuba formats requiring it. An upstream upgrade does not necessarily replace that fork. Validate vision projector and quant support independently.
3. ik_llama.cpp is a high-priority server experiment for AVX2 CPU plus Turing CUDA and MoE offload. It is not the preferred 890M GPU backend; Vulkan/ROCm performance is not its focus. Kolibri architecture support is a separate prerequisite.
4. ExLlamaV3/TabbyAPI is a secondary experiment for NVIDIA/EXL3 models. Current research reports experimental Turing support and AVX2 offload; confirm exact model and release compatibility before selecting it. It is not a GGUF substitute.
5. vLLM/SGLang are targeted options when concurrent throughput warrants their overhead. Framework support for a GPU does not establish support for all attention/quantization kernels. Do not assume native BF16/FP8 support on2080s. Avoid making these the default on an8GB-RAM laptop.
6. Beelink: compare Vulkan first against CPU, then HIP/ROCm with verified driver/kernel compatibility. Official Ryzen support has evolved, so blanket claims that890M cannot use ROCm are stale. Exact HX PRO370 SKU, Fedora host and quantized inference kernels still need checking against AMD's validated stack.
7. KTransformers now has an AVX2 path, but its documented24GB+ GPU recommendation is outside this fleet's individual cards. Lower priority.

Prefer routing complete requests among independent nodes over splitting one model across mixed machines. Server-local GPU splitting is reasonable; cross-node RPC is an experiment only after network bandwidth/latency testing. Keep RPC private. Do not infer usable distributed performance from summed VRAM. exo/distributed-llama merit investigation only for a concrete capacity need, not fleet-wide replacement.

Primary software references:
https://github.com/ggml-org/llama.cpp
https://github.com/PrismML-Eng/llama.cpp
https://github.com/ikawrakow/ik_llama.cpp
https://github.com/turboderp-org/exllamav3
https://github.com/theroyallab/tabbyAPI
https://docs.vllm.ai/en/latest/getting_started/installation/gpu/
https://docs.sglang.io/docs/get-started/install
https://rocm.docs.amd.com/projects/radeon-ryzen/en/latest/docs/compatibility/compatibilityryz/native_linux/native_linux_compatibility.html
https://github.com/kvcache-ai/ktransformers
https://github.com/ggml-org/llama.cpp/tree/master/tools/rpc

## Evaluation and rollout gates

Order:
1. Capture baseline outputs, executable hashes/build IDs, model hashes, templates, service configuration, resources and endpoint behavior. Resolve wrapper/process provenance and exposure before changing it.
2. Build a pinned current llama.cpp in a separate prefix. Verify old models first to isolate runtime gains/regressions from model changes. Preserve Prism services.
3. Saluki versus current Qwen/Bonsai/Nanbeige on fixed tool/research/code tasks; start server, then isolated core trial.
4. Add small embeddings/rerank and ASR/TTS comparisons where they remove generative work.
5. Kolibri server experiment with its required patch; Clef decision adapter experiment; Mitsuba only after artifact/support verification.
6. Promote only a passing candidate with a rollback and a verified service restart. Do not upgrade all nodes simultaneously.

Measure actual useful work:
- Exact Hermes/Arda tool schemas: nested arguments, no-tool decisions, multi-turn call IDs, streaming, failures, duplicate side effects, prompt injection in retrieved text. Score final state, not JSON validity alone.
- Coding: held-out local repository tasks, patch validity, compilation/tests, unintended edits, elapsed time.
- Research: fixed conflicting source packs, citation entailment, omission/unsupported claims and abstention.
- Retrieval: BM25 versus embedding versus reranking on labeled queries; Recall@k/nDCG and p95 latency.
- Vision: OCR/screenshots/tables and real image-to-prompt cases. Test projector/runtime together.
- Operational metrics: cold/warm load, time to first useful token/call, prompt throughput, generation rate, successful task time, peak RAM/VRAM, swap, GPU/CPU temperature, throttling, energy where measurable.
- Context4K/8K/16K and concurrency1/2/4 only where memory permits; expand context after correctness and capacity pass. Do not advertise1M context from a model card as local acceptance.

Remaining uncertainties: measured DDR4 bandwidth, sustained power/thermal limits, cross-node throughput, exact loaded LM Studio workloads, binary provenance on every service, supported large-model forks on these GPUs, comparative task quality, and Mitsuba download availability. No throughput winner or final optimal deployment is claimed without these tests.
