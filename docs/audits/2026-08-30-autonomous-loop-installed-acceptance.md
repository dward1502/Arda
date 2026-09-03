---
soterion:
  sigil: "SCROLL"
  role: "acceptance_evidence"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-01"
---

> 🜏 Soterion: 📜 acceptance_evidence | owner: PROMETHEUS | status: active | reviewed: 2026-09-01

# Autonomous Loop Installed Acceptance — 2026-08-30

This index preserves the bounded installed evidence summarized by the active program. The credential-free [machine evidence index](evidence/2026-08-30-autonomous-loop-installed-acceptance.json), [raw runtime snapshot](evidence/2026-08-30-autonomous-loop-installed-acceptance-raw.json), and [provider receipt snapshot](evidence/2026-08-30-autonomous-loop-provider-receipts.json) retain exact canonical queue rows, project contracts, memory records, provider/model/worker authority, receipt digests, project roots, and overlap timestamps. Canonical runtime ledgers remain mutation authority.

## Acceptance artifact identity

These were the installed artifacts observed during the acceptance runs; follow-up review repairs were rebuilt and redeployed afterward.

| Artifact | SHA-256 |
|---|---|
| `~/.local/bin/arda` | `1bcf03f3d0126ffcb51ca5dcb49f5fc7ef76335122ec7eb0e0d1244b28dab306` |
| `~/.local/bin/arda-cli` | `b21ae71713562125ae2050e18fe0ae8737e8a30564541e7bd5930c115dd2669a` |

The 2026-09-01 joined-close follow-up ran with installed `arda` SHA-256 `f86ea89e47c5e5e9f81cee69d2b240953a662bf8a8b2a4dc00b421be94b3c708` and installed `arda-cli` SHA-256 `c68a4349feac17d708e57dcd2ae7da6dea3c8eec4e5474144125fe74364a8f42`.

Attempt-scoped final council hardening was then installed as `arda` SHA-256 `ad4cf44c83f6d8112faa741be467fa7029f695b56309ceb2bbaa994589c63aee` and `arda-cli` SHA-256 `5873fb88ac507ccc9d8639dd15152c7d5a366c295a00d990b39d49414efece64`; rollback is `/var/home/mythos/.local/share/arda/rollback/20260901T141037Z-attempt-scoped-council-hardening`.

Source HEAD during final observation was `64a94e694331d5a6908c8bc65bec824a2214dcfb`; later uncommitted fixes are not represented by that identity.

## Verified installed behavior

| Gate | Evidence |
|---|---|
| Hermes control path | `operator-task-38ef7fd8a2b84e5e` exercised intake, revision, approval, pause, reprioritization, resume, cancellation, and post-terminal idle timer behavior. |
| Forced restart | The installed queue executor exited with status `86` after the execute continuation was durable; the user timer resumed later stages without another operator continuation. |
| Critic rejection and correction | `operator-task-218b6be4e50c73d8` attempt 1 rejected missing exact-byte evidence and selected `revise_task`; `queue-operator-task-218b6be4e50c73d8-attempt-2` closed successfully. |
| Corrected close receipt | `sha256:ced7cab0e714c23cf78352a924960b74271f7f1f6afe6a8b0a63a6ea7c3ceeeb` |
| Terminal memory outcome | `mem_65318f16b2794990b9ac8b11860ba00d` records the corrected terminal outcome in Mnemosyne. |
| Distinct-project overlap | Project A execute ran from `1788094523214` to `1788094590785` ms; project B execute ran from `1788094523341` to `1788094571042` ms. Both closed with project-native checks. |
| Reviewed shared objective | `operator-task-fb5a52e3a268ec2d` retained projects `b22c0000-e29b-41d4-a716-446655440002` and `c33d0000-e29b-41d4-a716-446655440003`; both project leaves and all four orchestration leaves completed under `read_only` authority. |
| Per-leaf receipt chain | Every leaf retains succeeded `execute`, `verify`, and independent `review` receipts under `data/runs/<run-id>/execution-receipts/`; the root carries the six terminal queue receipt digests. |
| Receipt-backed joined close | The root is `arda.workbench.objective_terminal.v1`, `completed`, `close_complete`. Its acceptance artifact is the final canonical critic receipt `data/runs/queue-operator-task-fb5a52e3a268ec2d__verify-acceptance--f2c11fab72358530/execution-receipts/review.json`. Root metadata supplied no synthetic acceptance artifact. |
| Replay posture | The accepted pre-fix reconciliation appended six identical root-terminal records in one pass. The deployed follow-up prevents further terminal appends; two installed replay invocations left the count at six. The append-only ledger was not rewritten. |

## Evidence boundaries

- The global JSONL queue and schedule ledgers are now frozen legacy inputs, not current objective or acceptance authority. `operator-task-020fddb7c36065cf` was interrupted during that subsystem's retirement and is classified as abandoned by architecture cutover, not as a provider failure or terminal result. Its canonical RunStore receipts remain historical evidence; its queue records cannot close Milestone 4.
- The historical overlapping provider runs belonged to separate objectives. Shared-objective joined close is now verified, but its two reviewed real-project execute receipts (`1788259095613` and `1788259726188` ms) were serial; same-objective overlap remains open.
- Forced restart and correction were proven across bounded installed scenarios, not one same-objective deferred/recurring scenario.
- The bridge was exercised with installed Hermes plugin code and authenticated event-shaped input; a genuine external messaging-platform receipt remains open.
- Mnemosyne terminal outcome binding is proven. Live Vairë context-use binding and explicit operator burden acceptance remain open.
- The Manwë inference-probe timer was restored after acceptance, then paused again after two `502` probe failures restarted `arda.service` and the supervised listener remained `503` through its bounded readiness window. This is an open operational-recovery defect, not successful acceptance evidence.
