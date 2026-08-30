---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-30"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-30

# Milestone 2 — Installed Scheduling and Restart

## Human-visible result

Arda continues a deferred or recurring objective through the installed scheduler, survives a forced restart and a forced failure, corrects the work, and closes without another operator instruction.

## Work

1. Restore and verify the user-systemd scheduler path used by the installed binary.
2. Bind the installed binary identity to the tested source revision and record it in the acceptance receipt.
3. Exercise immediate, deferred `wait_until`, recurring, pause/resume, and cancellation states through canonical schedule records.
4. Force process exit after a durable execution checkpoint and verify exactly-once resume.
5. Force verification failure and prove the continuation engine chooses retry, revision, or bounded stop according to persisted policy.
6. Prove terminal schedule state prevents a later wake.

## Acceptance scenario

A reversible real-project task runs once, defers until a near-term wake, resumes from the installed timer, is interrupted after a checkpoint, resumes after process restart, fails one declared check, corrects the defect within budget, passes verification, and closes with the expected artifact. No manual “continue” is issued.

## Required evidence

- Installed binary hash and source commit/tree.
- Canonical queue, schedule, run, and continuation lineage.
- Timer invocation and process-restart observations.
- Before/after artifact identity and declared check output.
- Proof that no duplicate attempt or post-terminal wake occurred.

## Exit gate

The complete installed scenario succeeds unattended. Package-only or direct-CLI simulation does not close this milestone.
