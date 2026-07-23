---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "governance_spine"
  owner: "ARDA-CORE / HADES"
  status: "active"
  last_reviewed: "2026-07-22"
---

# arda-core plan

Crate: `crates/spine/governance/arda-core`
Owner surface: governance spine primitives + contracts + loop + registry
Current baseline: `cargo check -p arda-core` OK; `cargo test -p arda-core` 84 passing

## 1. Objective
Make `arda-core` the stable, documented, receipt-backed foundation for
task/agent/governance/loop/ml/service-registry state in Arda. Nothing
upstream should reach around it for canonical types or execution policy.

## 2. Generation boundary
GEN1: existing live surface, evidence collection, and doc alignment.
GEN2: correctness/robustness work that does not grow public API.
GEN3: broader learning/observability/interop that waits until GEN2 is closed.

## 3. GEN1 — existing surface, evidence, docs
- Enumerate public surface in `src/lib.rs` and reconcile against
  `INDEX.md`, `README.md`, `BREAKDOWN.md`.
- Collect evidence paths for every major module.
- Capture known warnings/follow-ups from `BREAKDOWN.md` and confirm each
  with current code.
- Outcome: README/BREAKDOWN/STATUS describe reality, not wish list.

## 4. GEN2 — correctness and robustness
- Governance gate coverage: missing-provider, non-JSON payload, empty
  actions, unknown intent, state read/write fallback.
- Loop engine coverage: bounded dispatch, market-collapse behavior,
  budget exhaustion, triad veto/record-only split, alert emission.
- Learning coverage: routing bias update, best-agent selection, ledger
  round trip.
- Background gate coverage: poison-recovery, scaled limit floor, async
  and sync cap behavior.
- Ledger/message boundaries: malformed input tolerance, envelope
  metadata round trip.
- Service registry: upsert result handling, duplicate rejection,
  validator edge cases.
- Soterion: signature rendering determinism, index persistence recovery,
  watcher resilience.
- Status: baseline tests green; deferred GEN2 follow-up to preserve
  crate boundary stability unless a concrete correctness regression is
  found. GEN3 interop is deferred until needed.

## 5. GEN3 — learning/observability/interop
- Evaluate public learning/memory systems for concepts that can be
  adapted into Arda governance semantics without breaking append-only
  auditability.
- Add observability knobs for loop economy and decision latency.
- Consider interoperability views for `arda-core` contracts consumed
  by external tooling.
- Status: deferred. Record landscape context in task notes; do not merge
  behavior changes until GEN2 is fully closed.

## 6. Execution order
1. Reconcile docs to current code and write STATUS evidence.
2. Add missing tests for uncovered GEN2 behavior paths.
3. Fix small correctness issues from tests, smallest first.
4. Record GEN3 landscape and defer interop until needed.
5. Keep crate boundary stable across all steps.
