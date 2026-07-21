---
soterion:
  sigil: "SCROLL"
  glyph: "𓊝"
  role: "runtime_oracle"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-17"
---

# arda-mandos

Oracle reasoning engine for Arda: triad-gated verdict generation, truth
confidence scoring, pageindex query, notification formatting, and IPC/HTTP
daemon transport for the Oracle service.

Owner: hades | Sigil: 🜏 SCROLL | Status: active

## Summary

`arda-mandos` is the governance reasoning crate for the Arda oracle surface.
It provides:

- `OracleEngine` with triad-gated verdict generation (`Aurelius`/`Bacon`/`Sun Tzu`)
- `Verdict` model with outcome, gate scores, reasoning, resonance, and governance metadata
- `PageIndex` for document/page indexing and keyword search
- `OracleNotifier` for formatted output channels
- `TruthScorer` / `DefaultTruthScorer` / `score_gate()` for confidence/risk/readiness scoring
- `OracleService` runtime with status snapshot, verdict ledger, and background work/relationship signals to `Plutus`
- `OracleDaemon` with IPC Unix-socket + optional HTTP/SSE transport

## Where it lives

- Crate root: `/var/home/mythos/Eregion/Arda/crates/spine/runtime/arda-mandos`
- Data: `data/oracle/*`, env-overridable via `ARDA_MANDOS_HOME`
- Tests:
  - unit tests in `src/reasoning.rs` and `src/service.rs`
  - integration tests in `tests/target_local.rs`

## Verification status

- `cargo test -p arda-mandos`: 5 passed, 0 failed
- `cargo check -p arda-mandos`: OK
- Doc tests: 0
- Import alias fixed: `arda-plutus` renamed to `arda-economics`; `use arda_economics::PlutusService` updated in `src/service.rs`

## Agentic-OS abstractions

- **Oracle engine**: `OracleEngine` evaluates queries through triad gates with resonance, governance, and love-equation guard
- **Verdict model**: `VerdictOutcome` (`Pass`/`Fail`/`Conditional`), `TriadGates`, `GateReasoning`, `VerdictGovernance`
- **Query types**: `Market`, `Document`, `Financial`, `General`
- **Truth scoring**: `TruthScorer` trait with confidence/evidence; `score_gate()` returns truth confidence, operational risk, autonomy readiness, and gating decision
- **Page index**: `PageIndex`/`PageTree`/`TocEntry` for TOC-based document indexing with keyword search
- **Notify**: `OracleNotifier` formats verdict/query output for channels
- **Context**: `ReasoningContext` placeholder for tree-structured queries
- **Runtime service**: `OracleService` manages `runtime_status.json` and `verdict_history.jsonl`; emits background `joule` work and relationship signals to `PlutusService`
- **Daemon**: `OracleDaemonConfig` + `OracleDaemon` run IPC + optional HTTP; HTTP exposes `/status`, `/evaluate`, `/verdicts`, `/paths`, `/events`

## Crate layout

| Module | Role |
|--------|------|
| `lib.rs` | Public exports for all subsystems |
| `reasoning.rs` | `OracleEngine`, `OracleQuery`, `Verdict`, triad gates, governance |
| `pageindex.rs` | `PageIndex`, `PageTree`, `TocEntry`, `SearchResult` |
| `notify.rs` | `OracleNotifier` formatting for output channels |
| `context.rs` | `ReasoningContext` placeholder |
| `scoring.rs` | `TruthScorer`, `DefaultTruthScorer`, `score_gate()` |
| `service.rs` | `OracleService`: runtime orchestration, verdict persistence, Plutus integration |
| `transport/mod.rs` | Daemon config + runner |
| `transport/ipc.rs` | Unix socket server |
| `transport/http.rs` | Optional HTTP/SSE server |

## Consumer wiring

- Consumes `arda-economics::PlutusService` for background work tracking and relationship signals
- Consumes `arda-governance` for `triad_validate`, `bacon_lite_validate`, `Task`, `Ledger`
- Consumes `arda-core` for error types, daemon spawn helpers, task status
- Depends on: `arda-economics`, `arda-core`, `arda-governance`

## Ideas for improvement

1. Flesh out `ReasoningContext` into a real tree/traversal model
2. Add crate-level docs and module-level doc headers for `pageindex`, `notify`, `scoring`, `transport`
3. Add doc tests on public APIs (`OracleService`, `OracleEngine`, `PageIndex`)
4. Add explicit unit tests for `TruthScorer`/`DefaultTruthScorer` and `OracleNotifier`
5. Add migration/schema-version guard for `runtime_status.json` so state upgrades don't lose history
6. Expose truth-score gating as a policy hook in `arda-core` governance instead of local-only checks
7. Consider splitting transport into a feature-gated crate if daemon surface grows
8. Add operator-facing `oracle export` or HUD section so verdict state is visible without JSON inspection
