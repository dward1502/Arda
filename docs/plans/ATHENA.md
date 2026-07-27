---
soterion:
  sigil: "AEGIS"
  glyph: "◈"
  code_point: "U+25C8"
  role: "knowledge_governance"
  owner: "ARDA-VARDA"
  status: "active"
  last_reviewed: "2026-07-27"
crate: arda-varda
agent: athena
realm: knowledge
sigil: "𓁿"
status: operational
---

> Arda-VARDA: ◈ knowledge governance | owner: arda-varda | status: operational | reviewed: 2026-07-27

# arda-varda Plan Narrative

## Name / Identity

`ATHENA` is now implemented as the `arda-varda` executor. This document merges
the operator-facing plan narrative from the original `ATHENA` narration with the
current live crate/runtime surfaces, preserving detailed operational context
instead of only renaming paths.

## Overview

`arda-varda` owns ingest, digest, deep analysis, learning emissions, and
policy-readiness promotion for the sovereign knowledge corpus. The original
ATHENA narrative described ATHENA as owning ingest, digest, deep analysis, and
policy-readiness promotion for the sovereign knowledge corpus, capturing the
runtime posture, completed work, degraded surfaces, and next bounded operator
actions. That substance is preserved here; only crate paths, ownership, and
current names are updated.

## Current Runtime State

- Crate root: `crates/spine/executors/arda-varda`
- Data/core roots: `data/athena/*`, `core/state/*`, env-overridable persistence
- Tests/validation: implementation/testing surface is present

Prior externally observed runtime-state signals remain conceptually valid: the
knowledge stores use append-only JSONL persistence; recent counts are best read
from live heads rather than fixed snapshots.

## External Source Lane Ledger

`data/athena/external_source_lane_ledger.jsonl` can contain multiple lanes such
as `web`, `x_bookmarks`, `reddit`, `notebook_lm`, and `public_archives`. Policy
boundary remains in force: external lanes should keep `task_promotion_allowed=false`
until a canonical source receipt with evidence anchors is written by the
connector/operator path.

## Completed / Present Work

- Ingest/query/deep/digest command surface is live.
- JSONL read paths are hardened against malformed lines; appends are serialized to avoid interleaving corruption.
- Deep digestion survives deduplicated re-ingest on source books and can recombine prior graph/output artifacts.
- Appended-only JSONL persistence covers digest, books, deep queue, policy readiness, planning-task receipts, crawl receipts, and uncertainty selections.
- Source classification and ingestion across GitHub, scholarly, documentation, news, government, chat export, notes, PDF, and X/bookmark-like sources.
- Deep-analysis queue with extraction, implementation brief synthesis, scholarly title generation, and uncertainty sampling.
- Interceptor pipeline with Hades/Warden/Mnemosyne hooks and typed digestion events.
- Governance/telemetry hooks including triad validation, bacon-lite logging, resonance/love/joule scoring, and snapshot-style metrics.
- Transport surfaces: Unix-socket IPC plus optional HTTP/SSE daemon paths.
- Learning lane: knowledge-delta/TTL schema validation and JSONL emissions.
- Deterministic receipts and idempotent append behavior with malformed-line tolerance.
- Human-knowledge surface folded into this crate; old separate `arda-human` crate assumptions are historical.

## Degraded / Blocked Work

- No live external connector is active for all out-of-tree sources today.
- Deep-analysis events are polling-based; no SSE stream is exposed yet.
- Some sync/async mixing remains in store/bridge work.
- NotebookLM MCP candidate was noted in the historical ATHENA narrative as a possible surface; it remains unverified.

## Current Frontier

- Materialize and promote policy-ready evidence into implementation briefs/planning tasks once canonical receipts exist.
- Reduce duplicate layout roots and normalize WorkspaceLayout across ingest/human paths.
- Harden workstation-first execution, deterministic task emission, bounded memory lanes, and measurable runtime effectiveness.
- Make store async boundaries explicit or document sync-blocking regions.
- Replace manual JSONL append logic with a shared append-only trait.
- Expose synthesis/queue telemetry through engine/CLI for observability.
- Add schema-version migration for evolving JSONL stores.

## Hardening Contract

- Workstation is the canonical deep-ingest executor surface.
- Source provenance must survive ingest through policy-ready promotion and task emission.
- Memory lanes stay bounded across episodic, source-book, policy-ready, and implementation-ready surfaces.
- Task emission remains deterministic, idempotent, and receipt-backed.
- Runtime remains SELinux-safe, admission-gated, and observable.

## Primary Runtime Surfaces

- `crates/spine/executors/arda-varda`
- `crates/spine/executors/arda-varda/core/`
- `data/athena/`
- `core/state/`

## Verification

- `cargo check -p arda-varda`
- `cargo test -p arda-varda`

## Alignment with Arda Principles

- Evidence and append-only receipting are preserved through every ingest/gate surface.
- Governance-first promotion: nothing becomes a task until every canonical gate records evidence.
- Knowledge continuity is maintained through deep digestion and deterministic graph updates.

## Open Questions

- Is the NotebookLM MCP candidate approved for read-only inspection, or should agent synthesis remain non-canonical truth?
- When will `task_promotion_allowed` flip true for external source lanes?


## References

- Crate: `crates/spine/executors/arda-varda`
- Canonical completed checklist: `crates/spine/executors/arda-varda/PLAN.md`
- Implementation map: `crates/spine/executors/arda-varda/BREAKDOWN.md`
- Historical crate evidence: `docs/archive/arda-varda/`
- Original archive docs: `docs/archive/ARDA_VARDA_ATHENA_REINTEGRATION_PLAN.md`
- Archived tests snapshot: `docs/archive/arda-varda-tests/`
