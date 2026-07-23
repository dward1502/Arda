# arda-mandos implementation plan

Source: `crates/spine/runtime/arda-mandos`, `CHECKLIST.md`, live audit of `src/`, test evidence, workspace consumers in `arda-aule`, `arda-orome`, `arda-governance`.

## Current baseline evidence

Live implementation exists:
- typed query validation and bounded IPC/HTTP transport tests
- typed evidence provenance (`EvidenceRef` with stable SHA-256 anchors, revoked redaction, schema `arda.mandos.v2`)
- bounded reasoning context (`ReasoningContext` with cycles/dangling validation, schema `arda.mandos.v3`)
- PageIndex repair with stable `pageindex://` evidence references and percentage-based relevance scores
- 52 passing tests under all-features/no-default-features; crate-local strict Clippy passes
- API internals schema remains closed; Prometheus exposition remains output-consumer owned

## Canonical public path

- Crate root re-exports: `OracleEngine`, `OracleService`, typed query contract, verdict/event schemas, evidence model, reasoning context, PageIndex types, IPC/HTTP/CLI surfaces
- Primary integrations: `arda-aule` operator/CLI/HUD surfaces; `arda-governance` Triad/mandate interpretation; `arda-orome` ambient routing context

## Canonical contracts

- `OracleQuery` is the typed/validated request contract across IPC, HTTP, direct, and CLI.
- `OracleEngine` evaluates policy once per normalized query and writes an auditable verdict before exposing it.
- `Verdict` includes stable evidence references, policy version, confidence/uncertainty, conditional conditions, and monotone evidence scoring.
- `EvidenceRef` is the typed, redacted, digest-anchored provenance record for every cited source in audit and serialized payloads.

## Next implementation steps

1. Decide P0.2 escalation semantics and add typed `GateKind` plus explicit `Escalate` disposition when escalation differs from `Fail` (`src/scoring.rs`, `src/gate.rs`).
2. Normalize all gate/lexical text to case-insensitive matching and unify duplicate scoring paths (`src/scoring.rs`, `src/query.rs`).
3. Extend duplicate detection across service restart by hydrating persisted query identity from `evidence_plane` in P1.4 recovery (`src/persistence.rs`, `src/service.rs`).
4. Add Unicode-safe notifier formatting with character-boundary truncation and explicit score components (`src/notifier.rs`).
5. Return correct HTTP status codes from shared transport contract (`src/service.rs`, IPC/HTTP surfaces).
6. Atomic snapshot writes (`temp+flush+rename`) and bounded indexed recent-history reads (`src/persistence.rs`).

## Open risk items

- Persistence authority is partially migrated; `VerdictRecord` schema staged but not authoritative for engine counters on restart.
- HTTP transport contract returns HTTP 200 with structured errors instead of status-appropriate responses.
- Duplicate detection in-memory only; cross-restart hydration pending P1.4.
- Escalation semantics are undecided, blocking `Escalate` disposition in CLI/HUD and `arda-aule` mapping.

## Status

Canonical public path is intact; next-step evidence requirements center on policy/scoring refinements, persistence authority, and transport/diagnostic hardening rather than missing core functionality.
