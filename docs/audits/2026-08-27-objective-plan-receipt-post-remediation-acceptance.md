---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "acceptance_audit"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-27"
  tags: ["workbench", "objective-plan", "receipt", "post-remediation"]
---

> 🜏 Soterion: 📜 acceptance_audit | owner: RUMIL | status: active | reviewed: 2026-08-27

# Objective-plan receipt post-remediation acceptance

## Decision

The remediated external objective-plan receipt path is accepted for its bounded claim: governed queue tasks now have live post-remediation receipts outside `RunGraph`, each receipt is identity- and digest-bound, and each digest is the second parent in its live graph provenance. The scope and dependent gather tasks reached evidence-backed close, the active synthesize task independently reproduced the binding, and the declared project check passed.

This does not close the broader autonomous-completion program. It proves receipt persistence and graph provenance across two terminal tasks and their currently running dependent synthesis task, not terminal objective closure, dependency enforcement by the worker plan, independent review, or durable continuation.

## Evidence and capability truth

### Implemented

- `crates/spine/observability/arda-aule/src/prometheus/autopilot/workbench_executor.rs:271-279` persists or reloads the objective plan before graph creation and passes only its receipt digest into graph provenance.
- `workbench_executor.rs:778-884` rejects unsafe run IDs, caps receipts at 256 KiB, verifies the stored digest and run/objective identities, revalidates the typed plan, writes through a temporary file, and reuses the persisted plan on restart.
- `workbench_executor.rs:1025-1028` adds the objective-plan receipt digest to graph-level parent receipts without extending the Core `RunGraph` schema.
- `workbench_executor.rs:1676-1762` covers grounded source digests, external persistence, stable replay after source drift, absence of embedded plan data in the graph, unsafe path rejection, and the size limit.

### Configured runtime

- `data/workbench/projects.json:18-35` declares check `test` as `cargo test -p arda-core`; its supplied SHA-256 `ce3a769c4fb1b74cc2ea3a20bad172d392a3221d995dddaeb17944ce2156e0f4` matched the live file.
- All six supplied synthesis authorities matched at execution start: the project registry `ce3a769c…`, completion-loop plan `cbefe9d…`, whole-system plan `b3b556f…`, status report `ecb62ed…`, append-only queue `41afa254…`, and `git status --short` snapshot `bd231459…`. Earlier task receipts retain their own queue and repository snapshots rather than pretending the live authorities had not advanced.
- The active plans require source-bound retrieval, durable receipt lineage, declared checks, honest capability labels, and closure only after acceptance evidence (`docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md:65-85`; `docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md:49-64,173-181`).

### Live deployed proof

- `audit/workbench-queue/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__scope/objective_plan_receipt.json:1-125` is a live `arda.workbench.objective_plan_receipt.v1` for the exact run and task. It records validation `ok: true`, no errors, and topological order `recover-context → inspect-authorities → synthesize-findings → produce-outcome → verify-acceptance`.
- The receipt claims `sha256:910ae6439a81e170567bd83d412524d579730a626d84deb5b0188c9c462f04ea`. The live checkpoint binds that exact digest as its second provenance parent at `data/runs/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__scope/checkpoint.json:214-220`.
- The checkpoint records plan and approval succeeded and execute running under `execute_with_approval` (`checkpoint.json:7-94`). This is positive post-remediation execution evidence, improving the boundary documented in `docs/audits/2026-08-27-governed-execution-acceptance-closeout.md:26-43`, which correctly said the earlier run predated remediation.
- The scope run subsequently reached `close_complete` with closure digest `sha256:074673c59f5f065bcd965e9ee23c9019f93ec18bfca07409ad9e779481f5bec8` (`core/projects/tasks/queue.jsonl:98`), so the receipt-backed path is no longer only an in-flight proof.
- The dependent gather run independently persisted receipt `sha256:4c0f2488f2a37cab82bae669ab2efa0a995f35468d22a81af6e6b869dd628807`; its terminal checkpoint records that digest as the second provenance parent, and queue record 103 closes it with `close_complete` (`audit/workbench-queue/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__gather/objective_plan_receipt.json:105-106`; `data/runs/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__gather/checkpoint.json:220-226`; `core/projects/tasks/queue.jsonl:103`).
- The active synthesize run persisted receipt `sha256:041f65da38c7b67a785fb838e9e31c061c12a47e24b217bb497bc31fdba00445`; its checkpoint records that digest as the second graph-provenance parent while execute is running (`audit/workbench-queue/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__synthesize/objective_plan_receipt.json:105-106`; `data/runs/queue-tsk_20260827T154942Z_arda-receipt-acceptance-20260827__synthesize/checkpoint.json:214-220`).
- Declared check `cargo test -p arda-core` passed: 113 unit tests plus all package integration and doc-test groups completed with zero failures.

## Prioritized repair backlog

### P0 — Make receipt integrity adversarially complete

