# Setup Console Readiness Audit

Generated: 2026-07-26T05:57:37Z
Mode: read_only
Gate status: warn
Receipt: `/var/home/mythos/Eregion/Arda/audit/setup-console-runs/2026-07-26/setup-console-20260726T055735Z/setup_console_readiness_receipt.json`
ARDA projection state: `/var/home/mythos/Eregion/Arda/core/state/setup_console_readiness.json`

## Summary

- pass: 6
- warn: 2

## Portability projection

- Status: missing
- Active blockers: None
- Label: portability receipt missing
- Source: `audit/PORTABILITY_AUDIT_2026-05-24/summary.json`

## Checks

### AGENTS.md — PASS
- Title: Agent/project operating instructions available
- Severity: high
- Evidence: present: AGENTS.md
- Recommendation: Keep AGENTS.md current as setup-console operator context.

### ARDA_ROOT_PROTOCOL.md — PASS
- Title: Root protocol available
- Severity: high
- Evidence: present: ARDA_ROOT_PROTOCOL.md
- Recommendation: Preserve root protocol pointer for new-machine onboarding.

### docs.CODEMAP.md — PASS
- Title: Low-token codemap available
- Severity: medium
- Evidence: present: docs/CODEMAP.md
- Recommendation: Regenerate CODEMAP when repository structure materially changes.

### scripts.runtime_build_env.sh — PASS
- Title: Runtime build environment script available
- Severity: medium
- Evidence: present: scripts/runtime_build_env.sh
- Recommendation: Keep build output/cache paths centralized in runtime_build_env.sh.

### config.manwe.providers.toml — PASS
- Title: Manwe provider registry available
- Severity: medium
- Evidence: present: config/manwe.providers.toml
- Recommendation: Use provider registry values rather than hardcoded endpoints in setup flows.

### environment.surface — PASS
- Title: Environment profile/template surface discoverable
- Severity: medium
- Evidence: present: config/manwe.providers.toml; present: core/state/environment_profile.schema.json; missing: config/arda.toml; missing: config/.env.example; missing: .env.example
- Recommendation: Expose a single setup-console path from environment profile schema to local override templates.

### portability.receipt — WARN
- Title: Portability/config hygiene receipt available
- Severity: medium
- Evidence: missing: audit/PORTABILITY_AUDIT_2026-05-24/summary.json
- Recommendation: Run scripts/audit/portability_audit.py before setup-console readiness publication.

### endpoint.assumptions — WARN
- Title: Endpoint assumptions inventoried
- Severity: medium
- Evidence: portability receipt unavailable
- Recommendation: Generate portability receipt to classify hardcoded endpoint assumptions.

## Scope guard

This audit is read-only except for generated receipt/state/Markdown artifacts. It does not rewrite source files, configs, systemd units, secrets, or runtime services.
