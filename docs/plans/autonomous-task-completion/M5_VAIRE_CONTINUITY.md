---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-10-07"
  tags: [m5, vaire, continuity, operator-acceptance]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: active | reviewed: 2026-10-07

# M5 — Vaire Continuity and Operator Acceptance

## Prerequisites
- M4 closed for the admitted bounded source-review scope; see [M4](M4_REAL_MULTI_PROJECT.md).
- Vairë context available and bound to the objective lineage
- Fresh installed daemon readiness and authenticated ingress; do not reuse historical project counts.

## Interactive local CLI continuation — October 7, 2026

The operator selected the local CLI and explicitly delegated the continuation.
Two fresh interactive Hermes CLI sessions now recovered the completed two-project
review through the installed local bridge, with an Arda restart between their
turns. Each session used its own persisted, non-summary user row; the bridge
retained its content digest and returned HTTP 200 for canonical objective status.
Neither session resumed the other conversation or received its answer as input.
Local CLI is the selected operator surface; changing to Discord is not required.

Both sessions read the complete three active Vairë outcomes for the same completed
objective: WGTT (6,891 bytes), SkylightPros (7,576 bytes), and join (7,116 bytes).
The full strings were verified in the actual terminal tool responses. Initial
file-tool reads truncated long JSON content lines; the agents retrieved the full
content through read-only terminal commands before answering. They cross-checked
objective/project scope, execution-run identities and accepted receipt references.
They did not inspect either project's current source or prior provider transcripts.

The first turn produced a broad remediation roadmap. After restart, the second
fresh session narrowed the immediate deliverable to **WGTT authorization
regression-test design**, deferring SkylightPros implementation to the backlog.
The resulting checklist covers lookup-error versus missing-account behavior,
bootstrap membership, stored admin/non-admin roles, failed authentication and a
behavioral test gate distinct from lint. Staging checks and unresolved deployment
questions remain separate. This is a revised conversational plan; it does not
change or reopen the completed resident objective and does not execute fixes.

Evidence: `target/qualification/m5-cli-20261007/manifest.json`, with finalized
session/provenance captures, request text, answers, memory-content hashes,
service identities and before/after preservation audits. The second reviewed
answer is `cli-second-final-answer.txt` in that directory. Both completed M3/M4
objectives, run files, original provider sessions, context/use/outcome records and
both real project workspaces remained unchanged. Mesh-llm and its probe stayed off.
Both qualification CLI processes were closed after their turns completed.

The two CLI sessions report 9 and 14 API calls, respectively, and retain token
counts and session timestamps. Session duration includes idle UI time, so it is
not active model time. Billing reports `included` with no actual-dollar value;
zero estimated dollars is not a measured zero resource cost. These measurements
do not repair unknown usage counters on the original resident runs.

Boundaries retained:

- These were agent-submitted interpretations under the operator's explicit
  delegation, not independently human-authored CLI turns or a human verdict.
- Saved governed files were read and checked locally. No new typed Vairë
  ContextUseReceipt or resident execution receipt was emitted for these CLI plans.
- The initial sandbox attempt stalled before persisting a turn and was stopped.
  A host one-shot attempt was correctly rejected because its source was `oneshot`;
  normal interactive `cli` sessions succeeded without relabeling provenance.
- **M5 remains open for the applicable combined-scenario obligations and the
  operator's reduced-burden verdict.** M1/M2/G1 are not closed by these local plans.
  Correction/revocation/privacy source tests remain recorded below; no live memory
  was revoked or modified to manufacture a test.

## Qualification progress — October 6–7, 2026

The later interactive CLI evidence above advances the initial status-only result
below; retain its commands and limitations as the first qualification baseline.

**M5 remains open.** The completed two-project objective and separate M3 correction
were inspected without new worker execution. A root restart and a fresh
authenticated local Hermes session preserved their canonical evidence. This is
bounded installed continuity evidence, not the genuine combined messaging scenario
or an operator reduced-burden verdict.

Source baseline: `f5c8a361f87d95cf04a932a90a15fd52f15eb6d1`, with inherited
uncommitted M3/M4 and reliability changes. This qualification does not claim that
the baseline commit alone contains those changes. Installed root SHA-256:
`0d63d01ba60a7ac635ebc37178c2868f8fee1ee806a804a9a3a60dabbf3d4512`.
No production source or installed binary was changed in this pass.

Private evidence: `target/qualification/m5-continuity-20261006/manifest.json`.
The reusable [audit](../../../scripts/qualification/audit_m5_continuity.py) opens
SQLite read-only and checks selected memory against current governed content,
objective/project/contract scope, accepted-stage worker prompts, receipt parent
links, correction ancestry and unique successful outcomes. It compares retained
evidence across restart; it does not replace canonical receipt validation or
infer semantic influence from a receipt's automatic `used` label.