Evidence: the implementation rejects digest, identity, validation, path, and size failures, but the focused regression at `workbench_executor.rs:1676-1762` directly exercises only stable replay, path traversal, and oversize behavior. It does not mutate a stored receipt to prove digest mismatch, cross-run/objective substitution, malformed contract, or stale validation rejection.

Human-visible behavior: a corrupt or substituted objective-plan receipt stops the run with a precise reason before provider dispatch; restart never silently accepts a different plan.

Smallest authoritative implementation surface: extend the existing `workbench_executor.rs` test module only. Add table-driven mutations for payload, `run_id`, `objective_id`, `contract`, `validation`, and missing digest. No new schema or store is needed.

Acceptance: every mutation fails closed, the untouched receipt replays, and the focused package test passes.

### P0 — Bind acceptance artifacts to the objective plan, not optional queue metadata

Evidence: this receipt's plan requires a prioritized repair backlog (`objective_plan_receipt.json:5-9`), but the claimed queue task has no `meta.acceptance_artifact`. `validate_task_acceptance_artifact` returns success when metadata is absent (`workbench_executor.rs:552-554`). The report therefore exists because the worker followed the prompt, not because terminal acceptance structurally requires it.

Human-visible behavior: close reports the exact artifact path and digest that satisfied each plan criterion; omission or substitution cannot close the node.

Smallest authoritative implementation surface: in `objective_plan_for_task`/receipt construction, derive a typed artifact/evidence requirement from plan acceptance criteria or require the queue admission record to provide it. In `validate_task_acceptance_artifact`, validate against the persisted receipt rather than treating missing optional metadata as success.

Acceptance: removing, changing, or omitting the declared report prevents close even when Cargo tests pass; the accepted artifact digest appears in the execution and closure receipts.

### P1 — Detect source drift explicitly at dispatch and replay

Evidence: the persisted plan intentionally survives source drift (`workbench_executor.rs:1716-1727`), while the worker prompt says to read “live authorities” (`workbench_executor.rs:915-918`). During this audit, the five file authorities matched their receipt digests at the initial read, but current `git status --short` no longer matches the receipt's repository-state digest. The runtime has no typed distinction between acceptable post-plan drift and drift requiring replan.

Human-visible behavior: the operator sees “plan snapshot still valid” or “authority/repository changed; replan required,” with changed source identities. A worker never cites newer bytes as though they were the digest-bound snapshot.

Smallest authoritative implementation surface: before execute dispatch, recompute each context-source digest. Apply an explicit policy: immutable authority drift yields `replan_objective`; expected repository/runtime drift is retained as a new evidence observation with both old and current digests. Pass snapshot paths/digests and drift disposition to the worker.

Acceptance: an operator-plan mutation blocks stale execution; an allowed runtime-state change is disclosed and receipted; restart behavior remains deterministic.

### P1 — Harden concurrent and durable receipt installation

Evidence: `workbench_executor.rs:871-883` uses a deterministic `.json.tmp` path and rename. The queue lease reduces expected concurrency, but this receipt boundary itself does not use create-new semantics, synchronize file and parent directory, or arbitrate two writers for the same run.

Human-visible behavior: duplicate dispatches converge on one byte-identical receipt; conflicting writers fail with a named replay/concurrency error; power loss cannot leave a claimed receipt that was never durably installed.

Smallest authoritative implementation surface: the receipt writer in `workbench_executor.rs`. Use exclusive temporary creation, compare an existing winner by digest, fsync the file and parent directory where supported, and clean stale temporary files only under the same run lease.

Acceptance: a concurrency test produces one canonical receipt, conflicting plans do not overwrite each other, and an interrupted write leaves either the prior valid receipt or no receipt.

### P2 — Expose receipt provenance in the operator projection

Evidence: the live checkpoint carries the receipt digest, but the current human-readable status authority predates this path (`ARDA_SYSTEM_STATUS_REPORT.md:47,119-130`). Operators must inspect audit and run files to distinguish snapshot, current source state, validation, and drift.

Human-visible behavior: Hermes/HUD shows objective-plan receipt digest, validation state, source snapshot digests, current drift disposition, project-contract digest, and linked artifact evidence beside the current node.

Smallest authoritative implementation surface: project the existing receipt and checkpoint fields through the canonical objective projection; do not create another authority or queue.

Acceptance: one projection reconstructs this run's plan provenance without reading raw JSON and labels the current proof as running rather than closed.

## Dependency order and acceptance boundary

1. Add adversarial receipt tests.
2. Bind plan acceptance to artifacts/evidence before terminal close.
3. Add typed source-drift disposition before execute/replay.
4. Harden installation concurrency/durability.
5. Project the same canonical evidence to Hermes/HUD.

Post-remediation acceptance is therefore positive but bounded: external persistence, replay stability, validation, and graph provenance are implemented and live across the closed scope and gather tasks and the running synthesize task. Terminal acceptance for the overall multi-task objective remains pending, and the wider dependency, continuation, placement, context, and review gaps in `docs/audits/2026-08-27-continuous-governed-execution-gap-report.md` remain open.
