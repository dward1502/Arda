# HADES Plan Review

## Overview
HADES is the Arda lifecycle maintenance, cleanup, orphan-handling, and disposal-boundary subsystem. It owns repository hygiene surfaces, lifecycle projections, WARDEN/ATHENA handoff queues, storage hygiene evidence, and the final archive/retention review boundary for artifacts and ledgers.

The current quick reference is `docs/plans/HADES.md`, which points here as the operator-facing plan narrative and graph node.

## Core Runtime Surfaces
The reviewed HADES contract is represented by these primary surfaces:

- `docs/plans/HADES.md` — quick reference and plan pointer
- `core/state/hades_lifecycle.json` — lifecycle projection and current HADES queue
- `data/hades/hades_log.jsonl` — HADES runtime/sweep ledger
- `data/hades/warden_queue.jsonl` — WARDEN handoff queue
- `data/hades/athena_handoff_queue.jsonl` — ATHENA handoff queue
- `core/state/storage_hygiene.json` — storage hygiene audit surface
- `core/state/storage_hygiene_apply.json` — storage hygiene apply/delete receipt surface
- `core/state/active_ruleset.json` — task lifecycle rules naming HADES as final lifecycle boundary
- `core/state/hades_lifecycle.json` — HADES lifecycle projection/hints

## Current Contract
HADES currently owns:

1. **Lifecycle maintenance**: sweep, cleanup, orphan detection, repair signaling, and archive/retention review for system artifacts.
2. **Append-only discipline**: queue and lifecycle ledgers remain append-only; completion is not disposal, disposal is not deletion, and HADES controls the final lifecycle boundary.
3. **Orphan handling**: missing sigil headers and undocumented files are surfaced as investigate-orphan actions rather than silently deleted.
4. **WARDEN / ATHENA handoff**: HADES routes security-sensitive, governance-sensitive, or knowledge-classification work to the appropriate subsystem.
5. **Storage hygiene**: stale runtime backups, oversized runtime ledgers, and archive-retention candidates are audited and, when approved, applied through receipt-backed surfaces.
6. **Operator visibility**: ARDA hints and queue projections expose pending lifecycle actions without implying autonomous destructive authority.

## Observed Runtime / Plan State
The inspected surfaces show HADES is present and active:

- `core/state/hades_lifecycle.json` exists and is actively regenerated; latest inspected projection had `authority = hades_lifecycle_projection` and `pending_actions = 25` in ARDA hints.
- The HADES lifecycle queue primarily contains `investigate_orphan` actions for files missing sigil headers, with no `authorized_by`, `quorum_proof`, or `execute_after_utc` fields populated.
- Recent activity shows ATHENA handoffs for discovered orphan documentation/planning surfaces, including recent platform OS and substrate plan artifacts.
- JouleWork entries record HADES maintenance sweep telemetry with no actions taken and held-for-review counts, reinforcing read-only/audit-first behavior.
- `core/state/storage_hygiene.json` and `core/state/storage_hygiene_apply.json` document storage hygiene audit/apply contracts.
- `core/state/active_ruleset.json` names the task lifecycle pipeline and HADES final-boundary status.
- `core/state/hades_lifecycle.json` carries the live lifecycle projection.
- WARDEN/guardhouse handoff assumptions are historical until `core/state/warden_guardhouse.json` is reintroduced.

## Implementation Status

### Completed / Present
- HADES crate exists at `crates/annunimas-hades`.
- Quick reference exists at `docs/plans/HADES.md`.
- Lifecycle projection exists at `core/state/hades_lifecycle.json`.
- WARDEN and ATHENA handoff paths are represented through HADES queues and recent activity.
- Storage hygiene audit/apply surfaces exist and include receipt-backed cleanup evidence.
- Active ruleset distinguishes completion, disposal, archive/retention, and deletion boundaries.

### Degraded / Blocked
- Current HADES queue contains many orphan-investigation items without explicit authorization or quorum proof; these are review signals, not autonomous delete/apply authority.
- Live lifecycle and storage claims are timestamp-sensitive and should be refreshed before operational decisions.
- Some HADES surfaces are large runtime projections/ledgers; focused reads or JSON queries are preferable to broad repeated inspection.

### Follow-up Work
1. **Orphan review triage**
   - Classify current `investigate_orphan` queue entries by documentation, config, runtime, and historical/archive category.
   - Prefer adding missing Soterion/sigil metadata or routing to ATHENA over deletion.

2. **Storage hygiene continuity**
   - Keep `storage_hygiene.json` and `storage_hygiene_apply.json` aligned with receipt-backed cleanup operations.
   - Review runtime archive retention windows before any destructive cleanup.

3. **Lifecycle projection hardening**
   - Preserve append-only queue/task ledger integrity.
   - Continue treating completion, disposal, and deletion as separate lifecycle stages.

4. **Operator surface clarity**
   - Surface HADES pending actions in ARDA/Hermes as lifecycle signals.
   - Distinguish review-required orphan items from ready-to-apply cleanup receipts.

## Verification Commands
Useful focused checks for this plan surface:

```bash
python -m json.tool core/state/hades_lifecycle.json >/dev/null
python -m json.tool core/state/storage_hygiene.json >/dev/null
python -m json.tool core/state/storage_hygiene_apply.json >/dev/null
scripts/check_task_queue_append_only.sh
```

Refresh queue/lifecycle projection evidence before closeout or active queue selection:

```bash
cargo run -p arda-cli -- export queue-hygiene
```

## Alignment with Annunimas Principles
- **Evidence-first lifecycle:** HADES records sweep, orphan, storage, and handoff evidence before action.
- **No silent deletion:** orphan and cleanup candidates are investigated, handed off, archived, or receipt-backed rather than removed by assumption.
- **Append-only truth:** task and lifecycle ledgers are folded by latest same-id records, not rewritten.
- **Human/governance boundary:** destructive cleanup, retention changes, and final disposal require appropriate authority and receipts.

## Open Questions
1. Which orphan-investigation categories should be auto-routed to ATHENA versus held for HADES/operator review?
2. What retention window should apply to HADES runtime archive bundles after ledger compaction?
3. Should ARDA expose separate counts for review signals, approval-ready cleanup packets, and completed lifecycle receipts?

## References
- Quick reference: `docs/plans/HADES.md`
- Lifecycle projection: `core/state/hades_lifecycle.json`
- Storage hygiene audit: `core/state/storage_hygiene.json`
- Storage hygiene apply receipts: `core/state/storage_hygiene_apply.json`
- Active task lifecycle rules: `core/state/active_ruleset.json`
