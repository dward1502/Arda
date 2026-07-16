# Spine Tooling Migration — Index

Status: DRAFT v1 · Owner: dward · Created: 2026-07-13
Scope: replace heavy always-on `annunimas-*` daemons (athena, memory/mnemosyne, hades,
cli, hermes, charon, ...) with robust on-demand tooling.

This plan lives under `docs/plans/` in the canonical Arda root
(`/var/home/mythos/Eregion/Arda`). All 26 vendored annunimas crates currently
reside under `crates/spine/<layer>/annunimas-*`. The reverse-dependency map in
`spine-tooling-01-disposition-matrix.md` was computed directly from each crate's
`Cargo.toml` and is the binding constraint for any decommission.

## How to read these files
- `00-index.md` — this file: intent, principles, what you handle manually.
- `01-disposition-matrix.md` — every spine crate, its dependents, and a
  KEEP / SLIM / MERGE / DECOMMISSION disposition with an on-demand target.
- `02-focus-deep-dives.md` — the five crates you named (athena, mnemosyne,
  hades, cli, hermes) plus charon, with reroute plans for their dependents.
- `03-execution-batches.md` — ordered batches, each with a build/verify gate.

## Core principle: on-demand over always-on
The current model is a fleet of long-lived IPC daemons (athena, hades, hermes,
charon, mnemosyne, plutus, oracle, prometheus, apollo). `annunimas-cli` is the
orchestrator that `send_command`s to each daemon over IPC. For a single local
machine this is the bloat the REFACTOR_PLAN already flags. The replacement model:

- A capability is a *library function / small binary*, invoked when needed, not a
  daemon that must be supervised 24/7.
- State that currently lives in a daemon's memory moves to the shared store
  (mnemosyne / filesystem / the `arda` daemon's own state).
- The `arda` daemon (single entry point per REFACTOR_PLAN) hosts only what must
  be resident: the gateway (`manwe`, ex-charon) and the comms bridge (`orome`,
  ex-hermes). Everything else becomes callable tooling.

## Sections you said you will handle manually
You are moving items and removing crates by hand. For those, this plan gives the
*verified* disposition + the exact dependents that must be rerouted first, but
does NOT script the deletion. The "manual" crates and their reroute prerequisites
are tagged `[MANUAL]` in the matrix and deep-dives:

- `annunimas-athena` (executors) — reroute `cli` + `hermes` first.
- `annunimas-hades` (runtime) — reroute `cli` + `hermes` first.
- `annunimas-mnemosyne` (memory) — 8 dependents; this is the substrate, so it
  is converted to a library + thin on-demand store BEFORE its dependents change.
- `annunimas-cli` (interface) — becomes the on-demand `arda` CLI surface.
- `annunimas-hermes` (interface) — folds into `orome` comms bridge.

Everything else (apollo, charon→manwe, plutus, oracle, prometheus, etc.) has a
disposition in the matrix; the batches in `03-execution-batches.md` sequence the
work but you may execute them by hand.

## Confirmed architecture (verified 2026-07-13)

Process/library layering in the Arda root:
  1. `src/main.rs` (bin, ~155 lines) — thin shell: arg parsing, logging,
     ctrl-c, glue. NO domain logic.
  2. `crates/engine` (lib — `lib.rs` + `harness.rs` + `registry.rs` +
     `supervisor.rs`) — boot, service registry, supervisor, harness tap-in.
     This is where all daemon logic lives; it is unit-testable
     (`cargo test -p arda-engine`) precisely because it is NOT in `main.rs`.
  3. `crates/spine/*` (vendored Annunimas systems) — the substrate.
  4. `apps/arda-launcher` + `apps/arda-hud` — SEPARATE Tauri apps, each its
     own process + Rust backend (`apps/*/src-tauri/src/lib.rs` with
     `tauri::Builder` + `invoke_handler`). They are spawned as child processes
     by the daemon via `services.toml`, NOT compiled into `arda`.

`src/` convention: binary-only, no logic. The empty `boot.rs`/`router.rs`/
`supervisor.rs` stubs were deleted (unreferenced orphans) — `src/` now holds
only `main.rs`.

### Who owns what (daemon vs. app vs. engine)
- `arda` daemon (`src/main.rs` + engine) = process lifecycle ONLY. Spawns
  launcher → observes "seeded" marker → HUD renders. Does NOT broker data.
- `arda-launcher` (Tauri app) = the startup brain: recursive system/inference
  search, first-run config/onboarding, startup-tool suite. This lives in the
  launcher's own Rust backend, NOT in `src/main.rs` (confirmed: launcher's
  logic is app-end code; putting it in the daemon would break testability and
  the thin-shell model).
- `arda-hud` (Tauri app) = dumb consumer: reads seeded state, renders.
- Capabilities the launcher needs (e.g. first-run config) are provided as libs
  the app calls — `annunimas-onboarding` is already an `arda-engine` dep
  (`crates/spine/interface/annunimas-onboarding`); the launcher orchestrates
  UX, the engine provides the capability. No logic duplication.
- Seeding handoff = shared-store / gateway concern, not the daemon relaying
  every message. HUD polls the store (or manwe gateway, 7171) once seeded.

### Sequencing rule
Daemon spawns launcher, waits for its "done/seeded" marker, then the HUD
(already spawned or spawned-then) reads state. The launcher must NOT spawn the
HUD itself — that would break the single-supervisor model and the daemon loses
visibility into lifecycle.
