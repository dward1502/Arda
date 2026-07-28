---
soterion:
  sigil: "SCROLL"
  glyph: "𓊝"
  role: "runtime_oracle"
  owner: "HADES"
  status: "active"
  last_reviewed: "2026-07-28"
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

- `cargo test -p arda-mandos --all-features`: 75 unit and 2 integration tests passed
- `cargo test -p arda-mandos --no-default-features`: 68 unit and 2 integration tests passed
- Strict Clippy passed for all features and no default features with `--all-targets --no-deps -- -D warnings`
- Rustdoc passed with `RUSTDOCFLAGS='-D warnings'`, all features, and no dependencies
- Direct consumer all-feature tests and checks passed:
  - `arda-aule`: 164 library, 8 CLI, 14 focused integration, and 2 doc tests
  - `arda-orome`: 86 library and 10 integration tests

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
- **Runtime service**: `OracleService` manages atomic `runtime_status.json`, versioned digest-linked `verdict_history.jsonl`, restart hydration, degraded-prefix recovery, ledger verification, verified atomic export, and bounded telemetry delivery to `PlutusService`
- **Daemon**: `OracleDaemonConfig` + `OracleDaemon` supervise IPC and optional HTTP listeners; both transports share typed dispatch, redaction, validation, and structured errors. HTTP additionally exposes `/ledger/verify` and `/ledger/export`

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
| `transport/dispatch.rs` | Shared typed requests, dispatch, redaction, and structured errors |
| `transport/ipc.rs` | Unix socket server |
| `transport/http.rs` | Optional HTTP/SSE server |
| `tests/target_local.rs` | Target-local persistence and restart integration proof |

## Consumer wiring

- Consumes `arda-economics::PlutusService` for background work tracking and relationship signals
- Consumes `arda-governance` for `triad_validate`, `bacon_lite_validate`, `Task`, `Ledger`
- Consumes `arda-core` for error types, daemon spawn helpers, task status
- Depends on: `arda-economics`, `arda-core`, `arda-governance`

## Closed Packet 4 capability set

- Typed escalation, versioned policy, inclusive threshold boundaries, veto semantics, and bounded public reasoning are covered by executable invariants.
- Query, evidence, PageIndex, Unicode notification, persistence, and transport behavior use stable typed contracts.
- Verdict records are restart-safe and tamper-evident across JSON boundaries; recovery retains only the verified prefix and reports degraded reasons.
- IPC and HTTP enforce bounded inputs and expose shared structured errors; listener supervision owns cancellation and socket cleanup.
- Gate disposition counters and telemetry delivery counters remain low-cardinality and exclude query identifiers.
- Verified export refuses corrupt, degraded, legacy, or authoritative-destination output, preserves destination atomicity, verifies and writes one exact byte snapshot, and confines transport-requested destinations beneath the service export root.

Packet 4's temporary `CHECKLIST.md`, `CRATE_PLAN.md`, and `PLAN_CLOSEOUT.md` trackers were retired after strict producer and direct-consumer gates passed. Future work belongs in a new active plan backed by a newly observed behavior gap.
