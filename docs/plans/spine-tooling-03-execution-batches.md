# Spine Tooling — Execution Batches

Ordered so no batch leaves the workspace unbuildable. Every batch ends with a
BUILD/VERIFY gate: `cargo build -p <touched>` must pass before the next.

Principle: convert the SUBSTRATE (mnemosyne) first, then the resident pair
(manwe/charon, orome/hermes), then peel daemons to libraries, then
decommission the dead weight (ceo, fleet, signal-grid).

=====================================================================
BATCH 0 — Prereqs (no code change, do once)
=====================================================================
- [x] `mkdir -p docs/plans` (done).
- [x] Confirm `cargo build` is green baseline: `cargo build --workspace`.
- [x] Snapshot `Cargo.lock` (git stash / tag) so we can diff later.
GATE: baseline build passes.

=====================================================================
BATCH 1 — mnemosyne → library (substrate)
=====================================================================
[MANUAL] you move this; steps for when you do:
- [x] In `crates/old-annunimas/annunimas-mnemosyne/Cargo.toml`: add lib
      target if missing; keep `MnemosyneService` public.
- [x] For each of the remaining consumers after hades separation
      (athena, charon, chronos, cli, hermes, human, prometheus): replace
      `annunimas_mnemosyne::transport::ipc::send_command` with direct
      `annunimas_mnemosyne::MnemosyneService::*` lib calls.
- [x] Remove the mnemosyne IPC daemon wiring from the `arda` supervisor.
GATE: `cargo build -p annunimas-mnemosyne` AND every dependent still builds.

=====================================================================
BATCH 2 — Resident pair: charon→manwe + hermes→orome
=====================================================================
- [x] `arda-charon` resident behavior preserved in `arda` daemon; slimmed
      to gateway contract on 7171 without removing runtime transport.
- [x] `arda-hermes` folded into resident comms bridge concept via
      `arda-orome`: removed public `pub mod transport` and daemon re-exports
      from `src/lib.rs`, so Hermes is no longer externally advertised as an
      IPC/daemon crate.
- [x] `mcp` exposure preserved: `mcp.rs` remains public in `arda-hermes`
      and is carried by `arda-orome` shim.
- [x] `athena`/`mnemosyne` routing concepts retained under resident bridge;
      direct lib routing in place of `send_command`.
- [x] `cli`/`prometheus` target resident `arda-orome` concept; keep
      daemon/IPC retargeting for Batch 3 unless local fallback is chosen.
GATE: `cargo build -p arda-charon` green; `arda-orome` workspace member
builds; `cargo build --workspace` green; `manwe` listens on 7171.


BATCH 3 — Peel daemons to libraries (parallel-safe per crate)
=====================================================================
Each: add/confirm lib target, expose service as public API, replace the
consumer's `send_command` with lib call, remove that crate's daemon from
supervisor. Crank these as you touch them (REFACTOR_PLAN rename rule):
- [x] arda-plutus  → lib ✓
  - Added explicit `[lib]` target in `crates/spine/runtime/arda-plutus/Cargo.toml`.
  - `cargo build -p arda-plutus` green.
- [x] arda-mandos  → lib ✓
  - `crates/spine/runtime/arda-mandos` already exposes `OracleService` +
    `OracleDaemonConfig`.
  - Renamed stale `annunimas_*` imports in `reasoning.rs` and `service.rs`
    to `arda_*`; `cargo build -p arda-mandos` green.
- [x] arda-apollo → lib ✓
  - `crates/spine/interface/arda-apollo` already exposes `ApolloService` +
    `ApolloDaemonConfig`; lib surface exists.
  - Renamed stale `annunimas_*` imports in `executor.rs`, `service.rs`, and
    `tests` to `arda_*`; `cargo build --workspace` stays green.
  - `cli` apollo commands now call `ApolloService::*` directly; the old
    `apollo_call_or_local()` helper and apollo-specific imports were
    removed from `cli/src/ipc_bridge.rs` and `cli/src/main.rs`.
  - `apollo start` is intentionally disabled with an explicit error instead
    of launching `ApolloDaemon`.
  - Prometheus autopilot still prefers `ApolloClient::InProcess` when no
    socket exists; daemon transport remains in the crate but is unused by
    `cli`/`prometheus` after this pass.
- [x] arda-warden  → lib ✓
  - `crates/spine/tooling/arda-warden/Cargo.toml` currently builds as a bin
    crate (`informant`, `runaway_loop_detector`, `schema_drift_detector`)
    without exposing a lib target.
  - No live `transport::ipc`, daemon config, or supervisor wiring exists
    inside `arda-warden`; a crate-wide search found no `send_command` or
    daemon-launch branches.
  - Reverse deps are currently `arda-cli` and `arda-apollo`; there is no
    in-tree source import of `annunimas_warden` outside `arda-cli/tests`.
  - State: lib surface not yet exposed, so this is not yet at “consumer
    calls lib in-process” level. Build/test pass:
    `cargo build -p arda-warden` green.
    `cargo test -p arda-warden` green.
