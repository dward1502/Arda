arda-aule Cleanup & Improvement Plan
====================================
Source of truth for the 8-step cleanup checklist derived from:
- BREAKDOWN.md
- BASELINE.md
- DEPENDENCY_AUDIT.md
- Live repo inspection (2026-07-22 session)

Summary of current state
- cargo check -p arda-aule        : PASSES (baseline doc is stale)
- Project docs still call it: council blueprint stub
- Actual src surface: prometheus/, ceo/, cli/, telemetry/, export_surface/
- Biggest residual risk: dead annunimas_* import strings and council duplication
- Decision already made: keep these surfaces inside arda-aule

-------------------------------------------------------------
CONCRETE CHECKLIST
-------------------------------------------------------------

1) Update project identity docs
- Files:
  - BREAKDOWN.md
  - BASELINE.md
  - DEPENDENCY_AUDIT.md
- Actions:
  - Replace “council blueprint stub” language with the real module surface
    (prometheus/, ceo/, cli/, telemetry/, export_surface/).
  - Record explicit decision: these observability surfaces remain in arda-aule.
  - Reset BASELINE to current cargo check / cargo test output.
- Verification:
  - grep -n "council blueprint" BREAKDOWN.md BASELINE.md DEPENDENCY_AUDIT.md
  - Expect: 0 matches
  - cargo check -p arda-aule

2) Make lib.rs match observability role
- File:
  - src/lib.rs
- Actions:
  - Replace council-blueprint doc header with observability-home wording.
  - If public doc example still council-flavored, replace or remove it.
  - Keep crate examples in a state that compiles.
- Verification:
  - cargo check -p arda-aule
  - cargo test -p arda-aule

3) Replace live annunimas_* imports with Arda paths
Dependencies already declared in Cargo.toml:
  arda-core, arda-governance, arda-orome, arda-vaire,
  arda-mandos, arda-varda, arda-economics

Requires confirming equivalent public paths in those crates before patching.
Suggested mapping based on current grep:
  annunimas_core::error::Result                -> arda-core error
  annunimas_core::error::AnnunimasError         -> arda-core error
  annunimas_core::router::Router                -> arda-core router
  annunimas_core::ledger::Ledger                -> arda-core ledger
  annunimas_core::task::Task                    -> arda-core task
  annunimas_core::message::Message              -> arda-core message
  annunimas_core::agent::Agent                  -> arda-core agent
  annunimas_core::contract::*                   -> arda-core contract
  annunimas_core::state::*                      -> arda-core state
  annunimas_core::spawn_bounded_background      -> arda-core
  annunimas_core::try_run_bounded_async         -> arda-core
  annunimas_core::daemon::{Command...}          -> arda-core daemon
  annunimas_hermes::HermesService               -> arda-orome
  annunimas_mnemosyne::MnemosyneService         -> arda-vaire
  annunimas_governance::enqueue_bacon_lite      -> arda-governance
  annunimas_fleet::{...}                        -> needs explicit rust path decision

