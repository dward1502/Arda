---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "complete"
  reviewed: "2026-10-06"
  tags: [m3, critic, revision, live-rejection]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: complete | reviewed: 2026-10-06

# M3 — Live Critic Rejection and Revision

## Current resident qualification, October 6, 2026

The operator explicitly included M3 implementation and live qualification in the
M4 closeout. The missing continuation is now implemented and installed. The
historical queue chain is not being substituted for current resident evidence.

`arda correct-critic <leaf> <objective> <rejected-review-digest>` requires private
authenticated ingress, the owning operator, a paused objective, and a canonical
Execute → Verify → blocking Review chain with independent workers, real tool
evidence, provider/model provenance, artifact identity and original Vaire
bindings. It appends an immutable `revise_task` record and a successor leaf,
retaining the original run and completed siblings. A fresh objective approval
is required. The successor carries prior attempts against the same five-attempt
cap. This bounded API refuses a leaf with downstream dependents; changing that
dependency plan needs separately reviewed admission. Dirty Git protection still
applies to fresh successor workspaces.

Receipt projection was extracted before adding rejection validation. Native
verification instructions were separated from transport, then clarified to
attest exact declared-check results while recording semantic concerns for the
independent critic. Review retains semantic acceptance and blocks named defects.
Source-only Verify behavior is unchanged.

Validation: Engine/library, ObjectiveStore/Runtime, authenticated ingress and
Hermes adapter suites **342 passed / 27 ignored**, strict library Clippy and native
root build passed. One parallel maintenance-lock fixture encountered contention;
the full serial rerun passed, including that fixture. Adversarial tests cover
synthetic/workerless/stale/provenance-free/approving/wrong-parent evidence, wrong
operator/event, replay, fresh approval, immutable history and inherited retry cap.
These are source fixtures, not the live exit-gate evidence.

**Current resident M3 qualification is complete.** Authenticated objective
`operator-objective-072182cbd75bf44c` completed the full live rejection/correction
chain on the installed runtime. The authorized disposable helper initially
returned only `WGTT`; compilation passed, and the independent critic read
`README.md` and `src/report_projects.py`, then blocked the missing SkylightPros.
The immutable correction retained that rejection, required fresh approval, and
started a distinct successor run. It returned both names in the required order,
passed the exact native compilation check, and received fresh independent
`VERDICT: APPROVE` and Close receipts. Three distinct worker identities were bound
in each run, with admitted provider/model provenance; the profile-selected route
was shared, not separate models hardcoded into the roles.

Terminal objective digest:
`sha256:84e1f11d89021812c8d9a6bab3e98433ddc81e057f04ca1beee3ba7097c20b15`.
Parent attempt count is 2; successor count is 2 with inherited offset 2: **4 of 5**
permitted attempts. One parent critic process and one successor verifier process
timed out; same-run resumption preserved the completed stages. These timeouts
are recorded, not counted as successful worker calls.

Exact authenticated correction replay now acknowledges the committed record
without creating work. Both before approval and after terminal completion,
replay preserved canonical state. Installed root restart preserved both run
chains, context bindings, provider sessions/message/tool counts, the workspace
and exactly one governed memory outcome. The rejected chain remains failed;
only its approved successor closes the objective. The original M4 objective,
all 24 retained files, three memory outcomes and both real-project workspaces
still match their preservation baseline. Mesh-llm and its probe timer remain off.

Earlier diagnostics exposed verifier role confusion, fixture paths outside the
source-read allowlist, failed-receipt journal binding, and an overlong generated
successor run ID. They were cancelled through authenticated control and do not
supply acceptance evidence. The final source uses a bounded digest identity and
journal-bound rejection validation. Final targeted correction tests passed (2),
authenticated ingress passed (27, 2 ignored), strict Clippy and native root build
passed. The native environment includes the GTK/WebKit/protoc prerequisites.

Evidence: `target/qualification/m3-critic-20261006/manifest.json`,
`completed-audit.json`, `rejection-before.json`, `correction-admitted.json`,
`correction-replayed.json`, `completed-before-restart.json`,
`completed-after-restart.json`, and `completed-after-replay.json`.
This qualifies the current resident M3 boundary required by M4. It does not
replace the parent plan's genuine combined messaging scenario. Preserve the
subsequent October 7 bounded [M5 acceptance](M5_VAIRE_CONTINUITY.md); remaining
combined-lineage obligations are not a request to repeat that verdict.
Source delivery is retained in `6f11faa0`; general Git-backed correction still
requires a safely authorized rejected-artifact handoff without bypassing the
fresh-mutation dirty-work guard.

## Prerequisites
- Relevant installed scheduling/restart and bounded retry evidence qualified; the parent plan still tracks combined-scenario M2 acceptance.
- Daemon running on port 7878, 37 projects loaded
- Admitted provider/model route configured for implementer, verifier, and critic roles

## Step-by-step

### M3.1 — Bind independent identities
1. Configure three distinct identities for implementer, verifier, and critic roles
2. Bind each to an admitted provider/model provenance
3. Verify identities are materially independent (distinct worker IDs and sessions, with each role bound to an admitted route)

### M3.2 — Real critic rejection with durable revision
1. Create an objective with a deliberate semantic defect that passes compilation but fails declared acceptance
2. Run the implementer — produces defective artifact
3. Run the verifier — reports command truth, the artifact passes compilation but fails acceptance
4. Run the critic — rejects with named defects, rejected artifact identity, and parent receipt
5. Verify the rejection is durable (persisted in the run record)
6. Append `revise_task` continuation — implementer corrects the defect
7. Re-verify the corrected artifact — passes verification
8. Fresh critic receipt accepts the corrected artifact
9. Preserve the historical accepted chain; revalidate only the changed resident boundary

### M3.3 — Reject invalid critic receipts
1. Attempt to close with a synthetic critic receipt (no real rejection) — must be rejected
2. Attempt to close with a workerless critic receipt — must be rejected
3. Attempt to close with a stale critic receipt (cross-run) — must be rejected
4. Attempt to close with a provenance-free critic receipt — must be rejected
5. Verify a critic that only approves cannot close the objective

## Evidence required
- Critic rejection: named defects, rejected artifact identity, parent receipt
- Durable revision: revise_task append, corrected artifact, fresh verification
- Invalid receipt rejections: synthetic, workerless, stale, provenance-free attempts all rejected
- Receipt chain: rejection → named defect → durable revision → corrected artifact → independent acceptance

## Exit gate
- Receipt chain proves rejection → named defect → durable revision → corrected artifact → independent acceptance
- A critic that only approves does not satisfy this milestone
- MAX_OBJECTIVE_ATTEMPTS=5 cap verified in resident ObjectiveRuntime