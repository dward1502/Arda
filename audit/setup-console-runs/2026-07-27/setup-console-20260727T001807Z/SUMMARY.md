# Setup Console Readiness Audit

Generated: 2026-07-27T00:18:12Z
Mode: read_only
Gate status: warn
Receipt: `/var/home/mythos/Eregion/Arda/audit/setup-console-runs/2026-07-27/setup-console-20260727T001807Z/setup_console_readiness_receipt.json`
ARDA projection state: `/var/home/mythos/Eregion/Arda/core/state/setup_console_readiness.json`

## Summary

- pass: 6
- warn: 2

## Portability projection

- Status: warn
- Active blockers: 145
- Label: active portability blockers present
- Source: `audit/portability-audit-runs/2026-07-27/portability-audit-20260727T001807Z/summary.json`

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
- Title: Portability/config hygiene findings classified
- Severity: high
- Evidence: receipt: audit/portability-audit-runs/2026-07-27/portability-audit-20260727T001807Z/summary.json; total_findings=6366; active_blocker_findings=145; high=121; medium=24
- Recommendation: Parameterize high/medium portability findings before enabling automated setup actions.

### endpoint.assumptions — WARN
- Title: Hardcoded endpoint/local-path assumptions inventoried
- Severity: medium
- Evidence: portability_summary=audit/portability-audit-runs/2026-07-27/portability-audit-20260727T001807Z/summary.json; active_blocker_findings=145; active_config_must_parameterize=23; active_script_must_parameterize=1; active_source_must_fix=121; loopback_endpoint=147; private_lan_ip_endpoint=0; hardcoded_home_mythos=3014; hardcoded_var_home_mythos=3014; top_blocker=crates/spine/runtime/manwe/data/governance/bacon_lite.jsonl (50); top_blocker=apps/arda-hud/src/lib/systemActionBus.test.ts (18); top_blocker=config/routing/local_voice_model_lanes.toml (11); top_blocker=apps/arda-hud/src-tauri/src/lib.rs (6); top_blocker=config/templates/arda.local.profile.toml (5)
- Recommendation: Keep setup console read-only until assumptions are parameterized behind environment profiles.

## Scope guard

This audit is read-only except for generated receipt/state/Markdown artifacts. It does not rewrite source files, configs, systemd units, secrets, or runtime services.
