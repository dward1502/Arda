---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-04"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-04

# Milestone 2 — Installed Scheduling and Restart

## Status

Installed timer execution, paused-task suppression, terminal suppression, provider-backed execution, forced exit `86` after a durable execute receipt, timer-driven verify/review continuation, and unattended correction/closure are proven in the [installed acceptance index](../../audits/2026-08-30-autonomous-loop-installed-acceptance.md). One same-objective deferred or recurring wake across restart remains open.

## Human-visible result

Arda continues a deferred or recurring objective through the installed scheduler, survives a forced restart and a forced failure, corrects the work, and closes without another operator instruction.

## Work

1. Verify scheduling inside the resident `arda.service` runtime. Do not restore the retired JSONL queue executor or timer. The [cutover owner](../2026-09-01-arda-objective-runtime-cutover.md) owns due-schedule integration and recovery.
2. Bind the installed binary identity to the tested source revision and record it in the acceptance receipt.
3. Exercise immediate, deferred `wait_until`, recurring, pause/resume, and cancellation states through canonical schedule records.
4. Force process exit after a durable execution checkpoint and verify exactly-once resume.
5. Force verification failure and prove the continuation engine chooses retry, revision, or bounded stop according to persisted policy.
6. Prove terminal schedule state prevents a later wake.
7. Verify `MAX_OBJECTIVE_ATTEMPTS=5` retry cap prevents runaway re-claiming in the resident ObjectiveRuntime.

## Acceptance scenario

A reversible real-project task runs once, defers until a near-term wake, resumes from the installed timer, is interrupted after a checkpoint, resumes after process restart, fails one declared check, corrects the defect within budget, passes verification, and closes with the expected artifact. No manual “continue” is issued.

## Required evidence

- Installed binary hash and source commit/tree.
- Canonical ObjectiveStore objective/schedule/control and RunStore continuation lineage; legacy JSONL is historical only.
- Resident wake and process-restart observations. Historical timer proof does not qualify the resident replacement.
- Before/after artifact identity and declared check output.
- Proof that no duplicate attempt or post-terminal wake occurred; `MAX_OBJECTIVE_ATTEMPTS=5` retry cap verified in resident ObjectiveRuntime |

## Exit gate

The complete installed scenario succeeds unattended. Package-only or direct-CLI simulation does not close this milestone; the `MAX_OBJECTIVE_ATTEMPTS=5` retry cap must also be verified in the resident ObjectiveRuntime.