| Obligation | Evidence established | Remaining acceptance |
| --- | --- | --- |
| M5.1 | The M4 join selected one 6,891-byte governed prior outcome. Its complete content occurs in the accepted Execute, Verify and Review prompts, each matched to its exact use receipt. The accepted synthesis carries forward WGTT authorization, behavioral-test and timeout findings and distinguishes unresolved staging work. | Memory overlaps with admitted dependency reports and direct reads; independent causal attribution to Vairë is not established. Empty project/M3 capsules do not qualify memory consumption. Complete consumption proof on the genuine outcome remains open. |
| M5.2 | Three M4 successes and the corrected M3 success each have exactly one promoted outcome. The rejected M3 run has none. Its immutable rejection, correction plan and inherited attempts remain linked; source correction/revocation/privacy tests pass. | Live correction/revocation/privacy qualification on the genuine combined lineage remains open. The M3 helper is a separate objective. |
| M5.3 | Installed restart preserved all audited run files, context bindings, use/outcome records, correction records and provider session/message/tool counts. A different authenticated local session retrieved both completed objectives and canonical references. Both real project workspaces stayed unchanged. | This session inspected status only. A genuine messaging-session continuation and corrected decision have not been demonstrated. Do not reopen completed work to manufacture this evidence. |
| M5.4 | `policy-decisions.json` separates required approval/correction controls from recovery interventions and retains attempts, the inherited five-attempt cap and session start/end times. | The retained controls are not a full conversational intervention census. Zero provider counters with `cost_measurement=unknown` cannot establish zero cost, API use or active model time. |
| M5.5 | The verified result and limitations are ready for operator review. | Explicit reduced-burden acceptance or named defects is still required. No verdict is inferred from permission to continue this plan. |

The earlier discarded join's prompt remains in the retained transcript. The audit
matches the accepted stage's exact context-use receipt rather than treating any
prompt in that transcript as accepted evidence. Regression tests cover stale
attempts, cross-run/stage matches, metadata-only or truncated memory, changed
capsule commitments and checks under optimized Python.

Validation on the current working candidate:

- `python3 -O -m unittest discover -s scripts/qualification -p 'test_audit_m5_continuity.py' -v`:
  **6 passed**.
- `cargo test -p arda-vaire --all-features -j 2 -- --test-threads=1`:
  **99 passed, 1 ignored** on the host; includes content correction, revocation,
  privacy/domain checks, durable context and outcome replay tests.
- `cargo test -p arda-engine --test context_bootstrap_restart -j 2 -- --test-threads=1`:
  final host run **12 passed, 5 ignored**. The first host run had one empty initial
  claim in `resident_reconciles_completed_run_at_retry_limit_without_dispatch`;
  the isolated rerun and full rerun passed without source changes. Its cause is
  unresolved; retain the initial failure log rather than calling it repaired.
- Initial sandbox Rust runs could not bind local sockets. Host reruns above are
  the test results; ignored live cases were not implicitly exercised.
- Installed audit before restart, after restart and after the fresh local session:
  **identical canonical evidence**. Root remained active with zero automatic
  restarts; mesh-llm and its probe timer remained inactive.

Reproduce the private audit from the repository root, choosing a new output path:

```sh
python3 scripts/qualification/audit_m5_continuity.py \
  --objective operator-objective-c98d83591ddab744 \
  --objective operator-objective-072182cbd75bf44c \
  --output target/qualification/m5-audit-new.json
```

Add `--baseline <previous-report>` for an exact comparison. Reports contain private
canonical context, are written with mode 0600 and are not committed. The command
requires the retained local stores and provider transcripts; it is not a clean
checkout fixture. It performs no restart or operator message submission itself.

Next: continue the parent plan's genuine M1/M2 outcome and prove a relevant
new-session decision correction from canonical context, then obtain the explicit
operator verdict. Preserve accepted M3/M4 effects and keep mesh-llm off.

## Step-by-step

### M5.1 — Retrieve and consume Vairë context
1. Retrieve relevant authorized Vairë context actually consumed by planning/execution
2. Retain context-use references, scope, and evidence of its influence
3. Verify the context was actually used (not just present) — empty capsule or metadata reference is not evidence

### M5.2 — Bind outcomes to lineage
1. Bind results, failures, corrections, and unresolved follow-up to the same objective lineage
2. Test correction/revocation/privacy — corrected work replaces the defective result
3. Verify no duplicated Vairë outcomes on replay

### M5.3 — Resume in new session after daemon restart
1. Complete a session, then restart the daemon
2. In a new authenticated Hermes session, resume the objective
3. Inspect status and correct a decision without reconstructing context
4. Verify continuity — same lineage, no lost state

### M5.4 — Record policy decisions
1. Record required policy decisions separately from avoidable status/continue/context-restatement prompts
2. Include attempts, budget, and elapsed-use evidence for each decision

### M5.5 — Operator verdict
1. Present the verified result to the operator
2. Obtain the operator's explicit reduced-burden verdict or named defects
3. Defects return to their owning milestone for correction

## Evidence required
- Context-use references showing Vairë context was actually consumed
- Lineage binding: results, failures, corrections, follow-ups all linked
- Resume evidence: new session, daemon restart, corrected decision
- Policy decisions: recorded with attempts, budget, elapsed use
- Operator verdict: explicit acceptance or named defects

## Exit gate
- Operator accepts that the complete loop materially reduces management burden
- An agent summary, successful build, health response, or empty memory baseline cannot supply that verdict
