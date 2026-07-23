---
soterion:
  sigil: "SCROLL"
  glyph: "𓊝"
  role: "runtime_oracle"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-22"
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
- `Verdict` model with outcome, gate scores, typed evidence provenance, bounded reasoning graph, resonance, and governance metadata
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

- `cargo test -p arda-mandos --all-features`: 51 passed, 0 failed
- `cargo test -p arda-mandos --no-default-features`: 50 passed, 0 failed; warning-free
- Crate-local strict Clippy passes with `--no-deps`; full dependency-closure strict Clippy remains separately blocked by pre-existing `arda-core` findings
- Direct consumer check: `cargo check -p arda-orome` passes. The latest
  `cargo check -p arda-aule --features full-cli` reaches `arda-aule` and remains blocked by
  its pre-existing `serde_json::json!` recursion limit in `autopilot/runner.rs`.
- Doc tests: 0
- Import alias fixed: `arda-plutus` renamed to `arda-economics`; `use arda_economics::PlutusService` updated in `src/service.rs`

## Agentic-OS abstractions

- **Oracle engine**: `OracleEngine` evaluates queries through triad gates with resonance, governance, and love-equation guard
- **Verdict model**: `VerdictOutcome` (`Pass`/`Fail`/`Conditional`), `TriadGates`, bounded `ReasoningContext`, `VerdictGovernance`
- **Evidence provenance**: typed supplied/retrieved/inferred/unavailable references with observation-bound SHA-256 digests, per-gate assessments, explicit uncertainty/corroboration signals, integrity rejection, and export redaction
- **Query types**: `Market`, `Document`, `Financial`, `General`
- **Typed query contract**: centralized validation and limits, snake-case wire names,
  correlation/causation IDs, caller/evaluation timestamps, and in-process idempotency
- **Truth scoring**: `TruthScorer` trait with confidence/evidence; `score_gate()` returns truth confidence, operational risk, autonomy readiness, and gating decision
- **Page index**: `PageIndex`/`PageTree`/`TocEntry` for TOC-based document indexing with keyword search
- **Notify**: `OracleNotifier` formats verdict/query output for channels
- **Context**: bounded deterministic claim/evidence/objection/assumption graph with stable IDs, typed edges, validation, summaries, and public-only rationales
- **Runtime service**: `OracleService` manages `runtime_status.json` and `verdict_history.jsonl`; emits background `joule` work and relationship signals to `PlutusService`
- **Daemon**: `OracleDaemonConfig` + `OracleDaemon` run IPC + optional HTTP; HTTP exposes `/status`, `/evaluate`, `/verdicts`, `/paths`, `/events`

## Crate layout

| Module | Role |
|--------|------|
| `lib.rs` | Public exports for all subsystems |
| `evidence.rs` | Typed evidence references, integrity/freshness metadata, assessments, and signals |
| `reasoning.rs` | `OracleEngine`, `OracleQuery`, `Verdict`, triad gates, governance |
| `pageindex.rs` | `PageIndex`, `PageTree`, `TocEntry`, `SearchResult` |
| `notify.rs` | `OracleNotifier` formatting for output channels |
| `context.rs` | Bounded `ReasoningContext` graph, limits, validation, deterministic traversal, and summaries |
| `scoring.rs` | `TruthScorer`, `DefaultTruthScorer`, `score_gate()` |
| `service.rs` | `OracleService`: runtime orchestration, verdict persistence, Plutus integration |
| `transport/mod.rs` | Daemon config + runner |
| `transport/ipc.rs` | Unix socket server |
| `transport/http.rs` | Optional HTTP/SSE server |
| `CHECKLIST.md` | Prioritized Oracle improvement plan, acceptance criteria, and evidence log |

## Consumer wiring

- Consumes `arda-economics::PlutusService` for background work tracking and relationship signals
- Consumes `arda-governance` for `triad_validate`, `bacon_lite_validate`, `Task`, `Ledger`
- Consumes `arda-core` for error types, daemon spawn helpers, task status
- Depends on: `arda-economics`, `arda-core`, `arda-governance`

## Ideas for improvement

The implementation-ready plan is maintained in [`CHECKLIST.md`](CHECKLIST.md). The audit
prioritizes these improvement tracks:

1. **Decision correctness (P0):** add invariant tests, fix evidence scoring, replace
   majority-pass behavior with a versioned policy, and define veto/conditional/escalation
   semantics.
2. **Typed query and evidence contracts (P0):** validate every transport consistently,
   establish idempotency and wire compatibility, and replace free-form evidence with stable
   provenance references.
3. **PageIndex integrity (P0):** repair empty-tree navigation, deterministic multi-document
   search, document refresh, TOC ancestry, stable node IDs, and relevance normalization.
4. **Governance explainability (P1):** implement `ReasoningContext`, expose reproducible score
   components and uncertainty, and choose one authoritative relationship between Mandos's
   local triad and `arda-governance`.
5. **Audit and recovery (P1):** make ledger state restart-safe, detect corruption/schema drift,
   use atomic snapshots, and bound/index history reads.
6. **Runtime safety (P1):** unify IPC/HTTP contracts, return structured errors, enforce limits,
   supervise listener failure/shutdown, protect active Unix sockets, and make Plutus side
   effects observable.
7. **Consumers and operations (P2):** align `arda-aule`/`arda-orome`, add ledger export and
   verification, expose advisory authority and conditions in UIs, and add bounded metrics.
8. **Documentation and quality (P2):** replace stale Annunimas naming, add public API examples,
   document schemas/environment controls, and gate all/no-default feature builds plus docs and
   Clippy.

Transport remains in this crate until the contract stabilizes and measured growth justifies a
split; the checklist treats that as a later architecture decision rather than an immediate
refactor.
