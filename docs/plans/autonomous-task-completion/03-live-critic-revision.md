---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-30"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-30

# Milestone 3 — Live Critic Rejection and Revision

## Status

Complete. Task `operator-task-218b6be4e50c73d8` was rejected because the first receipt chain did not prove the exact artifact bytes, appended durable `revise_task`, ran `queue-operator-task-218b6be4e50c73d8-attempt-2`, and closed with receipt `sha256:ced7cab0e714c23cf78352a924960b74271f7f1f6afe6a8b0a63a6ea7c3ceeeb` after independent acceptance ([evidence](../../audits/2026-08-30-autonomous-loop-installed-acceptance.md)).

## Human-visible result

A real independent critic rejects inadequate work with named defects. Arda converts that rejection into a durable revision, executes the correction, re-verifies it, and closes only after the critic’s concerns are resolved.

## Existing foundation

Engine persists separate implementer, verifier, and critic identities; dispatches provider-backed verification/review; and binds provider/model provenance and receipt lineage before continuation.

## Work

1. Configure a concrete admitted provider/model route for implementer, verifier, and critic roles.
2. Ensure critic identity and context are materially independent from the implementer.
3. Convert named critic defects into the existing `revise_task` continuation contract.
4. Preserve defect text, rejected artifact identity, parent receipt, revised attempt, and final resolution.
5. Reject synthetic, workerless, stale, cross-run, or provenance-free critic receipts.

## Acceptance scenario

Use a reversible implementation fixture with a deliberate semantic defect that passes compilation but violates declared acceptance. The verifier reports command truth, the critic rejects the result with the expected defect, Arda appends `revise_task`, the implementer corrects it, verification passes, and a fresh critic receipt accepts the corrected artifact.

## Exit gate

The live receipt chain proves rejection → named defect → durable revision → corrected artifact → independent acceptance. A critic that only approves does not satisfy this milestone.
