---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-22"
  tags: [m3, critic, revision, live-rejection]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-22

# M3 — Live Critic Rejection and Revision

## Prerequisites
- M2 complete: scheduling, restart, and bounded retry proven
- Daemon running on port 7878, 37 projects loaded
- Admitted provider/model route configured for implementer, verifier, and critic roles

## Step-by-step

### M3.1 — Bind independent identities
1. Configure three distinct identities for implementer, verifier, and critic roles
2. Bind each to an admitted provider/model provenance
3. Verify identities are materially independent (different worker IDs/routes)

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