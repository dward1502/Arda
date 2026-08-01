# FOLDER-DOCS Good / Bad / Ugly Audit

Generated: 2026-07-26T05:57:37.263029+00:00
Run ID: `system-audit-20260726T055735Z`
Contract: `arda.audit.target_report.v1`
Score: **89/100**

## Duties
- Human-facing architecture, safety, operations, contracts, and plans.

## Score Breakdown
- mission_fit: 20/20
- implementation_completeness: 12/20
- reliability_safety: 20/20
- observability_auditability: 15/15
- portability_config_hygiene: 15/15
- ux_operator_experience: 7/10

## Good
- Folder surface `docs` exists with 85 tracked text files sampled for this audit.
- Found 2 test-like files under the primary target surface.
- Observed 581 audit/status/telemetry/logging token hits in scoped files.
- Latest portability receipt has no active blockers for this target/support surface.

## Bad
- None

## Ugly
- None

## Potential Changes
- Keep current useful operational surface, parameterize portability blockers, and remove only stale/generated/archive drift after a separate evidence pass.

## What Needs Removed
- No automatic removal recommendation; removal candidates require a follow-up evidence pass.

## Evidence
- primary target root (`docs`)
- workspace membership checked from repository manifest: False (`Cargo.toml`)
- sample tracked target file (`docs/CODEMAP.md`)
- sample tracked target file (`docs/INDEX.md`)
- sample tracked target file (`docs/MIRROMERE_RELIC_OUTPOST_VISION.md`)
- sample tracked target file (`docs/PROVENANCE_AND_ATTRIBUTION.md`)
- sample tracked target file (`docs/archive/AGENT_FRAMEWORK_COMPARATIVE.md`)

## Candidate Tasks
- None
