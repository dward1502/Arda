---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "contract"
  owner: "ATHENA"
  status: "active"
  reviewed: "2026-07-13"
---

> 🜏 Soterion: 📜 contract | owner: ATHENA | status: active | reviewed: 2026-07-13

# Evaluation and Learning Loop Contract

This contract defines the evaluation and learning loop as a receipt-producing subsystem.

## Scope

Evaluation produces machine-readable receipts.
Learning deltas are explicit artifacts.
Human approval remains explicit when divergence exceeds bounded thresholds.

## Receipt schema

Learning loop receipt fields:
- `receipt_id`
- `run_id`
- `observation`
- `reward`
- `policy_delta`
- `previous_policy_hash`
- `proposed_policy_hash`
- `verdict`
- `approver`
- `timestamp`
- `provenance`
- `schema_version`

## Execution rules

- A loop run is reproducible from receipt ledger alone.
- Reward computation uses treasury/reward state, not ad hoc bonuses.
- Human approval is explicit whenever:
  - `verdict = pending_review`
  - divergence exceeds bounded threshold
  - policy delta touches governance-relevant capabilities

## Evidence classes

| evidence class | meaning |
|---------------|---------|
| `documentation` | eval task schemas and learning receipt schema documented |
| `local_heuristic` | one eval task implemented |
| `source_metadata` | schema_version emitted on all receipts |
| `runtime_receipts` | `core/projects/tasks/queue.jsonl`, `data/prometheus/` contain durable receipts |
| `policy_enforcement` | approval gates are explicit for threshold divergences |
| `independent_review_receipts` | independent review receipts validate learning claims |
| `scoped_autonomy_policy` | named loop scope has explicit execution policy |

## Current default projection

Expected current level: `runtime_receipted` for existing lane receipts; `policy_enforced` after explicit approval-gate implementation.

## CLI behavior

- `arda eval run <task_id>` executes one smoker eval task.
- `arda eval receipt <run_id>` prints eval receipt bundle.
- `arda learning delta <run_id>` prints learning delta receipt.

## Stop condition

Loop run is reproducible from receipt ledger alone.

---
---
soterion:
  sigil: "SCROLL"
  glyph: "📜"
  code_point: "U+1F4DC"
  role: "implementation_guide"
  owner: "HADES"
  status: "active"
  reviewed: "2026-07-13"
---
