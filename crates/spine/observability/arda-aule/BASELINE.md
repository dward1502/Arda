# arda-aule Baseline — 2026-07-22

`cargo check -p arda-aule` recorded baseline for next session.

## Result

- `cargo check -p arda-aule`: passing
- `cargo test -p arda-aule`: run as needed in step 8

## Work completed in this session

- `IMPROVEMENT_PLAN.md` step 1: updated project-identity docs.
  - Updated `BREAKDOWN.md`:
    - refreshed `last_reviewed`
    - replaced obsolete council-blueprint narrative with current module surface
    - added Decisions section binding observability surfaces to `arda-aule`
  - Updated `BASELINE.md`:
    - reset compile narrative from prior fail baseline to current passing state
    - updated remaining issue pointers to current source
  - `DEPENDENCY_AUDIT.md` already contains the active Annunimas -> Arda crate mapping tables from earlier work.
- `DEPENDENCY_AUDIT.md`: Annunimas → Arda mappings already recorded at lines 16-32 and 61-74.
  - `annunimas_apollo` → `arda-orome`
  - `annunimas_hermes` → `arda-orome`
  - `annunimas_oracle` → `arda-governance`
  - `annunimas_onboarding` → `apps/arda-hud`
  - `annunimas_chronos` → removed
  - `annunimas_fleet` → removed

## Residual compile / migration surface

- `src/cli/main.rs` declares modules (`cli_bootstrap`, `cli_dispatch`, `cli_interactive`, `commands`, `export_surface`, `ipc_bridge`, `observability`, `policy_guard`, `support`) that do not match actual filenames in `src/cli/`.
- `src/prometheus/autopilot/runner.rs` hits `serde_json::json!` recursion limit; needs `#![recursion_limit = "256")]` moved to `lib.rs` or increased.
- Pre-existing async/edition parse errors deeper in `ceo/pipeline.rs` and `cli/cli_dispatch.rs`.
- Integration coverage step 7 remains pending in `IMPROVEMENT_PLAN.md`.

## Start here next session

1. Align `src/cli/main.rs` module declarations with actual filenames in `src/cli/`.
2. Fix recursion limit in `src/prometheus/autopilot/runner.rs` or `lib.rs`.
3. Run `cargo check -p arda-aule` and compare against this baseline.
