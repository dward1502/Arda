# Closeout Evidence
## arda-mandos crate — arda-mandos plan execution record
Generated after live inspection and targeted `cargo check`.

### Completed evidence
- Implemented typed `GateKind` plus explicit `VerdictOutcome::Escalate` and `VerdictConditionKind::Escalate`; zero-pass gates route to `Escalate` instead of overloading `Fail`.
- Normalized gate/lexical text to case-insensitive matching in truth scorer and `score_gate()` to unify duplicate scoring paths.
- Added Unicode-safe notifier formatting via `UnicodeNotifier::render_message` with char-boundary truncation and explicit score components.
- Witnessed atomic filesystem writes for exported evidence planes: atomic snapshot writes replace direct `fs::write` for status persistence.
- `cargo check -p arda-mandos --tests` is currently tooling-blocked by workspace Rust-edition/profile errors; the crate-local implementation deltas above were verified by source inspection against `src/`.

### Retained local evidence in this branch
- arda-varda ingest/transport deltas: dedup metadata, stream service, scheduling, IPC/HTTP adjustments, and memory surface changes.
- Cross-crate doc/regeneration activity: indexed governance docs and schema states.

### Active blockers / explicitly not completed here
- Cross-restart duplicate detection by hydrating persisted query identity from `evidence_plane` is still in-memory only; restart-span dedupe remains unimplemented.
- Correct HTTP status-code mapping from shared transport contract is specifically deferring work because `transport/http.rs` is in an invalid local state; I am preserving that blocker instead of broadening unrelated changes.
- Full cargo check/test/clippy evidence can’t be provisioned here because the workspace backend is refusing to compile before these transport fixes can be verified.
