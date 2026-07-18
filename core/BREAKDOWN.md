---
soterion:
  sigil: "SCROLL"
  glyph: "🗄️"
  code_point: "U+1F5C4"
  role: "core_assessment"
  owner: "HADES"
  status: "draft"
  last_reviewed: "2026-07-17"
---

# Core Assessment & Cleanup Plan

Root: `/var/home/mythos/Eregion/Arda/core`

## Current state

`core/` is half state store, half project archive, half code. It mixes hundreds of runtime JSON state files, large snapshot blobs (~1.5 MB each), 222 `tick_output_*.txt` log files, metrics snapshots with timestamped full-tree copies, actual Rust source in `realm/` and `personal/`, plan docs that overlap with `crates/spine/...`, and edge/fleet/knowledge configs of unclear ownership.

This is the messiest tree in the repo. It needs domain separation, not just file moves.

## Classification

### Keep — likely live runtime state
- `state/fleet_*`, `state/governance_runtime.json`, `state/package_*`, `state/queue_*`, `state/runtime_*`, `state/operator_*`, `state/source_*`, `state/warden_*`, `state/task_lifecycle_runtime.json`, `state/tool_*`, `state/memory/*`, `state/plans/`, `state/ledger/`, `state/alerts/`
- `queue/queue.jsonl`
- `knowledge/*.jsonl`

### Keep but relocate
- `realm/*.rs` → proper crate under `crates/`
- `personal/` → `core/personal_crate/` or app-local
- `edge/*` → `config/edge/` after confirming consumers
- `clients/*` → `integrations/clients/` if not core-state
- `projects/` → `docs/plans/` or archive; plan artifacts do not belong under `core/`
- `metrics/` → `ops/metrics/` or `monitoring/metrics/`
- `core_audit.json` → `audit/`
- `state/_archive/*` → top-level `archive/core_state/`

### Retire or archive
- `state/tick_output_0.txt` .. `tick_output_221.txt` — 222 committed log files; never should have been committed.
- `metrics/history/20260422*` and similar timestamped full-tree copies — deduplicate or archive.
- `state/arda_snapshot.json`, `state/system_snapshot.json` — ~1.5 MB each; likely regenerable exports.
- Heavy state blobs: `source_absorption_pipeline.json` (~361 KB), `operations_flow.json` (~292 KB), `task_lifecycle_runtime.json` (~96 KB), `warden_guardhouse.json` (~83 KB), `warden_policy_authority.json` (~36 KB) — verify live read paths before archiving.
- Annunimas-era lookup-table duplicates already covered by newer files, e.g. `_archive/numenor_prime_promotion_batch_01..06.json`, `_archive/valinor_promotion_batch_01..03.json`.

### Legacy naming to review/archive
- `numenor_*`, `valinor_*`, `aipkg_*`, `legion_hierarchy.json`, `openfang_alignment.json`, `playwright_mcp_productization_contract.json`, `scrapling_runtime_contract.json`, `crawl4ai_runtime_contract.json`, `eliza_alignment.json`, `soterion_*`

Rename to Arda-era terms or mark explicitly as archived/legacy in filenames.

## Immediate cleanup priority

1. Remove committed log spam: `state/tick_output_*.txt`
2. Move `state/_archive/*` to top-level `archive/core_state/`
3. Separate `realm/` and `personal/` code from state; keep only non-code state in `core/`
4. Decide ownership of `edge/`, `clients/`, `metrics/`, `projects/`; move or archive
5. Regenerate or archive large snapshot blobs after confirming live consumers
6. Retire Annunimas-era duplicate batches in `_archive/`

## Proposed structure

```
core/
├── README.md
├── BREAKDOWN.md
├── state/
│   ├── runtime/            # settings, topology, budget, admission
│   ├── memory/             # identity, episodic, reflections
│   ├── fleet/              # nodes, health, steward actions
│   ├── governance/         # runtime, signals, warden policy
│   ├── tasks/              # lifecycle, agent boundaries
│   ├── tools/              # garage, harness contracts
│   ├── operators/          # runtime, actions, legibility
│   ├── sources/            # ecosystem, absorption, lesson registry
│   ├── plans/              # active runtime plan JSON
│   ├── ledger/             # ledger data
│   └── alerts/             # alert state
├── queue/
├ knowledge/
├── edge/                   # or move to config/edge/
├── metrics/                # or move to ops/metrics/
├── clients/                # or move to integrations/clients/
└── archive/                # retired state/plans/metrics
```
