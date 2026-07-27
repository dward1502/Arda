# Arda Portability and Config Hygiene Audit

Generated: `2026-07-27T00:18:12.656938+00:00`
Contract: `arda.portability_config_hygiene_audit.v1`
Root: `/var/home/mythos/Eregion/Arda`
Scan source: `git_ls_files`

## Summary

- Files scanned: 2678
- Files skipped: 239
- Total findings: 6366
- Active blocker findings: 145

## Classification Counts

- `active_config_must_parameterize`: 23
- `active_script_must_parameterize`: 1
- `active_source_must_fix`: 121
- `archive_historical_ok`: 23
- `documentation_example_review`: 100
- `generated_runtime_state_ignore_or_regenerate`: 6091
- `test_fixture_ok`: 7

## Pattern Counts

- `hardcoded_home_mythos`: 3014
- `hardcoded_var_home_mythos`: 3014
- `loopback_endpoint`: 147
- `tailscale_hostname`: 191

## Top Active Blockers

- `crates/spine/runtime/manwe/data/governance/bacon_lite.jsonl`: 50 findings
- `apps/arda-hud/src/lib/systemActionBus.test.ts`: 18 findings
- `config/routing/local_voice_model_lanes.toml`: 11 findings
- `apps/arda-hud/src-tauri/src/lib.rs`: 6 findings
- `config/templates/arda.local.profile.toml`: 5 findings
- `apps/arda-hud/src/lib/weathertop.ts`: 4 findings
- `apps/arda-launcher/src-tauri/src/onboarding/private_config.rs`: 4 findings
- `apps/arda-hud/src/components/arda/modules/HermesDashboardModule.test.tsx`: 3 findings
- `apps/arda-hud/src/lib/endpointConfig.test.ts`: 3 findings
- `crates/spine/runtime/manwe/src/adaptive/service/full/hermes_proxy_driver.rs`: 3 findings
- `crates/spine/runtime/manwe/src/config.rs`: 3 findings
- `crates/spine/runtime/manwe/src/provider.rs`: 3 findings
- `services.toml`: 3 findings
- `src/main.rs`: 3 findings
- `apps/arda-hud/src/App.tsx`: 2 findings
- `apps/arda-hud/src/components/arda/modules/SettingsModule.tsx`: 2 findings
- `apps/arda-hud/src/lib/ardaHudSettings.ts`: 2 findings
- `apps/arda-hud/src/lib/boardroomSlotSettings.test.ts`: 2 findings
- `apps/arda-launcher/src-tauri/src/onboarding/readiness.rs`: 2 findings
- `apps/arda-launcher/src-tauri/src/onboarding/service_plan.rs`: 2 findings

## Sample Active Findings

- `apps/arda-hud/src/App.tsx:2227` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/App.tsx:2252` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/components/arda/modules/HermesDashboardModule.test.tsx:42` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/components/arda/modules/HermesDashboardModule.test.tsx:58` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/components/arda/modules/HermesDashboardModule.test.tsx:73` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/components/arda/modules/SettingsModule.tsx:177` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/components/arda/modules/SettingsModule.tsx:479` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/ardaHudSettings.ts:83` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/ardaHudSettings.ts:83` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/boardroomSlotSettings.test.ts:172` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/boardroomSlotSettings.test.ts:185` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/endpointConfig.test.ts:7` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/endpointConfig.test.ts:8` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/endpointConfig.test.ts:16` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:132` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:132` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:138` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:138` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:154` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:154` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:171` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:171` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:177` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:177` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:200` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:200` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:648` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:648` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:661` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:661` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:678` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/systemActionBus.test.ts:678` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/weathertop.ts:135` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/weathertop.ts:135` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src/lib/weathertop.ts:138` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src/lib/weathertop.ts:138` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src-tauri/src/lib.rs:374` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src-tauri/src/lib.rs:374` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src-tauri/src/lib.rs:401` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src-tauri/src/lib.rs:401` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src-tauri/src/lib.rs:402` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-hud/src-tauri/src/lib.rs:402` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-hud/src-tauri/tauri.conf.json:8` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-launcher/src-tauri/src/onboarding/private_config.rs:80` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-launcher/src-tauri/src/onboarding/private_config.rs:92` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-launcher/src-tauri/src/onboarding/private_config.rs:104` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-launcher/src-tauri/src/onboarding/private_config.rs:128` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.
- `apps/arda-launcher/src-tauri/src/onboarding/readiness.rs:41` `active_source_must_fix` `hardcoded_var_home_mythos` — Use $HOME for the operator home or $ARDA_ROOT for the repository root.
- `apps/arda-launcher/src-tauri/src/onboarding/readiness.rs:41` `active_source_must_fix` `hardcoded_home_mythos` — Use $HOME rather than a named user's home directory.
- `apps/arda-launcher/src-tauri/src/onboarding/service_plan.rs:181` `active_source_must_fix` `loopback_endpoint` — Move endpoint defaults behind config/env such as CHARON_BASE_URL or HERMES_BASE_URL.

## Read-Only Guarantee

This Phase 1 runner only scans text files and writes audit receipts under the requested output directory. It does not rewrite matched source/config/script files.
