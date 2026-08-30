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
