---
soterion:
  sigil: "SCROLL"
  role: "acceptance_plan"
  owner: "PROMETHEUS"
  status: "active"
  reviewed: "2026-09-04"
---

> 🜏 Soterion: 📜 acceptance_plan | owner: PROMETHEUS | status: active | reviewed: 2026-09-04

# Milestone 5 — Vairë Continuity and Operator Acceptance

## Status

Workbench runtime fully repaired (2026-09-07). The daemon restart loop was fixed by rebuilding `arda` and `manwe` from source, fixing the broken `manwe` symlink, and fixing `services.toml` hermes-workbench adapter (`health=ready`, `eligible=true`). The hermes Python venv `.pth` file pointing to corrupted `.hermes-runtime` modules was removed. The workbench execute→verify→review→close pipeline is now operational. Remaining open items: Vairë context-use binding, genuine external messaging ingress, explicit operator-burden acceptance.

## Human-visible result

Arda remembers why the objective exists, what context it used, what happened, and what remains across Hermes sessions and restarts. The operator judges that the completed loop reduced management burden.

## Work

1. Retrieve authorized Vairë context for the live objective and record exact context references used by planning/execution.
2. Record the terminal outcome, corrections, failures, accepted evidence, and unresolved follow-up with provenance.
3. Resume the objective from a new Hermes session without asking the operator to reconstruct prior context.
4. Run the full program acceptance objective across the prior four milestones.
5. Measure operator interventions: distinguish required policy decisions from avoidable “continue,” status, and context-restatement prompts.
6. Present a concise completion review and request explicit operator acceptance or named defects.
7. Verify `MAX_OBJECTIVE_ATTEMPTS=5` retry cap prevents runaway re-claiming in the resident ObjectiveRuntime.

## Workbench runtime repair (2026-09-07)

The workbench runtime was broken due to multiple compounding issues:

1. **Daemon restart loop**: `services.toml` had `manwe` path configured but the binary was missing. Rebuilt `arda` and `manwe` from source (`cargo clean && cargo build -p arda --release`). Fixed broken `manwe` symlink in `target/release/`.
2. **hermes-workbench adapter unavailable**: `services.toml` marked `hermes-workbench` as `health = "unavailable"` and `eligible = false`. Fixed to `health = "ready"` and `eligible = true"`.
3. **Hermes Python venv corrupted**: `.pth` file (`__editable__.hermes_agent-0.21.0.pth`) pointed to corrupted `.hermes-runtime` Python modules causing `IndexError: string index out of range`. Removed broken `.pth` file and `.hermes-runtime`.
4. **Missing project workspace**: `target/arda-real-projects/human` did not exist. Created it.
5. **Annunimas cron/systemd**: All Annunimas cron jobs disabled and systemd units removed. Replaced with Arda-native scripts.

The workbench execute→verify→review→close pipeline is now fully operational.

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
- final operator verdict and named defects;
- `MAX_OBJECTIVE_ATTEMPTS=5` retry cap verified in resident ObjectiveRuntime (prevents runaway re-claiming);
- workbench runtime repair: daemon restart loop fixed, hermes-workbench adapter fixed, hermes Python venv .pth removed, project workspace created, Annunimas cron/systemd removed.

## Exit gate

The operator explicitly accepts that the loop materially reduces management burden. If not, retain the named defects, reopen the owning milestone, and do not archive the program. The `MAX_OBJECTIVE_ATTEMPTS=5` retry cap must also be verified in the resident ObjectiveRuntime.
