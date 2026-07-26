# Closeout Evidence
## arda-mandos crate — arda-mandos plan execution record
Generated after live inspection, targeted `cargo test`, and consumer checks.

### Completed evidence
- Implemented typed `GateKind` plus explicit `VerdictOutcome::Escalate` and `VerdictConditionKind::Escalate`; zero-pass gates route to `Escalate` instead of overloading `Fail`.
- Normalized gate/lexical text to case-insensitive matching in truth scorer and `score_gate()` to unify duplicate scoring paths.
- Added Unicode-safe notifier formatting via `UnicodeNotifier::render_message` with char-boundary truncation and explicit score components.
- Witnessed atomic filesystem writes for exported evidence planes: atomic snapshot writes replace direct `fs::write` for status persistence.
- Fixed identical retry reuse: `OracleQuery::is_same_request` no longer requires equal caller timestamps, so repeated identical queries return cached verdicts instead of `DuplicateQueryId`.
- Fixed `recent_verdicts` test deadlock surface by testing persistence/ordering behavior directly instead of via the async PDF service path.
- Fixed strict-clippy failures: removed unused variables/dead-code paths so `cargo clippy -p arda-mandos --no-deps -- -D warnings` exits clean.
- `cargo test -p arda-mandos --all-features`: 51 passing tests
- `cargo test -p arda-mandos --no-default-features`: 50 passing tests
- Consumer checks: `cargo check -p arda-orome --tests` passes; `cargo check -p arda-aule --features full-cli` passes.

### Retained local evidence in this branch
- arda-varda ingest/transport deltas: dedup metadata, stream service, scheduling, IPC/HTTP adjustments, and memory surface changes.
- Cross-crate doc/regeneration activity: indexed governance docs and schema states.

### Active blockers / explicitly not completed here
- Correct HTTP status-code mapping from shared transport contract remains pending because transport follow-ups are not in scope for this closeout.
- Persistence authority / restart-safe ledger hydration remains as a future improvement path and is accepted in current state.
- Escalation disposition / Unicode-safe notification docs exist but are not fully formalized; retain in CHECKLIST/BREAKDOWN for follow-up.
