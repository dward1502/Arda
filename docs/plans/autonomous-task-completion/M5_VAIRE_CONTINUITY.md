---
soterion:
  sigil: "SCROLL"
  role: "execution_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-22"
  tags: [m5, vaire, continuity, operator-acceptance]
---

> Soterion: execution_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-22

# M5 — Vaire Continuity and Operator Acceptance

## Prerequisites
- M4 complete: real multi-project execution proven
- Vairë context available and bound to the objective lineage
- Daemon running on port 7878, 37 projects loaded

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