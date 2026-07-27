# AIPKG Plan Narrative

`AIPKG` is the current Arda package/preflight/receipt contract surface. Historic narrative is preserved at `docs/plans/AIPKG.md`.

Status: active
Owner: prometheus
- `docs/plans/AIPKG.md`
- `crates/spine/governance/arda-core/src/aipkg.rs`
- `crates/spine/governance/arda-core/src/loop_engine.rs`
- `spec/aipkg/v0.1/`
Task ledger: `core/state/queue.jsonl`

## Overview

AIPKG (Arda Intelligent Package) is a package management system designed for
autonomous agents, with a focus on safety, governance, and receipt-based
validation.

## Live Runtime Surface

| Surface | Role |
|--|--|
| `crates/spine/governance/arda-core/src/aipkg.rs` | Manifest, preflight receipt, validation receipt types + `validate()` / `preflight_check()` |
| `crates/spine/governance/arda-core/src/task.rs` | Optional `Task::aipkg_manifest` attachment point; serde-skipped when absent |
| `crates/spine/governance/arda-core/src/loop_engine.rs` | Dispatch preflight gate: tasks carrying a manifest must pass `validate()` before bids/joules/triad/council are spent |
| `spec/aipkg/v0.1/` | Manifest schema, example, execution request schema, container doc, receipt schema |

## Current Behavior

- `Task::aipkg_manifest` is optional and backward-compatible.
- In `loop_engine`, if `task.aipkg_manifest` is `Some`, dispatch runs `manifest.validate()` first.
- Valid manifests increment `DispatchPass::aipkg_preflight_passed` and proceed normally.
- Invalid manifests are recorded in `DispatchPass::aipkg_preflight_blocked` as `"{task_id}:{error}"` and skipped before bid/triad/council allocation.
- Existing tests cover both accept and block paths in `loop_engine`.

## What AIPKG Is For

- `.aipkg` defines the sovereign package contract, mandatory preflight, and receipt truth.
- Governance is open standard law, separate from marketplace/financial layers.
- Profiles: `wasm-wasi`, `oci-sandboxed`, `local-sovereign`.
- Receipts: preflight, execution, validation, signed attestation; settlement is optional.

## Operator Runtime Truth

If an attached manifest is invalid, the dispatcher records evidence and does not execute the task.
This makes package compliance an enforced dispatch gate, not an optional check.

## Next Steps

1. Keep executor/validation receipt wiring with `AipkgExecutionReceipt` / `AipkgValidationReceipt`.
2. If needed, add workspace-wide spec validation script to detect drift between `spec/aipkg/v0.1/*` and `arda-core` types/tests.

## References

- Surface: `docs/plans/AIPKG.md`
- `core/state/aipkg_contract.json`
- Spec root: `spec/aipkg/v0.1/`
- Arda governance: `docs/SAFETY_MODEL.md`
- Triad validation: `docs/operations/TRIAD_GATE_OPERATIONS.md`
