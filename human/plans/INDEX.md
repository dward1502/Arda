# INDEX Plan Review

## Overview
INDEX is the Annunimas plan-directory navigation surface for `core/projects/Plans`. It is the operator and agent entry point for discovering core plan quick references and ensuring the plan portfolio remains visible, deterministic, and aligned with generated queue projections.

The active queue packet is `tsk_20260619_queue_index_plan_review` / `Queue: INDEX plan review`. This human plan narrative documents the observed INDEX contract and closes the missing operator-facing artifact gap for the plan review batch.

## Core Runtime Surfaces
The reviewed INDEX contract is represented by these primary surfaces:

- `core/projects/Plans/INDEX.md` — deterministic directory index for core plan quick references
- `core/projects/Plans/README.md` — generated directory overview pointing to the deterministic index
- `core/state/plan_map.json` — plan-index export projection
- `core/state/queue_active.json` — active task selection projection
- `core/state/queue_summary.json` — compact queue and plan count projection
- `docs/contracts/index-contract.md` — index contract pattern for generated documentation indexes
- `core/projects/tasks/queue.jsonl` — append-only task ledger and closeout target

## Current Contract
INDEX currently owns:

1. **Plan directory discoverability**: keep the canonical core plan quick references visible from a short deterministic index.
2. **Operator navigation**: provide a stable entry point for humans and agents reviewing plan surfaces.
3. **Generated index discipline**: preserve Soterion frontmatter, HADES ownership metadata, deterministic child listing, and no silent deletion of plan links.
4. **Projection alignment**: reconcile generated plan/index projections with the current `core/projects/Plans` directory and active queue surfaces.
5. **Queue review visibility**: ensure plan review queue entries can map back to the core quick-reference files they describe.

## Observed Runtime / Plan State
The inspected surfaces show INDEX is present but projection coverage is partially stale:

- `core/projects/Plans/INDEX.md` exists with Soterion directory-index metadata and lists current core plan quick references.
- `core/projects/Plans/README.md` exists and points operators to `INDEX.md` for deterministic child listing.
- `core/state/queue_active.json` lists `tsk_20260619_queue_index_plan_review` as the next high-priority active task after HADES closeout.
- `core/state/queue_summary.json` reports 15 plan paths, including `core/projects/Plans/INDEX.md` and `core/projects/Plans/PLATFORM_OS.md`.
- `core/state/plan_map.json` is narrower than the current directory summary: inspected output showed only the AIPKG plan in the plan-index export, while queue summary sees the broader plan set. This is a projection-refresh or exporter-coverage follow-up, not a blocker to INDEX narrative creation.
- `docs/contracts/index-contract.md` describes the expected responsibilities and verification shape for generated index files: deterministic discovery, review before canonical replacement, and minimum evidence records.

## Implementation Status

### Completed / Present
- Deterministic core plan index exists at `core/projects/Plans/INDEX.md`.
- Directory overview exists at `core/projects/Plans/README.md`.
- Queue active/summary projections identify INDEX as an active plan-review task.
- Index contract guidance exists in `docs/contracts/index-contract.md`.
- This operator-facing narrative now exists at `human/plans/INDEX.md`.

### Degraded / Blocked
- `core/state/plan_map.json` appears stale or under-inclusive relative to `core/state/queue_summary.json`; it listed only AIPKG in the inspected export while queue summary listed 15 plan paths.
- Existing `core/projects/Plans/INDEX.md` does not list `PLATFORM_OS.md` in the inspected content even though queue summary includes `core/projects/Plans/PLATFORM_OS.md` as a plan path. This should be resolved by the plan-index regeneration path rather than manually rewriting unrelated index content during this review closeout.
- No autonomous destructive action is implied by this plan review; INDEX maintenance remains documentation/index projection work.

## Follow-up Work
1. **Plan-map exporter coverage**
   - Re-run or repair `cargo run -p annunimas-cli -- export plan-index` so `core/state/plan_map.json` reflects all current core plan quick references, not just AIPKG.

2. **Core plan index reconciliation**
   - Regenerate or patch `core/projects/Plans/INDEX.md` through the approved HADES/index workflow if `PLATFORM_OS.md` is still missing from the deterministic listing after export refresh.

3. **Index contract hardening**
   - Apply the contract pattern from `docs/contracts/index-contract.md` to plan indexes explicitly if a dedicated `annunimas.plan.index.v1` contract surface is needed.

4. **Queue projection consistency**
   - Keep `queue_active`, `queue_summary`, and `plan_map` aligned so agents can select active plan-review packets without falling back to stale raw queue rows.

## Verification Commands
Useful focused checks for this plan surface:

```bash
python -m json.tool core/state/plan_map.json >/dev/null
python -m json.tool core/state/queue_active.json >/dev/null
python -m json.tool core/state/queue_summary.json >/dev/null
test -f core/projects/Plans/INDEX.md
test -f core/projects/Plans/README.md
test -f human/plans/INDEX.md
scripts/check_task_queue_append_only.sh
```

Refresh projection evidence before closeout or next active queue selection:

```bash
cargo run -p annunimas-cli -- export queue-hygiene
```

If the plan-index exporter is in scope for the next packet, also run:

```bash
cargo run -p annunimas-cli -- export plan-index
```

## Alignment with Annunimas Principles
- **Evidence-first navigation:** index surfaces describe what exists and should be regenerated or reconciled when projections drift.
- **Append-only truth:** queue closeout is recorded through same-id terminal records rather than raw ledger rewrites.
- **HADES ownership:** generated index files preserve Soterion metadata and HADES review authority.
- **No silent deletion:** missing or stale links become reconciliation tasks, not unreviewed removals.

## Open Questions
1. Should `core/state/plan_map.json` be repaired immediately to include all 15 current plan paths?
2. Should `core/projects/Plans/INDEX.md` be regenerated to include `PLATFORM_OS.md` as part of the INDEX review or left to a dedicated plan-index exporter repair task?
3. Should a dedicated `docs/contracts/plan-index-contract.md` be created to distinguish plan indexes from operations indexes?

## References
- Core plan index: `core/projects/Plans/INDEX.md`
- Core plan README: `core/projects/Plans/README.md`
- Plan-map projection: `core/state/plan_map.json`
- Active queue projection: `core/state/queue_active.json`
- Queue summary projection: `core/state/queue_summary.json`
- Index contract pattern: `docs/contracts/index-contract.md`
