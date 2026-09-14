---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "bounded_acceptance_evidence"
  owner: "RUMIL"
  status: "active"
  reviewed: "2026-08-27"
  tags: ["t2", "t5", "durable-leaves", "continuation", "installed-timer"]
---

> 🜏 Soterion: 📜 bounded_acceptance_evidence | owner: RUMIL | status: active | reviewed: 2026-08-27

# T2/T5 Durable-Leaf and Continuation Acceptance

## Decision

The T2/T5 source slice is implemented and package-verified. Installed-timer evidence is positive but incomplete, so this audit does not mark T2 or T5 workflow-proven.

## Verified source behavior

- A validated objective becomes independently durable canonical queue leaves with explicit dependencies, project, authority, verification checks, evidence requirements, derived budgets, and acceptance metadata.
- Embedded leaf plans are rebound to their persisted SHA-256 receipt and objective/run identity before dispatch. Unsafe receipt run IDs fail closed.
- Retry and revision remain on the same task/objective lineage but receive incremented continuation state and fresh attempt-qualified Workbench run IDs.
- Replan metadata is validated before queue append; malformed roots fail instead of poisoning effective queue state.
- Persisted `max_attempts` bounds retries.
- Startup reconciliation converges after a process crash between terminal leaf append and successor/continuation append.
- Objective closure requires every leaf receipt plus the declared acceptance artifact and required markers.

## Automated evidence

- `cargo test -p arda-aule --features full-cli`: 227 library, 8 CLI, 22 integration, and 2 doc tests passed.
- `cargo clippy -p arda-aule --features full-cli --all-targets -- -D warnings`: passed.
- Focused regressions cover durable leaf reconstruction, corrected revision directives, receipt tamper rejection, distinct retry run identity, max-attempt exhaustion, crash-time successor reconciliation, and evidence-bound closure.
- Release and installed binary SHA-256: `3f66032b89fcceda8fba70937cb930ee57cb77dd762bef978193c6092ea60443`.

## Bounded installed-runtime evidence

Objective `operator-objective-t2-t5-live-20260827-v2` was consumed by the installed executor without another operator instruction. Its root reached `waiting`; five durable leaves were appended; four stayed dependency-blocked; and `recover-context` became the eligible claimed leaf with Workbench run `queue-operator-objective-t2-t5-live-20260827-v2__recover-context`. The objective and its five leaves were later retired through append-only terminal records after the acceptance environment became unusable.

## Open acceptance gates

The user systemd bus currently fails with `Failed to connect to user scope bus via local transport: Connection refused`. Consequently, this run does not prove:

1. process restart between durable leaves;
2. installed-timer consumption of a forced verification failure;
3. corrected same-lineage revision followed by a fresh provider attempt;
4. unattended terminal closure; or
5. live artifact/evidence-bound acceptance.

After user-systemd recovery, use a fresh objective ID and preserve queue, service, journal, run, receipt, and artifact evidence across the entire chain. Do not reuse the retired v1 or v2 objectives.