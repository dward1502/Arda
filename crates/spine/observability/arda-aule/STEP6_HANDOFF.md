# arda-aule Step 6/7 Handoff — Real Surface Wiring

## Completed
- Removed fake `cli-default`/`cli-full` feature split; restored single `full-cli` feature in `Cargo.toml`.
- Un-gated `src/cli/mod.rs` and `src/cli/commands/mod.rs`.
- Deleted dead duplicate `src/cli/commands/charon.rs`.
- Added real `arda_root()` helper in `src/cli/support.rs`.
- Expanded `src/cli/main.rs` `Commands` enum to match the real dispatch surface in `cli_dispatch.rs`.

## Current Verification State
- `cargo check -p arda-aule`: passes
- `cargo check --features cli-default -p arda-aule`: passes
- `cargo check --features full-cli -p arda-aule`: **still fails** (~1238 errors)

## Exact Remaining Debt (3 buckets)

### 1. Restore-this-now (accidental overwrite)
- `src/cli/commands/aipkg.rs` was overwritten with only an `arda_root()` stub. The original `AipkgCommands` enum and `handle()` body need restoring.

### 2. Dead dispatch arms in `src/cli/cli_dispatch.rs`
These arms reference missing modules/types:
- `learning::handle(command)` — `src/cli/commands/learning.rs` does not exist.
- `apollo::handle(command)` — `src/cli/commands/apollo.rs` has Rust 2015 edition parse errors (`async fn`).
- `forge::handle(command)` — depends on missing `arda_forge_mind` crate.
- `manwe::handle(command)` — depends on missing `crate::commands::manwe` module.
- `Iterate { ... }` — depends on missing `arda_forge_mind` crate.
- `export_surface::run(command)` — the ` Export { command }` arm dispatches into `export_surface`; this path is not yet connected to `main.rs`.
- `control::handle(command)`, `Council`, `Venture`, `Utility`, `Pipeline`, `Aipkg`, `Athena`, `Prometheus`, `Mnemosyne`, `Hades`, `Hermes`, `Chronos`, `Plutus`, `Oracle`, `Metrics`, `State`, `Onboarding`, `Halt`, `Warden`, `Loop` — dead unless wired.

### 3. Missing imports in `src/cli/policy_guard.rs`
- `Commands::Hades`, `Commands::Hermes`, `Commands::Manwe`
- `HadesCommands::Remove`, `HermesCommands::Send`, `ManweCommands::Route`, etc.
- Needs actual imports of those enums/commands from the command modules.

### 4. Missing dependencies/types referenced across the surface
- `arda_forge_mind`, `annunimas_fleet`, `sha1`, `sysinfo`, `axum` referenced but not in `Cargo.toml`.
- `crate::compat`, `arda_core::orders`, `crate::registry`, `crate::core_link`, `crate::heartbeat`, `arda_orome::HermesService`, `arda_mandos::reasoning::build_runtime_snapshot` — unresolved imports.

## What to do in the next session
1. Restore `src/cli/commands/aipkg.rs` from git.
2. Remove or wire the dead dispatch arms in `cli_dispatch.rs` — don’t leave them calling missing modules.
3. Add missing imports to `policy_guard.rs` for the enums it already references.
4. Decide on missing dependencies: add them to `Cargo.toml` or remove the code paths that need them.
5. Provide real `ControlCommands`, `CouncilCommands`, `VentureCommands`, `UtilityCommands`, `PipelineCommands`, `AipkgCommands`, `AthenaCommands`, `PrometheusCommands`, `ManweCommands`, `MnemosyneCommands`, `HadesCommands`, `HermesCommands`, `ChronosCommands`, `ApolloCommands`, `PlutusCommands`, `OracleCommands`, `MetricsCommands`, `StateCommands`, `OnboardingCommands`, `HaltCommands`, `WardenCommands`, `LoopCommands`, `ForgeCommands` enum definitions in their respective files, and wire `main()` to call `cli_dispatch::execute()`.
6. Verify with `cargo check --features full-cli -p arda-aule`.

## Important
- Do not restore `cli-default`/`cli-full`. The user explicitly wants a single working surface.
- Do not add `#[cfg(feature = "full-cli")]` around bad code as a substitute for wiring.
- If a backend service doesn’t exist yet (`arda_forge_mind`, `annunimas_fleet`, etc.), remove the command arm or mark it deferred in `IMPROVEMENT_PLAN.md`.
