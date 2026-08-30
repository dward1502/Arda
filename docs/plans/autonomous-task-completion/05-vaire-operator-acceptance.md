---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-08-30"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-08-30

# Milestone 5 — Vairë Continuity and Operator Acceptance

## Human-visible result

Arda remembers why the objective exists, what context it used, what happened, and what remains across Hermes sessions and restarts. The operator judges that the completed loop reduced management burden.

## Work

1. Retrieve authorized Vairë context for the live objective and record exact context references used by planning/execution.
2. Record the terminal outcome, corrections, failures, accepted evidence, and unresolved follow-up with provenance.
3. Resume the objective from a new Hermes session without asking the operator to reconstruct prior context.
4. Run the full program acceptance objective across the prior four milestones.
5. Measure operator interventions: distinguish required policy decisions from avoidable “continue,” status, and context-restatement prompts.
6. Present a concise completion review and request explicit operator acceptance or named defects.

## Acceptance scenario

The operator states one multi-project outcome once, leaves, returns in a new session after restart, asks for status, corrects one decision if needed, and later receives the verified result. Vairë provides the relevant prior context and retains the outcome without leaking unauthorized scope or fabricating memory.

## Acceptance record

Record:

- initial operator messages;
- automatic continuations and scheduler wakes;
- genuinely required operator decisions;
- avoidable prompts or manual interventions;
- context-use and outcome receipt IDs;
- elapsed time and attempt/budget use;
- final operator verdict and named defects.

## Exit gate

The operator explicitly accepts that the loop materially reduces management burden. If not, retain the named defects, reopen the owning milestone, and do not archive the program.