Target files (from current grep):
  src/prometheus/transport/mod.rs
  src/prometheus/transport/ipc.rs
  src/prometheus/transport/http.rs
  src/prometheus/thought.rs
  src/prometheus/error.rs
  src/prometheus/service/support.rs
  src/prometheus/service/status.rs
  src/prometheus/service/runtime.rs
  src/prometheus/service/execution_intents.rs
  src/prometheus/service/drift.rs
  src/prometheus/service.rs
  src/prometheus/router.rs
  src/prometheus/planner.rs
  src/prometheus/pipeline.rs
  src/prometheus/pipeline/*.rs
  src/prometheus/core_link/*.rs
  src/prometheus/orders.rs
  src/ceo/*.rs
  src/cli/*

- Verification:
  - rg -n "annunimas_" crates/spine/observability/arda-aule/src
  - Expect: 0 matches after migration

4) Replace string/path/name residue
Target residues to replace:
  ANNUNIMAS_ROOT
  annunimas_root()
  /tmp/annunimas-target
  "annunimas_totality"
  cargo run -p annunimas-cli
  annunimas-cli-*
  crates/annunimas-...
  annunimas_prometheus
  annunimas-ceo-autopilot-supervised.service

Target files:
  src/prometheus/service/support.rs
  src/orders.rs
  src/prometheus/core_link/*.rs
  src/cli/*
  src/prometheus/transport/http.rs
  tests/* if present

- Verification:
  - rg -n "annunimas" crates/spine/observability/arda-aule
  - Expect: 0 matches (or only historical prose in docs)

5) Retire council duplication in arda-aule
- Decision gate first:
  - Confirmed no `arda-council` crate exists in the workspace.
- Actions:
  - Convert `src/council.rs` from duplicated council types to a thin re-export shim over `prometheus/council.rs`.
  - Keep `prometheus/council.rs` as the single implementation to avoid removing functionality from staged prometheus daemon boat surface.
  - Note: `arda-council` does not exist yet; avoid adding a dependency on it until one is created.
- Target files:
  - src/council.rs
  - src/prometheus/council.rs
  - src/prometheus/mod.rs
  - src/prometheus/lib.rs
- Verification:
  - rg -n "council" crates/spine/observability/arda-aule/src
  - rg -rn "arda_aule::council|council::" crates/ | grep -v "arda-aule/src/council.rs|arda-aule/tests/"
  - Confirm `tests/contract_smoke.rs` passes `cargo check --test contract_smoke -p arda-aule` end-to-end

6) Reduce or gate dead/aspirational CLI commands
- Actions:
  - Introduce `cli-default` for minimal/default shell and `cli-full` for the active command surface.
  - Gate `src/cli/mod.rs` and `src/cli/commands/mod.rs` behind `full-cli` so aspirational modules do not load by default.
  - Removed duplicate `src/cli/commands/charon.rs`; `manwe` telemetry path is served by `charon_telemetry.rs`.
- Target files:
  - Cargo.toml
  - src/cli/mod.rs
  - src/cli/commands/mod.rs
  - src/cli/commands/charon.rs
- Verification:
  - `cargo check -p arda-aule --features cli-default`
  - `cargo check -p arda-aule --features full-cli`

7) Add observability verification coverage
- Actions:
  - Add default-feature smoke test proving `arda_aule::council::` resolves without cross-crate assumptions.
  - Add `full-cli` smoke test exercising council gate runtime/live path, gated separately.
- Target files:
  - tests/council_surface.rs
  - tests/council_surface_full.rs
- Verification:
  - cargo test --test council_surface -p arda-aule
  - cargo test --features full-cli --test council_surface_full -p arda-aule
- Current evidence:
  - Default-feature council_surface: PASSES
  - Full-cli council_surface_full: BLOCKED by pre-existing aspirational CLI stubs in policy_guard/commands; not step 7 coverage debt

8) Re-baseline arc
- Commands to run after each major step:
  - cargo check -p arda-aule
  - cargo test  -p arda-aule
  - cargo check -p arda-aule --features full-cli
  - cargo test  -p arda-aule --features full-cli
- Doc updates required at end:
  - BASELINE.md   : current compile/test baseline
  - DEPENDENCY_AUDIT.md : resolved/deferred with files and decisions
  - BREAKDOWN.md  : current state, decision records, last_reviewed

-------------------------------------------------------------
EXECUTION ORDER
-------------------------------------------------------------
Recommended strict order:
  step 1 -> step 3 -> step 4 -> step 5 -> step 6 -> step 7 -> step 8
Alternative shortcut:
  step 2 can be done either early or after structural cleanup.

-------------------------------------------------------------
RISKS
-------------------------------------------------------------
- Some annunimas_fleet types may not have direct arda equivalents yet.
  If a live mapping is missing, record the exact type and defer with a note
  in DEPENDENCY_AUDIT.md instead of fabricating a path.
- If a direct consumer imports arda-aule council surface externally,
  step 5 must pend on that migration or become a re-export gate only.
