# Repeated Audit Automation Summary

Run ID: `repeated-audit-20260727T001807Z`
Generated: 2026-07-27T00:18:12Z
Contract: `arda.audit.repeated_run.v1`
Gate status: **pass**

## Source receipts

- portability: present — `audit/portability-audit-runs/2026-07-27/portability-audit-20260727T001807Z/summary.json`
- setup_console: present — `audit/setup-console-runs/2026-07-27/setup-console-20260727T001807Z/setup_console_readiness_receipt.json`
- system_audit: present — `audit/system-audit-runs/2026-07-26/system-audit-20260726T055735Z/summary.json`

## Trend comparison

- Baseline: first_repeated_run

## Regressions

- None detected

## Candidate tasks

- [medium] Review ARDA-BARROW-WIGHT audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review ARDA-CORE audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review ARDA-HADHAFANG audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review ARDA-LORIEN audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review ARDA-MANDOS audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review ARDA-ULE audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review FOLDER-ARCHIVE audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review FOLDER-ARCHIVED-SCRIPTS audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review FOLDER-AUDIT audit score below 80 (71/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review FOLDER-HUMAN audit score below 80 (61/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [medium] Review FOLDER-TESTS audit score below 80 (71/100) and create bounded remediation slice. (read_only_then_bounded_fix)
- [high] Parameterize portability blocker crates/spine/runtime/manwe/data/governance/bacon_lite.jsonl (50 findings) after focused test coverage. (bounded_parameterization)
- [high] Parameterize portability blocker apps/arda-hud/src/lib/systemActionBus.test.ts (18 findings) after focused test coverage. (bounded_parameterization)
- [high] Parameterize portability blocker config/routing/local_voice_model_lanes.toml (11 findings) after focused test coverage. (bounded_parameterization)
- [high] Parameterize portability blocker apps/arda-hud/src-tauri/src/lib.rs (6 findings) after focused test coverage. (bounded_parameterization)
- [high] Parameterize portability blocker config/templates/arda.local.profile.toml (5 findings) after focused test coverage. (bounded_parameterization)

## Visibility

- Portability status: active portability blockers present (145 active blockers)
- Setup-console portability status: active portability blockers present (145 active blockers)
- Portability zero-active-blocker projection: False
- ARDA/Hermes state: `core/state/repeated_audit_status.json`
- Regression JSONL: `audit/repeated-audit-runs/2026-07-27/repeated-audit-20260727T001807Z/regressions.jsonl`
- Candidate task JSONL: `audit/repeated-audit-runs/2026-07-27/repeated-audit-20260727T001807Z/tasks-candidate.jsonl`

## Scope guard

This Phase 7 runner is read-only except for generated receipt/state/Markdown artifacts. It does not perform autonomous destructive cleanup, source rewrites, config rewrites, service restarts, or queue mutation.

## Severity counts

- none: 0
