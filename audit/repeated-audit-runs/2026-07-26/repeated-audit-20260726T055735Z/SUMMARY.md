# Repeated Audit Automation Summary

Run ID: `repeated-audit-20260726T055735Z`
Generated: 2026-07-26T05:57:37Z
Contract: `arda.audit.repeated_run.v1`
Gate status: **warn**

## Source receipts

- portability: missing — `audit/PORTABILITY_AUDIT_2026-05-24/summary.json`
- setup_console: missing — `audit/SETUP_CONSOLE_READINESS_2026-05-25/setup_console_readiness_receipt.json`
- system_audit: present — `audit/system-audit-runs/2026-07-26/system-audit-20260726T055735Z/summary.json`

## Trend comparison

- Baseline: first_repeated_run

## Regressions

- HIGH: Portability summary is unavailable for cyclic comparison.
- MEDIUM: Setup console readiness receipt is unavailable for ARDA/Hermes visibility.

## Candidate tasks

- [high] Portability summary is unavailable for cyclic comparison. (read_only_or_bounded_fix)
- [medium] Setup console readiness receipt is unavailable for ARDA/Hermes visibility. (read_only_or_bounded_fix)
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

## Visibility

- Portability status: active portability blockers present (None active blockers)
- Setup-console portability status: None (None active blockers)
- Portability zero-active-blocker projection: False
- ARDA/Hermes state: `core/state/repeated_audit_status.json`
- Regression JSONL: `audit/repeated-audit-runs/2026-07-26/repeated-audit-20260726T055735Z/regressions.jsonl`
- Candidate task JSONL: `audit/repeated-audit-runs/2026-07-26/repeated-audit-20260726T055735Z/tasks-candidate.jsonl`

## Scope guard

This Phase 7 runner is read-only except for generated receipt/state/Markdown artifacts. It does not perform autonomous destructive cleanup, source rewrites, config rewrites, service restarts, or queue mutation.

## Severity counts

- high: 1
- medium: 1
