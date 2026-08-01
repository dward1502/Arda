# FOLDER-APPS Good / Bad / Ugly Audit

Generated: 2026-07-26T05:57:36.911567+00:00
Run ID: `system-audit-20260726T055735Z`
Contract: `arda.audit.target_report.v1`
Score: **93/100**

## Duties
- Frontend and device applications including ARDA HUD and CITADEL avatar.

## Score Breakdown
- mission_fit: 20/20
- implementation_completeness: 20/20
- reliability_safety: 16/20
- observability_auditability: 15/15
- portability_config_hygiene: 15/15
- ux_operator_experience: 7/10

## Good
- Folder surface `apps` exists with 138 tracked text files sampled for this audit.
- Found 1 test-like files under the primary target surface.
- Observed 416 audit/status/telemetry/logging token hits in scoped files.
- Latest portability receipt has no active blockers for this target/support surface.

## Bad
- Scoped files contain 4 crash-path tokens (`unwrap`, `expect`, `panic`, `todo`, or `unimplemented`) requiring manual review.

## Ugly
- None

## Potential Changes
- Classify crash-path tokens as production, test, or impossible-state assertions and remove production `unwrap()` usages.
- Keep current useful operational surface, parameterize portability blockers, and remove only stale/generated/archive drift after a separate evidence pass.

## What Needs Removed
- No automatic removal recommendation; removal candidates require a follow-up evidence pass.

## Evidence
- primary target root (`apps`)
- workspace membership checked from repository manifest: False (`Cargo.toml`)
- sample tracked target file (`apps/README.md`)
- sample tracked target file (`apps/arda-hud/ARDA_HUD_PUBLIC_PRODUCT_STRATEGY.md`)
- sample tracked target file (`apps/arda-hud/BREAKDOWN.md`)
- sample tracked target file (`apps/arda-hud/README.md`)
- sample tracked target file (`apps/arda-hud/arda_hud.settings.json`)

## Candidate Tasks
- Classify and remediate FOLDER-APPS crash-path tokens
