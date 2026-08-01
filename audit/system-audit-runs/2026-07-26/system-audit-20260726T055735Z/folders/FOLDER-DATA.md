# FOLDER-DATA Good / Bad / Ugly Audit

Generated: 2026-07-26T05:57:37.193012+00:00
Run ID: `system-audit-20260726T055735Z`
Contract: `arda.audit.target_report.v1`
Score: **87/100**

## Duties
- Runtime outputs, receipts, ledgers, snapshots, and telemetry.

## Score Breakdown
- mission_fit: 20/20
- implementation_completeness: 10/20
- reliability_safety: 20/20
- observability_auditability: 15/15
- portability_config_hygiene: 15/15
- ux_operator_experience: 7/10

## Good
- Folder surface `data` exists with 6 tracked text files sampled for this audit.
- Observed 1723 audit/status/telemetry/logging token hits in scoped files.
- Latest portability receipt has no active blockers for this target/support surface.

## Bad
- None

## Ugly
- None

## Potential Changes
- Review whether this folder needs explicit validation or only upstream crate/script tests; do not add tests mechanically.
- Keep current useful operational surface, parameterize portability blockers, and remove only stale/generated/archive drift after a separate evidence pass.

## What Needs Removed
- No automatic removal recommendation; removal candidates require a follow-up evidence pass.

## Evidence
- primary target root (`data`)
- workspace membership checked from repository manifest: False (`Cargo.toml`)
- sample tracked target file (`data/manwe/bandit.json`)
- sample tracked target file (`data/manwe/lane_fitness.json`)
- sample tracked target file (`data/manwe/package_runtime_signals.json`)
- sample tracked target file (`data/manwe/provider_capability_receipts.json`)
- sample tracked target file (`data/manwe/provider_runtime_state.json`)

## Candidate Tasks
- None
