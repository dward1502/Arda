---
soterion:
  sigil: "AEGIS"
  glyph: "◈"
  code_point: "U+25C8"
  role: "knowledge_governance"
  owner: "ATHENA"
  status: "active"
  last_reviewed: "2026-04-30"
---

> 🜏 ATHENA: ◈ knowledge governance | owner: ATHENA | status: active | reviewed: 2026-04-30

# ATHENA Plan Narrative

## Overview

ATHENA owns ingest, digest, deep analysis, and policy-readiness promotion for the sovereign knowledge corpus. This narrative captures the current runtime posture, completed work, degraded surfaces, and the next bounded operator actions.

## Current Runtime State (2026-06-22)

- `core/state/athena_runtime.json` is populated and shows steady recent activity:
  - `deep_graph_recent`: 16
  - `deep_queue_recent`: 16
  - `digest_recent`: 16
  - `policy_ready_recent`: 123
  - `reference_only_recent`: 276
  - `planning_task_receipts_recent`: 1
- ARDA hint posture: `policy_readiness_status: review_pressure` with `next_operator_action: preview_policy_ready_promotion`.
- Deep graph nodes are receiving truth-confidence tagging (`triad_passed: true`, `confidence` in the 0.69–0.77 band) and being linked into governance/research tag surfaces.

## External Source Lane Ledger

`data/athena/external_source_lane_ledger.jsonl` currently contains 5 lanes:

- `web` / `x_bookmarks`: connector gated but connector-ready; task promotion remains off.
- `reddit` / `notebook_lm` / `public_archives`: connector needed; blocked until canonical source ledger exists.

Policy boundary enforced: external lanes stay `task_promotion_allowed=false` until a canonical source receipt with evidence anchors is written by the connector/operator path.

## Completed / Present Work

- Ingest/query/deep/digest command surface is live.
- JSONL read paths are hardened against malformed lines; appends are serialized to avoid interleaving corruption.
- Deep digestion survives deduplicated re-ingest on human and machine source books.
- Scrapling runtime and provider policy are materialized in `core/state/scrapling_runtime_contract.json`.
- External source lane ledger is materialized and emitted as `annunimas.athena.external_source_lane.v1`.

## Degraded / Blocked Work

- No live external connector is active today for Reddit, NotebookLM, or public archives.
- `policy_ready_recent` is high but task promotion is still gated behind canonical receipts; the frontier is not yet sovereign-default Scrapling.
- NotebookLM MCP candidate is noted but unverified.

## Current Frontier

- Materialize and promote policy-ready evidence into implementation briefs/planning tasks once canonical receipts exist.
- Harden workstation-first execution, deterministic task emission, bounded memory lanes, and measurable runtime effectiveness.
- Move Scrapling from preferred direction toward sovereign default only after bounded runtime contract gates pass.

## Hardening Contract

- Workstation = canonical deep-ingest executor; laptop = operator ingress and optional fallback.
- Source provenance must survive ingest through policy-ready promotion and task emission.
- Memory lanes stay bounded across episodic, source-book, policy-ready, and implementation-ready surfaces.
- Task emission must be deterministic, idempotent, and receipt-backed.
- Runtime must remain SELinux-safe, admission-gated, and observable.

## Primary Runtime Surfaces

- `core/state/athena_runtime.json`
- `data/athena/digest.jsonl`
- `data/athena/books/`
- `data/athena/policy_readiness.jsonl`
- `data/athena/external_source_lane_ledger.jsonl`
- `human/library/athena/sources/`

## Verification Commands

```bash
cargo test -p annunimas-athena
cargo run -p annunimas-cli -- status
cargo run -p annunimas-cli -- export queue-hygiene
```

## Alignment with Annunimas Principles

- Evidence and append-only receipting are preserved through every ingest/gate surface.
- Governance-first promotion: nothing becomes a task until every canonical gate records evidence.
- Knowledge continuity is maintained through deep digestion and deterministic graph updates.

## Open Questions

- Is the NotebookLM MCP candidate approved for read-only inspection, or should ATHENA keep agent synthesis as non-canonical truth?
- When will `task_promotion_allowed` flip true for `web`/`x_bookmarks` lanes?

## References

- Quick reference: `core/projects/Plans/ATHENA.md`
- Runtime contract: `core/state/athena_runtime.json`
- Scrapling contract: `core/state/scrapling_runtime_contract.json`
- Ledger: `data/athena/external_source_lane_ledger.jsonl`
- Ledger narrative: `data/athena/external_source_lane_ledger.md`