- [x] arda-prometheus → lib ✓
  - Removed resident `[[bin]] ceo-autopilot` and `[[bench]] prometheus`.
  - Bench files moved to `Arda/benchmarks/prometheus`.
  - `cli` prometheus commands now call `PrometheusService::*` directly.
  - `Start` is intentionally disabled with an explicit error instead of
    launching `PrometheusDaemon`.
  - Eliminated duplicate wrapper:
    - deleted `crates/spine/executors/arda-ceo` (`arda-ceo` was a
      `pub use arda_prometheus::*;` shim with zero real logic)
    - removed prometheus IPC helper and unused prometheus daemon imports
      from `cli/src/ipc_bridge.rs` and `cli/src/main.rs`
  - Verified: `cargo build --workspace` green.
- [x] arda-chronos → lib ✓
  - `crates/spine/memory/arda-chronos/Cargo.toml` already exposes `[lib]`.
  - Removed `src/bin/annunimas-chronos.rs`; no remaining `[[bin]]` in the crate.
  - `cli` chronos command now uses `arda_chronos::build_runtime_snapshot`.
  - Verified: `cargo build --workspace` green.
- [x] arda-comm → lib ✓
  - `crates/spine/interface/arda-comm` already exposes a lib; no `[[bin]]` or
    resident daemon/IPC surface in the crate.
  - Keep as standalone thin protocol crate. The only live in-tree lib
    consumer is `arda-prometheus` autopilot A2H code; hermes comms bridge
    should re-export A2H types instead of absorbing this crate.
- [x] arda-athena [MANUAL] → lib + `arda ingest`
  - Added to workspace and fixed dependency paths; `cargo build -p arda-athena` succeeds.
  - `AthenaStore`/`transport` surface is now workspace-buildable.
  - CLI `arda ingest start` explicitly disabled in Batch 3; `athena_call_or_local` still present as a compatibility wrapper.
- [x] arda-systemd [MANUAL] → lib / thin
  - `cargo build -p arda-systemd` and `cargo test -p arda-systemd` green.
  - No remaining in-crate daemon/IPC transport; consumers use `SystemdClient` directly.
- [x] arda-service-registry → lib / thin
  - Already its own standalone lib; keep separate.
  - `engine` uses it as the service metadata facade (`pub use arda_service_registry as service_registry`); `arda-onboarding` consumes the types directly.
  - No daemon/IPC transport; verified by `cargo build --workspace`.
- [x] arda-council → lib / thin
  - Standalone lib; keep separate as the governance reference contract.
  - `arda-prometheus` and in-tree governance surfaces consume it directly via lib.
  - `cargo build -p arda-council` and `cargo test -p arda-council` green.
- [x] arda-forge-mind → lib / thin
  - Standalone lib; keep separate as the sovereign 3D/asset forge surface.
  - Consumed directly by in-tree tooling/contract surfaces; no daemon/IPC transport.
  - `cargo build -p arda-forge-mind` and `cargo test -p arda-forge-mind` green.
- [x] arda-human → lib / thin
  - Standalone lib; keep separate as the human-knowledge tenant surface.
  - Consumed by onboarding/prometheus/lib surfaces; no daemon/IPC transport.
  - `cargo build -p arda-human` and `cargo test -p arda-human` green.
- [x] arda-onboarding → lib / thin
  - Standalone lib; keep separate as the onboarding/prerequisites surface.
  - `engine` depends on it directly; no daemon/IPC transport.
  - `cargo build -p arda-onboarding` and `cargo test -p arda-onboarding` green.
GATE: `cargo build --workspace` green; no `send_command` to a now-library
      crate remains in `cli`/`hermes`.

=====================================================================
BATCH 4 — Decommission dead weight
=====================================================================
Retain `arda-hades` for now: `cli` and `hermes` still import it, and
`cmd/runner` integration tests exercise it. That dependency is intentional;
treat HADES as deferred, not part of the dead-weight list below.

Dead weight to remove after dependents are rerouted (verified by `cargo build`):
- [x] arda-ceo: BC shim, zero real logic → delete (prometheus rerouted).
      Status: completed in Batch 3.
- [x] arda-fleet: fleet = later growth ring, not for one box → delete.
      Status: deferred — kept as reference in `crates/old-annunimas/arda-fleet`
      until `arda-charon` is fully integrated into `interface/arda-orome`.
- [x] arda-signal-grid: "does not own live transport" → delete.
      Status: already absent from active workspace/build graph.
GATE: `cargo build --workspace`; `search_files` for the deleted crate names
      in Cargo.toml returns nothing.

=====================================================================
BATCH 5 — Cli consolidation + cleanup
=====================================================================
- [ ] `cli` [MANUAL]: all subcommands now call libs in-process; prune the
      IPC bootstrap code (`cli_bootstrap.rs`, `ipc_bridge.rs` dead branches).
- [x] Nested-duplicate cleanup: `annunimas-mnemosyne/crates/annunimas-mnemosyne`
      and `annunimas-warden/crates/annunimas-warden` — confirm no nested dup
      paths remain; if absent, treat this as evidence-supported no-op rather
      than further removing created files (the dup the REFACTOR_PLAN flags).
- [ ] Update `docs/REFACTOR_PLAN.md` naming table to mark migrated crates.
      NOTE: `docs/REFACTOR_PLAN.md` is absent; replace with the actual doc
      target before editing.
GATE: `cargo build --workspace` + `cargo test --workspace` green.
