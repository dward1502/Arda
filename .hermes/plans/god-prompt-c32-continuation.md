# God Prompt: C3.2 Runtime Cutover Continuation

## Purpose
Fresh-session prompt for continuing C3.2 qualification. Prevents contamination
from the current session's execution state, intermediate results, or dirty
working tree. Read this file in a new session; do not carry over state from
the session that generated it.

## Authority
You have full execution authority for C3.2 S1–S8 source/test work. You do NOT
have authority for:
- Live recovery or restart (requires operator approval)
- Fourth Verify start, counter reset, Execute replay, or grant extension
- Any provider-free success path that fabricates missing evidence
- Commits/pushes or service changes without independent review

## Current Status (as of this prompt)
- C1.1–C1.5: Complete, reviewed, committed
- C2.1–C2.3: Complete, reviewed, committed
- C3.1: Complete (intake/control through Hermes CLI)
- C3.2: S1–S7 installed and reviewed (commit 3ed6bf44, release e467fdf9). S8
  authority-blocked — the paired objective's Verify grant expired; no durable
  recovery outcome exists.
- C3.2 JSON deserialization fixes: COMPLETE (4 errors fixed on POST /v1/runs/plan)
  - checkpoint null → CheckpointMetadata struct
  - provenance project_id → project_contract_digest + created_by
  - decision "PolicySafe" → lowercase "policy_safe"
  - ledger_writes missing → added to TaskApprovalEnvelope
- C3.2 paired objective (operator-objective-460e1655522bde9e): PLANNED & APPROVED
  - Run: c32-paired-project-1-attempt-1, project 5dd4e2d9-e208-5634-b918-6cec818459c1
  - Plan: succeeded, Approval: succeeded, waiting in execution queue
- C3.2 fresh objective (operator-objective-82fbfe980fd04746): PLANNED & APPROVED
  - Run: c32-fresh-project-2-attempt-1, project 2cc70def-50d6-5f1e-b3a0-eb94571a2090
  - Plan: succeeded, Approval: succeeded, waiting in execution queue
- C3.3: Not started
- C3.4: Not started

## Paused Objective
operator-objective-ccab5b4ffe08a3ed / project-1 run
`objective-operator-objective-ccab5b4ffe08a3ed-leaf-operator-objective-ccab5b4ffe08a3ed-project-1-attempt-1`
- Ends at sequence 20 / Verify provider-running:3
- Grant expired 2026-09-17T23:10:13.065Z (one extra start consumed)
- No fourth Verify, no counter reset, no synthetic overlap

## Evidence (read these, not this prompt, for truth)
- Plan: docs/plans/2026-09-01-arda-objective-runtime-cutover.md (C3.2 section)
- Next-session notes: target/qualification/c32-next-session.txt
- Installation: target/qualification/s7-release/
- Paired admission: target/qualification/c32-paired-admission.json
- Installed manifests: target/qualification/c32-installed.json,
  c32-concurrency-installed.json, c32-reexec-installed.json

## Contamination Warnings
- Repository is dirty; staged metrics and doc changes exist — do not commit
  them without review
- Current session PIDs (keeper 2306586, gateway 2349379, daemon 2404050) may
  be stale — recheck live PIDs before any operation
- Private rollback/ contains prior executable and SQLite backup — do not
  blindly restore authority state
- Historical process IDs from this session are NOT evidence of current state

## Next Gate
Obtain explicit bounded changed control/authority decision for the paused
paired outcome. Then S8 qualification (interruption-first supervision).

## Work Style
- Read the plan file once, extract all tasks
- Do not repeat completed S1–S7 work
- Independent review before commits
- Report evidence, not labels

## C3.2 PlanRunRequest Schema (correct format)
POST /v1/runs/plan with PlanRunRequest:
- project_id, expected_project_contract_digest, graph, envelope
- NOT command/idempotency_key/input/objective

envelope.approval.schema_version: "arda.orome.task_approval.v1"
envelope.approval.decision: lowercase "policy_safe"
envelope.approval.ledger_writes: required Vec<String>

provenance: project_contract_digest + created_by (NOT project_id)
checkpoint: CheckpointMetadata { sequence, recovery_token, checkpoint_digest } (NOT null)