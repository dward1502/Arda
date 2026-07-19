# arda-aule Baseline — 2026-07-18

`cargo check -p arda-aule` recorded baseline for next session.

## Result

- errors: 11
- warnings: 1

## Work completed in this session

- `DEPENDENCY_AUDIT.md`: updated Annunimas → Arda mappings at lines 16-32 and 61-74.
  - `annunimas_apollo` → `arda-orome`
  - `annunimas_hermes` → `arda-orome`
  - `annunimas_oracle` → `arda-governance`
  - `annunimas_onboarding` → `apps/arda-hud`
  - `annunimas_chronos` → removed
  - `annunimas_fleet` → removed
- Source compile blockers from audit section 2:
  - #1 `ceo/mod.rs`: removed stale `pub mod service;` declaration.
  - #3 `cli/mod.rs`: replaced stale module declarations with existing file names.

## Remaining issues

- `src/cli/main.rs` declares modules (`cli_bootstrap`, `cli_dispatch`, `cli_interactive`, `commands`, `export_surface`, `ipc_bridge`, `observability`, `policy_guard`, `support`) that do not match actual filenames in `src/cli/`.
- `src/prometheus/autopilot/runner.rs` hits `serde_json::json!` recursion limit; needs `#![recursion_limit = "256")]` moved to `lib.rs` or increased.
- Pre-existing async/edition parse errors deeper in `ceo/pipeline.rs` and `cli/cli_dispatch.rs`.
- Audit #2 work was intentionally left untouched per user request.

## Start here next session

1.-align `src/cli/main.rs` module declarations with actual filenames in `src/cli/`.
2. Fix recursion limit in `src/prometheus/autopilot/runner.rs` or `lib.rs`.
3. Re-run `cargo check -p arda-aule` and compare against this baseline.
