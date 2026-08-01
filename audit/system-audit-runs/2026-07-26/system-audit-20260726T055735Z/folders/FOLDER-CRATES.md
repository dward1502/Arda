# FOLDER-CRATES Good / Bad / Ugly Audit

Generated: 2026-07-26T05:57:36.826665+00:00
Run ID: `system-audit-20260726T055735Z`
Contract: `arda.audit.target_report.v1`
Score: **88/100**

## Duties
- Rust workspace and agent crate surface.

## Score Breakdown
- mission_fit: 20/20
- implementation_completeness: 20/20
- reliability_safety: 8/20
- observability_auditability: 15/15
- portability_config_hygiene: 15/15
- ux_operator_experience: 10/10

## Good
- Folder surface `crates` exists with 483 tracked text files sampled for this audit.
- Found 36 test-like files under the primary target surface.
- Observed 2575 audit/status/telemetry/logging token hits in scoped files.
- Latest portability receipt has no active blockers for this target/support surface.

## Bad
- Scoped files contain 16 crash-path tokens (`unwrap`, `expect`, `panic`, `todo`, or `unimplemented`) requiring manual review.

## Ugly
- None

## Potential Changes
- Classify crash-path tokens as production, test, or impossible-state assertions and remove production `unwrap()` usages.
- Keep current useful operational surface, parameterize portability blockers, and remove only stale/generated/archive drift after a separate evidence pass.

## What Needs Removed
- No automatic removal recommendation; removal candidates require a follow-up evidence pass.

## Evidence
- primary target root (`crates`)
- workspace membership checked from repository manifest: False (`Cargo.toml`)
- sample tracked target file (`crates/README.md`)
- sample tracked target file (`crates/engine/BREAKDOWN.md`)
- sample tracked target file (`crates/engine/CHECKLIST.md`)
- sample tracked target file (`crates/engine/Cargo.toml`)
- sample tracked target file (`crates/engine/INDEX.md`)
- support path (`Cargo.toml`)

## Candidate Tasks
- Classify and remediate FOLDER-CRATES crash-path tokens
